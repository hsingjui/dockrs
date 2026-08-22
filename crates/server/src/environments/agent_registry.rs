use std::{collections::HashMap, time::Instant};

use axum::{
    extract::{Path, State},
    http::HeaderMap,
};
use dockrs_docker::DockerSnapshot;

use crate::{
    AppState,
    error::{ApiError, ApiJson, ApiResponse},
};

use super::{
    AgentRegistrationRequest, AgentRegistrationResponse, AgentSnapshotRequest, CachedAgentSnapshot,
    validate_name,
};

const MAX_AGENT_SNAPSHOTS: usize = 64;

#[utoipa::path(
    post,
    path = "/api/environments/agents/register",
    request_body = AgentRegistrationRequest,
    responses((status = 200, body = AgentRegistrationResponse, description = "Agent 注册成功"))
)]
pub(crate) async fn register_agent(
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
pub(crate) async fn agent_heartbeat(
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
pub(crate) async fn agent_snapshot(
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

pub(crate) async fn register_connected_agent(
    state: &AppState,
    id: String,
    name: String,
    endpoint: String,
    agent_id: Option<String>,
) -> Result<(), ApiError> {
    let id = validate_agent_id(id)?;
    if id == "local" {
        return Err(ApiError::bad_request("Agent ID 无效"));
    }
    let name = validate_name(name)?;
    let endpoint = endpoint.trim().to_owned();
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

    let agent_id = agent_id
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
    Ok(())
}

pub(crate) async fn touch_agent(state: &AppState, id: &str) -> Result<(), ApiError> {
    let result = sqlx::query(
        "UPDATE environments
         SET last_seen_at = datetime('now'), updated_at = datetime('now')
         WHERE id = ? AND kind = 'agent'",
    )
    .bind(id)
    .execute(&state.pool)
    .await
    .map_err(|error| {
        tracing::error!(environment_id = %id, error = %error, "更新 Agent 心跳失败");
        ApiError::internal("更新 Agent 心跳失败，请稍后重试")
    })?;
    if result.rows_affected() == 0 {
        return Err(ApiError::not_found("Agent 环境不存在"));
    }
    Ok(())
}

pub(crate) async fn store_agent_snapshot(
    state: &AppState,
    id: String,
    snapshot: DockerSnapshot,
) -> Result<(), ApiError> {
    touch_agent(state, &id).await?;
    let mut snapshots = state.agent_snapshots.write().await;
    snapshots.insert(
        id,
        CachedAgentSnapshot {
            snapshot,
            received_at: Instant::now(),
        },
    );
    trim_agent_snapshots(&mut snapshots);
    Ok(())
}

fn validate_agent_id(id: String) -> Result<String, ApiError> {
    let id = id.trim().to_owned();
    if id.is_empty()
        || id.chars().count() > 128
        || !id
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_'))
    {
        return Err(ApiError::bad_request("Agent ID 无效"));
    }
    Ok(id)
}

pub(crate) fn require_agent_token(headers: &HeaderMap, state: &AppState) -> Result<(), ApiError> {
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

#[cfg(test)]
mod tests {
    use super::validate_agent_id;

    #[test]
    fn agent_id_is_trimmed_and_rejects_unsafe_values() {
        assert_eq!(
            validate_agent_id(" agent-01 ".to_owned()).unwrap(),
            "agent-01"
        );
        assert!(validate_agent_id("".to_owned()).is_err());
        assert!(validate_agent_id("agent id".to_owned()).is_err());
        assert!(validate_agent_id("agent/id".to_owned()).is_err());
        assert!(validate_agent_id("a".repeat(129)).is_err());
    }
}
