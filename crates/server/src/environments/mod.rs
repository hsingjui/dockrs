use std::{
    collections::HashMap,
    time::{Duration, Instant},
};

use axum::{
    Router,
    extract::{Path, State},
    routing::{get, post},
};
use dockrs_docker::{
    ContainerCounts as DockerContainerCounts, DockerSnapshot, MemoryMetrics as DockerMemoryMetrics,
};
use serde::{Deserialize, Serialize};
use sqlx::SqlitePool;
use tokio::time::timeout;
use tower_sessions::Session;
use utoipa::ToSchema;

pub(crate) mod agent_registry;

pub(crate) use agent_registry::{
    agent_heartbeat, agent_snapshot, register_agent, register_connected_agent, require_agent_token,
    store_agent_snapshot, touch_agent,
};

use crate::{
    AppState,
    auth::require_user,
    error::{ApiError, ApiJson, ApiResponse},
};

const AGENT_ONLINE_WINDOW_SECS: i64 = 90;
const AGENT_SNAPSHOT_TTL: Duration = Duration::from_secs(30);
const DOCKER_REQUEST_TIMEOUT: Duration = Duration::from_secs(5);

#[derive(Clone)]
pub(crate) struct CachedAgentSnapshot {
    pub(crate) snapshot: DockerSnapshot,
    pub(crate) received_at: Instant,
}

#[derive(Debug, sqlx::FromRow)]
struct EnvironmentRow {
    id: String,
    name: String,
    kind: String,
    endpoint: String,
    last_seen_at: Option<String>,
    is_online: i64,
}

#[derive(Clone, Copy, Debug, Deserialize, Serialize, ToSchema)]
#[serde(rename_all = "lowercase")]
pub enum EnvironmentKind {
    Local,
    Agent,
}

#[derive(Clone, Copy, Debug, Deserialize, Serialize, ToSchema)]
#[serde(rename_all = "lowercase")]
pub enum EnvironmentStatus {
    Online,
    Offline,
}

#[derive(Clone, Debug, Deserialize, Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct ContainerCountsResponse {
    pub total: u64,
    pub running: u64,
    pub paused: u64,
    pub stopped: u64,
}

#[derive(Clone, Debug, Deserialize, Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct MemoryMetricsResponse {
    pub used_bytes: u64,
    pub total_bytes: u64,
    pub percent: f64,
}

#[derive(Clone, Debug, Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct EnvironmentResponse {
    pub id: String,
    pub name: String,
    pub kind: EnvironmentKind,
    pub status: EnvironmentStatus,
    pub endpoint: String,
    pub docker_version: Option<String>,
    pub containers: ContainerCountsResponse,
    pub cpu_percent: Option<f64>,
    pub memory: Option<MemoryMetricsResponse>,
    pub metrics_collected_at: Option<i64>,
    pub metrics_error: Option<String>,
    pub offline_reason: Option<String>,
    pub last_seen_at: Option<String>,
}

#[derive(Debug, Deserialize, ToSchema)]
pub struct RenameEnvironmentRequest {
    pub name: String,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct EnvironmentNameResponse {
    pub id: String,
    pub name: String,
}

#[derive(Debug, Deserialize, ToSchema)]
pub struct AgentRegistrationRequest {
    pub id: String,
    pub name: String,
    pub endpoint: String,
    pub agent_id: Option<String>,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct AgentRegistrationResponse {
    pub id: String,
}

/// Agent 通过既有通信链路上报的 Docker 汇总快照；该数据只进入进程内缓存。
#[derive(Debug, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct AgentSnapshotRequest {
    pub docker_version: String,
    pub containers: ContainerCountsResponse,
    pub cpu_percent: Option<f64>,
    pub memory: Option<MemoryMetricsResponse>,
    pub metrics_error: Option<String>,
    pub metrics_collected_at: i64,
}

impl From<AgentSnapshotRequest> for DockerSnapshot {
    fn from(value: AgentSnapshotRequest) -> Self {
        Self {
            docker_version: value.docker_version,
            containers: DockerContainerCounts {
                total: value.containers.total,
                running: value.containers.running,
                paused: value.containers.paused,
                stopped: value.containers.stopped,
            },
            cpu_percent: value.cpu_percent,
            memory: value.memory.map(|memory| DockerMemoryMetrics {
                used_bytes: memory.used_bytes,
                total_bytes: memory.total_bytes,
                percent: memory.percent,
            }),
            metrics_error: value.metrics_error,
            metrics_collected_at: value.metrics_collected_at,
        }
    }
}

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/environments", get(list_environments))
        .route(
            "/environments/{id}",
            get(get_environment)
                .patch(update_environment_name)
                .delete(delete_environment),
        )
        .route("/environments/agents/register", post(register_agent))
        .route("/environments/agents/{id}/heartbeat", post(agent_heartbeat))
        .route("/environments/agents/{id}/snapshot", post(agent_snapshot))
}

pub async fn ensure_local(pool: &SqlitePool, endpoint: &str) -> sqlx::Result<()> {
    sqlx::query(
        "INSERT INTO environments (id, name, kind, endpoint)
         VALUES ('local', 'local-engine', 'local', ?)
         ON CONFLICT(id) DO UPDATE SET endpoint = excluded.endpoint",
    )
    .bind(endpoint)
    .execute(pool)
    .await?;
    Ok(())
}

#[utoipa::path(
    get,
    path = "/api/environments",
    responses(
        (status = 200, body = [EnvironmentResponse], description = "环境列表"),
        (status = 401, description = "未登录")
    )
)]
pub async fn list_environments(
    State(state): State<AppState>,
    session: Session,
) -> Result<ApiResponse<Vec<EnvironmentResponse>>, ApiError> {
    require_user(&session).await?;

    let rows = sqlx::query_as::<_, EnvironmentRow>(
        "SELECT id, name, kind, endpoint, last_seen_at,
                CASE
                    WHEN kind = 'agent'
                        AND last_seen_at IS NOT NULL
                        AND datetime(last_seen_at) >= datetime('now', ?)
                    THEN 1
                    ELSE 0
                END AS is_online
         FROM environments
         ORDER BY CASE WHEN id = 'local' THEN 0 ELSE 1 END, name COLLATE NOCASE",
    )
    .bind(format!("-{AGENT_ONLINE_WINDOW_SECS} seconds"))
    .fetch_all(&state.pool)
    .await
    .map_err(|error| {
        tracing::error!(error = %error, "查询环境失败");
        ApiError::internal("获取环境列表失败，请稍后重试")
    })?;

    let (local_snapshot, agent_snapshots) = runtime_snapshots(&state).await;

    let mut environments = Vec::with_capacity(rows.len());
    for row in rows {
        environments.push(response_for_row(
            &row,
            local_snapshot.as_ref(),
            &agent_snapshots,
        )?);
    }

    Ok(ApiResponse::success(environments))
}

#[utoipa::path(
    get,
    path = "/api/environments/{id}",
    params(("id" = String, Path, description = "环境 ID")),
    responses(
        (status = 200, body = EnvironmentResponse, description = "环境详情"),
        (status = 401, description = "未登录"),
        (status = 404, description = "环境不存在")
    )
)]
pub(crate) async fn get_environment(
    State(state): State<AppState>,
    Path(id): Path<String>,
    session: Session,
) -> Result<ApiResponse<EnvironmentResponse>, ApiError> {
    require_user(&session).await?;
    Ok(ApiResponse::success(
        environment_response_by_id(&state, &id).await?,
    ))
}

pub(crate) async fn environment_response_by_id(
    state: &AppState,
    id: &str,
) -> Result<EnvironmentResponse, ApiError> {
    let row = sqlx::query_as::<_, EnvironmentRow>(
        "SELECT id, name, kind, endpoint, last_seen_at,
                CASE
                    WHEN kind = 'agent'
                        AND last_seen_at IS NOT NULL
                        AND datetime(last_seen_at) >= datetime('now', ?)
                    THEN 1
                    ELSE 0
                END AS is_online
         FROM environments
         WHERE id = ?",
    )
    .bind(format!("-{AGENT_ONLINE_WINDOW_SECS} seconds"))
    .bind(id)
    .fetch_optional(&state.pool)
    .await
    .map_err(|error| {
        tracing::error!(environment_id = %id, error = %error, "查询环境详情失败");
        ApiError::internal("获取环境详情失败，请稍后重试")
    })?
    .ok_or_else(|| ApiError::not_found("环境不存在"))?;

    let (local_snapshot, agent_snapshots) = runtime_snapshots(state).await;
    response_for_row(&row, local_snapshot.as_ref(), &agent_snapshots)
}

async fn runtime_snapshots(
    state: &AppState,
) -> (Option<DockerSnapshot>, HashMap<String, CachedAgentSnapshot>) {
    let local_snapshot = match state.local_docker.as_ref() {
        Some(collector) => match timeout(DOCKER_REQUEST_TIMEOUT, collector.snapshot()).await {
            Ok(Ok(snapshot)) => Some(snapshot),
            Ok(Err(error)) => {
                tracing::warn!(error = %error, "本地 Docker 不可用");
                None
            }
            Err(_) => {
                tracing::warn!("本地 Docker 请求超时");
                None
            }
        },
        None => None,
    };

    let agent_snapshots = {
        let mut snapshots = state.agent_snapshots.write().await;
        snapshots.retain(|_, cached| cached.received_at.elapsed() <= AGENT_SNAPSHOT_TTL);
        snapshots.clone()
    };

    (local_snapshot, agent_snapshots)
}

fn response_for_row(
    row: &EnvironmentRow,
    local_snapshot: Option<&DockerSnapshot>,
    agent_snapshots: &HashMap<String, CachedAgentSnapshot>,
) -> Result<EnvironmentResponse, ApiError> {
    let kind = parse_kind(&row.kind).ok_or_else(|| {
        tracing::error!(environment_id = %row.id, kind = %row.kind, "环境类型无效");
        ApiError::internal("环境数据无效")
    })?;

    let response = match kind {
        EnvironmentKind::Local => match local_snapshot {
            Some(snapshot) => {
                response_from_snapshot(row, EnvironmentStatus::Online, snapshot, None)
            }
            None => unavailable_response(
                row,
                EnvironmentStatus::Offline,
                "无法连接 Docker Engine，请检查 Docker Socket",
            ),
        },
        EnvironmentKind::Agent => {
            if row.is_online == 0 {
                unavailable_response(
                    row,
                    EnvironmentStatus::Offline,
                    "Agent 心跳超时，节点当前离线",
                )
            } else if let Some(cached) = agent_snapshots.get(&row.id) {
                response_from_snapshot(row, EnvironmentStatus::Online, &cached.snapshot, None)
            } else {
                let mut response = unavailable_response(
                    row,
                    EnvironmentStatus::Online,
                    "等待 Agent 上报 Docker 指标",
                );
                response.offline_reason = None;
                response
            }
        }
    };
    Ok(response)
}

#[utoipa::path(
    patch,
    path = "/api/environments/{id}",
    params(("id" = String, Path, description = "环境 ID")),
    request_body = RenameEnvironmentRequest,
    responses(
        (status = 200, body = EnvironmentNameResponse, description = "名称更新成功"),
        (status = 400, description = "名称无效或环境不支持修改"),
        (status = 401, description = "未登录"),
        (status = 404, description = "环境不存在")
    )
)]
pub async fn update_environment_name(
    State(state): State<AppState>,
    Path(id): Path<String>,
    session: Session,
    ApiJson(request): ApiJson<RenameEnvironmentRequest>,
) -> Result<ApiResponse<EnvironmentNameResponse>, ApiError> {
    require_user(&session).await?;
    if id != "local" {
        return Err(ApiError::bad_request("当前仅支持修改本地环境名称"));
    }

    let name = validate_name(request.name)?;
    let result = sqlx::query(
        "UPDATE environments
         SET name = ?, updated_at = datetime('now')
         WHERE id = 'local' AND kind = 'local'",
    )
    .bind(&name)
    .execute(&state.pool)
    .await
    .map_err(|error| {
        tracing::error!(error = %error, "更新环境名称失败");
        ApiError::internal("更新环境名称失败，请稍后重试")
    })?;

    if result.rows_affected() == 0 {
        return Err(ApiError::not_found("本地环境不存在"));
    }

    Ok(ApiResponse::success(EnvironmentNameResponse { id, name }))
}

#[utoipa::path(
    delete,
    path = "/api/environments/{id}",
    params(("id" = String, Path, description = "环境 ID")),
    responses(
        (status = 200, description = "删除成功"),
        (status = 400, description = "本地环境不支持删除"),
        (status = 401, description = "未登录"),
        (status = 404, description = "环境不存在")
    )
)]
pub async fn delete_environment(
    State(state): State<AppState>,
    Path(id): Path<String>,
    session: Session,
) -> Result<ApiResponse<()>, ApiError> {
    require_user(&session).await?;
    if id == "local" {
        return Err(ApiError::bad_request("本地环境不支持删除"));
    }

    let result = sqlx::query("DELETE FROM environments WHERE id = ? AND kind = 'agent'")
        .bind(&id)
        .execute(&state.pool)
        .await
        .map_err(|error| {
            tracing::error!(environment_id = %id, error = %error, "删除环境失败");
            ApiError::internal("删除环境失败，请稍后重试")
        })?;

    if result.rows_affected() == 0 {
        return Err(ApiError::not_found("环境不存在"));
    }

    state.agent_snapshots.write().await.remove(&id);

    Ok(ApiResponse::success(()))
}

fn response_from_snapshot(
    row: &EnvironmentRow,
    status: EnvironmentStatus,
    snapshot: &DockerSnapshot,
    offline_reason: Option<String>,
) -> EnvironmentResponse {
    EnvironmentResponse {
        id: row.id.clone(),
        name: row.name.clone(),
        kind: parse_kind(&row.kind).unwrap_or(EnvironmentKind::Local),
        status,
        endpoint: row.endpoint.clone(),
        docker_version: Some(snapshot.docker_version.clone()),
        containers: counts_from_snapshot(&snapshot.containers),
        cpu_percent: snapshot.cpu_percent,
        memory: snapshot.memory.as_ref().map(memory_from_snapshot),
        metrics_collected_at: Some(snapshot.metrics_collected_at),
        metrics_error: snapshot.metrics_error.clone(),
        offline_reason,
        last_seen_at: row.last_seen_at.clone(),
    }
}

fn unavailable_response(
    row: &EnvironmentRow,
    status: EnvironmentStatus,
    reason: &str,
) -> EnvironmentResponse {
    EnvironmentResponse {
        id: row.id.clone(),
        name: row.name.clone(),
        kind: parse_kind(&row.kind).unwrap_or(EnvironmentKind::Local),
        status,
        endpoint: row.endpoint.clone(),
        docker_version: None,
        containers: ContainerCountsResponse {
            total: 0,
            running: 0,
            paused: 0,
            stopped: 0,
        },
        cpu_percent: None,
        memory: None,
        metrics_collected_at: None,
        metrics_error: Some(reason.to_owned()),
        offline_reason: Some(reason.to_owned()),
        last_seen_at: row.last_seen_at.clone(),
    }
}

fn counts_from_snapshot(counts: &DockerContainerCounts) -> ContainerCountsResponse {
    ContainerCountsResponse {
        total: counts.total,
        running: counts.running,
        paused: counts.paused,
        stopped: counts.stopped,
    }
}

fn memory_from_snapshot(memory: &DockerMemoryMetrics) -> MemoryMetricsResponse {
    MemoryMetricsResponse {
        used_bytes: memory.used_bytes,
        total_bytes: memory.total_bytes,
        percent: memory.percent,
    }
}

fn parse_kind(kind: &str) -> Option<EnvironmentKind> {
    match kind {
        "local" => Some(EnvironmentKind::Local),
        "agent" => Some(EnvironmentKind::Agent),
        _ => None,
    }
}

fn validate_name(name: String) -> Result<String, ApiError> {
    let name = name.trim().to_owned();
    if name.is_empty() {
        return Err(ApiError::bad_request("环境名称不能为空"));
    }
    if name.chars().count() > 64 || name.chars().any(char::is_control) {
        return Err(ApiError::bad_request("环境名称无效"));
    }
    Ok(name)
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;

    use super::{
        AgentSnapshotRequest, ContainerCountsResponse, DockerSnapshot, EnvironmentKind,
        EnvironmentRow, EnvironmentStatus, MemoryMetricsResponse, parse_kind, response_for_row,
        validate_name,
    };

    #[test]
    fn environment_kind_parser_accepts_only_known_values() {
        assert!(matches!(parse_kind("local"), Some(EnvironmentKind::Local)));
        assert!(matches!(parse_kind("agent"), Some(EnvironmentKind::Agent)));
        assert!(parse_kind("remote").is_none());
    }

    #[test]
    fn environment_name_is_trimmed_and_bounded() {
        assert_eq!(
            validate_name("  家用 Docker  ".to_owned()).unwrap(),
            "家用 Docker"
        );
        assert!(validate_name("   ".to_owned()).is_err());
        assert!(validate_name("x".repeat(65)).is_err());
        assert!(validate_name("bad\nname".to_owned()).is_err());
    }

    #[test]
    fn agent_snapshot_maps_to_docker_snapshot() {
        let snapshot: DockerSnapshot = AgentSnapshotRequest {
            docker_version: "27.0".to_owned(),
            containers: ContainerCountsResponse {
                total: 4,
                running: 2,
                paused: 1,
                stopped: 1,
            },
            cpu_percent: Some(12.5),
            memory: Some(MemoryMetricsResponse {
                used_bytes: 40,
                total_bytes: 100,
                percent: 40.0,
            }),
            metrics_error: None,
            metrics_collected_at: 123,
        }
        .into();

        assert_eq!(snapshot.docker_version, "27.0");
        assert_eq!(snapshot.containers.running, 2);
        assert_eq!(
            snapshot.memory.as_ref().map(|memory| memory.used_bytes),
            Some(40)
        );
        assert_eq!(snapshot.cpu_percent, Some(12.5));
    }

    #[test]
    fn response_preserves_local_snapshot_metrics() {
        let row = EnvironmentRow {
            id: "local".to_owned(),
            name: "local-engine".to_owned(),
            kind: "local".to_owned(),
            endpoint: "unix:///var/run/docker.sock".to_owned(),
            last_seen_at: None,
            is_online: 1,
        };
        let snapshot = DockerSnapshot {
            docker_version: "27.0".to_owned(),
            containers: dockrs_docker::ContainerCounts {
                total: 4,
                running: 2,
                paused: 1,
                stopped: 1,
            },
            cpu_percent: Some(12.5),
            memory: Some(dockrs_docker::MemoryMetrics {
                used_bytes: 40,
                total_bytes: 100,
                percent: 40.0,
            }),
            metrics_error: None,
            metrics_collected_at: 123,
        };

        let response = response_for_row(&row, Some(&snapshot), &HashMap::new())
            .expect("有效环境类型应能生成响应");

        assert!(matches!(response.kind, EnvironmentKind::Local));
        assert!(matches!(response.status, EnvironmentStatus::Online));
        assert_eq!(response.docker_version.as_deref(), Some("27.0"));
        assert_eq!(response.containers.running, 2);
        assert_eq!(
            response.memory.as_ref().map(|memory| memory.used_bytes),
            Some(40)
        );
    }
}
