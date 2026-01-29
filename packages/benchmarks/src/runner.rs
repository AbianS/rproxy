//! Benchmark runner - executes load tests against proxies

use crate::config::{ProxyConfig, ScenarioConfig};
use crate::metrics::MetricsCollector;
use crate::workload::WorkloadGenerator;
use anyhow::Result;
use chrono::{DateTime, Utc};
use colored::Colorize;
use hdrhistogram::Histogram;
use indicatif::{ProgressBar, ProgressStyle};
use serde::{Deserialize, Serialize};
use std::path::Path;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::Mutex;

/// Benchmark result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BenchmarkResult {
    pub run_id: String,
    pub timestamp: DateTime<Utc>,
    pub proxy: String,
    pub scenario: String,
    pub config: ScenarioSummary,
    pub results: ResultsSummary,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScenarioSummary {
    pub duration_secs: u64,
    pub target_rps: u32,
    pub concurrency: u32,
    pub warmup_secs: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResultsSummary {
    pub throughput: ThroughputMetrics,
    pub latency_ms: LatencyMetrics,
    pub memory_mb: MemoryMetrics,
    pub cpu_percent: CpuMetrics,
    pub network: NetworkMetrics,
    pub errors: ErrorMetrics,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ThroughputMetrics {
    pub total_requests: u64,
    pub successful: u64,
    pub failed: u64,
    pub requests_per_second: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LatencyMetrics {
    pub min: f64,
    pub max: f64,
    pub mean: f64,
    pub p50: f64,
    pub p90: f64,
    pub p95: f64,
    pub p99: f64,
    pub p999: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemoryMetrics {
    pub idle: f64,
    pub peak: f64,
    pub average: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CpuMetrics {
    pub peak: f64,
    pub average: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NetworkMetrics {
    pub rx_bytes: u64,
    pub tx_bytes: u64,
    pub rx_mb: f64,
    pub tx_mb: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ErrorMetrics {
    pub timeout: u64,
    pub connection_refused: u64,
    pub status_4xx: u64,
    pub status_5xx: u64,
    pub other: u64,
}

impl BenchmarkResult {
    pub fn save(&self, path: &Path) -> Result<()> {
        let json = serde_json::to_string_pretty(self)?;
        std::fs::write(path, json)?;
        Ok(())
    }

    pub fn load(path: &Path) -> Result<Self> {
        let content = std::fs::read_to_string(path)?;
        Ok(serde_json::from_str(&content)?)
    }
}

/// Shared state for benchmark workers
struct BenchmarkState {
    histogram: Mutex<Histogram<u64>>,
    successful: AtomicU64,
    failed: AtomicU64,
    errors: Mutex<ErrorMetrics>,
}

impl BenchmarkState {
    fn new() -> Self {
        Self {
            histogram: Mutex::new(Histogram::new(3).unwrap()),
            successful: AtomicU64::new(0),
            failed: AtomicU64::new(0),
            errors: Mutex::new(ErrorMetrics {
                timeout: 0,
                connection_refused: 0,
                status_4xx: 0,
                status_5xx: 0,
                other: 0,
            }),
        }
    }
}

/// Run a benchmark scenario
pub async fn run_scenario(
    config: &ScenarioConfig,
    proxy_name: &str,
    warmup: bool,
    upstream_override: Option<&str>,
) -> Result<BenchmarkResult> {
    let proxy = ProxyConfig::for_proxy(proxy_name);
    let base_url = upstream_override
        .map(|s| s.to_string())
        .unwrap_or(proxy.url.clone());

    // Initialize metrics collector
    let metrics = MetricsCollector::new(&proxy.container_name)?;

    // Measure idle memory
    println!("  {} idle memory...", "Measuring".dimmed());
    let idle_memory = metrics.measure_idle().await.unwrap_or(0.0);
    println!("  {} {:.1} MB", "Idle memory:".dimmed(), idle_memory);

    // Start metrics collection
    let metrics_handle = metrics.start();

    // Initialize workload generator
    let workload = WorkloadGenerator::new(&config.request, &base_url)?;

    // Create shared state
    let state = Arc::new(BenchmarkState::new());

    // Warmup phase
    if warmup && config.load.warmup_secs > 0 {
        println!("  {} ({} seconds)...", "Warmup".yellow(), config.load.warmup_secs);
        run_load_phase(
            &workload,
            config.load.target_rps / 2, // Half load during warmup
            config.load.concurrency / 2,
            Duration::from_secs(config.load.warmup_secs),
            &state,
            false, // No progress bar for warmup
        )
        .await?;

        // Reset state after warmup
        state.successful.store(0, Ordering::SeqCst);
        state.failed.store(0, Ordering::SeqCst);
        *state.histogram.lock().await = Histogram::new(3).unwrap();
        *state.errors.lock().await = ErrorMetrics {
            timeout: 0,
            connection_refused: 0,
            status_4xx: 0,
            status_5xx: 0,
            other: 0,
        };
    }

    // Main test phase
    let start_time = Instant::now();

    // Check for special test types
    if let Some(ref stress_config) = config.stress {
        println!("  {} stress test...", "Running".green());
        run_stress_test(
            &workload,
            stress_config,
            config.load.concurrency,
            &state,
        )
        .await?;
    } else if let Some(ref burst_config) = config.burst {
        println!("  {} burst test...", "Running".green());
        run_burst_test(&workload, burst_config, &state).await?;
    } else {
        println!(
            "  {} load test ({} req/s for {} seconds)...",
            "Running".green(),
            config.load.target_rps,
            config.load.duration_secs
        );
        run_load_phase(
            &workload,
            config.load.target_rps,
            config.load.concurrency,
            Duration::from_secs(config.load.duration_secs),
            &state,
            true,
        )
        .await?;
    }

    let elapsed = start_time.elapsed();

    // Stop metrics collection
    let metrics_summary = metrics.stop().await;
    metrics_handle.abort();

    // Build results
    let histogram = state.histogram.lock().await;
    let successful = state.successful.load(Ordering::SeqCst);
    let failed = state.failed.load(Ordering::SeqCst);
    let total = successful + failed;
    let errors = state.errors.lock().await.clone();

    let result = BenchmarkResult {
        run_id: format!(
            "{}-{}-{}",
            Utc::now().format("%Y%m%d-%H%M%S"),
            config.name,
            proxy_name
        ),
        timestamp: Utc::now(),
        proxy: proxy_name.to_string(),
        scenario: config.name.clone(),
        config: ScenarioSummary {
            duration_secs: config.load.duration_secs,
            target_rps: config.load.target_rps,
            concurrency: config.load.concurrency,
            warmup_secs: config.load.warmup_secs,
        },
        results: ResultsSummary {
            throughput: ThroughputMetrics {
                total_requests: total,
                successful,
                failed,
                requests_per_second: total as f64 / elapsed.as_secs_f64(),
            },
            latency_ms: LatencyMetrics {
                min: histogram.min() as f64 / 1000.0,
                max: histogram.max() as f64 / 1000.0,
                mean: histogram.mean() / 1000.0,
                p50: histogram.value_at_percentile(50.0) as f64 / 1000.0,
                p90: histogram.value_at_percentile(90.0) as f64 / 1000.0,
                p95: histogram.value_at_percentile(95.0) as f64 / 1000.0,
                p99: histogram.value_at_percentile(99.0) as f64 / 1000.0,
                p999: histogram.value_at_percentile(99.9) as f64 / 1000.0,
            },
            memory_mb: MemoryMetrics {
                idle: metrics_summary.memory_mb_idle,
                peak: metrics_summary.memory_mb_peak,
                average: metrics_summary.memory_mb_avg,
            },
            cpu_percent: CpuMetrics {
                peak: metrics_summary.cpu_percent_peak,
                average: metrics_summary.cpu_percent_avg,
            },
            network: NetworkMetrics {
                rx_bytes: metrics_summary.network_rx_bytes,
                tx_bytes: metrics_summary.network_tx_bytes,
                rx_mb: metrics_summary.network_rx_bytes as f64 / 1024.0 / 1024.0,
                tx_mb: metrics_summary.network_tx_bytes as f64 / 1024.0 / 1024.0,
            },
            errors,
        },
    };

    // Print summary
    print_result_summary(&result);

    Ok(result)
}

/// Run a sustained load phase
async fn run_load_phase(
    workload: &WorkloadGenerator,
    target_rps: u32,
    concurrency: u32,
    duration: Duration,
    state: &Arc<BenchmarkState>,
    show_progress: bool,
) -> Result<()> {
    let interval = if target_rps > 0 {
        Duration::from_secs_f64(1.0 / target_rps as f64)
    } else {
        Duration::from_millis(1)
    };

    let progress = if show_progress {
        let pb = ProgressBar::new(duration.as_secs());
        pb.set_style(
            ProgressStyle::default_bar()
                .template("  [{elapsed_precise}] [{bar:40.cyan/blue}] {pos}/{len}s | {msg}")
                .unwrap()
                .progress_chars("=>-"),
        );
        Some(pb)
    } else {
        None
    };

    let start = Instant::now();
    let mut handles = Vec::new();

    // Spawn worker tasks
    for _ in 0..concurrency {
        let workload = workload.clone();
        let state = state.clone();
        let duration = duration;
        let interval = interval;

        let handle = tokio::spawn(async move {
            let worker_start = Instant::now();

            while worker_start.elapsed() < duration {
                let req_start = Instant::now();

                match workload.send_request().await {
                    Ok(status) => {
                        let latency_us = req_start.elapsed().as_micros() as u64;
                        state.histogram.lock().await.record(latency_us).ok();

                        if status.is_success() {
                            state.successful.fetch_add(1, Ordering::Relaxed);
                        } else if status.is_client_error() {
                            state.failed.fetch_add(1, Ordering::Relaxed);
                            state.errors.lock().await.status_4xx += 1;
                        } else if status.is_server_error() {
                            state.failed.fetch_add(1, Ordering::Relaxed);
                            state.errors.lock().await.status_5xx += 1;
                        }
                    }
                    Err(e) => {
                        state.failed.fetch_add(1, Ordering::Relaxed);
                        let mut errors = state.errors.lock().await;
                        if e.is_timeout() {
                            errors.timeout += 1;
                        } else if e.is_connect() {
                            errors.connection_refused += 1;
                        } else {
                            errors.other += 1;
                        }
                    }
                }

                // Rate limiting
                let elapsed = req_start.elapsed();
                if elapsed < interval {
                    tokio::time::sleep(interval - elapsed).await;
                }
            }
        });

        handles.push(handle);
    }

    // Update progress bar
    if let Some(ref pb) = progress {
        while start.elapsed() < duration {
            tokio::time::sleep(Duration::from_secs(1)).await;
            pb.set_position(start.elapsed().as_secs());
            let rps = state.successful.load(Ordering::Relaxed) as f64 / start.elapsed().as_secs_f64();
            pb.set_message(format!("{:.0} req/s", rps));
        }
        pb.finish_with_message("done");
    }

    // Wait for all workers
    for handle in handles {
        let _ = handle.await;
    }

    Ok(())
}

/// Run a stress test (ramp-up until failure)
async fn run_stress_test(
    workload: &WorkloadGenerator,
    stress: &crate::config::StressConfig,
    concurrency: u32,
    state: &Arc<BenchmarkState>,
) -> Result<()> {
    let mut current_rps = stress.start_rps;

    let pb = ProgressBar::new(stress.max_rps as u64);
    pb.set_style(
        ProgressStyle::default_bar()
            .template("  [{elapsed_precise}] [{bar:40.yellow/red}] {pos}/{len} RPS | {msg}")
            .unwrap()
            .progress_chars("=>-"),
    );

    while current_rps <= stress.max_rps {
        pb.set_position(current_rps as u64);
        pb.set_message(format!("Testing {} RPS", current_rps));

        let _step_start = Instant::now();
        let before_failed = state.failed.load(Ordering::SeqCst);
        let before_successful = state.successful.load(Ordering::SeqCst);

        // Run at current RPS
        run_load_phase(
            workload,
            current_rps,
            concurrency,
            Duration::from_secs(stress.step_duration_secs),
            state,
            false,
        )
        .await?;

        // Calculate error rate for this step
        let step_failed = state.failed.load(Ordering::SeqCst) - before_failed;
        let step_successful = state.successful.load(Ordering::SeqCst) - before_successful;
        let step_total = step_failed + step_successful;
        let error_rate = if step_total > 0 {
            (step_failed as f64 / step_total as f64) * 100.0
        } else {
            0.0
        };

        pb.set_message(format!(
            "Testing {} RPS - {:.1}% errors",
            current_rps, error_rate
        ));

        // Check if we've exceeded error threshold
        if error_rate > stress.error_threshold {
            pb.finish_with_message(format!(
                "Breaking point: {} RPS ({:.1}% errors)",
                current_rps, error_rate
            ));
            break;
        }

        current_rps += stress.step;
    }

    Ok(())
}

/// Run a burst test
async fn run_burst_test(
    workload: &WorkloadGenerator,
    burst: &crate::config::BurstConfig,
    state: &Arc<BenchmarkState>,
) -> Result<()> {
    let pb = ProgressBar::new(burst.burst_count as u64);
    pb.set_style(
        ProgressStyle::default_bar()
            .template("  [{elapsed_precise}] [{bar:40.magenta/blue}] Burst {pos}/{len} | {msg}")
            .unwrap()
            .progress_chars("=>-"),
    );

    for i in 0..burst.burst_count {
        pb.set_position(i as u64);
        pb.set_message(format!("Sending {} requests", burst.burst_size));

        // Send burst
        let mut handles = Vec::new();
        let requests_per_worker = burst.burst_size / burst.concurrency;

        for _ in 0..burst.concurrency {
            let workload = workload.clone();
            let state = state.clone();

            let handle = tokio::spawn(async move {
                for _ in 0..requests_per_worker {
                    let req_start = Instant::now();

                    match workload.send_request().await {
                        Ok(status) => {
                            let latency_us = req_start.elapsed().as_micros() as u64;
                            state.histogram.lock().await.record(latency_us).ok();

                            if status.is_success() {
                                state.successful.fetch_add(1, Ordering::Relaxed);
                            } else {
                                state.failed.fetch_add(1, Ordering::Relaxed);
                            }
                        }
                        Err(_) => {
                            state.failed.fetch_add(1, Ordering::Relaxed);
                        }
                    }
                }
            });

            handles.push(handle);
        }

        for handle in handles {
            let _ = handle.await;
        }

        // Pause between bursts
        if i < burst.burst_count - 1 {
            pb.set_message(format!("Pausing {} seconds", burst.pause_secs));
            tokio::time::sleep(Duration::from_secs(burst.pause_secs)).await;
        }
    }

    pb.finish_with_message("done");
    Ok(())
}

/// Print result summary to console
fn print_result_summary(result: &BenchmarkResult) {
    println!();
    println!("  {}", "Results Summary".green().bold());
    println!("  {}", "─".repeat(50));

    println!(
        "  {} {:.0} req/s ({} total, {} failed)",
        "Throughput:".cyan(),
        result.results.throughput.requests_per_second,
        result.results.throughput.total_requests,
        result.results.throughput.failed
    );

    println!(
        "  {} p50={:.2}ms p95={:.2}ms p99={:.2}ms max={:.2}ms",
        "Latency:".cyan(),
        result.results.latency_ms.p50,
        result.results.latency_ms.p95,
        result.results.latency_ms.p99,
        result.results.latency_ms.max
    );

    println!(
        "  {} idle={:.1}MB peak={:.1}MB avg={:.1}MB",
        "Memory:".cyan(),
        result.results.memory_mb.idle,
        result.results.memory_mb.peak,
        result.results.memory_mb.average
    );

    println!(
        "  {} peak={:.1}% avg={:.1}%",
        "CPU:".cyan(),
        result.results.cpu_percent.peak,
        result.results.cpu_percent.average
    );

    let error_rate = if result.results.throughput.total_requests > 0 {
        (result.results.throughput.failed as f64 / result.results.throughput.total_requests as f64)
            * 100.0
    } else {
        0.0
    };

    if error_rate > 0.0 {
        println!(
            "  {} {:.2}% (timeout={}, refused={}, 4xx={}, 5xx={})",
            "Errors:".red(),
            error_rate,
            result.results.errors.timeout,
            result.results.errors.connection_refused,
            result.results.errors.status_4xx,
            result.results.errors.status_5xx
        );
    }
}
