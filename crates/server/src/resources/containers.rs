use axum::extract::{Path, Query, State};
use dockrs_agent::AgentOperation;
use dockrs_docker::{ContainerSummary, DockerError};
use tower_sessions::Session;

use crate::{
    AppState,
    auth::require_user,
    error::{ApiError, ApiResponse},
};

use super::{
    ContainerListQuery, ContainerPortResponse, ContainerResponse, MutationResponse, ResourceClient,
    resource_client_for_environment, run_agent, run_agent_unit, run_docker, validate_filter,
};

pub(super) async fn list_containers_on(
    state: &AppState,
    client: &ResourceClient,
    all: bool,
    status: Option<&str>,
    query: Option<&str>,
) -> Result<Vec<ContainerSummary>, ApiError> {
    match client {
        ResourceClient::Local(client) => {
            run_docker(client.clone(), |client| async move {
                client.list_containers_filtered(all, status, query).await
            })
            .await
        }
        ResourceClient::Agent(connection) => {
            run_agent(
                state,
                connection,
                AgentOperation::ListContainers {
                    all,
                    status: status.map(str::to_owned),
                    query: query.map(str::to_owned),
                },
            )
            .await
        }
    }
}

async fn container_action_on(
    state: &AppState,
    client: &ResourceClient,
    id: &str,
    action: &str,
) -> Result<(), ApiError> {
    match client {
        ResourceClient::Local(client) => {
            let id = id.to_owned();
            let action = action.to_owned();
            run_docker(client.clone(), |client| async move {
                match action.as_str() {
                    "start" => client.start_container(&id).await,
                    "stop" => client.stop_container(&id).await,
                    "restart" => client.restart_container(&id).await,
                    "pause" => client.pause_container(&id).await,
                    "unpause" => client.unpause_container(&id).await,
                    _ => Err(DockerError::Request("不支持的容器操作".to_owned())),
                }
            })
            .await
        }
        ResourceClient::Agent(connection) => {
            run_agent_unit(
                state,
                connection,
                AgentOperation::ContainerAction {
                    id: id.to_owned(),
                    action: action.to_owned(),
                },
            )
            .await
        }
    }
}

async fn remove_container_on(
    state: &AppState,
    client: &ResourceClient,
    id: &str,
) -> Result<(), ApiError> {
    match client {
        ResourceClient::Local(client) => {
            let id = id.to_owned();
            run_docker(client.clone(), |client| async move {
                client.remove_container(&id).await
            })
            .await
        }
        ResourceClient::Agent(connection) => {
            run_agent_unit(
                state,
                connection,
                AgentOperation::RemoveContainer { id: id.to_owned() },
            )
            .await
        }
    }
}

fn validate_container_status(status: Option<String>) -> Result<Option<String>, ApiError> {
    let Some(status) = status else {
        return Ok(None);
    };
    let status = status.trim().to_owned();
    if [
        "created",
        "restarting",
        "running",
        "removing",
        "paused",
        "exited",
        "dead",
        "stopping",
    ]
    .contains(&status.as_str())
    {
        Ok(Some(status))
    } else {
        Err(ApiError::unprocessable_entity("容器状态筛选条件无效"))
    }
}

#[utoipa::path(
    get,
    path = "/api/environments/{id}/containers",
    params(("id" = String, Path, description = "环境 ID")),
    responses((status = 200, body = [ContainerResponse], description = "容器列表"))
)]
pub(crate) async fn list_containers(
    State(state): State<AppState>,
    Path(id): Path<String>,
    Query(query): Query<ContainerListQuery>,
    session: Session,
) -> Result<ApiResponse<Vec<ContainerResponse>>, ApiError> {
    require_user(&session).await?;
    let query_text = validate_filter(query.q)?;
    let status = validate_container_status(query.status)?;
    let client = resource_client_for_environment(&state, &id).await?;
    let containers = list_containers_on(
        &state,
        &client,
        query.all.unwrap_or(true),
        status.as_deref(),
        query_text.as_deref(),
    )
    .await?;

    Ok(ApiResponse::success(
        containers.into_iter().map(container_response).collect(),
    ))
}

#[utoipa::path(
    post,
    path = "/api/environments/{id}/containers/{container_id}/{action}",
    params(
        ("id" = String, Path, description = "环境 ID"),
        ("container_id" = String, Path, description = "容器 ID"),
        ("action" = String, Path, description = "容器操作")
    ),
    responses((status = 200, body = MutationResponse, description = "操作成功"))
)]
pub(crate) async fn container_action(
    State(state): State<AppState>,
    Path((id, container_id, action)): Path<(String, String, String)>,
    session: Session,
) -> Result<ApiResponse<MutationResponse>, ApiError> {
    require_user(&session).await?;
    if !["start", "stop", "restart", "pause", "unpause"].contains(&action.as_str()) {
        return Err(ApiError::unprocessable_entity("不支持的容器操作"));
    }
    let client = resource_client_for_environment(&state, &id).await?;
    let response_id = container_id.clone();
    container_action_on(&state, &client, &container_id, &action).await?;
    Ok(ApiResponse::success(MutationResponse {
        id: Some(response_id),
    }))
}

#[utoipa::path(
    delete,
    path = "/api/environments/{id}/containers/{container_id}",
    params(
        ("id" = String, Path, description = "环境 ID"),
        ("container_id" = String, Path, description = "容器 ID")
    ),
    responses((status = 200, body = MutationResponse, description = "删除成功"))
)]
pub(crate) async fn remove_container(
    State(state): State<AppState>,
    Path((id, container_id)): Path<(String, String)>,
    session: Session,
) -> Result<ApiResponse<MutationResponse>, ApiError> {
    require_user(&session).await?;
    let client = resource_client_for_environment(&state, &id).await?;
    let response_id = container_id.clone();
    remove_container_on(&state, &client, &container_id).await?;
    Ok(ApiResponse::success(MutationResponse {
        id: Some(response_id),
    }))
}

fn container_response(container: ContainerSummary) -> ContainerResponse {
    let id = container.id.unwrap_or_default();
    let names = container
        .names
        .unwrap_or_default()
        .into_iter()
        .map(|name| name.trim_start_matches('/').to_owned())
        .collect::<Vec<_>>();
    let name = names.first().cloned().unwrap_or_else(|| id.clone());
    let ports = container
        .ports
        .unwrap_or_default()
        .into_iter()
        .map(|port| ContainerPortResponse {
            container_port: port.private_port,
            host_port: port.public_port,
            protocol: port
                .typ
                .map(|value| value.to_string())
                .unwrap_or_else(|| "tcp".to_owned()),
        })
        .collect();
    let stack = container
        .labels
        .as_ref()
        .and_then(|labels| labels.get("com.docker.stack.namespace").cloned());
    ContainerResponse {
        id,
        name,
        names,
        image: container.image,
        state: container
            .state
            .map(|state| state.to_string())
            .unwrap_or_else(|| "unknown".to_owned()),
        status: container.status,
        created: container.created,
        ports,
        stack,
    }
}
