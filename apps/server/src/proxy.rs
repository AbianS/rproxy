use std::net::IpAddr;
use std::sync::Arc;
use std::time::Duration;

use bytes::Bytes;
use http_body_util::{BodyExt, Empty, Full, combinators::BoxBody};
use hyper::body::Incoming;
use hyper::{Request, Response, StatusCode};
use hyper_util::client::legacy::{Client, connect::HttpConnector};
use hyper_util::rt::TokioExecutor;
use tracing::{debug, error};

use crate::config::Config;
use crate::error::{ProxyError, Result};

/// HTTP client with connection pooling
type HttpClient = Client<HttpConnector, Incoming>;

pub struct ProxyHandler {
    config: Arc<Config>,
    client: HttpClient,
    upstream_uri: String,
}

impl ProxyHandler {
    pub fn new(config: Arc<Config>) -> Result<Self> {
        // Create HTTP connector with connection pooling
        let mut connector = HttpConnector::new();
        connector.set_nodelay(true);
        connector.set_keepalive(Some(Duration::from_secs(60)));
        connector.enforce_http(true);

        // Build client with connection pool
        let client = Client::builder(TokioExecutor::new())
            .pool_idle_timeout(Duration::from_secs(60))
            .pool_max_idle_per_host(256) // Keep many connections ready
            .pool_timer(hyper_util::rt::TokioTimer::new())
            .build(connector);

        // Pre-build upstream URI
        let upstream_uri = format!("http://{}", config.upstream.address);

        Ok(Self {
            config,
            client,
            upstream_uri,
        })
    }

    pub async fn handle(
        &self,
        mut req: Request<Incoming>,
        client_ip: IpAddr,
    ) -> Result<Response<BoxBody<Bytes, hyper::Error>>> {
        // Add forwarding headers
        self.add_forwarding_headers(&mut req, client_ip);

        // Strip hop-by-hop headers
        self.strip_hop_by_hop_headers(&mut req);

        // Rewrite URI to upstream
        let path_and_query = req
            .uri()
            .path_and_query()
            .map(|pq| pq.as_str())
            .unwrap_or("/");

        let upstream_uri = format!("{}{}", self.upstream_uri, path_and_query);

        *req.uri_mut() = upstream_uri
            .parse()
            .map_err(|e| ProxyError::UpstreamConnection(format!("Invalid URI: {}", e)))?;

        // Update Host header
        if let Ok(host) = self.config.upstream.address.parse() {
            req.headers_mut().insert("host", host);
        }

        // Forward request using pooled connection
        let timeout_duration = self.config.upstream.timeout;

        let response = tokio::time::timeout(timeout_duration, self.client.request(req))
            .await
            .map_err(|_| {
                debug!("Upstream request timeout");
                ProxyError::UpstreamTimeout
            })?
            .map_err(|e| {
                error!("Upstream request failed: {}", e);
                ProxyError::UpstreamConnection(e.to_string())
            })?;

        // Convert response body
        let (parts, body) = response.into_parts();
        let boxed_body = body.boxed();

        Ok(Response::from_parts(parts, boxed_body))
    }

    fn add_forwarding_headers(&self, req: &mut Request<Incoming>, client_ip: IpAddr) {
        let headers = req.headers_mut();

        // X-Forwarded-For
        let xff = if let Some(existing) = headers.get("x-forwarded-for") {
            format!("{}, {}", existing.to_str().unwrap_or(""), client_ip)
        } else {
            client_ip.to_string()
        };
        headers.insert("x-forwarded-for", xff.parse().unwrap());

        // X-Real-IP (only set if not already present)
        if !headers.contains_key("x-real-ip") {
            headers.insert("x-real-ip", client_ip.to_string().parse().unwrap());
        }

        // X-Forwarded-Proto
        if !headers.contains_key("x-forwarded-proto") {
            let proto = if self.config.tls.enabled {
                "https"
            } else {
                "http"
            };
            headers.insert("x-forwarded-proto", proto.parse().unwrap());
        }

        // X-Forwarded-Host
        if !headers.contains_key("x-forwarded-host") {
            if let Some(host) = headers.get("host").cloned() {
                headers.insert("x-forwarded-host", host);
            }
        }
    }

    fn strip_hop_by_hop_headers(&self, req: &mut Request<Incoming>) {
        const HOP_BY_HOP: &[&str] = &[
            "connection",
            "keep-alive",
            "proxy-authenticate",
            "proxy-authorization",
            "te",
            "trailer",
            "transfer-encoding",
        ];

        let headers = req.headers_mut();
        for header in HOP_BY_HOP {
            headers.remove(*header);
        }
    }
}

pub fn empty_response(status: StatusCode) -> Response<BoxBody<Bytes, hyper::Error>> {
    Response::builder()
        .status(status)
        .body(Empty::new().map_err(|never| match never {}).boxed())
        .unwrap()
}

pub fn text_response(status: StatusCode, text: &str) -> Response<BoxBody<Bytes, hyper::Error>> {
    Response::builder()
        .status(status)
        .header("content-type", "text/plain; charset=utf-8")
        .body(
            Full::new(Bytes::from(text.to_string()))
                .map_err(|never| match never {})
                .boxed(),
        )
        .unwrap()
}
