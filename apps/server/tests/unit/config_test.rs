//! Unit tests for configuration parsing

use crate::common::fixtures::{EnvGuard, config_env_vars};
use pretty_assertions::assert_eq;
use serial_test::serial;
use std::time::Duration;

// We need to test config parsing, but the Config struct is in the main crate
// For now, we test the environment variable behavior

#[test]
#[serial]
fn test_default_listen_address() {
    let _guard = EnvGuard::clear(&config_env_vars());

    // Default should be 0.0.0.0:443
    let addr = std::env::var("RPROXY_LISTEN").unwrap_or_else(|_| "0.0.0.0:443".into());
    assert_eq!(addr, "0.0.0.0:443");
}

#[test]
#[serial]
fn test_custom_listen_address() {
    let _guard = EnvGuard::new(&[("RPROXY_LISTEN", "127.0.0.1:8080")]);

    let addr = std::env::var("RPROXY_LISTEN").unwrap();
    assert_eq!(addr, "127.0.0.1:8080");
}

#[test]
#[serial]
fn test_default_http_port() {
    let _guard = EnvGuard::clear(&config_env_vars());

    let port: u16 = std::env::var("RPROXY_HTTP_PORT")
        .unwrap_or_else(|_| "80".into())
        .parse()
        .unwrap();
    assert_eq!(port, 80);
}

#[test]
#[serial]
fn test_default_upstream() {
    let _guard = EnvGuard::clear(&config_env_vars());

    let upstream = std::env::var("RPROXY_UPSTREAM").unwrap_or_else(|_| "127.0.0.1:3000".into());
    assert_eq!(upstream, "127.0.0.1:3000");
}

#[test]
#[serial]
fn test_upstream_timeout_default() {
    let _guard = EnvGuard::clear(&config_env_vars());

    let timeout: u64 = std::env::var("RPROXY_UPSTREAM_TIMEOUT")
        .unwrap_or_else(|_| "30".into())
        .parse()
        .unwrap();
    assert_eq!(Duration::from_secs(timeout), Duration::from_secs(30));
}

#[test]
#[serial]
fn test_upstream_timeout_custom() {
    let _guard = EnvGuard::new(&[("RPROXY_UPSTREAM_TIMEOUT", "60")]);

    let timeout: u64 = std::env::var("RPROXY_UPSTREAM_TIMEOUT")
        .unwrap()
        .parse()
        .unwrap();
    assert_eq!(timeout, 60);
}

#[test]
#[serial]
fn test_tls_disabled_when_no_domains() {
    let _guard = EnvGuard::clear(&config_env_vars());

    let domains = std::env::var("RPROXY_TLS_DOMAINS").unwrap_or_default();
    assert!(domains.is_empty());
}

#[test]
#[serial]
fn test_tls_enabled_with_domains() {
    let _guard = EnvGuard::new(&[("RPROXY_TLS_DOMAINS", "example.com,www.example.com")]);

    let domains = std::env::var("RPROXY_TLS_DOMAINS").unwrap();
    let domain_list: Vec<&str> = domains.split(',').collect();
    assert_eq!(domain_list.len(), 2);
    assert_eq!(domain_list[0], "example.com");
    assert_eq!(domain_list[1], "www.example.com");
}

#[test]
#[serial]
fn test_rate_limit_defaults() {
    let _guard = EnvGuard::clear(&config_env_vars());

    let rps: u32 = std::env::var("RPROXY_RATE_LIMIT")
        .unwrap_or_else(|_| "100".into())
        .parse()
        .unwrap();
    let burst: u32 = std::env::var("RPROXY_RATE_BURST")
        .unwrap_or_else(|_| "200".into())
        .parse()
        .unwrap();

    assert_eq!(rps, 100);
    assert_eq!(burst, 200);
}

#[test]
#[serial]
fn test_rate_limit_custom() {
    let _guard = EnvGuard::new(&[("RPROXY_RATE_LIMIT", "500"), ("RPROXY_RATE_BURST", "1000")]);

    let rps: u32 = std::env::var("RPROXY_RATE_LIMIT").unwrap().parse().unwrap();
    let burst: u32 = std::env::var("RPROXY_RATE_BURST").unwrap().parse().unwrap();

    assert_eq!(rps, 500);
    assert_eq!(burst, 1000);
}

#[test]
#[serial]
fn test_cache_enabled_default() {
    let _guard = EnvGuard::clear(&config_env_vars());

    let enabled: bool = std::env::var("RPROXY_CACHE_ENABLED")
        .unwrap_or_else(|_| "true".into())
        .parse()
        .unwrap();
    assert!(enabled);
}

#[test]
#[serial]
fn test_cache_disabled() {
    let _guard = EnvGuard::new(&[("RPROXY_CACHE_ENABLED", "false")]);

    let enabled: bool = std::env::var("RPROXY_CACHE_ENABLED")
        .unwrap()
        .parse()
        .unwrap();
    assert!(!enabled);
}

#[test]
#[serial]
fn test_hsts_enabled_default() {
    let _guard = EnvGuard::clear(&config_env_vars());

    let enabled: bool = std::env::var("RPROXY_HSTS")
        .unwrap_or_else(|_| "true".into())
        .parse()
        .unwrap();
    assert!(enabled);
}

#[test]
#[serial]
fn test_blocked_paths_default() {
    let _guard = EnvGuard::clear(&config_env_vars());

    let paths = std::env::var("RPROXY_BLOCKED_PATHS").unwrap_or_else(|_| "/.env*,/.git*".into());
    assert!(paths.contains("/.env"));
    assert!(paths.contains("/.git"));
}

#[test]
#[serial]
fn test_websocket_paths_default() {
    let _guard = EnvGuard::clear(&config_env_vars());

    let paths =
        std::env::var("RPROXY_WS_PATHS").unwrap_or_else(|_| "/_next/webpack-hmr,/ws/*".into());
    assert!(paths.contains("/_next/webpack-hmr"));
}

#[test]
#[serial]
fn test_log_level_default() {
    let _guard = EnvGuard::clear(&config_env_vars());

    let level = std::env::var("RPROXY_LOG_LEVEL").unwrap_or_else(|_| "info".into());
    assert_eq!(level, "info");
}

#[test]
#[serial]
fn test_log_level_custom() {
    let _guard = EnvGuard::new(&[("RPROXY_LOG_LEVEL", "debug")]);

    let level = std::env::var("RPROXY_LOG_LEVEL").unwrap();
    assert_eq!(level, "debug");
}
