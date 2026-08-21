mod auth;
mod environments;
mod error;

use std::{collections::HashMap, net::SocketAddr, sync::Arc};

use dockrs_docker::DockerSnapshotCollector;
use sqlx::sqlite::SqlitePool;
use time::Duration;
use tokio::sync::RwLock;
use tower_sessions::{ExpiredDeletion, Expiry, SessionManagerLayer, cookie::SameSite};
use tower_sessions_sqlx_store::SqliteStore as SessionStore;
use utoipa::OpenApi;

#[derive(Clone)]
pub(crate) struct AppState {
    pub(crate) pool: SqlitePool,
    pub(crate) local_docker: Option<DockerSnapshotCollector>,
    pub(crate) agent_token: Option<String>,
    pub(crate) agent_snapshots: Arc<RwLock<HashMap<String, environments::CachedAgentSnapshot>>>,
}

#[derive(OpenApi)]
#[openapi(
    paths(
        auth::login,
        auth::logout,
        auth::change_password,
        auth::me,
        environments::list_environments,
        environments::update_environment_name,
        environments::register_agent,
        environments::agent_heartbeat,
        environments::agent_snapshot
    ),
    components(schemas(
        auth::LoginRequest,
        auth::ChangePasswordRequest,
        auth::UserResponse,
        environments::EnvironmentResponse,
        environments::EnvironmentNameResponse,
        environments::RenameEnvironmentRequest,
        environments::AgentRegistrationRequest,
        environments::AgentRegistrationResponse,
        environments::AgentSnapshotRequest,
        environments::ContainerCountsResponse,
        environments::MemoryMetricsResponse,
        environments::EnvironmentKind,
        environments::EnvironmentStatus
    ))
)]
struct ApiDoc;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::builder()
                .with_default_directive(tracing_subscriber::filter::LevelFilter::INFO.into())
                .from_env_lossy(),
        )
        .init();

    let db_url = std::env::var("DOCKRS_DATABASE_URL")
        .unwrap_or_else(|_| "sqlite:./dockrs.db?mode=rwc".into());
    let pool = SqlitePool::connect(&db_url).await?;
    sqlx::migrate!("./migrations").run(&pool).await?;

    let (docker_host, docker_endpoint) = local_docker_host();
    environments::ensure_local(&pool, &docker_endpoint).await?;
    let local_docker = match DockerSnapshotCollector::connect(&docker_host) {
        Ok(collector) => Some(collector),
        Err(error) => {
            tracing::warn!(error = %error, "初始化本地 Docker 客户端失败");
            None
        }
    };

    let session_store = SessionStore::new(pool.clone());
    session_store.migrate().await?;

    auth::seed_admin(&pool).await?;

    // ponytail: 后台定期清理过期 session；规模增长后可改为按需清理或调整周期
    tokio::spawn(
        session_store
            .clone()
            .continuously_delete_expired(std::time::Duration::from_secs(60)),
    );

    // secure 默认 false 以兼容 HTTP 本地部署；生产经反向代理 TLS 时设 DOCKRS_COOKIE_SECURE=1
    let session_layer = SessionManagerLayer::new(session_store)
        .with_secure(
            std::env::var("DOCKRS_COOKIE_SECURE")
                .map(|v| v == "1" || v.eq_ignore_ascii_case("true"))
                .unwrap_or(false),
        )
        .with_same_site(SameSite::Lax)
        .with_http_only(true)
        .with_expiry(Expiry::OnInactivity(Duration::days(7)));

    let state = AppState {
        pool,
        local_docker,
        agent_token: std::env::var("DOCKRS_AGENT_TOKEN").ok(),
        agent_snapshots: Arc::new(RwLock::new(HashMap::new())),
    };

    let app = axum::Router::new()
        .nest("/api", auth::router().merge(environments::router()))
        .route("/api/openapi.json", axum::routing::get(openapi))
        .method_not_allowed_fallback(error::method_not_allowed)
        .fallback(error::not_found)
        .layer(session_layer)
        .with_state(state);

    let listen = std::env::var("DOCKRS_LISTEN").unwrap_or_else(|_| "0.0.0.0:8080".into());
    let addr: SocketAddr = listen.parse()?;
    let listener = tokio::net::TcpListener::bind(addr).await?;
    tracing::info!(addr = %addr, "dockrs server 已启动");

    axum::serve(listener, app).await?;
    Ok(())
}

async fn openapi() -> axum::Json<utoipa::openapi::OpenApi> {
    axum::Json(ApiDoc::openapi())
}

fn local_docker_host() -> (String, String) {
    let raw = std::env::var("DOCKRS_DOCKER_HOST")
        .or_else(|_| std::env::var("DOCKER_HOST"))
        .unwrap_or_else(|_| "unix:///var/run/docker.sock".to_owned());
    let host = if raw.contains("://") {
        raw
    } else {
        format!("unix://{raw}")
    };
    let endpoint = host.strip_prefix("unix://").unwrap_or(&host).to_owned();
    (host, endpoint)
}
