use dockrs_docker::{
    DockerClient, DockerError, DockerSnapshot, NetworkCreateRequest, VolumeCreateRequest,
};
use serde::{Deserialize, Serialize};
use serde_json::Value;

pub const PROTOCOL_VERSION: u16 = 1;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum AgentMessage {
    Hello {
        protocol_version: u16,
        agent_id: String,
        name: String,
        endpoint: String,
    },
    Heartbeat,
    Snapshot {
        snapshot: Box<DockerSnapshot>,
    },
    Command {
        request_id: String,
        operation: Box<AgentOperation>,
    },
    Result {
        request_id: String,
        ok: bool,
        data: Option<Value>,
        error: Option<AgentErrorKind>,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", content = "payload", rename_all = "snake_case")]
pub enum AgentOperation {
    ListContainers {
        all: bool,
        status: Option<String>,
        query: Option<String>,
    },
    ListImages {
        query: Option<String>,
        dangling: Option<bool>,
    },
    ListVolumes {
        query: Option<String>,
    },
    ListNetworks {
        query: Option<String>,
    },
    NetworkContainerCounts,
    IsSwarmManager,
    ListServices,
    ListTasks,
    ContainerAction {
        id: String,
        action: String,
    },
    RemoveContainer {
        id: String,
    },
    PullImage {
        reference: String,
    },
    RemoveImage {
        id: String,
    },
    CreateVolume {
        config: VolumeCreateRequest,
    },
    RemoveVolume {
        name: String,
    },
    CreateNetwork {
        config: NetworkCreateRequest,
    },
    RemoveNetwork {
        id: String,
    },
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AgentErrorKind {
    NotFound,
    Conflict,
    Request,
    Connection,
    InvalidResponse,
}

pub async fn execute(
    client: &DockerClient,
    operation: AgentOperation,
) -> Result<Value, AgentErrorKind> {
    match operation {
        AgentOperation::ListContainers { all, status, query } => encode(
            client
                .list_containers_filtered(all, status.as_deref(), query.as_deref())
                .await?,
        ),
        AgentOperation::ListImages { query, dangling } => encode(
            client
                .list_images_filtered(query.as_deref(), dangling)
                .await?,
        ),
        AgentOperation::ListVolumes { query } => {
            encode(client.list_volumes_filtered(query.as_deref()).await?)
        }
        AgentOperation::ListNetworks { query } => {
            encode(client.list_networks_filtered(query.as_deref()).await?)
        }
        AgentOperation::NetworkContainerCounts => encode(client.network_container_counts().await?),
        AgentOperation::IsSwarmManager => encode(client.is_swarm_manager().await?),
        AgentOperation::ListServices => encode(client.list_services().await?),
        AgentOperation::ListTasks => encode(client.list_tasks().await?),
        AgentOperation::ContainerAction { id, action } => {
            match action.as_str() {
                "start" => client.start_container(&id).await?,
                "stop" => client.stop_container(&id).await?,
                "restart" => client.restart_container(&id).await?,
                "pause" => client.pause_container(&id).await?,
                "unpause" => client.unpause_container(&id).await?,
                _ => return Err(AgentErrorKind::Request),
            }
            encode(())
        }
        AgentOperation::RemoveContainer { id } => encode(client.remove_container(&id).await?),
        AgentOperation::PullImage { reference } => encode(client.pull_image(&reference).await?),
        AgentOperation::RemoveImage { id } => encode(client.remove_image(&id).await?),
        AgentOperation::CreateVolume { config } => encode(client.create_volume(config).await?),
        AgentOperation::RemoveVolume { name } => encode(client.remove_volume(&name).await?),
        AgentOperation::CreateNetwork { config } => encode(client.create_network(config).await?),
        AgentOperation::RemoveNetwork { id } => encode(client.remove_network(&id).await?),
    }
}

fn encode<T: Serialize>(value: T) -> Result<Value, AgentErrorKind> {
    serde_json::to_value(value).map_err(|_| AgentErrorKind::InvalidResponse)
}

impl From<DockerError> for AgentErrorKind {
    fn from(error: DockerError) -> Self {
        match error {
            DockerError::NotFound(_) => Self::NotFound,
            DockerError::Conflict(_) => Self::Conflict,
            DockerError::Connection(_) => Self::Connection,
            DockerError::Request(_) => Self::Request,
            DockerError::InvalidResponse(_) => Self::InvalidResponse,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{AgentErrorKind, AgentMessage, AgentOperation, DockerError};

    #[test]
    fn command_messages_round_trip_through_json() {
        let message = AgentMessage::Command {
            request_id: "request-1".to_owned(),
            operation: Box::new(AgentOperation::ContainerAction {
                id: "container-1".to_owned(),
                action: "restart".to_owned(),
            }),
        };
        let encoded = serde_json::to_string(&message).expect("Agent 消息应可序列化");
        let decoded: AgentMessage = serde_json::from_str(&encoded).expect("Agent 消息应可反序列化");

        match decoded {
            AgentMessage::Command {
                request_id,
                operation,
            } => {
                assert_eq!(request_id, "request-1");
                assert!(matches!(
                    *operation,
                    AgentOperation::ContainerAction { id, action }
                        if id == "container-1" && action == "restart"
                ));
            }
            _ => panic!("反序列化后消息类型不正确"),
        }
    }

    #[test]
    fn docker_errors_map_to_the_matching_agent_error_kind() {
        assert!(matches!(
            AgentErrorKind::from(DockerError::NotFound("missing".to_owned())),
            AgentErrorKind::NotFound
        ));
        assert!(matches!(
            AgentErrorKind::from(DockerError::Conflict("busy".to_owned())),
            AgentErrorKind::Conflict
        ));
        assert!(matches!(
            AgentErrorKind::from(DockerError::Connection("offline".to_owned())),
            AgentErrorKind::Connection
        ));
        assert!(matches!(
            AgentErrorKind::from(DockerError::Request("failed".to_owned())),
            AgentErrorKind::Request
        ));
        assert!(matches!(
            AgentErrorKind::from(DockerError::InvalidResponse("invalid".to_owned())),
            AgentErrorKind::InvalidResponse
        ));
    }
}
