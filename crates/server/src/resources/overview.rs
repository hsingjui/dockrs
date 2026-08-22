use axum::extract::{Path, State};
use tower_sessions::Session;

use crate::{
    AppState,
    auth::require_user,
    environments::{self, EnvironmentKind},
    error::{ApiError, ApiResponse},
};

use super::{
    ContainerCountOverview, EnvironmentOverviewResponse, OverviewResources, ResourceClient,
    ResourceCountResponse, containers::list_containers_on, images::list_images_on,
    networks::list_networks_on, resource_client_for_environment, stacks::load_stacks,
    volumes::list_volumes_on,
};

#[utoipa::path(
    get,
    path = "/api/environments/{id}/overview",
    params(("id" = String, Path, description = "环境 ID")),
    responses((status = 200, body = EnvironmentOverviewResponse, description = "环境仪表盘概要"))
)]
pub(crate) async fn environment_overview(
    State(state): State<AppState>,
    Path(id): Path<String>,
    session: Session,
) -> Result<ApiResponse<EnvironmentOverviewResponse>, ApiError> {
    require_user(&session).await?;
    let environment = environments::environment_response_by_id(&state, &id).await?;
    let resources = match resource_client_for_environment(&state, &id).await {
        Ok(client) => load_overview_resources(&state, client).await,
        Err(_error) => {
            let message = if matches!(environment.kind, EnvironmentKind::Agent) {
                "远程 Agent 当前不可用"
            } else {
                "Docker Engine 当前不可用"
            };
            unavailable_overview(message)
        }
    };
    Ok(ApiResponse::success(EnvironmentOverviewResponse {
        environment,
        resources,
    }))
}

async fn load_overview_resources(state: &AppState, client: ResourceClient) -> OverviewResources {
    let (stacks, containers, images, volumes, networks) = tokio::join!(
        load_stacks(state, &client, None),
        list_containers_on(state, &client, true, None, None),
        list_images_on(state, &client, None, None),
        list_volumes_on(state, &client, None),
        list_networks_on(state, &client, None),
    );

    let stacks = match stacks {
        Ok(stacks) if stacks.available => ResourceCountResponse {
            total: Some(stacks.items.len() as u64),
            active: Some(
                stacks
                    .items
                    .iter()
                    .filter(|stack| stack.status == "running")
                    .count() as u64,
            ),
            error: None,
        },
        Ok(stacks) => ResourceCountResponse {
            total: None,
            active: None,
            error: stacks.reason,
        },
        Err(error) => resource_error(error),
    };
    let containers = match containers {
        Ok(containers) => {
            let mut counts = ContainerCountOverview {
                total: Some(containers.len() as u64),
                running: Some(0),
                paused: Some(0),
                stopped: Some(0),
                error: None,
            };
            for container in containers {
                let state = container.state.map(|state| state.to_string());
                match state.as_deref() {
                    Some("running") => counts.running = counts.running.map(|value| value + 1),
                    Some("paused") => counts.paused = counts.paused.map(|value| value + 1),
                    _ => counts.stopped = counts.stopped.map(|value| value + 1),
                }
            }
            counts
        }
        Err(error) => ContainerCountOverview {
            total: None,
            running: None,
            paused: None,
            stopped: None,
            error: Some(resource_error_message(error)),
        },
    };
    OverviewResources {
        stacks,
        containers,
        images: count_result(images),
        volumes: count_result(volumes),
        networks: count_result(networks),
    }
}

fn count_result<T>(result: Result<Vec<T>, ApiError>) -> ResourceCountResponse {
    match result {
        Ok(items) => ResourceCountResponse {
            total: Some(items.len() as u64),
            active: None,
            error: None,
        },
        Err(error) => ResourceCountResponse {
            total: None,
            active: None,
            error: Some(resource_error_from_api(error)),
        },
    }
}

fn resource_error(error: ApiError) -> ResourceCountResponse {
    ResourceCountResponse {
        total: None,
        active: None,
        error: Some(resource_error_from_api(error)),
    }
}

fn resource_error_from_api(_error: ApiError) -> String {
    "Docker 资源暂不可用".to_owned()
}

fn resource_error_message(_error: ApiError) -> String {
    "Docker 资源暂不可用".to_owned()
}

fn unavailable_overview(message: &str) -> OverviewResources {
    let count = || ResourceCountResponse {
        total: None,
        active: None,
        error: Some(message.to_owned()),
    };
    OverviewResources {
        stacks: count(),
        containers: ContainerCountOverview {
            total: None,
            running: None,
            paused: None,
            stopped: None,
            error: Some(message.to_owned()),
        },
        images: count(),
        volumes: count(),
        networks: count(),
    }
}
