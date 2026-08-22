use std::{
    collections::HashMap,
    fmt::{Display, Formatter},
    sync::Arc,
};

pub use bollard::models::{
    ContainerStatsResponse, ContainerSummary, ImageSummary, Network, NetworkCreateRequest, Service,
    ServiceSpec, Task, TaskState, Volume, VolumeCreateRequest,
};

use bollard::{
    Docker,
    query_parameters::{
        CreateImageOptionsBuilder, ListContainersOptionsBuilder, ListImagesOptionsBuilder,
        ListNetworksOptionsBuilder, ListServicesOptionsBuilder, ListVolumesOptionsBuilder,
        RemoveContainerOptionsBuilder, RemoveImageOptionsBuilder, RemoveVolumeOptionsBuilder,
        StatsOptionsBuilder,
    },
};
use futures_util::StreamExt;

mod snapshot;

pub use snapshot::{
    ContainerCounts, DockerSnapshot, DockerSnapshotCollector, MemoryMetrics, calculate_cpu_percent,
    memory_used_bytes,
};

#[derive(Debug)]
pub enum DockerError {
    Connection(String),
    Request(String),
    NotFound(String),
    Conflict(String),
    InvalidResponse(String),
}

impl Display for DockerError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Connection(message) => write!(f, "Docker 连接失败: {message}"),
            Self::Request(message) => write!(f, "Docker 请求失败: {message}"),
            Self::NotFound(message) => write!(f, "Docker 资源不存在: {message}"),
            Self::Conflict(message) => write!(f, "Docker 资源状态冲突: {message}"),
            Self::InvalidResponse(message) => write!(f, "Docker 响应无效: {message}"),
        }
    }
}

impl std::error::Error for DockerError {}

#[derive(Clone)]
pub struct DockerClient {
    inner: Arc<Docker>,
}

impl DockerClient {
    pub fn connect(host: &str) -> Result<Self, DockerError> {
        if host.trim().is_empty() {
            return Err(DockerError::Connection("Docker 地址不能为空".to_owned()));
        }

        Docker::connect_with_host(host)
            .map(|docker| Self {
                inner: Arc::new(docker),
            })
            .map_err(|error| DockerError::Connection(error.to_string()))
    }

    pub async fn version(&self) -> Result<String, DockerError> {
        self.inner
            .version()
            .await
            .map_err(|error| DockerError::Request(format!("读取 Docker 版本失败: {error}")))?
            .version
            .ok_or_else(|| DockerError::InvalidResponse("Docker 版本为空".to_owned()))
    }

    pub async fn info(&self) -> Result<DockerInfo, DockerError> {
        let info = self
            .inner
            .info()
            .await
            .map_err(|error| DockerError::Request(format!("读取 Docker 信息失败: {error}")))?;

        Ok(DockerInfo {
            total_containers: non_negative(info.containers),
            running_containers: non_negative(info.containers_running),
            paused_containers: non_negative(info.containers_paused),
            stopped_containers: non_negative(info.containers_stopped),
            cpu_count: non_negative(info.ncpu),
            total_memory_bytes: info.mem_total.and_then(|memory| u64::try_from(memory).ok()),
        })
    }

    pub async fn list_containers(&self, all: bool) -> Result<Vec<ContainerSummary>, DockerError> {
        self.inner
            .list_containers(Some(ListContainersOptionsBuilder::new().all(all).build()))
            .await
            .map_err(|error| DockerError::Request(format!("读取容器列表失败: {error}")))
    }

    pub async fn list_containers_filtered(
        &self,
        all: bool,
        status: Option<&str>,
        query: Option<&str>,
    ) -> Result<Vec<ContainerSummary>, DockerError> {
        let mut filters = HashMap::new();
        if let Some(status) = status.filter(|value| !value.trim().is_empty()) {
            filters.insert("status".to_owned(), vec![status.trim().to_owned()]);
        }
        let options = ListContainersOptionsBuilder::new()
            .all(all)
            .filters(&filters)
            .build();
        let mut containers = self
            .inner
            .list_containers(Some(options))
            .await
            .map_err(|error| map_bollard_error("读取容器列表失败", error))?;

        if let Some(query) = query.map(str::trim).filter(|value| !value.is_empty()) {
            let query = query.to_lowercase();
            containers.retain(|container| {
                container
                    .id
                    .as_deref()
                    .is_some_and(|value| value.to_lowercase().contains(&query))
                    || container
                        .image
                        .as_deref()
                        .is_some_and(|value| value.to_lowercase().contains(&query))
                    || container.names.as_ref().is_some_and(|names| {
                        names
                            .iter()
                            .any(|value| value.to_lowercase().contains(&query))
                    })
            });
        }
        Ok(containers)
    }

    pub async fn list_images_filtered(
        &self,
        query: Option<&str>,
        dangling: Option<bool>,
    ) -> Result<Vec<bollard::models::ImageSummary>, DockerError> {
        let mut filters = HashMap::new();
        if let Some(dangling) = dangling {
            filters.insert("dangling".to_owned(), vec![dangling.to_string()]);
        }
        let options = ListImagesOptionsBuilder::new()
            .all(true)
            .filters(&filters)
            .build();
        let mut images = self
            .inner
            .list_images(Some(options))
            .await
            .map_err(|error| map_bollard_error("读取镜像列表失败", error))?;
        if let Some(query) = query.map(str::trim).filter(|value| !value.is_empty()) {
            let query = query.to_lowercase();
            images.retain(|image| {
                image.id.to_lowercase().contains(&query)
                    || image
                        .repo_tags
                        .iter()
                        .any(|tag| tag.to_lowercase().contains(&query))
                    || image
                        .repo_digests
                        .iter()
                        .any(|digest| digest.to_lowercase().contains(&query))
            });
        }
        Ok(images)
    }

    pub async fn list_volumes_filtered(
        &self,
        query: Option<&str>,
    ) -> Result<Vec<Volume>, DockerError> {
        let mut filters = HashMap::new();
        if let Some(query) = query.map(str::trim).filter(|value| !value.is_empty()) {
            filters.insert("name".to_owned(), vec![query.to_owned()]);
        }
        let options = ListVolumesOptionsBuilder::new().filters(&filters).build();
        let volumes = self
            .inner
            .list_volumes(Some(options))
            .await
            .map_err(|error| map_bollard_error("读取存储卷列表失败", error))?;
        Ok(volumes.volumes.unwrap_or_default())
    }

    pub async fn list_networks_filtered(
        &self,
        query: Option<&str>,
    ) -> Result<Vec<bollard::models::Network>, DockerError> {
        let mut filters = HashMap::new();
        if let Some(query) = query.map(str::trim).filter(|value| !value.is_empty()) {
            filters.insert("name".to_owned(), vec![query.to_owned()]);
        }
        let options = ListNetworksOptionsBuilder::new().filters(&filters).build();
        let networks = self
            .inner
            .list_networks(Some(options))
            .await
            .map_err(|error| map_bollard_error("读取网络列表失败", error))?;
        Ok(networks)
    }

    pub async fn start_container(&self, id: &str) -> Result<(), DockerError> {
        self.inner
            .start_container(id, None)
            .await
            .map_err(|error| map_bollard_error("启动容器失败", error))
    }

    pub async fn stop_container(&self, id: &str) -> Result<(), DockerError> {
        self.inner
            .stop_container(id, None)
            .await
            .map_err(|error| map_bollard_error("停止容器失败", error))
    }

    pub async fn restart_container(&self, id: &str) -> Result<(), DockerError> {
        self.inner
            .restart_container(id, None)
            .await
            .map_err(|error| map_bollard_error("重启容器失败", error))
    }

    pub async fn pause_container(&self, id: &str) -> Result<(), DockerError> {
        self.inner
            .pause_container(id)
            .await
            .map_err(|error| map_bollard_error("暂停容器失败", error))
    }

    pub async fn unpause_container(&self, id: &str) -> Result<(), DockerError> {
        self.inner
            .unpause_container(id)
            .await
            .map_err(|error| map_bollard_error("恢复容器失败", error))
    }

    pub async fn remove_container(&self, id: &str) -> Result<(), DockerError> {
        let options = RemoveContainerOptionsBuilder::new().force(false).build();
        self.inner
            .remove_container(id, Some(options))
            .await
            .map_err(|error| map_bollard_error("删除容器失败", error))
    }

    pub async fn pull_image(&self, reference: &str) -> Result<(), DockerError> {
        let options = CreateImageOptionsBuilder::new()
            .from_image(reference)
            .build();
        let mut stream = self.inner.create_image(Some(options), None, None);
        while let Some(result) = stream.next().await {
            result.map_err(|error| map_bollard_error("拉取镜像失败", error))?;
        }
        Ok(())
    }

    pub async fn remove_image(&self, id: &str) -> Result<(), DockerError> {
        self.inner
            .remove_image(id, Some(RemoveImageOptionsBuilder::new().build()), None)
            .await
            .map_err(|error| map_bollard_error("删除镜像失败", error))
            .map(|_| ())
    }

    pub async fn create_volume(&self, config: VolumeCreateRequest) -> Result<Volume, DockerError> {
        self.inner
            .create_volume(config)
            .await
            .map_err(|error| map_bollard_error("创建存储卷失败", error))
    }

    pub async fn remove_volume(&self, name: &str) -> Result<(), DockerError> {
        self.inner
            .remove_volume(name, Some(RemoveVolumeOptionsBuilder::new().build()))
            .await
            .map_err(|error| map_bollard_error("删除存储卷失败", error))
    }

    pub async fn create_network(
        &self,
        config: NetworkCreateRequest,
    ) -> Result<String, DockerError> {
        self.inner
            .create_network(config)
            .await
            .map_err(|error| map_bollard_error("创建网络失败", error))
            .map(|response| response.id)
    }

    pub async fn remove_network(&self, id: &str) -> Result<(), DockerError> {
        self.inner
            .remove_network(id)
            .await
            .map_err(|error| map_bollard_error("删除网络失败", error))
    }

    pub async fn network_container_counts(&self) -> Result<HashMap<String, u64>, DockerError> {
        let containers = self.list_containers(true).await?;
        let mut counts = HashMap::new();
        for container in containers {
            let Some(networks) = container
                .network_settings
                .and_then(|settings| settings.networks)
            else {
                continue;
            };
            for network_id in networks.keys() {
                *counts.entry(network_id.clone()).or_insert(0) += 1;
            }
        }
        Ok(counts)
    }

    pub async fn is_swarm_manager(&self) -> Result<bool, DockerError> {
        let info = self
            .inner
            .info()
            .await
            .map_err(|error| map_bollard_error("读取 Swarm 状态失败", error))?;
        Ok(info
            .swarm
            .as_ref()
            .is_some_and(|swarm| swarm.control_available == Some(true)))
    }

    pub async fn list_services(&self) -> Result<Vec<Service>, DockerError> {
        self.inner
            .list_services(Some(ListServicesOptionsBuilder::new().status(true).build()))
            .await
            .map_err(|error| map_bollard_error("读取 Swarm 服务失败", error))
    }

    pub async fn list_tasks(&self) -> Result<Vec<Task>, DockerError> {
        self.inner
            .list_tasks(None)
            .await
            .map_err(|error| map_bollard_error("读取 Swarm 任务失败", error))
    }

    async fn running_container_ids(&self) -> Result<Vec<String>, DockerError> {
        let containers = self.list_containers(false).await?;
        Ok(containers
            .into_iter()
            .filter_map(|container| container.id)
            .collect())
    }

    pub async fn stats(&self, id: &str) -> Result<ContainerStatsResponse, DockerError> {
        let mut stream = self.inner.stats(
            id,
            Some(
                StatsOptionsBuilder::new()
                    .stream(false)
                    .one_shot(true)
                    .build(),
            ),
        );

        match stream.next().await {
            Some(Ok(stats)) => Ok(stats),
            Some(Err(error)) => Err(DockerError::Request(format!(
                "读取容器 {id} 指标失败: {error}"
            ))),
            None => Err(DockerError::InvalidResponse(format!(
                "容器 {id} 没有返回指标"
            ))),
        }
    }
}

#[derive(Clone, Copy, Debug)]
pub struct DockerInfo {
    pub total_containers: u64,
    pub running_containers: u64,
    pub paused_containers: u64,
    pub stopped_containers: u64,
    pub cpu_count: u64,
    pub total_memory_bytes: Option<u64>,
}

fn map_bollard_error(operation: &str, error: bollard::errors::Error) -> DockerError {
    match error {
        bollard::errors::Error::DockerResponseServerError {
            status_code: 404,
            message,
        } => DockerError::NotFound(format!("{operation}: {message}")),
        bollard::errors::Error::DockerResponseServerError {
            status_code: 409,
            message,
        } => DockerError::Conflict(format!("{operation}: {message}")),
        error => DockerError::Request(format!("{operation}: {error}")),
    }
}

fn non_negative(value: Option<i64>) -> u64 {
    value.unwrap_or_default().max(0) as u64
}
