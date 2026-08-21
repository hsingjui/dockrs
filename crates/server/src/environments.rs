use std::{
    collections::HashMap,
    time::{Duration, Instant},
};

use axum::{
    Router,
    extract::{Path, State},
    http::HeaderMap,
    routing::{get, patch, post},
};
use dockrs_docker::{
    ContainerCounts as DockerContainerCounts, DockerSnapshot, MemoryMetrics as DockerMemoryMetrics,
};
use serde::{Deserialize, Serialize};
use sqlx::SqlitePool;
use tokio::time::timeout;
use tower_sessions::Session;
use utoipa::ToSchema;

use crate::{
    AppState,
    auth::require_user,
    error::{ApiError, ApiJson, ApiResponse},
};

const AGENT_ONLINE_WINDOW_SECS: i64 = 90;
const AGENT_SNAPSHOT_TTL: Duration = Duration::from_secs(30);
const DOCKER_REQUEST_TIMEOUT: Duration = Duration::from_secs(5);
const MAX_AGENT_SNAPSHOTS: usize = 64;

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
        .route("/environments/{id}", patch(update_environment_name))
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

    let mut environments = Vec::with_capacity(rows.len());
    for row in rows {
        let kind = parse_kind(&row.kind).ok_or_else(|| {
            tracing::error!(environment_id = %row.id, kind = %row.kind, "环境类型无效");
            ApiError::internal("环境数据无效")
        })?;

        let response = match kind {
            EnvironmentKind::Local => match local_snapshot.as_ref() {
                Some(snapshot) => {
                    response_from_snapshot(&row, EnvironmentStatus::Online, snapshot, None)
                }
                None => unavailable_response(
                    &row,
                    EnvironmentStatus::Offline,
                    "无法连接 Docker Engine，请检查 Docker Socket",
                ),
            },
            EnvironmentKind::Agent => {
                if row.is_online == 0 {
                    unavailable_response(
                        &row,
                        EnvironmentStatus::Offline,
                        "Agent 心跳超时，节点当前离线",
                    )
                } else if let Some(cached) = agent_snapshots.get(&row.id) {
                    response_from_snapshot(&row, EnvironmentStatus::Online, &cached.snapshot, None)
                } else {
                    let mut response = unavailable_response(
                        &row,
                        EnvironmentStatus::Online,
                        "等待 Agent 上报 Docker 指标",
                    );
                    response.offline_reason = None;
                    response
                }
            }
        };
        environments.push(response);
    }

    Ok(ApiResponse::success(environments))
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
    post,
    path = "/api/environments/agents/register",
    request_body = AgentRegistrationRequest,
    responses((status = 200, body = AgentRegistrationResponse, description = "Agent 注册成功"))
)]
async fn register_agent(
    State(state): State<AppState>,
    headers: HeaderMap,
    ApiJson(request): ApiJson<AgentRegistrationRequest>,
) -> Result<ApiResponse<AgentRegistrationResponse>, ApiError> {
    require_agent_token(&headers, &state)?;

    let id = validate_agent_id(request.id)?;
    if id == "local" {
        return Err(ApiError::bad_request("Agent ID 无效"));
    }
    let name = validate_name(request.name)?;
    let endpoint = request.endpoint.trim().to_owned();
    if endpoint.is_empty() || endpoint.chars().count() > 256 {
        return Err(ApiError::bad_request("Agent 地址无效"));
    }

    let existing_kind =
        sqlx::query_scalar::<_, String>("SELECT kind FROM environments WHERE id = ?")
            .bind(&id)
            .fetch_optional(&state.pool)
            .await
            .map_err(|error| {
                tracing::error!(error = %error, "查询 Agent 环境失败");
                ApiError::internal("Agent 注册失败，请稍后重试")
            })?;
    if existing_kind.as_deref() == Some("local") {
        return Err(ApiError::bad_request("环境 ID 已被本地环境占用"));
    }

    let agent_id = request
        .agent_id
        .filter(|agent_id| !agent_id.trim().is_empty())
        .unwrap_or_else(|| id.clone());
    sqlx::query(
        "INSERT INTO environments (id, name, kind, endpoint, agent_id, last_seen_at)
         VALUES (?, ?, 'agent', ?, ?, datetime('now'))
         ON CONFLICT(id) DO UPDATE SET
             name = excluded.name,
             endpoint = excluded.endpoint,
             agent_id = excluded.agent_id,
             last_seen_at = datetime('now'),
             updated_at = datetime('now')",
    )
    .bind(&id)
    .bind(&name)
    .bind(endpoint)
    .bind(agent_id)
    .execute(&state.pool)
    .await
    .map_err(|error| {
        tracing::error!(error = %error, "写入 Agent 环境失败");
        ApiError::internal("Agent 注册失败，请稍后重试")
    })?;

    Ok(ApiResponse::success(AgentRegistrationResponse { id }))
}

#[utoipa::path(
    post,
    path = "/api/environments/agents/{id}/heartbeat",
    params(("id" = String, Path, description = "Agent 环境 ID")),
    responses(
        (status = 200, description = "心跳已记录"),
        (status = 401, description = "Agent 凭据无效"),
        (status = 404, description = "Agent 环境不存在")
    )
)]
async fn agent_heartbeat(
    State(state): State<AppState>,
    Path(id): Path<String>,
    headers: HeaderMap,
) -> Result<ApiResponse<serde_json::Value>, ApiError> {
    require_agent_token(&headers, &state)?;

    let result = sqlx::query(
        "UPDATE environments
         SET last_seen_at = datetime('now'), updated_at = datetime('now')
         WHERE id = ? AND kind = 'agent'",
    )
    .bind(&id)
    .execute(&state.pool)
    .await
    .map_err(|error| {
        tracing::error!(error = %error, "更新 Agent 心跳失败");
        ApiError::internal("更新 Agent 心跳失败，请稍后重试")
    })?;
    if result.rows_affected() == 0 {
        return Err(ApiError::not_found("Agent 环境不存在"));
    }

    Ok(ApiResponse::success(serde_json::json!({})))
}

#[utoipa::path(
    post,
    path = "/api/environments/agents/{id}/snapshot",
    params(("id" = String, Path, description = "Agent 环境 ID")),
    request_body = AgentSnapshotRequest,
    responses(
        (status = 200, description = "指标已接收"),
        (status = 400, description = "指标格式错误"),
        (status = 401, description = "Agent 凭据无效"),
        (status = 404, description = "Agent 环境不存在")
    )
)]
async fn agent_snapshot(
    State(state): State<AppState>,
    Path(id): Path<String>,
    headers: HeaderMap,
    ApiJson(request): ApiJson<AgentSnapshotRequest>,
) -> Result<ApiResponse<serde_json::Value>, ApiError> {
    require_agent_token(&headers, &state)?;

    let result = sqlx::query(
        "UPDATE environments
         SET last_seen_at = datetime('now'), updated_at = datetime('now')
         WHERE id = ? AND kind = 'agent'",
    )
    .bind(&id)
    .execute(&state.pool)
    .await
    .map_err(|error| {
        tracing::error!(error = %error, "更新 Agent 指标时间失败");
        ApiError::internal("接收 Agent 指标失败，请稍后重试")
    })?;
    if result.rows_affected() == 0 {
        return Err(ApiError::not_found("Agent 环境不存在"));
    }

    let mut snapshots = state.agent_snapshots.write().await;
    snapshots.insert(
        id,
        CachedAgentSnapshot {
            snapshot: request.into(),
            received_at: Instant::now(),
        },
    );
    trim_agent_snapshots(&mut snapshots);

    Ok(ApiResponse::success(serde_json::json!({})))
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
    if name.chars().count() > 64 {
        return Err(ApiError::bad_request("环境名称不能超过 64 个字符"));
    }
    Ok(name)
}

fn validate_agent_id(id: String) -> Result<String, ApiError> {
    let id = id.trim().to_owned();
    if id.is_empty() || id.chars().count() > 128 {
        return Err(ApiError::bad_request("Agent ID 无效"));
    }
    Ok(id)
}

fn require_agent_token(headers: &HeaderMap, state: &AppState) -> Result<(), ApiError> {
    let Some(expected) = state.agent_token.as_deref() else {
        return Err(ApiError::unauthorized("Agent 通信尚未配置"));
    };
    let provided = headers
        .get("x-dockrs-agent-token")
        .and_then(|value| value.to_str().ok())
        .or_else(|| {
            headers
                .get("authorization")
                .and_then(|value| value.to_str().ok())
                .and_then(|value| value.strip_prefix("Bearer "))
        });
    if provided != Some(expected) {
        return Err(ApiError::unauthorized("Agent 凭据无效"));
    }
    Ok(())
}

fn trim_agent_snapshots(snapshots: &mut HashMap<String, CachedAgentSnapshot>) {
    while snapshots.len() > MAX_AGENT_SNAPSHOTS {
        let oldest_id = snapshots
            .iter()
            .min_by_key(|(_, snapshot)| snapshot.received_at)
            .map(|(id, _)| id.clone());
        let Some(oldest_id) = oldest_id else {
            break;
        };
        snapshots.remove(&oldest_id);
    }
}
