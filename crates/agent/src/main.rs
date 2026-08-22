use std::time::Duration;

use dockrs_agent::{AgentErrorKind, AgentMessage, PROTOCOL_VERSION, execute};
use dockrs_docker::{DockerClient, DockerSnapshotCollector};
use futures_util::{SinkExt, StreamExt};
use tokio::time::{interval, sleep, timeout};
use tokio_tungstenite::{
    connect_async,
    tungstenite::{Message, client::IntoClientRequest},
};
use tracing::{info, warn};

const COMMAND_TIMEOUT: Duration = Duration::from_secs(30);
const RECONNECT_DELAY: Duration = Duration::from_secs(3);
const HEARTBEAT_INTERVAL: Duration = Duration::from_secs(30);
const SNAPSHOT_TIMEOUT: Duration = Duration::from_secs(5);

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::builder()
                .with_default_directive(tracing_subscriber::filter::LevelFilter::INFO.into())
                .from_env_lossy(),
        )
        .init();

    let token = required_env("DOCKRS_AGENT_TOKEN")?;
    let agent_id = env_or("DOCKRS_AGENT_ID", "agent");
    let name = env_or("DOCKRS_AGENT_NAME", &agent_id);
    let endpoint = env_or("DOCKRS_AGENT_ENDPOINT", "docker");
    let server_url = stream_url(
        &env_or("DOCKRS_SERVER_URL", "ws://127.0.0.1:8080"),
        &agent_id,
    );
    let docker_host = env_or("DOCKER_HOST", "unix:///var/run/docker.sock");
    let collector = DockerSnapshotCollector::connect(&docker_host)?;
    let docker = collector.client();

    info!(agent_id = %agent_id, server = %server_url, "Dockrs Agent started");
    loop {
        if let Err(error) = connect_once(
            &server_url,
            &token,
            &agent_id,
            &name,
            &endpoint,
            &docker,
            &collector,
        )
        .await
        {
            warn!(error = %error, "Agent connection ended");
        }
        sleep(RECONNECT_DELAY).await;
    }
}

async fn connect_once(
    server_url: &str,
    token: &str,
    agent_id: &str,
    name: &str,
    endpoint: &str,
    docker: &DockerClient,
    collector: &DockerSnapshotCollector,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let mut request = server_url.to_owned().into_client_request()?;
    request
        .headers_mut()
        .insert("x-dockrs-agent-token", token.parse()?);
    let (mut socket, _) = connect_async(request).await?;
    send_message(
        &mut socket,
        &AgentMessage::Hello {
            protocol_version: PROTOCOL_VERSION,
            agent_id: agent_id.to_owned(),
            name: name.to_owned(),
            endpoint: endpoint.to_owned(),
        },
    )
    .await?;
    if let Ok(Ok(snapshot)) = timeout(SNAPSHOT_TIMEOUT, collector.snapshot()).await {
        send_message(
            &mut socket,
            &AgentMessage::Snapshot {
                snapshot: Box::new(snapshot),
            },
        )
        .await?;
    }

    let mut heartbeat = interval(HEARTBEAT_INTERVAL);
    loop {
        tokio::select! {
            incoming = socket.next() => {
                match incoming {
                    Some(Ok(Message::Text(text))) => {
                        let message: AgentMessage = serde_json::from_str(text.as_ref())?;
                        if let AgentMessage::Command { request_id, operation } = message {
                            let result = timeout(COMMAND_TIMEOUT, execute(docker, *operation)).await;
                            let response = match result {
                                Ok(Ok(data)) => AgentMessage::Result {
                                    request_id,
                                    ok: true,
                                    data: Some(data),
                                    error: None,
                                },
                                Ok(Err(error)) => AgentMessage::Result {
                                    request_id,
                                    ok: false,
                                    data: None,
                                    error: Some(error),
                                },
                                Err(_) => AgentMessage::Result {
                                    request_id,
                                    ok: false,
                                    data: None,
                                    error: Some(AgentErrorKind::Request),
                                },
                            };
                            send_message(&mut socket, &response).await?;
                        }
                    }
                    Some(Ok(Message::Ping(payload))) => {
                        socket.send(Message::Pong(payload)).await?;
                    }
                    Some(Ok(Message::Close(_))) | None => return Ok(()),
                    Some(Ok(_)) => {}
                    Some(Err(error)) => return Err(error.into()),
                }
            }
            _ = heartbeat.tick() => {
                send_message(&mut socket, &AgentMessage::Heartbeat).await?;
                match timeout(SNAPSHOT_TIMEOUT, collector.snapshot()).await {
                    Ok(Ok(snapshot)) => send_message(&mut socket, &AgentMessage::Snapshot { snapshot: Box::new(snapshot) }).await?,
                    Ok(Err(error)) => warn!(error = %error, "Agent snapshot failed"),
                    Err(_) => warn!("Agent snapshot timed out"),
                }
            }
        }
    }
}

async fn send_message(
    socket: &mut tokio_tungstenite::WebSocketStream<
        tokio_tungstenite::MaybeTlsStream<tokio::net::TcpStream>,
    >,
    message: &AgentMessage,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let text = serde_json::to_string(message)?;
    socket.send(Message::Text(text.into())).await?;
    Ok(())
}

fn required_env(name: &str) -> Result<String, Box<dyn std::error::Error>> {
    std::env::var(name).map_err(|_| format!("缺少环境变量 {name}").into())
}

fn env_or(name: &str, default: &str) -> String {
    std::env::var(name).unwrap_or_else(|_| default.to_owned())
}

fn stream_url(server_url: &str, agent_id: &str) -> String {
    let server_url = server_url.trim_end_matches('/');
    let server_url = server_url
        .strip_prefix("http://")
        .map(|rest| format!("ws://{rest}"))
        .or_else(|| {
            server_url
                .strip_prefix("https://")
                .map(|rest| format!("wss://{rest}"))
        })
        .unwrap_or_else(|| server_url.to_owned());
    format!(
        "{server_url}/api/environments/agents/{}/stream",
        encode_path_segment(agent_id)
    )
}

fn encode_path_segment(value: &str) -> String {
    let mut encoded = String::with_capacity(value.len());
    for byte in value.bytes() {
        if byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_') {
            encoded.push(byte as char);
        } else {
            encoded.push_str(&format!("%{byte:02X}"));
        }
    }
    encoded
}
