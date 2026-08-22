use std::{
    collections::HashMap,
    time::{Duration, Instant},
};

use axum::{
    Router,
    extract::{
        Path, State, WebSocketUpgrade,
        ws::{Message, WebSocket},
    },
    response::{IntoResponse, Response},
    routing::get,
};
use dockrs_agent::{AgentErrorKind, AgentMessage, AgentOperation, PROTOCOL_VERSION};
use serde_json::Value;
use tokio::{
    sync::{mpsc, oneshot},
    time::{interval, timeout},
};

use crate::{AppState, environments, error::ApiError};

const REQUEST_TIMEOUT: Duration = Duration::from_secs(8);
const HELLO_TIMEOUT: Duration = Duration::from_secs(10);
const PENDING_TIMEOUT: Duration = Duration::from_secs(10);
const MAX_PENDING_REQUESTS: usize = 64;
const CHANNEL_CAPACITY: usize = 32;

#[derive(Clone)]
pub(crate) struct AgentConnection {
    id: u64,
    sender: mpsc::Sender<AgentRequest>,
}

struct AgentRequest {
    request_id: String,
    operation: AgentOperation,
    response: oneshot::Sender<AgentCommandResult>,
}

struct AgentCommandResult {
    ok: bool,
    data: Option<Value>,
    error: Option<AgentErrorKind>,
}

struct PendingResponse {
    response: oneshot::Sender<AgentCommandResult>,
    deadline: Instant,
}

pub(crate) fn router() -> Router<AppState> {
    Router::new().route("/environments/agents/{id}/stream", get(agent_stream))
}

pub(crate) async fn connection_for_environment(
    state: &AppState,
    id: &str,
) -> Result<AgentConnection, ApiError> {
    state
        .agent_connections
        .read()
        .await
        .get(id)
        .cloned()
        .ok_or_else(|| ApiError::bad_gateway("远程 Agent 当前离线"))
}

pub(crate) async fn request(
    state: &AppState,
    connection: &AgentConnection,
    operation: AgentOperation,
) -> Result<Value, ApiError> {
    let request_id = state
        .next_agent_sequence
        .fetch_add(1, std::sync::atomic::Ordering::Relaxed)
        .to_string();
    let (response_tx, response_rx) = oneshot::channel();
    let request = AgentRequest {
        request_id,
        operation,
        response: response_tx,
    };
    connection.sender.try_send(request).map_err(|error| {
        if matches!(error, mpsc::error::TrySendError::Full(_)) {
            ApiError::bad_gateway("远程 Agent 请求队列已满")
        } else {
            ApiError::bad_gateway("远程 Agent 当前不可用")
        }
    })?;

    let result = timeout(REQUEST_TIMEOUT, response_rx)
        .await
        .map_err(|_| ApiError::bad_gateway("远程 Agent 请求超时"))?
        .map_err(|_| ApiError::bad_gateway("远程 Agent 连接已断开"))?;
    if !result.ok {
        return Err(map_agent_error(result.error));
    }
    result
        .data
        .ok_or_else(|| ApiError::bad_gateway("远程 Agent 返回了无效响应"))
}

fn map_agent_error(error: Option<AgentErrorKind>) -> ApiError {
    match error.unwrap_or(AgentErrorKind::InvalidResponse) {
        AgentErrorKind::NotFound => ApiError::not_found("Docker 资源不存在"),
        AgentErrorKind::Conflict => ApiError::conflict("资源当前正在使用或状态不允许此操作"),
        AgentErrorKind::Request | AgentErrorKind::Connection | AgentErrorKind::InvalidResponse => {
            ApiError::bad_gateway("远程 Agent 执行失败")
        }
    }
}

async fn agent_stream(
    State(state): State<AppState>,
    Path(id): Path<String>,
    headers: axum::http::HeaderMap,
    ws: WebSocketUpgrade,
) -> Result<Response, ApiError> {
    environments::require_agent_token(&headers, &state)?;
    Ok(ws
        .on_upgrade(move |socket| handle_socket(socket, state, id))
        .into_response())
}

async fn handle_socket(mut socket: WebSocket, state: AppState, expected_id: String) {
    let hello = match timeout(HELLO_TIMEOUT, socket.recv()).await {
        Ok(Some(Ok(Message::Text(text)))) => {
            match serde_json::from_str::<AgentMessage>(text.as_str()) {
                Ok(message) => message,
                Err(error) => {
                    tracing::debug!(environment_id = %expected_id, error = %error, "Agent hello 解析失败");
                    return;
                }
            }
        }
        Ok(Some(Ok(_))) => {
            tracing::debug!(environment_id = %expected_id, "Agent hello 消息无效");
            return;
        }
        Ok(Some(Err(error))) => {
            tracing::debug!(environment_id = %expected_id, error = %error, "Agent hello 读取失败");
            return;
        }
        Ok(None) | Err(_) => return,
    };
    let AgentMessage::Hello {
        protocol_version,
        agent_id,
        name,
        endpoint,
    } = hello
    else {
        tracing::debug!(environment_id = %expected_id, "Agent 首条消息不是 hello");
        return;
    };

    if protocol_version != PROTOCOL_VERSION || agent_id != expected_id {
        tracing::warn!(
            environment_id = %expected_id,
            agent_id = %agent_id,
            protocol_version,
            "Agent 协议或身份不匹配",
        );
        return;
    }
    if let Err(error) = environments::register_connected_agent(
        &state,
        expected_id.clone(),
        name,
        endpoint,
        Some(agent_id),
    )
    .await
    {
        tracing::warn!(environment_id = %expected_id, error = %error_message(&error), "Agent 注册失败");
        return;
    }

    let connection_id = state
        .next_agent_sequence
        .fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    let (sender, mut requests) = mpsc::channel(CHANNEL_CAPACITY);
    state.agent_connections.write().await.insert(
        expected_id.clone(),
        AgentConnection {
            id: connection_id,
            sender,
        },
    );

    let mut pending = HashMap::<String, PendingResponse>::new();
    let mut cleanup = interval(Duration::from_secs(1));
    loop {
        tokio::select! {
            incoming = socket.recv() => {
                let Some(incoming) = incoming else { break };
                let Ok(message) = incoming else { break };
                match message {
                    Message::Text(text) => match serde_json::from_str::<AgentMessage>(text.as_str()) {
                        Ok(AgentMessage::Result { request_id, ok, data, error }) => {
                            if let Some(response) = pending.remove(&request_id) {
                                let _ = response.response.send(AgentCommandResult { ok, data, error });
                            }
                        }
                        Ok(AgentMessage::Snapshot { snapshot }) => {
                            if let Err(error) = environments::store_agent_snapshot(&state, expected_id.clone(), *snapshot).await {
                                tracing::debug!(environment_id = %expected_id, error = %error_message(&error), "保存 Agent 指标失败");
                            }
                        }
                        Ok(AgentMessage::Heartbeat) => {
                            if let Err(error) = environments::touch_agent(&state, &expected_id).await {
                                tracing::debug!(environment_id = %expected_id, error = %error_message(&error), "更新 Agent 心跳失败");
                            }
                        }
                        Ok(_) => {}
                        Err(error) => {
                            tracing::debug!(environment_id = %expected_id, error = %error, "Agent 消息解析失败");
                            break;
                        }
                    },
                    Message::Ping(payload) => {
                        if socket.send(Message::Pong(payload)).await.is_err() { break; }
                    }
                    Message::Close(_) => break,
                    _ => {}
                }
            }
            request = requests.recv() => {
                let Some(request) = request else { break };
                if pending.len() >= MAX_PENDING_REQUESTS {
                    let _ = request.response.send(AgentCommandResult {
                        ok: false,
                        data: None,
                        error: Some(AgentErrorKind::Request),
                    });
                    continue;
                }
                let request_id = request.request_id.clone();
                let message = AgentMessage::Command {
                    request_id: request_id.clone(),
                    operation: Box::new(request.operation),
                };
                let text = match serde_json::to_string(&message) {
                    Ok(text) => text,
                    Err(_) => {
                        let _ = request.response.send(AgentCommandResult {
                            ok: false,
                            data: None,
                            error: Some(AgentErrorKind::InvalidResponse),
                        });
                        continue;
                    }
                };
                pending.insert(request_id, PendingResponse {
                    response: request.response,
                    deadline: Instant::now() + PENDING_TIMEOUT,
                });
                if socket.send(Message::Text(text.into())).await.is_err() { break; }
            }
            _ = cleanup.tick() => {
                let now = Instant::now();
                let expired = pending
                    .iter()
                    .filter(|(_, response)| response.deadline <= now)
                    .map(|(request_id, _)| request_id.clone())
                    .collect::<Vec<_>>();
                for request_id in expired {
                    if let Some(response) = pending.remove(&request_id) {
                        let _ = response.response.send(AgentCommandResult {
                            ok: false,
                            data: None,
                            error: Some(AgentErrorKind::Request),
                        });
                    }
                }
            }
        }
    }

    pending.clear();
    let mut connections = state.agent_connections.write().await;
    if connections
        .get(&expected_id)
        .is_some_and(|connection| connection.id == connection_id)
    {
        connections.remove(&expected_id);
    }
}

fn error_message(error: &ApiError) -> String {
    format!("{error:?}")
}
