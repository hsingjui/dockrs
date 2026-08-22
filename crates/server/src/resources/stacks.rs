use std::collections::HashMap;

use axum::extract::{Path, Query, State};
use dockrs_agent::AgentOperation;
use dockrs_docker::{Service, Task, TaskState};
use tower_sessions::Session;

use crate::{
    AppState,
    auth::require_user,
    error::{ApiError, ApiResponse},
};

use super::{
    ResourceClient, StackListQuery, StackListResponse, StackResponse, StackServiceResponse,
    StackTaskResponse, resource_client_for_environment, run_agent, run_docker, validate_filter,
    validate_resource_name,
};

async fn is_swarm_manager_on(state: &AppState, client: &ResourceClient) -> Result<bool, ApiError> {
    match client {
        ResourceClient::Local(client) => {
            run_docker(client.clone(), |client| async move {
                client.is_swarm_manager().await
            })
            .await
        }
        ResourceClient::Agent(connection) => {
            run_agent(state, connection, AgentOperation::IsSwarmManager).await
        }
    }
}

async fn list_services_on(
    state: &AppState,
    client: &ResourceClient,
) -> Result<Vec<Service>, ApiError> {
    match client {
        ResourceClient::Local(client) => {
            run_docker(client.clone(), |client| async move {
                client.list_services().await
            })
            .await
        }
        ResourceClient::Agent(connection) => {
            run_agent(state, connection, AgentOperation::ListServices).await
        }
    }
}

async fn list_tasks_on(state: &AppState, client: &ResourceClient) -> Result<Vec<Task>, ApiError> {
    match client {
        ResourceClient::Local(client) => {
            run_docker(
                client.clone(),
                |client| async move { client.list_tasks().await },
            )
            .await
        }
        ResourceClient::Agent(connection) => {
            run_agent(state, connection, AgentOperation::ListTasks).await
        }
    }
}

#[utoipa::path(
    get,
    path = "/api/environments/{id}/stacks",
    params(("id" = String, Path, description = "环境 ID")),
    responses((status = 200, body = StackListResponse, description = "Swarm Stack 列表"))
)]
pub(crate) async fn list_stacks(
    State(state): State<AppState>,
    Path(id): Path<String>,
    Query(query): Query<StackListQuery>,
    session: Session,
) -> Result<ApiResponse<StackListResponse>, ApiError> {
    require_user(&session).await?;
    let query_text = validate_filter(query.q)?;
    let client = resource_client_for_environment(&state, &id).await?;
    Ok(ApiResponse::success(
        load_stacks(&state, &client, query_text.as_deref()).await?,
    ))
}

#[utoipa::path(
    get,
    path = "/api/environments/{id}/stacks/{stack_name}",
    params(
        ("id" = String, Path, description = "环境 ID"),
        ("stack_name" = String, Path, description = "Stack 名称")
    ),
    responses((status = 200, body = StackResponse, description = "Stack 详情"))
)]
pub(crate) async fn get_stack(
    State(state): State<AppState>,
    Path((id, stack_name)): Path<(String, String)>,
    session: Session,
) -> Result<ApiResponse<StackResponse>, ApiError> {
    require_user(&session).await?;
    let stack_name = validate_resource_name(stack_name, "Stack 名称")?;
    let client = resource_client_for_environment(&state, &id).await?;
    let stacks = load_stacks(&state, &client, Some(&stack_name)).await?;
    let response = stacks
        .items
        .into_iter()
        .find(|stack| stack.name == stack_name)
        .ok_or_else(|| ApiError::not_found("Stack 不存在"))?;
    Ok(ApiResponse::success(response))
}

pub(super) async fn load_stacks(
    state: &AppState,
    client: &ResourceClient,
    query: Option<&str>,
) -> Result<StackListResponse, ApiError> {
    if !is_swarm_manager_on(state, client).await? {
        return Ok(StackListResponse {
            available: false,
            reason: Some("当前 Docker 环境不是 Swarm manager".to_owned()),
            items: Vec::new(),
        });
    }
    let services = list_services_on(state, client).await?;
    let tasks = list_tasks_on(state, client).await?;
    let items = aggregate_stacks(services, tasks, query);
    Ok(StackListResponse {
        available: true,
        reason: None,
        items,
    })
}

#[derive(Default)]
struct StackAccumulator {
    services: Vec<StackServiceResponse>,
    tasks: Vec<StackTaskResponse>,
    running_replicas: u64,
    failed_tasks: u64,
}

fn aggregate_stacks(
    services: Vec<Service>,
    tasks: Vec<Task>,
    query: Option<&str>,
) -> Vec<StackResponse> {
    let mut service_stack = HashMap::new();
    let mut stacks: HashMap<String, StackAccumulator> = HashMap::new();
    for service in services {
        let Some(spec) = service.spec.as_ref() else {
            continue;
        };
        let Some(stack_name) = spec
            .labels
            .as_ref()
            .and_then(|labels| labels.get("com.docker.stack.namespace"))
            .cloned()
        else {
            continue;
        };
        if query.is_some_and(|query| !stack_name.to_lowercase().contains(&query.to_lowercase())) {
            continue;
        }
        let service_id = service.id.clone().unwrap_or_default();
        let service_name = spec.name.clone().unwrap_or_else(|| service_id.clone());
        let desired = desired_replicas(&service);
        service_stack.insert(service_id.clone(), stack_name.clone());
        stacks
            .entry(stack_name)
            .or_default()
            .services
            .push(StackServiceResponse {
                id: service_id,
                name: service_name,
                desired_replicas: desired,
                running_replicas: 0,
            });
    }

    for task in tasks {
        let Some(service_id) = task.service_id.as_ref() else {
            continue;
        };
        let Some(stack_name) = service_stack.get(service_id) else {
            continue;
        };
        let Some(stack) = stacks.get_mut(stack_name) else {
            continue;
        };
        let state = task.status.as_ref().and_then(|status| status.state);
        let error = task.status.as_ref().and_then(|status| status.err.clone());
        let failed = matches!(
            state,
            Some(TaskState::FAILED | TaskState::REJECTED | TaskState::ORPHANED)
        ) || error.is_some();
        if state == Some(TaskState::RUNNING) {
            stack.running_replicas += 1;
            if let Some(service) = stack
                .services
                .iter_mut()
                .find(|service| service.id == *service_id)
            {
                service.running_replicas += 1;
            }
        }
        if failed {
            stack.failed_tasks += 1;
        }
        stack.tasks.push(StackTaskResponse {
            id: task.id.unwrap_or_default(),
            name: task.name,
            state: state.map(|state| state.to_string()),
            error,
            container_id: task
                .status
                .and_then(|status| status.container_status)
                .and_then(|status| status.container_id),
        });
    }

    let mut items = stacks
        .into_iter()
        .map(|(name, stack)| {
            let desired_replicas = stack
                .services
                .iter()
                .map(|service| service.desired_replicas)
                .sum::<u64>();
            let status = if stack.failed_tasks > 0 {
                "degraded"
            } else if stack.running_replicas >= desired_replicas {
                "running"
            } else {
                "pending"
            };
            StackResponse {
                name,
                status: status.to_owned(),
                service_count: stack.services.len() as u64,
                desired_replicas,
                running_replicas: stack.running_replicas,
                failed_tasks: stack.failed_tasks,
                services: stack.services,
                tasks: stack.tasks,
            }
        })
        .collect::<Vec<_>>();
    items.sort_by_key(|stack| stack.name.to_lowercase());
    items
}

fn desired_replicas(service: &Service) -> u64 {
    if let Some(desired) = service
        .service_status
        .as_ref()
        .and_then(|status| status.desired_tasks)
    {
        return desired;
    }

    let Some(mode) = service.spec.as_ref().and_then(|spec| spec.mode.as_ref()) else {
        return 0;
    };
    mode.replicated
        .as_ref()
        .and_then(|replicated| replicated.replicas)
        .map(|replicas| replicas.max(0) as u64)
        .or_else(|| {
            mode.replicated_job
                .as_ref()
                .and_then(|job| job.total_completions)
                .map(|replicas| replicas.max(0) as u64)
        })
        .unwrap_or(0)
}
