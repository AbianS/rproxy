//! HTTP workload generator for benchmarks

use crate::config::RequestConfig;
use anyhow::Result;
use rand::Rng;
use reqwest::{Client, Method, StatusCode};
use std::time::Duration;

/// Generates HTTP requests for benchmarks
#[derive(Clone)]
pub struct WorkloadGenerator {
    client: Client,
    base_url: String,
    config: RequestConfig,
    body_data: Option<Vec<u8>>,
}

impl WorkloadGenerator {
    /// Create a new workload generator
    pub fn new(config: &RequestConfig, base_url: &str) -> Result<Self> {
        let mut client_builder = Client::builder()
            .timeout(Duration::from_secs(30))
            .pool_max_idle_per_host(100)
            .tcp_nodelay(true);

        if !config.keep_alive {
            client_builder = client_builder.pool_max_idle_per_host(0);
        }

        if config.compression {
            client_builder = client_builder
                .gzip(true)
                .brotli(true);
        }

        let client = client_builder.build()?;

        // Pre-generate body data if needed
        let body_data = if config.body_size > 0 {
            Some(generate_body(config.body_size))
        } else {
            None
        };

        Ok(Self {
            client,
            base_url: base_url.to_string(),
            config: config.clone(),
            body_data,
        })
    }

    /// Send a single request
    pub async fn send_request(&self) -> Result<StatusCode, reqwest::Error> {
        let (method, path, body) = self.generate_request();
        let url = format!("{}{}", self.base_url, path);

        let mut request = self.client.request(method, &url);

        // Add custom headers
        for (key, value) in &self.config.headers {
            request = request.header(key, value);
        }

        // Add body if present
        if let Some(body) = body {
            request = request.body(body);
            request = request.header("Content-Type", "application/json");
        }

        let response = request.send().await?;
        Ok(response.status())
    }

    /// Generate request parameters based on config
    fn generate_request(&self) -> (Method, String, Option<Vec<u8>>) {
        if let Some(ref mix) = self.config.mix {
            self.generate_mixed_request(mix)
        } else {
            let method = match self.config.method.to_uppercase().as_str() {
                "GET" => Method::GET,
                "POST" => Method::POST,
                "PUT" => Method::PUT,
                "DELETE" => Method::DELETE,
                "PATCH" => Method::PATCH,
                "HEAD" => Method::HEAD,
                _ => Method::GET,
            };

            let body = if method == Method::POST || method == Method::PUT || method == Method::PATCH
            {
                self.body_data.clone()
            } else {
                None
            };

            (method, self.config.path.clone(), body)
        }
    }

    /// Generate a request from a mixed workload
    fn generate_mixed_request(
        &self,
        mix: &crate::config::RequestMix,
    ) -> (Method, String, Option<Vec<u8>>) {
        let mut rng = rand::rng();
        let roll: u8 = rng.random_range(0..100);

        let method = if roll < mix.get_percent {
            Method::GET
        } else if roll < mix.get_percent + mix.post_percent {
            Method::POST
        } else if roll < mix.get_percent + mix.post_percent + mix.put_percent {
            Method::PUT
        } else {
            Method::DELETE
        };

        let path = if mix.paths.is_empty() {
            self.config.path.clone()
        } else {
            let idx = rng.random_range(0..mix.paths.len());
            mix.paths[idx].clone()
        };

        let body = if method == Method::POST || method == Method::PUT {
            self.body_data.clone()
        } else {
            None
        };

        (method, path, body)
    }
}

/// Generate random JSON body of specified size
fn generate_body(size: usize) -> Vec<u8> {
    let mut rng = rand::rng();

    // Create a JSON-like structure
    let mut body = String::with_capacity(size);
    body.push_str(r#"{"data":""#);

    let padding_size = size.saturating_sub(20); // Account for JSON structure
    for _ in 0..padding_size {
        let c = (rng.random_range(0..26u8) + b'a') as char;
        body.push(c);
    }

    body.push_str(r#""}"#);

    // Trim or pad to exact size
    if body.len() > size {
        body.truncate(size - 2);
        body.push_str(r#""}"#);
    }

    body.into_bytes()
}

/// Predefined workload patterns
#[allow(dead_code)]
pub mod patterns {
    use super::*;

    /// Simple GET request workload
    pub fn simple_get() -> RequestConfig {
        RequestConfig {
            method: "GET".to_string(),
            path: "/".to_string(),
            body_size: 0,
            headers: vec![],
            keep_alive: true,
            compression: false,
            mix: None,
        }
    }

    /// API-like workload with mixed methods
    pub fn api_workload() -> RequestConfig {
        RequestConfig {
            method: "GET".to_string(),
            path: "/api/data".to_string(),
            body_size: 1024,
            headers: vec![
                ("Accept".to_string(), "application/json".to_string()),
                ("X-Request-ID".to_string(), "benchmark".to_string()),
            ],
            keep_alive: true,
            compression: true,
            mix: Some(crate::config::RequestMix {
                get_percent: 60,
                post_percent: 25,
                put_percent: 10,
                delete_percent: 5,
                paths: vec![
                    "/api/users".to_string(),
                    "/api/products".to_string(),
                    "/api/orders".to_string(),
                    "/api/search".to_string(),
                ],
            }),
        }
    }

    /// Heavy POST workload (large bodies)
    pub fn heavy_post() -> RequestConfig {
        RequestConfig {
            method: "POST".to_string(),
            path: "/api/upload".to_string(),
            body_size: 65536, // 64KB
            headers: vec![("Content-Type".to_string(), "application/json".to_string())],
            keep_alive: true,
            compression: true,
            mix: None,
        }
    }

    /// Static asset workload (simulates serving files)
    pub fn static_assets() -> RequestConfig {
        RequestConfig {
            method: "GET".to_string(),
            path: "/static/bundle.js".to_string(),
            body_size: 0,
            headers: vec![
                ("Accept-Encoding".to_string(), "gzip, br".to_string()),
                ("Cache-Control".to_string(), "no-cache".to_string()),
            ],
            keep_alive: true,
            compression: true,
            mix: Some(crate::config::RequestMix {
                get_percent: 100,
                post_percent: 0,
                put_percent: 0,
                delete_percent: 0,
                paths: vec![
                    "/static/app.js".to_string(),
                    "/static/vendor.js".to_string(),
                    "/static/styles.css".to_string(),
                    "/static/image.png".to_string(),
                    "/_next/static/chunks/main.js".to_string(),
                ],
            }),
        }
    }

    /// WebSocket-heavy workload (simulates HMR traffic)
    pub fn websocket_simulation() -> RequestConfig {
        RequestConfig {
            method: "GET".to_string(),
            path: "/_next/webpack-hmr".to_string(),
            body_size: 0,
            headers: vec![
                ("Connection".to_string(), "Upgrade".to_string()),
                ("Upgrade".to_string(), "websocket".to_string()),
            ],
            keep_alive: true,
            compression: false,
            mix: None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_generate_body_size() {
        let body = generate_body(1000);
        assert!(body.len() <= 1000);
        assert!(body.len() >= 990); // Allow some variance
    }

    #[test]
    fn test_body_is_valid_json() {
        let body = generate_body(100);
        let s = String::from_utf8(body).unwrap();
        assert!(s.starts_with(r#"{"data":""#));
        assert!(s.ends_with(r#""}"#));
    }
}
