use axum::extract::{Path, Query, State};
use dockrs_agent::AgentOperation;
use dockrs_docker::ImageSummary;
use tower_sessions::Session;

use crate::{
    AppState,
    auth::require_user,
    error::{ApiError, ApiJson, ApiResponse},
};

use super::{
    ImageResponse, MutationResponse, PullImageRequest, ResourceClient, ResourceListQuery,
    resource_client_for_environment, run_agent, run_agent_unit, run_docker, validate_filter,
};

pub(super) async fn list_images_on(
    state: &AppState,
    client: &ResourceClient,
    query: Option<&str>,
    dangling: Option<bool>,
) -> Result<Vec<ImageSummary>, ApiError> {
    match client {
        ResourceClient::Local(client) => {
            run_docker(client.clone(), |client| async move {
                client.list_images_filtered(query, dangling).await
            })
            .await
        }
        ResourceClient::Agent(connection) => {
            run_agent(
                state,
                connection,
                AgentOperation::ListImages {
                    query: query.map(str::to_owned),
                    dangling,
                },
            )
            .await
        }
    }
}

async fn pull_image_on(
    state: &AppState,
    client: &ResourceClient,
    reference: &str,
) -> Result<(), ApiError> {
    match client {
        ResourceClient::Local(client) => {
            let reference = reference.to_owned();
            run_docker(client.clone(), |client| async move {
                client.pull_image(&reference).await
            })
            .await
        }
        ResourceClient::Agent(connection) => {
            run_agent_unit(
                state,
                connection,
                AgentOperation::PullImage {
                    reference: reference.to_owned(),
                },
            )
            .await
        }
    }
}

async fn remove_image_on(
    state: &AppState,
    client: &ResourceClient,
    id: &str,
) -> Result<(), ApiError> {
    match client {
        ResourceClient::Local(client) => {
            let id = id.to_owned();
            run_docker(client.clone(), |client| async move {
                client.remove_image(&id).await
            })
            .await
        }
        ResourceClient::Agent(connection) => {
            run_agent_unit(
                state,
                connection,
                AgentOperation::RemoveImage { id: id.to_owned() },
            )
            .await
        }
    }
}

#[utoipa::path(
    get,
    path = "/api/environments/{id}/images",
    params(("id" = String, Path, description = "环境 ID")),
    responses((status = 200, body = [ImageResponse], description = "镜像列表"))
)]
pub(crate) async fn list_images(
    State(state): State<AppState>,
    Path(id): Path<String>,
    Query(query): Query<ResourceListQuery>,
    session: Session,
) -> Result<ApiResponse<Vec<ImageResponse>>, ApiError> {
    require_user(&session).await?;
    let query_text = validate_filter(query.q)?;
    let client = resource_client_for_environment(&state, &id).await?;
    let images = list_images_on(&state, &client, query_text.as_deref(), query.dangling).await?;
    Ok(ApiResponse::success(
        images.into_iter().map(image_response).collect(),
    ))
}

#[utoipa::path(
    post,
    path = "/api/environments/{id}/images/pull",
    params(("id" = String, Path, description = "环境 ID")),
    request_body = PullImageRequest,
    responses((status = 200, body = MutationResponse, description = "镜像拉取成功"))
)]
pub(crate) async fn pull_image(
    State(state): State<AppState>,
    Path(id): Path<String>,
    session: Session,
    ApiJson(request): ApiJson<PullImageRequest>,
) -> Result<ApiResponse<MutationResponse>, ApiError> {
    require_user(&session).await?;
    let reference = validate_reference(request.reference)?;
    let client = resource_client_for_environment(&state, &id).await?;
    pull_image_on(&state, &client, &reference).await?;
    Ok(ApiResponse::success(MutationResponse { id: None }))
}

#[utoipa::path(
    delete,
    path = "/api/environments/{id}/images/{image_id}",
    params(
        ("id" = String, Path, description = "环境 ID"),
        ("image_id" = String, Path, description = "镜像 ID")
    ),
    responses((status = 200, body = MutationResponse, description = "删除成功"))
)]
pub(crate) async fn remove_image(
    State(state): State<AppState>,
    Path((id, image_id)): Path<(String, String)>,
    session: Session,
) -> Result<ApiResponse<MutationResponse>, ApiError> {
    require_user(&session).await?;
    let client = resource_client_for_environment(&state, &id).await?;
    let response_id = image_id.clone();
    remove_image_on(&state, &client, &image_id).await?;
    Ok(ApiResponse::success(MutationResponse {
        id: Some(response_id),
    }))
}

fn validate_reference(value: String) -> Result<String, ApiError> {
    let value = value.trim().to_owned();
    if value.is_empty() || value.chars().count() > 256 || value.chars().any(char::is_control) {
        return Err(ApiError::unprocessable_entity("镜像引用无效"));
    }
    Ok(value)
}

fn image_response(image: ImageSummary) -> ImageResponse {
    let repository = image.repo_tags.first().and_then(|tag| {
        tag.rsplit_once(':')
            .map(|(repository, _)| repository.to_owned())
    });
    ImageResponse {
        id: image.id,
        repository,
        dangling: image.repo_tags.is_empty(),
        tags: image.repo_tags,
        digests: image.repo_digests,
        created: image.created,
        size: image.size,
        containers: image.containers,
    }
}
