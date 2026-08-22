use std::{collections::HashMap, future::Future, time::Duration};

use axum::{
    Router,
    routing::{delete, get, post},
};
use dockrs_agent::AgentOperation;
use dockrs_docker::{DockerClient, DockerError};
use serde::{Deserialize, Serialize, de::DeserializeOwned};
use sqlx::FromRow;
use tokio::time::timeout;
use utoipa::ToSchema;

use crate::{AppState, agent, environments::EnvironmentResponse, error::ApiError};

pub(crate) mod containers;
pub(crate) mod images;
pub(crate) mod networks;
pub(crate) mod overview;
pub(crate) mod stacks;
pub(crate) mod volumes;

pub(crate) use containers::{container_action, list_containers, remove_container};
pub(crate) use images::{list_images, pull_image, remove_image};
pub(crate) use networks::{create_network, list_networks, remove_network};
pub(crate) use overview::environment_overview;
pub(crate) use stacks::{get_stack, list_stacks};
pub(crate) use volumes::{create_volume, list_volumes, remove_volume};

const DOCKER_REQUEST_TIMEOUT: Duration = Duration::from_secs(8);
const MAX_FILTER_LENGTH: usize = 128;

#[derive(Debug, FromRow)]
struct EnvironmentTarget {
    kind: String,
}

#[derive(Debug, Deserialize)]
pub struct ContainerListQuery {
    pub all: Option<bool>,
    pub status: Option<String>,
    pub q: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct ResourceListQuery {
    pub q: Option<String>,
    pub dangling: Option<bool>,
}

#[derive(Debug, Deserialize)]
pub struct StackListQuery {
    pub q: Option<String>,
}

#[derive(Debug, Deserialize, ToSchema)]
pub struct PullImageRequest {
    pub reference: String,
}

#[derive(Debug, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct CreateVolumeRequest {
    pub name: String,
    pub driver: Option<String>,
    pub options: Option<HashMap<String, String>>,
}

#[derive(Debug, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct CreateNetworkRequest {
    pub name: String,
    pub driver: Option<String>,
    pub internal: Option<bool>,
}

#[derive(Debug, Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct MutationResponse {
    pub id: Option<String>,
}

#[derive(Debug, Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct ContainerPortResponse {
    pub container_port: u16,
    pub host_port: Option<u16>,
    pub protocol: String,
}

#[derive(Debug, Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct ContainerResponse {
    pub id: String,
    pub name: String,
    pub names: Vec<String>,
    pub image: Option<String>,
    pub state: String,
    pub status: Option<String>,
    pub created: Option<i64>,
    pub ports: Vec<ContainerPortResponse>,
    pub stack: Option<String>,
}

#[derive(Debug, Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct ImageResponse {
    pub id: String,
    pub repository: Option<String>,
    pub tags: Vec<String>,
    pub digests: Vec<String>,
    pub created: i64,
    pub size: i64,
    pub containers: i64,
    pub dangling: bool,
}

#[derive(Debug, Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct VolumeResponse {
    pub name: String,
    pub driver: String,
    pub scope: Option<String>,
    pub mountpoint: String,
    pub labels: HashMap<String, String>,
    pub usage_containers: Option<u64>,
}

#[derive(Debug, Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct NetworkResponse {
    pub id: String,
    pub name: String,
    pub driver: Option<String>,
    pub scope: Option<String>,
    pub subnet: Option<String>,
    pub gateway: Option<String>,
    pub labels: HashMap<String, String>,
    pub container_count: Option<u64>,
    pub system: bool,
}

#[derive(Debug, Clone, Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct StackServiceResponse {
    pub id: String,
    pub name: String,
    pub desired_replicas: u64,
    pub running_replicas: u64,
}

#[derive(Debug, Clone, Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct StackTaskResponse {
    pub id: String,
    pub name: Option<String>,
    pub state: Option<String>,
    pub error: Option<String>,
    pub container_id: Option<String>,
}

#[derive(Debug, Clone, Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct StackResponse {
    pub name: String,
    pub status: String,
    pub service_count: u64,
    pub desired_replicas: u64,
    pub running_replicas: u64,
    pub failed_tasks: u64,
    pub services: Vec<StackServiceResponse>,
    pub tasks: Vec<StackTaskResponse>,
}

#[derive(Debug, Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct StackListResponse {
    pub available: bool,
    pub reason: Option<String>,
    pub items: Vec<StackResponse>,
}

#[derive(Debug, Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct ResourceCountResponse {
    pub total: Option<u64>,
    pub active: Option<u64>,
    pub error: Option<String>,
}

#[derive(Debug, Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct ContainerCountOverview {
    pub total: Option<u64>,
    pub running: Option<u64>,
    pub paused: Option<u64>,
    pub stopped: Option<u64>,
    pub error: Option<String>,
}

#[derive(Debug, Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct OverviewResources {
    pub stacks: ResourceCountResponse,
    pub containers: ContainerCountOverview,
    pub images: ResourceCountResponse,
    pub volumes: ResourceCountResponse,
    pub networks: ResourceCountResponse,
}

#[derive(Debug, Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct EnvironmentOverviewResponse {
    pub environment: EnvironmentResponse,
    pub resources: OverviewResources,
}

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/environments/{id}/overview", get(environment_overview))
        .route("/environments/{id}/containers", get(list_containers))
        .route(
            "/environments/{id}/containers/{container_id}/{action}",
            post(container_action),
        )
        .route(
            "/environments/{id}/containers/{container_id}",
            delete(remove_container),
        )
        .route("/environments/{id}/images", get(list_images))
        .route("/environments/{id}/images/pull", post(pull_image))
        .route("/environments/{id}/images/{image_id}", delete(remove_image))
        .route(
            "/environments/{id}/volumes",
            get(list_volumes).post(create_volume),
        )
        .route(
            "/environments/{id}/volumes/{volume_name}",
            delete(remove_volume),
        )
        .route(
            "/environments/{id}/networks",
            get(list_networks).post(create_network),
        )
        .route(
            "/environments/{id}/networks/{network_id}",
            delete(remove_network),
        )
        .route("/environments/{id}/stacks", get(list_stacks))
        .route("/environments/{id}/stacks/{stack_name}", get(get_stack))
}

#[derive(Clone)]
enum ResourceClient {
    Local(DockerClient),
    Agent(agent::AgentConnection),
}

async fn environment_target(state: &AppState, id: &str) -> Result<EnvironmentTarget, ApiError> {
    sqlx::query_as::<_, EnvironmentTarget>("SELECT kind FROM environments WHERE id = ?")
        .bind(id)
        .fetch_optional(&state.pool)
        .await
        .map_err(|error| {
            tracing::error!(environment_id = %id, error = %error, "查询资源环境失败");
            ApiError::internal("读取环境失败，请稍后重试")
        })?
        .ok_or_else(|| ApiError::not_found("环境不存在"))
}

async fn resource_client_for_environment(
    state: &AppState,
    id: &str,
) -> Result<ResourceClient, ApiError> {
    let target = environment_target(state, id).await?;
    match target.kind.as_str() {
        "local" => state
            .local_docker
            .as_ref()
            .map(|collector| ResourceClient::Local(collector.client()))
            .ok_or_else(|| ApiError::bad_gateway("Docker Engine 当前不可用")),
        "agent" => agent::connection_for_environment(state, id)
            .await
            .map(ResourceClient::Agent),
        _ => Err(ApiError::internal("环境类型无效")),
    }
}

async fn run_agent<T: DeserializeOwned>(
    state: &AppState,
    connection: &agent::AgentConnection,
    operation: AgentOperation,
) -> Result<T, ApiError> {
    let value = agent::request(state, connection, operation).await?;
    serde_json::from_value(value).map_err(|error| {
        tracing::error!(error = %error, "解析 Agent 资源响应失败");
        ApiError::bad_gateway("远程 Agent 返回了无效响应")
    })
}

async fn run_agent_unit(
    state: &AppState,
    connection: &agent::AgentConnection,
    operation: AgentOperation,
) -> Result<(), ApiError> {
    let _: serde_json::Value = run_agent(state, connection, operation).await?;
    Ok(())
}

async fn run_docker<T, F, Fut>(client: DockerClient, operation: F) -> Result<T, ApiError>
where
    F: FnOnce(DockerClient) -> Fut,
    Fut: Future<Output = Result<T, DockerError>>,
{
    match timeout(DOCKER_REQUEST_TIMEOUT, operation(client)).await {
        Ok(Ok(value)) => Ok(value),
        Ok(Err(error)) => Err(map_docker_error(error)),
        Err(_) => Err(ApiError::bad_gateway("Docker Engine 请求超时")),
    }
}

fn map_docker_error(error: DockerError) -> ApiError {
    match error {
        DockerError::NotFound(_) => ApiError::not_found("Docker 资源不存在"),
        DockerError::Conflict(_) => ApiError::conflict("资源当前正在使用或状态不允许此操作"),
        DockerError::Connection(_) | DockerError::Request(_) | DockerError::InvalidResponse(_) => {
            ApiError::bad_gateway("Docker Engine 当前不可用")
        }
    }
}

fn validate_filter(value: Option<String>) -> Result<Option<String>, ApiError> {
    let Some(value) = value else {
        return Ok(None);
    };
    let value = value.trim().to_owned();
    if value.chars().count() > MAX_FILTER_LENGTH {
        return Err(ApiError::unprocessable_entity("筛选条件过长"));
    }
    Ok((!value.is_empty()).then_some(value))
}

fn validate_resource_name(value: String, label: &str) -> Result<String, ApiError> {
    let value = value.trim().to_owned();
    if value.is_empty()
        || value.chars().count() > MAX_FILTER_LENGTH
        || value.chars().any(char::is_control)
    {
        return Err(ApiError::unprocessable_entity(format!("{label}无效")));
    }
    Ok(value)
}

fn validate_optional_name(value: Option<String>, label: &str) -> Result<Option<String>, ApiError> {
    value
        .map(|value| validate_resource_name(value, label))
        .transpose()
}

fn validate_options(
    options: Option<HashMap<String, String>>,
) -> Result<Option<HashMap<String, String>>, ApiError> {
    let Some(options) = options else {
        return Ok(None);
    };
    if options.len() > 32
        || options.iter().any(|(key, value)| {
            key.is_empty()
                || key.chars().count() > MAX_FILTER_LENGTH
                || value.chars().count() > MAX_FILTER_LENGTH
                || key.chars().any(char::is_control)
                || value.chars().any(char::is_control)
        })
    {
        return Err(ApiError::unprocessable_entity("资源选项无效"));
    }
    Ok(Some(options))
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;

    use super::{MAX_FILTER_LENGTH, validate_filter, validate_optional_name, validate_options};

    #[test]
    fn filter_values_are_trimmed_and_blank_values_are_ignored() {
        assert_eq!(
            validate_filter(Some("  nginx  ".to_owned())).unwrap(),
            Some("nginx".to_owned())
        );
        assert_eq!(validate_filter(Some("   ".to_owned())).unwrap(), None);
        assert!(validate_filter(Some("x".repeat(MAX_FILTER_LENGTH + 1))).is_err());
    }

    #[test]
    fn optional_resource_names_and_options_are_validated() {
        assert_eq!(
            validate_optional_name(Some("  data  ".to_owned()), "卷").unwrap(),
            Some("data".to_owned())
        );
        assert_eq!(validate_optional_name(None, "卷").unwrap(), None);
        assert!(validate_optional_name(Some("bad\nname".to_owned()), "卷").is_err());

        let valid = HashMap::from([("type".to_owned(), "nfs".to_owned())]);
        assert_eq!(validate_options(Some(valid.clone())).unwrap(), Some(valid));

        let invalid = HashMap::from([(String::new(), "value".to_owned())]);
        assert!(validate_options(Some(invalid)).is_err());

        let too_many = (0..33)
            .map(|index| (index.to_string(), "value".to_owned()))
            .collect();
        assert!(validate_options(Some(too_many)).is_err());
    }
}
