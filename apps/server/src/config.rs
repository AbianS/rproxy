use std::env;
use std::net::SocketAddr;
use std::path::PathBuf;
use std::time::Duration;

use crate::error::{ProxyError, Result};

#[derive(Debug, Clone)]
pub struct Config {
    pub server: ServerConfig,
    pub upstream: UpstreamConfig,
    pub tls: TlsConfig,
    pub rate_limit: RateLimitConfig,
    pub cache: CacheConfig,
    pub security: SecurityConfig,
    pub filter: FilterConfig,
    pub websocket: WebSocketConfig,
    pub log_level: String,
}

#[derive(Debug, Clone)]
pub struct ServerConfig {
    pub listen_addr: SocketAddr,
    pub http_port: u16,
}

#[derive(Debug, Clone)]
pub struct UpstreamConfig {
    pub address: String,
    pub timeout: Duration,
}

#[derive(Debug, Clone)]
pub struct TlsConfig {
    pub enabled: bool,
    pub domains: Vec<String>,
    pub email: String,
    pub cache_dir: PathBuf,
}

#[derive(Debug, Clone)]
pub struct RateLimitConfig {
    pub enabled: bool,
    pub requests_per_second: u32,
    pub burst_size: u32,
}

#[derive(Debug, Clone)]
pub struct CacheConfig {
    pub enabled: bool,
    pub max_size: u64,
    pub paths: Vec<String>,
}

#[derive(Debug, Clone)]
pub struct SecurityConfig {
    pub hsts_enabled: bool,
    pub headers: Vec<(String, String)>,
}

#[derive(Debug, Clone)]
pub struct FilterConfig {
    pub enabled: bool,
    pub blocked_paths: Vec<String>,
}

#[derive(Debug, Clone)]
pub struct WebSocketConfig {
    pub enabled: bool,
    pub paths: Vec<String>,
}

impl Config {
    pub fn from_env() -> Result<Self> {
        Ok(Self {
            server: ServerConfig {
                listen_addr: parse_env("RPROXY_LISTEN", "0.0.0.0:443")?,
                http_port: parse_env("RPROXY_HTTP_PORT", "80")?,
            },
            upstream: UpstreamConfig {
                address: env::var("RPROXY_UPSTREAM").unwrap_or_else(|_| "127.0.0.1:3000".into()),
                timeout: Duration::from_secs(parse_env("RPROXY_UPSTREAM_TIMEOUT", "30")?),
            },
            tls: TlsConfig {
                enabled: !env::var("RPROXY_TLS_DOMAINS")
                    .unwrap_or_default()
                    .is_empty(),
                domains: parse_list("RPROXY_TLS_DOMAINS"),
                email: env::var("RPROXY_TLS_EMAIL").unwrap_or_default(),
                cache_dir: env::var("RPROXY_TLS_CACHE_DIR")
                    .unwrap_or_else(|_| "/var/lib/rproxy/certs".into())
                    .into(),
            },
            rate_limit: RateLimitConfig {
                enabled: parse_env("RPROXY_RATE_LIMIT_ENABLED", "true")?,
                requests_per_second: parse_env("RPROXY_RATE_LIMIT", "100")?,
                burst_size: parse_env("RPROXY_RATE_BURST", "200")?,
            },
            cache: CacheConfig {
                enabled: parse_env("RPROXY_CACHE_ENABLED", "true")?,
                max_size: parse_env("RPROXY_CACHE_SIZE", "500")?,
                paths: parse_list_with_default(
                    "RPROXY_CACHE_PATHS",
                    "/_next/static/*,/static/*,*.css,*.js,*.woff2,*.png,*.jpg,*.svg",
                ),
            },
            security: SecurityConfig {
                hsts_enabled: parse_env("RPROXY_HSTS", "true")?,
                headers: vec![
                    ("X-Content-Type-Options".into(), "nosniff".into()),
                    ("X-Frame-Options".into(), "DENY".into()),
                    ("X-XSS-Protection".into(), "1; mode=block".into()),
                    (
                        "Referrer-Policy".into(),
                        "strict-origin-when-cross-origin".into(),
                    ),
                ],
            },
            filter: FilterConfig {
                enabled: parse_env("RPROXY_FILTER_ENABLED", "true")?,
                blocked_paths: parse_list_with_default(
                    "RPROXY_BLOCKED_PATHS",
                    "/.env*,/.git*,/.svn*,/.hg*,/*.sql,/*.bak,/docker-compose*.yml",
                ),
            },
            websocket: WebSocketConfig {
                enabled: parse_env("RPROXY_WS_ENABLED", "true")?,
                paths: parse_list_with_default(
                    "RPROXY_WS_PATHS",
                    "/_next/webpack-hmr,/ws,/ws/*,/socket.io/*",
                ),
            },
            log_level: env::var("RPROXY_LOG_LEVEL").unwrap_or_else(|_| "info".into()),
        })
    }
}

fn parse_env<T: std::str::FromStr>(key: &str, default: &str) -> Result<T>
where
    T::Err: std::fmt::Display,
{
    let value = env::var(key).unwrap_or_else(|_| default.into());
    value
        .parse()
        .map_err(|e| ProxyError::Config(format!("Invalid value for {key}: {e}")))
}

fn parse_list(key: &str) -> Vec<String> {
    env::var(key)
        .unwrap_or_default()
        .split(',')
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
        .collect()
}

fn parse_list_with_default(key: &str, default: &str) -> Vec<String> {
    let value = env::var(key).unwrap_or_else(|_| default.into());
    value
        .split(',')
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
        .collect()
}
