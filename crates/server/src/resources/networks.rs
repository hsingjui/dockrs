use std::collections::HashMap;

use axum::extract::{Path, Query, State};
use dockrs_agent::AgentOperation;
use dockrs_docker::{Network, NetworkCreateRequest};
use tower_sessions::Session;

use crate::{
    AppState,
    auth::require_user,
    error::{ApiError, ApiJson, ApiResponse},
};

use super::{
    CreateNetworkRequest, MutationResponse, NetworkResponse, ResourceClient, ResourceListQuery,
    resource_client_for_environment, run_agent, run_agent_unit, run_docker, validate_filter,
    validate_optional_name, validate_resource_name,
};

pub(super) async fn list_networks_on(
    state: &AppState,
    client: &ResourceClient,
    query: Option<&str>,
) -> Result<Vec<Network>, ApiError> {
    match client {
        ResourceClient::Local(client) => {
            run_docker(client.clone(), |client| async move {
                client.list_networks_filtered(query).await
            })
            .await
        }
        ResourceClient::Agent(connection) => {
            run_agent(
                state,
                connection,
                AgentOperation::ListNetworks {
                    query: query.map(str::to_owned),
                },
            )
            .await
        }
    }
}

async fn network_counts_on(
    state: &AppState,
    client: &ResourceClient,
) -> Result<HashMap<String, u64>, ApiError> {
    match client {
        ResourceClient::Local(client) => {
            run_docker(client.clone(), |client| async move {
                client.network_container_counts().await
            })
            .await
        }
        ResourceClient::Agent(connection) => {
            run_agent(state, connection, AgentOperation::NetworkContainerCounts).await
        }
    }
}

async fn create_network_on(
    state: &AppState,
    client: &ResourceClient,
    config: NetworkCreateRequest,
) -> Result<String, ApiError> {
    match client {
        ResourceClient::Local(client) => {
            run_docker(client.clone(), |client| async move {
                client.create_network(config).await
            })
            .await
        }
        ResourceClient::Agent(connection) => {
            run_agent(state, connection, AgentOperation::CreateNetwork { config }).await
        }
    }
}

async fn remove_network_on(
    state: &AppState,
    client: &ResourceClient,
    id: &str,
) -> Result<(), ApiError> {
    match client {
        ResourceClient::Local(client) => {
            let id = id.to_owned();
            run_docker(client.clone(), |client| async move {
                client.remove_network(&id).await
            })
            .await
        }
        ResourceClient::Agent(connection) => {
            run_agent_unit(
                state,
                connection,
                AgentOperation::RemoveNetwork { id: id.to_owned() },
            )
            .await
        }
    }
}

#[utoipa::path(
    get,
    path = "/api/environments/{id}/networks",
    params(("id" = String, Path, description = "环境 ID")),
    responses((status = 200, body = [NetworkResponse], description = "网络列表"))
)]
pub(crate) async fn list_networks(
    State(state): State<AppState>,
    Path(id): Path<String>,
    Query(query): Query<ResourceListQuery>,
    session: Session,
) -> Result<ApiResponse<Vec<NetworkResponse>>, ApiError> {
    require_user(&session).await?;
    let query_text = validate_filter(query.q)?;
    let client = resource_client_for_environment(&state, &id).await?;
    let networks = list_networks_on(&state, &client, query_text.as_deref()).await?;
    let container_counts = network_counts_on(&state, &client).await.ok();
    let responses = networks
        .into_iter()
        .map(|network| {
            let count = container_counts.as_ref().and_then(|counts| {
                network
                    .id
                    .as_deref()
                    .and_then(|id| counts.get(id).copied())
                    .or_else(|| {
                        network
                            .name
                            .as_deref()
                            .and_then(|name| counts.get(name).copied())
                    })
            });
            network_response(network, count)
        })
        .collect();
    Ok(ApiResponse::success(responses))
}

#[utoipa::path(
    post,
    path = "/api/environments/{id}/networks",
    params(("id" = String, Path, description = "环境 ID")),
    request_body = CreateNetworkRequest,
    responses((status = 200, body = MutationResponse, description = "创建成功"))
)]
pub(crate) async fn create_network(
    State(state): State<AppState>,
    Path(id): Path<String>,
    session: Session,
    ApiJson(request): ApiJson<CreateNetworkRequest>,
) -> Result<ApiResponse<MutationResponse>, ApiError> {
    require_user(&session).await?;
    let name = validate_resource_name(request.name, "网络名称")?;
    let driver = validate_optional_name(request.driver, "网络 driver")?;
    let client = resource_client_for_environment(&state, &id).await?;
    let network_id = create_network_on(
        &state,
        &client,
        NetworkCreateRequest {
            name,
            driver,
            internal: request.internal,
            ..Default::default()
        },
    )
    .await?;
    Ok(ApiResponse::success(MutationResponse {
        id: Some(network_id),
    }))
}

#[utoipa::path(
    delete,
    path = "/api/environments/{id}/networks/{network_id}",
    params(
        ("id" = String, Path, description = "环境 ID"),
        ("network_id" = String, Path, description = "网络 ID")
    ),
    responses((status = 200, body = MutationResponse, description = "删除成功"))
)]
pub(crate) async fn remove_network(
    State(state): State<AppState>,
    Path((id, network_id)): Path<(String, String)>,
    session: Session,
) -> Result<ApiResponse<MutationResponse>, ApiError> {
    require_user(&session).await?;
    let client = resource_client_for_environment(&state, &id).await?;
    let networks = list_networks_on(&state, &client, None).await?;
    if networks.iter().any(|network| {
        (network.id.as_deref() == Some(network_id.as_str())
            || network.name.as_deref() == Some(network_id.as_str()))
            && network
                .name
                .as_deref()
                .is_some_and(|name| ["bridge", "host", "none", "ingress"].contains(&name))
    }) {
        return Err(ApiError::conflict("系统网络不可删除"));
    }
    let response_id = network_id.clone();
    remove_network_on(&state, &client, &network_id).await?;
    Ok(ApiResponse::success(MutationResponse {
        id: Some(response_id),
    }))
}

fn network_response(network: Network, container_count: Option<u64>) -> NetworkResponse {
    let name = network.name.unwrap_or_default();
    let system = ["bridge", "host", "none", "ingress"].contains(&name.as_str());
    let (subnet, gateway) = network
        .ipam
        .as_ref()
        .and_then(|ipam| ipam.config.as_ref())
        .and_then(|configs| configs.first())
        .map(|config| (config.subnet.clone(), config.gateway.clone()))
        .unwrap_or((None, None));
    NetworkResponse {
        id: network.id.unwrap_or_else(|| name.clone()),
        name,
        driver: network.driver,
        scope: network.scope,
        subnet,
        gateway,
        labels: network.labels.unwrap_or_default(),
        container_count,
        system,
    }
}
