use std::{
    collections::{HashMap, HashSet},
    fmt::{Display, Formatter},
    sync::Arc,
    time::{Duration, Instant, SystemTime, UNIX_EPOCH},
};

use bollard::{
    Docker,
    models::{ContainerStatsResponse, ContainerSummary},
    query_parameters::{ListContainersOptionsBuilder, StatsOptionsBuilder},
};
use futures_util::{StreamExt, stream};
use serde::{Deserialize, Serialize};
use tokio::sync::Mutex;

const DEFAULT_CACHE_TTL: Duration = Duration::from_secs(10);
const MAX_CPU_SAMPLES: usize = 512;
const MAX_CONCURRENT_STATS: usize = 8;

#[derive(Debug)]
pub enum DockerError {
    Connection(String),
    Request(String),
    InvalidResponse(String),
}

impl Display for DockerError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Connection(message) => write!(f, "Docker 连接失败: {message}"),
            Self::Request(message) => write!(f, "Docker 请求失败: {message}"),
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

#[derive(Clone, Debug, Default, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ContainerCounts {
    pub total: u64,
    pub running: u64,
    pub paused: u64,
    pub stopped: u64,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MemoryMetrics {
    pub used_bytes: u64,
    pub total_bytes: u64,
    pub percent: f64,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DockerSnapshot {
    pub docker_version: String,
    pub containers: ContainerCounts,
    pub cpu_percent: Option<f64>,
    pub memory: Option<MemoryMetrics>,
    pub metrics_error: Option<String>,
    pub metrics_collected_at: i64,
}

#[derive(Clone)]
pub struct DockerSnapshotCollector {
    client: DockerClient,
    state: Arc<Mutex<CollectorState>>,
    cache_ttl: Duration,
}

struct CollectorState {
    cache: Option<CachedSnapshot>,
    cpu_samples: HashMap<String, CpuSample>,
}

struct CachedSnapshot {
    created_at: Instant,
    snapshot: DockerSnapshot,
}

#[derive(Clone, Copy)]
struct CpuSample {
    total_usage: u64,
    system_usage: u64,
    online_cpus: u64,
    seen_at: Instant,
}

impl DockerSnapshotCollector {
    pub fn new(client: DockerClient) -> Self {
        Self {
            client,
            state: Arc::new(Mutex::new(CollectorState {
                cache: None,
                cpu_samples: HashMap::new(),
            })),
            cache_ttl: DEFAULT_CACHE_TTL,
        }
    }

    pub fn connect(host: &str) -> Result<Self, DockerError> {
        Ok(Self::new(DockerClient::connect(host)?))
    }

    pub async fn snapshot(&self) -> Result<DockerSnapshot, DockerError> {
        let mut state = self.state.lock().await;
        if let Some(cached) = state.cache.as_ref() {
            if cached.created_at.elapsed() < self.cache_ttl {
                return Ok(cached.snapshot.clone());
            }
        }

        let snapshot = self.collect(&mut state).await?;
        state.cache = Some(CachedSnapshot {
            created_at: Instant::now(),
            snapshot: snapshot.clone(),
        });
        Ok(snapshot)
    }

    async fn collect(&self, state: &mut CollectorState) -> Result<DockerSnapshot, DockerError> {
        let docker_version = self.client.version().await?;
        let info = self.client.info().await?;
        let container_ids = self.client.running_container_ids().await?;
        let active_ids: HashSet<&str> = container_ids.iter().map(String::as_str).collect();
        state
            .cpu_samples
            .retain(|id, _| active_ids.contains(id.as_str()));

        let results = stream::iter(container_ids.iter().cloned().map(|id| {
            let client = self.client.clone();
            async move { (id.clone(), client.stats(&id).await) }
        }))
        .buffer_unordered(MAX_CONCURRENT_STATS)
        .collect::<Vec<_>>()
        .await;

        let mut cpu_total = 0.0;
        let mut cpu_ready = false;
        let mut memory_used = 0u64;
        let mut memory_samples = 0u64;
        let mut failed_containers = HashSet::new();

        for (id, result) in results {
            let stats = match result {
                Ok(stats) => stats,
                Err(error) => {
                    tracing::debug!(container_id = %id, error = %error, "容器指标采集失败");
                    failed_containers.insert(id);
                    continue;
                }
            };

            let current_cpu = cpu_sample(&stats, info.cpu_count);
            if let Some(current) = current_cpu {
                if let Some(previous) = state.cpu_samples.insert(id.clone(), current) {
                    if let (Some(cpu_delta), Some(system_delta)) = (
                        current.total_usage.checked_sub(previous.total_usage),
                        current.system_usage.checked_sub(previous.system_usage),
                    ) {
                        if let Some(percent) = calculate_cpu_percent(
                            cpu_delta,
                            system_delta,
                            current.online_cpus,
                            info.cpu_count,
                        ) {
                            cpu_total += percent;
                            cpu_ready = true;
                        }
                    }
                }
            } else {
                failed_containers.insert(id.clone());
            }

            if let Some(usage) = memory_sample(&stats) {
                memory_used = memory_used.saturating_add(usage);
                memory_samples += 1;
            } else {
                failed_containers.insert(id);
            }
        }

        trim_cpu_samples(&mut state.cpu_samples);

        let cpu_percent = if container_ids.is_empty() {
            Some(0.0)
        } else if cpu_ready {
            Some(cpu_total.clamp(0.0, 100.0))
        } else {
            None
        };

        let memory = info.total_memory_bytes.and_then(|total_bytes| {
            if !container_ids.is_empty() && memory_samples == 0 {
                return None;
            }
            let used_bytes = if container_ids.is_empty() {
                0
            } else {
                memory_used
            };
            let percent = if total_bytes == 0 {
                0.0
            } else {
                (used_bytes as f64 / total_bytes as f64 * 100.0).clamp(0.0, 100.0)
            };
            Some(MemoryMetrics {
                used_bytes,
                total_bytes,
                percent,
            })
        });

        let metrics_error = if failed_containers.is_empty() {
            None
        } else {
            Some(format!(
                "{} 个运行中容器的指标暂不可用",
                failed_containers.len()
            ))
        };

        Ok(DockerSnapshot {
            docker_version,
            containers: ContainerCounts {
                total: info.total_containers,
                running: info.running_containers,
                paused: info.paused_containers,
                stopped: info.stopped_containers,
            },
            cpu_percent,
            memory,
            metrics_error,
            metrics_collected_at: now_unix_seconds(),
        })
    }
}

pub fn calculate_cpu_percent(
    cpu_delta: u64,
    system_delta: u64,
    online_cpus: u64,
    engine_cpus: u64,
) -> Option<f64> {
    if system_delta == 0 || online_cpus == 0 || engine_cpus == 0 {
        return None;
    }

    Some(
        (cpu_delta as f64 / system_delta as f64 * online_cpus as f64 * 100.0 / engine_cpus as f64)
            .clamp(0.0, 100.0),
    )
}

pub fn memory_used_bytes(usage: u64, stats: &HashMap<String, u64>) -> u64 {
    let cache = ["total_inactive_file", "inactive_file", "cache"]
        .into_iter()
        .find_map(|key| stats.get(key).copied())
        .unwrap_or(0);
    usage.saturating_sub(cache)
}

fn cpu_sample(stats: &ContainerStatsResponse, engine_cpus: u64) -> Option<CpuSample> {
    let cpu_stats = stats.cpu_stats.as_ref()?;
    let cpu_usage = cpu_stats.cpu_usage.as_ref()?;
    let total_usage = cpu_usage.total_usage?;
    let system_usage = cpu_stats.system_cpu_usage?;
    let online_cpus = cpu_stats
        .online_cpus
        .map(u64::from)
        .filter(|cpus| *cpus > 0)
        .unwrap_or(engine_cpus);

    Some(CpuSample {
        total_usage,
        system_usage,
        online_cpus,
        seen_at: Instant::now(),
    })
}

fn memory_sample(stats: &ContainerStatsResponse) -> Option<u64> {
    let memory_stats = stats.memory_stats.as_ref()?;
    let usage = memory_stats.usage?;
    let used = memory_stats
        .stats
        .as_ref()
        .map_or(usage, |cgroup_stats| memory_used_bytes(usage, cgroup_stats));
    Some(used)
}

fn trim_cpu_samples(samples: &mut HashMap<String, CpuSample>) {
    while samples.len() > MAX_CPU_SAMPLES {
        let oldest_id = samples
            .iter()
            .min_by_key(|(_, sample)| sample.seen_at)
            .map(|(id, _)| id.clone());
        let Some(oldest_id) = oldest_id else {
            break;
        };
        samples.remove(&oldest_id);
    }
}

fn non_negative(value: Option<i64>) -> u64 {
    value.unwrap_or_default().max(0) as u64
}

fn now_unix_seconds() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_secs().min(i64::MAX as u64) as i64)
        .unwrap_or_default()
}
