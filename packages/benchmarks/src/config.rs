//! Benchmark configuration management

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::path::Path;

/// Main benchmark scenario configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScenarioConfig {
    pub name: String,
    pub description: String,

    #[serde(default)]
    pub load: LoadConfig,

    #[serde(default)]
    pub request: RequestConfig,

    #[serde(default)]
    pub docker: DockerConfig,

    #[serde(default)]
    pub stress: Option<StressConfig>,

    #[serde(default)]
    pub burst: Option<BurstConfig>,
}

/// Load configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LoadConfig {
    /// Target requests per second
    #[serde(default = "default_rps")]
    pub target_rps: u32,

    /// Number of concurrent connections
    #[serde(default = "default_concurrency")]
    pub concurrency: u32,

    /// Test duration in seconds
    #[serde(default = "default_duration")]
    pub duration_secs: u64,

    /// Warmup duration in seconds
    #[serde(default = "default_warmup")]
    pub warmup_secs: u64,

    /// Connection timeout in milliseconds
    #[serde(default = "default_timeout")]
    pub timeout_ms: u64,
}

/// Request configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RequestConfig {
    /// HTTP method
    #[serde(default = "default_method")]
    pub method: String,

    /// Request path
    #[serde(default = "default_path")]
    pub path: String,

    /// Request body size in bytes (0 for GET)
    #[serde(default)]
    pub body_size: usize,

    /// Add custom headers
    #[serde(default)]
    pub headers: Vec<(String, String)>,

    /// Enable keep-alive
    #[serde(default = "default_true")]
    pub keep_alive: bool,

    /// Enable compression (Accept-Encoding: gzip, br)
    #[serde(default)]
    pub compression: bool,

    /// Mix of request types
    #[serde(default)]
    pub mix: Option<RequestMix>,
}

/// Request mix for realistic workloads
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RequestMix {
    /// Percentage of GET requests
    #[serde(default = "default_70")]
    pub get_percent: u8,

    /// Percentage of POST requests
    #[serde(default = "default_20")]
    pub post_percent: u8,

    /// Percentage of PUT requests
    #[serde(default = "default_5")]
    pub put_percent: u8,

    /// Percentage of DELETE requests
    #[serde(default = "default_5")]
    pub delete_percent: u8,

    /// Different paths to request
    #[serde(default)]
    pub paths: Vec<String>,
}

/// Docker resource limits
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DockerConfig {
    /// CPU limit (number of cores)
    #[serde(default = "default_cpu")]
    pub cpu_limit: f64,

    /// Memory limit in MB
    #[serde(default = "default_memory")]
    pub memory_mb: u64,

    /// Network mode
    #[serde(default = "default_network")]
    pub network: String,
}

/// Stress test configuration (ramp-up until failure)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StressConfig {
    /// Starting RPS
    #[serde(default = "default_start_rps")]
    pub start_rps: u32,

    /// Maximum RPS
    #[serde(default = "default_max_rps")]
    pub max_rps: u32,

    /// RPS increment per step
    #[serde(default = "default_step")]
    pub step: u32,

    /// Duration of each step in seconds
    #[serde(default = "default_step_duration")]
    pub step_duration_secs: u64,

    /// Error rate threshold to stop (percentage)
    #[serde(default = "default_error_threshold")]
    pub error_threshold: f64,
}

/// Burst test configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BurstConfig {
    /// Number of requests per burst
    #[serde(default = "default_burst_size")]
    pub burst_size: u32,

    /// Number of bursts
    #[serde(default = "default_burst_count")]
    pub burst_count: u32,

    /// Pause between bursts in seconds
    #[serde(default = "default_pause")]
    pub pause_secs: u64,

    /// Concurrency during burst
    #[serde(default = "default_burst_concurrency")]
    pub concurrency: u32,
}

// Default value functions
fn default_rps() -> u32 {
    100
}
fn default_concurrency() -> u32 {
    10
}
fn default_duration() -> u64 {
    60
}
fn default_warmup() -> u64 {
    5
}
fn default_timeout() -> u64 {
    30000
}
fn default_method() -> String {
    "GET".to_string()
}
fn default_path() -> String {
    "/".to_string()
}
fn default_true() -> bool {
    true
}
fn default_cpu() -> f64 {
    1.0
}
fn default_memory() -> u64 {
    256
}
fn default_network() -> String {
    "bench-network".to_string()
}
fn default_start_rps() -> u32 {
    100
}
fn default_max_rps() -> u32 {
    10000
}
fn default_step() -> u32 {
    100
}
fn default_step_duration() -> u64 {
    30
}
fn default_error_threshold() -> f64 {
    5.0
}
fn default_burst_size() -> u32 {
    10000
}
fn default_burst_count() -> u32 {
    5
}
fn default_pause() -> u64 {
    10
}
fn default_burst_concurrency() -> u32 {
    100
}
fn default_70() -> u8 {
    70
}
fn default_20() -> u8 {
    20
}
fn default_5() -> u8 {
    5
}

impl Default for LoadConfig {
    fn default() -> Self {
        Self {
            target_rps: default_rps(),
            concurrency: default_concurrency(),
            duration_secs: default_duration(),
            warmup_secs: default_warmup(),
            timeout_ms: default_timeout(),
        }
    }
}

impl Default for RequestConfig {
    fn default() -> Self {
        Self {
            method: default_method(),
            path: default_path(),
            body_size: 0,
            headers: Vec::new(),
            keep_alive: true,
            compression: false,
            mix: None,
        }
    }
}

impl Default for DockerConfig {
    fn default() -> Self {
        Self {
            cpu_limit: default_cpu(),
            memory_mb: default_memory(),
            network: default_network(),
        }
    }
}

impl ScenarioConfig {
    /// Load configuration from a TOML file
    pub fn from_file<P: AsRef<Path>>(path: P) -> Result<Self> {
        let content = std::fs::read_to_string(&path)
            .with_context(|| format!("Failed to read config file: {}", path.as_ref().display()))?;

        toml::from_str(&content)
            .with_context(|| format!("Failed to parse config file: {}", path.as_ref().display()))
    }

    /// Get a preset scenario configuration
    pub fn preset(name: &str) -> Result<Self> {
        match name {
            "baseline" => Ok(Self::baseline()),
            "sustained" => Ok(Self::sustained()),
            "burst" => Ok(Self::burst()),
            "stress" => Ok(Self::stress()),
            "memory" => Ok(Self::memory()),
            "concurrent" => Ok(Self::concurrent()),
            _ => anyhow::bail!(
                "Unknown scenario: {}. Use 'list' to see available scenarios.",
                name
            ),
        }
    }

    /// Baseline test - minimal load for latency measurement
    fn baseline() -> Self {
        Self {
            name: "baseline".to_string(),
            description: "Minimal load (1 req/s) - measures base latency with zero contention"
                .to_string(),
            load: LoadConfig {
                target_rps: 1,
                concurrency: 1,
                duration_secs: 60,
                warmup_secs: 5,
                timeout_ms: 30000,
            },
            request: RequestConfig::default(),
            docker: DockerConfig::default(),
            stress: None,
            burst: None,
        }
    }

    /// Sustained load test - 1k RPS for 5 minutes
    fn sustained() -> Self {
        Self {
            name: "sustained".to_string(),
            description: "Sustained load (1k req/s for 5 min) - memory stability test".to_string(),
            load: LoadConfig {
                target_rps: 1000,
                concurrency: 50,
                duration_secs: 300,
                warmup_secs: 10,
                timeout_ms: 30000,
            },
            request: RequestConfig::default(),
            docker: DockerConfig::default(),
            stress: None,
            burst: None,
        }
    }

    /// Burst test - traffic spikes
    fn burst() -> Self {
        Self {
            name: "burst".to_string(),
            description: "Traffic spikes (10k requests, pause, repeat) - spike handling"
                .to_string(),
            load: LoadConfig {
                target_rps: 0, // Controlled by burst config
                concurrency: 100,
                duration_secs: 120,
                warmup_secs: 5,
                timeout_ms: 30000,
            },
            request: RequestConfig::default(),
            docker: DockerConfig::default(),
            stress: None,
            burst: Some(BurstConfig {
                burst_size: 10000,
                burst_count: 5,
                pause_secs: 10,
                concurrency: 100,
            }),
        }
    }

    /// Stress test - ramp up until failure
    fn stress() -> Self {
        Self {
            name: "stress".to_string(),
            description: "Ramp-up until 5% error rate - find breaking point".to_string(),
            load: LoadConfig {
                target_rps: 0, // Controlled by stress config
                concurrency: 100,
                duration_secs: 600,
                warmup_secs: 10,
                timeout_ms: 10000,
            },
            request: RequestConfig::default(),
            docker: DockerConfig::default(),
            stress: Some(StressConfig {
                start_rps: 100,
                max_rps: 50000,
                step: 500,
                step_duration_secs: 30,
                error_threshold: 5.0,
            }),
            burst: None,
        }
    }

    /// Memory test - long running for leak detection
    fn memory() -> Self {
        Self {
            name: "memory".to_string(),
            description: "Long-running (30 min) at moderate load - memory leak detection"
                .to_string(),
            load: LoadConfig {
                target_rps: 500,
                concurrency: 25,
                duration_secs: 1800, // 30 minutes
                warmup_secs: 10,
                timeout_ms: 30000,
            },
            request: RequestConfig::default(),
            docker: DockerConfig::default(),
            stress: None,
            burst: None,
        }
    }

    /// Concurrent connections test
    fn concurrent() -> Self {
        Self {
            name: "concurrent".to_string(),
            description: "High concurrency (1k connections) - connection handling".to_string(),
            load: LoadConfig {
                target_rps: 5000,
                concurrency: 1000,
                duration_secs: 120,
                warmup_secs: 10,
                timeout_ms: 30000,
            },
            request: RequestConfig::default(),
            docker: DockerConfig::default(),
            stress: None,
            burst: None,
        }
    }
}

/// Proxy configuration
#[derive(Debug, Clone)]
#[allow(dead_code)]
pub struct ProxyConfig {
    pub name: String,
    pub container_name: String,
    pub port: u16,
    pub url: String,
}

impl ProxyConfig {
    pub fn for_proxy(name: &str) -> Self {
        match name {
            "rproxy" => Self {
                name: "rproxy".to_string(),
                container_name: "bench-rproxy".to_string(),
                port: 8080,
                url: "http://localhost:8080".to_string(),
            },
            "nginx" => Self {
                name: "nginx".to_string(),
                container_name: "bench-nginx".to_string(),
                port: 8081,
                url: "http://localhost:8081".to_string(),
            },
            "traefik" => Self {
                name: "traefik".to_string(),
                container_name: "bench-traefik".to_string(),
                port: 8082,
                url: "http://localhost:8082".to_string(),
            },
            "caddy" => Self {
                name: "caddy".to_string(),
                container_name: "bench-caddy".to_string(),
                port: 8083,
                url: "http://localhost:8083".to_string(),
            },
            "envoy" => Self {
                name: "envoy".to_string(),
                container_name: "bench-envoy".to_string(),
                port: 8084,
                url: "http://localhost:8084".to_string(),
            },
            _ => panic!("Unknown proxy: {}", name),
        }
    }
}
