mod cache;
mod filter;
mod rate_limit;
mod security;

use std::net::SocketAddr;
use std::sync::Arc;

use bytes::Bytes;
use http_body_util::combinators::BoxBody;
use hyper::body::Incoming;
use hyper::{Request, Response, StatusCode};
use tracing::{debug, info, warn};

use crate::config::Config;
use crate::proxy::{ProxyHandler, empty_response, text_response};
use crate::websocket::{handle_websocket, is_websocket_upgrade};

pub use cache::CacheMiddleware;
pub use filter::FilterMiddleware;
pub use rate_limit::RateLimitMiddleware;
pub use security::SecurityMiddleware;

pub struct MiddlewareStack {
    config: Arc<Config>,
    rate_limiter: RateLimitMiddleware,
    filter: FilterMiddleware,
    cache: CacheMiddleware,
    security: SecurityMiddleware,
}

impl MiddlewareStack {
    pub fn new(config: Arc<Config>) -> Self {
        Self {
            rate_limiter: RateLimitMiddleware::new(&config.rate_limit),
            filter: FilterMiddleware::new(&config.filter),
            cache: CacheMiddleware::new(&config.cache),
            security: SecurityMiddleware::new(&config.security),
            config,
        }
    }

    pub async fn handle(
        &self,
        req: Request<Incoming>,
        remote_addr: SocketAddr,
        proxy: &ProxyHandler,
    ) -> std::result::Result<Response<BoxBody<Bytes, hyper::Error>>, hyper::Error> {
        let client_ip = remote_addr.ip();
        let method = req.method().clone();
        let path = req.uri().path().to_string();

        debug!("{} {} from {}", method, path, client_ip);

        // 1. Rate limiting
        if self.config.rate_limit.enabled {
            if !self.rate_limiter.check(client_ip) {
                warn!("Rate limited: {} from {}", path, client_ip);
                return Ok(text_response(
                    StatusCode::TOO_MANY_REQUESTS,
                    "Rate limit exceeded",
                ));
            }
        }

        // 2. Path filtering
        if self.config.filter.enabled {
            if self.filter.is_blocked(&path) {
                info!("Blocked path: {} from {}", path, client_ip);
                return Ok(empty_response(StatusCode::NOT_FOUND));
            }
        }

        // 3. Check if WebSocket upgrade
        if self.config.websocket.enabled && is_websocket_upgrade(&req) {
            if self.is_websocket_path(&path) {
                info!("WebSocket upgrade: {} from {}", path, client_ip);
                match handle_websocket(req, self.config.clone()).await {
                    Ok(response) => return Ok(response),
                    Err(e) => {
                        warn!("WebSocket error: {}", e);
                        return Ok(empty_response(StatusCode::BAD_GATEWAY));
                    }
                }
            }
        }

        // 4. Check cache for GET/HEAD requests
        if self.config.cache.enabled && (method == "GET" || method == "HEAD") {
            if let Some(cached) = self.cache.get(&path).await {
                debug!("Cache hit: {}", path);
                return Ok(cached);
            }
        }

        // 5. Forward to upstream
        let mut response = match proxy.handle(req, client_ip).await {
            Ok(resp) => resp,
            Err(e) => {
                warn!("Proxy error: {}", e);
                return Ok(text_response(StatusCode::BAD_GATEWAY, "Upstream error"));
            }
        };

        // 6. Add security headers
        if self.config.security.hsts_enabled || !self.config.security.headers.is_empty() {
            self.security.add_headers(response.headers_mut());
        }

        // 7. Cache response if cacheable
        if self.config.cache.enabled
            && (method == "GET" || method == "HEAD")
            && response.status().is_success()
            && self.cache.is_cacheable_path(&path)
        {
            // Note: Caching the full response would require body buffering
            // For MVP, we skip actual response caching to avoid memory overhead
            debug!("Cacheable response: {} (caching disabled for MVP)", path);
        }

        Ok(response)
    }

    fn is_websocket_path(&self, path: &str) -> bool {
        self.config.websocket.paths.iter().any(|pattern| {
            if pattern.ends_with("/*") {
                let prefix = &pattern[..pattern.len() - 2];
                path.starts_with(prefix)
            } else if pattern.ends_with("*") {
                let prefix = &pattern[..pattern.len() - 1];
                path.starts_with(prefix)
            } else {
                path == pattern
            }
        })
    }
}
