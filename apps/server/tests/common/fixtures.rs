//! Test fixtures and helper functions

use std::collections::HashMap;
use std::env;

/// Helper to set environment variables for a test and restore them after
#[allow(dead_code)]
pub struct EnvGuard {
    original: HashMap<String, Option<String>>,
}

#[allow(dead_code)]
impl EnvGuard {
    /// Create a new EnvGuard and set the given environment variables
    ///
    /// # Safety
    /// This modifies environment variables which is not thread-safe.
    /// Tests using this should be marked with #[serial].
    pub fn new(vars: &[(&str, &str)]) -> Self {
        let mut original = HashMap::new();

        for (key, value) in vars {
            original.insert(key.to_string(), env::var(key).ok());
            // SAFETY: Tests are run serially with #[serial]
            unsafe { env::set_var(key, value) };
        }

        Self { original }
    }

    /// Clear specific environment variables
    ///
    /// # Safety
    /// This modifies environment variables which is not thread-safe.
    /// Tests using this should be marked with #[serial].
    pub fn clear(vars: &[&str]) -> Self {
        let mut original = HashMap::new();

        for key in vars {
            original.insert(key.to_string(), env::var(key).ok());
            // SAFETY: Tests are run serially with #[serial]
            unsafe { env::remove_var(key) };
        }

        Self { original }
    }
}

impl Drop for EnvGuard {
    fn drop(&mut self) {
        for (key, value) in &self.original {
            // SAFETY: Tests are run serially with #[serial]
            unsafe {
                match value {
                    Some(v) => env::set_var(key, v),
                    None => env::remove_var(key),
                }
            }
        }
    }
}

/// Default test configuration environment variables
#[allow(dead_code)]
pub fn default_test_env() -> Vec<(&'static str, &'static str)> {
    vec![
        ("RPROXY_LISTEN", "127.0.0.1:0"),
        ("RPROXY_UPSTREAM", "127.0.0.1:9999"),
        ("RPROXY_TLS_DOMAINS", ""),
        ("RPROXY_RATE_LIMIT", "100"),
        ("RPROXY_RATE_BURST", "200"),
        ("RPROXY_CACHE_ENABLED", "true"),
        ("RPROXY_HSTS", "true"),
        ("RPROXY_LOG_LEVEL", "error"),
    ]
}

/// Environment variables that should be cleared before config tests
#[allow(dead_code)]
pub fn config_env_vars() -> Vec<&'static str> {
    vec![
        "RPROXY_LISTEN",
        "RPROXY_HTTP_PORT",
        "RPROXY_UPSTREAM",
        "RPROXY_UPSTREAM_TIMEOUT",
        "RPROXY_TLS_DOMAINS",
        "RPROXY_TLS_EMAIL",
        "RPROXY_TLS_CACHE_DIR",
        "RPROXY_RATE_LIMIT_ENABLED",
        "RPROXY_RATE_LIMIT",
        "RPROXY_RATE_BURST",
        "RPROXY_CACHE_ENABLED",
        "RPROXY_CACHE_SIZE",
        "RPROXY_CACHE_PATHS",
        "RPROXY_HSTS",
        "RPROXY_FILTER_ENABLED",
        "RPROXY_BLOCKED_PATHS",
        "RPROXY_WS_ENABLED",
        "RPROXY_WS_PATHS",
        "RPROXY_LOG_LEVEL",
    ]
}
