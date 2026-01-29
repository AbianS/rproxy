//! Docker container metrics collection

use anyhow::Result;
use bollard::container::StatsOptions;
use bollard::Docker;
use futures::StreamExt;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::Arc;
use tokio::sync::Mutex;

/// Container resource statistics
#[derive(Debug, Clone, Default)]
#[allow(dead_code)]
pub struct ContainerStats {
    pub cpu_percent: f64,
    pub memory_mb: f64,
    pub memory_peak_mb: f64,
    pub network_rx_bytes: u64,
    pub network_tx_bytes: u64,
    pub io_read_bytes: u64,
    pub io_write_bytes: u64,
}

/// Metrics collector for a Docker container
pub struct MetricsCollector {
    docker: Docker,
    container_name: String,
    running: Arc<AtomicBool>,
    samples: Arc<Mutex<Vec<ContainerStats>>>,
    idle_memory_mb: Arc<AtomicU64>,
}

impl MetricsCollector {
    /// Create a new metrics collector
    pub fn new(container_name: &str) -> Result<Self> {
        let docker = Docker::connect_with_local_defaults()?;

        Ok(Self {
            docker,
            container_name: container_name.to_string(),
            running: Arc::new(AtomicBool::new(false)),
            samples: Arc::new(Mutex::new(Vec::new())),
            idle_memory_mb: Arc::new(AtomicU64::new(0)),
        })
    }

    /// Measure idle memory before test starts
    pub async fn measure_idle(&self) -> Result<f64> {
        let stats = get_container_stats(&self.docker, &self.container_name).await?;
        self.idle_memory_mb
            .store(stats.memory_mb.to_bits(), Ordering::SeqCst);
        Ok(stats.memory_mb)
    }

    /// Start collecting metrics in background
    pub fn start(&self) -> tokio::task::JoinHandle<()> {
        let docker = self.docker.clone();
        let container = self.container_name.clone();
        let running = self.running.clone();
        let samples = self.samples.clone();

        running.store(true, Ordering::SeqCst);

        tokio::spawn(async move {
            let options = StatsOptions {
                stream: true,
                one_shot: false,
            };

            let mut stream = docker.stats(&container, Some(options));

            while running.load(Ordering::SeqCst) {
                tokio::select! {
                    Some(result) = stream.next() => {
                        if let Ok(stats) = result {
                            let sample = parse_docker_stats(&stats);
                            samples.lock().await.push(sample);
                        }
                    }
                    _ = tokio::time::sleep(std::time::Duration::from_millis(100)) => {
                        if !running.load(Ordering::SeqCst) {
                            break;
                        }
                    }
                }
            }
        })
    }

    /// Stop collecting and return aggregated results
    pub async fn stop(&self) -> MetricsSummary {
        self.running.store(false, Ordering::SeqCst);

        // Give the collector time to finish
        tokio::time::sleep(std::time::Duration::from_millis(200)).await;

        let samples = self.samples.lock().await;
        let idle_memory = f64::from_bits(self.idle_memory_mb.load(Ordering::SeqCst));

        if samples.is_empty() {
            return MetricsSummary::default();
        }

        // Calculate aggregates
        let cpu_values: Vec<f64> = samples.iter().map(|s| s.cpu_percent).collect();
        let mem_values: Vec<f64> = samples.iter().map(|s| s.memory_mb).collect();

        let cpu_avg = cpu_values.iter().sum::<f64>() / cpu_values.len() as f64;
        let cpu_peak = cpu_values.iter().cloned().fold(0.0, f64::max);

        let mem_avg = mem_values.iter().sum::<f64>() / mem_values.len() as f64;
        let mem_peak = mem_values.iter().cloned().fold(0.0, f64::max);

        // Network totals (last sample has cumulative values)
        let last = samples.last().unwrap();
        let first = samples.first().unwrap();

        MetricsSummary {
            cpu_percent_avg: cpu_avg,
            cpu_percent_peak: cpu_peak,
            memory_mb_idle: idle_memory,
            memory_mb_avg: mem_avg,
            memory_mb_peak: mem_peak,
            network_rx_bytes: last.network_rx_bytes.saturating_sub(first.network_rx_bytes),
            network_tx_bytes: last.network_tx_bytes.saturating_sub(first.network_tx_bytes),
            io_read_bytes: last.io_read_bytes.saturating_sub(first.io_read_bytes),
            io_write_bytes: last.io_write_bytes.saturating_sub(first.io_write_bytes),
            sample_count: samples.len(),
        }
    }
}

/// Aggregated metrics summary
#[derive(Debug, Clone, Default, serde::Serialize, serde::Deserialize)]
pub struct MetricsSummary {
    pub cpu_percent_avg: f64,
    pub cpu_percent_peak: f64,
    pub memory_mb_idle: f64,
    pub memory_mb_avg: f64,
    pub memory_mb_peak: f64,
    pub network_rx_bytes: u64,
    pub network_tx_bytes: u64,
    pub io_read_bytes: u64,
    pub io_write_bytes: u64,
    pub sample_count: usize,
}

/// Get current stats for a container
pub async fn get_container_stats(docker: &Docker, container: &str) -> Result<ContainerStats> {
    let options = StatsOptions {
        stream: false,
        one_shot: true,
    };

    let mut stream = docker.stats(container, Some(options));

    if let Some(result) = stream.next().await {
        let stats = result?;
        return Ok(parse_docker_stats(&stats));
    }

    anyhow::bail!("No stats available for container: {}", container)
}

/// Parse Docker stats into our format
fn parse_docker_stats(stats: &bollard::container::Stats) -> ContainerStats {
    // CPU calculation
    let cpu_percent = calculate_cpu_percent(stats);

    // Memory
    let memory_mb = stats
        .memory_stats
        .usage
        .map(|u| u as f64 / 1024.0 / 1024.0)
        .unwrap_or(0.0);

    let memory_peak_mb = stats
        .memory_stats
        .max_usage
        .map(|u| u as f64 / 1024.0 / 1024.0)
        .unwrap_or(0.0);

    // Network
    let (rx_bytes, tx_bytes) = stats
        .networks
        .as_ref()
        .map(|networks| {
            networks
                .values()
                .fold((0u64, 0u64), |(rx, tx), net| {
                    (rx + net.rx_bytes, tx + net.tx_bytes)
                })
        })
        .unwrap_or((0, 0));

    // I/O
    let (read_bytes, write_bytes) = stats
        .blkio_stats
        .io_service_bytes_recursive
        .as_ref()
        .map(|ios| {
            ios.iter().fold((0u64, 0u64), |(r, w), io| {
                match io.op.as_str() {
                    "read" | "Read" => (r + io.value, w),
                    "write" | "Write" => (r, w + io.value),
                    _ => (r, w),
                }
            })
        })
        .unwrap_or((0, 0));

    ContainerStats {
        cpu_percent,
        memory_mb,
        memory_peak_mb,
        network_rx_bytes: rx_bytes,
        network_tx_bytes: tx_bytes,
        io_read_bytes: read_bytes,
        io_write_bytes: write_bytes,
    }
}

/// Calculate CPU percentage from Docker stats
fn calculate_cpu_percent(stats: &bollard::container::Stats) -> f64 {
    let cpu_delta = stats.cpu_stats.cpu_usage.total_usage as f64
        - stats.precpu_stats.cpu_usage.total_usage as f64;

    let system_delta = stats.cpu_stats.system_cpu_usage.unwrap_or(0) as f64
        - stats.precpu_stats.system_cpu_usage.unwrap_or(0) as f64;

    if system_delta > 0.0 && cpu_delta > 0.0 {
        let num_cpus = stats
            .cpu_stats
            .online_cpus
            .unwrap_or(1) as f64;

        (cpu_delta / system_delta) * num_cpus * 100.0
    } else {
        0.0
    }
}

/// Check if a container is running
#[allow(dead_code)]
pub async fn is_container_running(docker: &Docker, container: &str) -> bool {
    docker
        .inspect_container(container, None)
        .await
        .map(|info| {
            info.state
                .and_then(|s| s.running)
                .unwrap_or(false)
        })
        .unwrap_or(false)
}

/// Wait for a container to be healthy
#[allow(dead_code)]
pub async fn wait_for_healthy(docker: &Docker, container: &str, timeout_secs: u64) -> Result<()> {
    let start = std::time::Instant::now();
    let timeout = std::time::Duration::from_secs(timeout_secs);

    loop {
        if start.elapsed() > timeout {
            anyhow::bail!("Timeout waiting for container {} to be healthy", container);
        }

        if let Ok(info) = docker.inspect_container(container, None).await {
            if let Some(state) = info.state {
                if let Some(ref health) = state.health {
                    if health.status == Some(bollard::secret::HealthStatusEnum::HEALTHY) {
                        return Ok(());
                    }
                } else if state.running == Some(true) {
                    // If no health check configured, just check if running
                    return Ok(());
                }
            }
        }

        tokio::time::sleep(std::time::Duration::from_millis(500)).await;
    }
}
