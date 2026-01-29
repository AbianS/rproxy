use std::sync::Arc;
use std::time::Duration;

use bytes::Bytes;
use globset::{Glob, GlobSet, GlobSetBuilder};
use http_body_util::combinators::BoxBody;
use http_body_util::{BodyExt, Full};
use hyper::header::{CACHE_CONTROL, CONTENT_TYPE};
use hyper::{HeaderMap, Response, StatusCode};
use moka::future::Cache;
use tracing::debug;

use crate::config::CacheConfig;

pub struct CacheMiddleware {
    cache: Option<Cache<String, Arc<CachedResponse>>>,
    cacheable_paths: Option<GlobSet>,
    #[allow(dead_code)]
    max_entry_size: usize,
}

#[derive(Clone)]
pub struct CachedResponse {
    pub status: StatusCode,
    pub headers: HeaderMap,
    pub body: Bytes,
}

impl CacheMiddleware {
    pub fn new(config: &CacheConfig) -> Self {
        if !config.enabled {
            return Self {
                cache: None,
                cacheable_paths: None,
                max_entry_size: 0,
            };
        }

        let cache = Cache::builder()
            .max_capacity(config.max_size)
            .time_to_live(Duration::from_secs(3600)) // 1 hour default TTL
            .time_to_idle(Duration::from_secs(600)) // 10 min idle
            .build();

        // Build cacheable paths globset
        let mut builder = GlobSetBuilder::new();
        for pattern in &config.paths {
            let glob_pattern = if pattern.starts_with('/') || pattern.starts_with('*') {
                pattern.clone()
            } else {
                format!("**/{}", pattern)
            };

            if let Ok(glob) = Glob::new(&glob_pattern) {
                builder.add(glob);
            }
        }

        let cacheable_paths = builder.build().ok();

        Self {
            cache: Some(cache),
            cacheable_paths,
            max_entry_size: 5 * 1024 * 1024, // 5 MB max per entry
        }
    }

    pub async fn get(&self, path: &str) -> Option<Response<BoxBody<Bytes, hyper::Error>>> {
        let cache = self.cache.as_ref()?;
        let cached = cache.get(path).await?;

        debug!("Cache hit for: {}", path);

        // Reconstruct response from cached data
        let mut response = Response::builder().status(cached.status);

        // Copy headers
        if let Some(headers) = response.headers_mut() {
            for (key, value) in cached.headers.iter() {
                headers.insert(key.clone(), value.clone());
            }
            // Add cache indicator header
            headers.insert("x-cache", "HIT".parse().unwrap());
        }

        Some(
            response
                .body(
                    Full::new(cached.body.clone())
                        .map_err(|never| match never {})
                        .boxed(),
                )
                .unwrap(),
        )
    }

    #[allow(dead_code)]
    pub async fn set(
        &self,
        path: &str,
        response: &Response<BoxBody<Bytes, hyper::Error>>,
        body: Bytes,
    ) {
        let cache = match &self.cache {
            Some(c) => c,
            None => return,
        };

        // Don't cache if body is too large
        if body.len() > self.max_entry_size {
            debug!("Response too large to cache: {} bytes", body.len());
            return;
        }

        // Check cache-control header
        if let Some(cc) = response.headers().get(CACHE_CONTROL) {
            if let Ok(cc_str) = cc.to_str() {
                // Don't cache if no-store or private
                if cc_str.contains("no-store") || cc_str.contains("private") {
                    debug!("Not caching due to cache-control: {}", cc_str);
                    return;
                }
            }
        }

        let cached = CachedResponse {
            status: response.status(),
            headers: response.headers().clone(),
            body,
        };

        debug!("Caching response for: {}", path);
        cache.insert(path.to_string(), Arc::new(cached)).await;
    }

    pub fn is_cacheable_path(&self, path: &str) -> bool {
        match &self.cacheable_paths {
            Some(set) => set.is_match(path),
            None => false,
        }
    }

    #[allow(dead_code)]
    pub fn is_cacheable_response(&self, response: &Response<BoxBody<Bytes, hyper::Error>>) -> bool {
        // Only cache successful responses
        if !response.status().is_success() {
            return false;
        }

        // Check content type - only cache static assets
        if let Some(ct) = response.headers().get(CONTENT_TYPE) {
            if let Ok(ct_str) = ct.to_str() {
                // Cache static content types
                return ct_str.contains("text/css")
                    || ct_str.contains("text/javascript")
                    || ct_str.contains("application/javascript")
                    || ct_str.contains("application/json")
                    || ct_str.contains("image/")
                    || ct_str.contains("font/")
                    || ct_str.contains("text/html");
            }
        }

        false
    }
}
