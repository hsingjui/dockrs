use std::{
    collections::{HashMap, HashSet},
    sync::Arc,
    time::{Duration, Instant, SystemTime, UNIX_EPOCH},
};

use bollard::models::ContainerStatsResponse;
use futures_util::{StreamExt, stream};
use serde::{Deserialize, Serialize};
use tokio::sync::Mutex;

use crate::{DockerClient, DockerError};

const DEFAULT_CACHE_TTL: Duration = Duration::from_secs(10);
const MAX_CPU_SAMPLES: usize = 512;
const MAX_CONCURRENT_STATS: usize = 8;

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

    pub fn client(&self) -> DockerClient {
        self.client.clone()
    }

    pub async fn snapshot(&self) -> Result<DockerSnapshot, DockerError> {
        let mut state = self.state.lock().await;
        if let Some(cached) = state.cache.as_ref()
            && cached.created_at.elapsed() < self.cache_ttl
        {
            return Ok(cached.snapshot.clone());
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
            if let Some(current) = current_cpu
                && let Some(previous) = state.cpu_samples.insert(id.clone(), current)
                && let (Some(cpu_delta), Some(system_delta)) = (
                    current.total_usage.checked_sub(previous.total_usage),
                    current.system_usage.checked_sub(previous.system_usage),
                )
                && let Some(percent) = calculate_cpu_percent(
                    cpu_delta,
                    system_delta,
                    current.online_cpus,
                    info.cpu_count,
                )
            {
                cpu_total += percent;
                cpu_ready = true;
            } else if current_cpu.is_none() {
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

fn now_unix_seconds() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_secs().min(i64::MAX as u64) as i64)
        .unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;

    use super::{calculate_cpu_percent, memory_used_bytes};

    #[test]
    fn cpu_percent_rejects_invalid_denominators() {
        assert!(calculate_cpu_percent(1, 0, 1, 1).is_none());
        assert!(calculate_cpu_percent(1, 1, 0, 1).is_none());
        assert!(calculate_cpu_percent(1, 1, 1, 0).is_none());
    }

    #[test]
    fn cpu_percent_scales_and_clamps() {
        assert_eq!(calculate_cpu_percent(1, 4, 2, 2), Some(25.0));
        assert_eq!(calculate_cpu_percent(4, 1, 8, 1), Some(100.0));
    }

    #[test]
    fn memory_usage_subtracts_the_preferred_cache_metric() {
        let stats = HashMap::from([
            ("total_inactive_file".to_owned(), 40),
            ("inactive_file".to_owned(), 80),
            ("cache".to_owned(), 90),
        ]);

        assert_eq!(memory_used_bytes(100, &stats), 60);
        assert_eq!(memory_used_bytes(100, &HashMap::new()), 100);
        assert_eq!(
            memory_used_bytes(10, &HashMap::from([("cache".to_owned(), 20)])),
            0
        );
    }
}
