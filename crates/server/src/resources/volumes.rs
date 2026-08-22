use axum::extract::{Path, Query, State};
use dockrs_agent::AgentOperation;
use dockrs_docker::{Volume, VolumeCreateRequest};
use tower_sessions::Session;

use crate::{
    AppState,
    auth::require_user,
    error::{ApiError, ApiJson, ApiResponse},
};

use super::{
    CreateVolumeRequest, MutationResponse, ResourceClient, ResourceListQuery, VolumeResponse,
    resource_client_for_environment, run_agent, run_agent_unit, run_docker, validate_filter,
    validate_optional_name, validate_options, validate_resource_name,
};

pub(super) async fn list_volumes_on(
    state: &AppState,
    client: &ResourceClient,
    query: Option<&str>,
) -> Result<Vec<Volume>, ApiError> {
    match client {
        ResourceClient::Local(client) => {
            run_docker(client.clone(), |client| async move {
                client.list_volumes_filtered(query).await
            })
            .await
        }
        ResourceClient::Agent(connection) => {
            run_agent(
                state,
                connection,
                AgentOperation::ListVolumes {
                    query: query.map(str::to_owned),
                },
            )
            .await
        }
    }
}

async fn create_volume_on(
    state: &AppState,
    client: &ResourceClient,
    config: VolumeCreateRequest,
) -> Result<Volume, ApiError> {
    match client {
        ResourceClient::Local(client) => {
            run_docker(client.clone(), |client| async move {
                client.create_volume(config).await
            })
            .await
        }
        ResourceClient::Agent(connection) => {
            run_agent(state, connection, AgentOperation::CreateVolume { config }).await
        }
    }
}

async fn remove_volume_on(
    state: &AppState,
    client: &ResourceClient,
    name: &str,
) -> Result<(), ApiError> {
    match client {
        ResourceClient::Local(client) => {
            let name = name.to_owned();
            run_docker(client.clone(), |client| async move {
                client.remove_volume(&name).await
            })
            .await
        }
        ResourceClient::Agent(connection) => {
            run_agent_unit(
                state,
                connection,
                AgentOperation::RemoveVolume {
                    name: name.to_owned(),
                },
            )
            .await
        }
    }
}

#[utoipa::path(
    get,
    path = "/api/environments/{id}/volumes",
    params(("id" = String, Path, description = "环境 ID")),
    responses((status = 200, body = [VolumeResponse], description = "存储卷列表"))
)]
pub(crate) async fn list_volumes(
    State(state): State<AppState>,
    Path(id): Path<String>,
    Query(query): Query<ResourceListQuery>,
    session: Session,
) -> Result<ApiResponse<Vec<VolumeResponse>>, ApiError> {
    require_user(&session).await?;
    let query_text = validate_filter(query.q)?;
    let client = resource_client_for_environment(&state, &id).await?;
    let volumes = list_volumes_on(&state, &client, query_text.as_deref()).await?;
    Ok(ApiResponse::success(
        volumes.into_iter().map(volume_response).collect(),
    ))
}

#[utoipa::path(
    post,
    path = "/api/environments/{id}/volumes",
    params(("id" = String, Path, description = "环境 ID")),
    request_body = CreateVolumeRequest,
    responses((status = 200, body = VolumeResponse, description = "创建成功"))
)]
pub(crate) async fn create_volume(
    State(state): State<AppState>,
    Path(id): Path<String>,
    session: Session,
    ApiJson(request): ApiJson<CreateVolumeRequest>,
) -> Result<ApiResponse<VolumeResponse>, ApiError> {
    require_user(&session).await?;
    let name = validate_resource_name(request.name, "存储卷名称")?;
    let driver = validate_optional_name(request.driver, "存储卷 driver")?;
    let options = validate_options(request.options)?;
    let config = VolumeCreateRequest {
        name: Some(name),
        driver,
        driver_opts: options,
        ..Default::default()
    };
    let client = resource_client_for_environment(&state, &id).await?;
    let volume = create_volume_on(&state, &client, config).await?;
    Ok(ApiResponse::success(volume_response(volume)))
}

#[utoipa::path(
    delete,
    path = "/api/environments/{id}/volumes/{volume_name}",
    params(
        ("id" = String, Path, description = "环境 ID"),
        ("volume_name" = String, Path, description = "存储卷名称")
    ),
    responses((status = 200, body = MutationResponse, description = "删除成功"))
)]
pub(crate) async fn remove_volume(
    State(state): State<AppState>,
    Path((id, volume_name)): Path<(String, String)>,
    session: Session,
) -> Result<ApiResponse<MutationResponse>, ApiError> {
    require_user(&session).await?;
    let client = resource_client_for_environment(&state, &id).await?;
    let response_id = volume_name.clone();
    remove_volume_on(&state, &client, &volume_name).await?;
    Ok(ApiResponse::success(MutationResponse {
        id: Some(response_id),
    }))
}

fn volume_response(volume: Volume) -> VolumeResponse {
    VolumeResponse {
        name: volume.name,
        driver: volume.driver,
        scope: volume.scope.map(|scope| scope.to_string()),
        mountpoint: volume.mountpoint,
        labels: volume.labels,
        usage_containers: volume
            .usage_data
            .and_then(|usage| u64::try_from(usage.ref_count).ok()),
    }
}
