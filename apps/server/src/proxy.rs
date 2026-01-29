use std::net::IpAddr;
use std::sync::Arc;

use bytes::Bytes;
use http_body_util::{BodyExt, Empty, Full, combinators::BoxBody};
use hyper::body::Incoming;
use hyper::client::conn::http1::Builder as Http1Builder;
use hyper::{Request, Response, StatusCode};
use hyper_util::rt::TokioIo;
use tokio::net::TcpStream;
use tracing::{debug, error};

use crate::config::Config;
use crate::error::{ProxyError, Result};

pub struct ProxyHandler {
    config: Arc<Config>,
}

impl ProxyHandler {
    pub fn new(config: Arc<Config>) -> Result<Self> {
        Ok(Self { config })
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

        // Connect to upstream with timeout
        let upstream_addr = &self.config.upstream.address;
        let timeout_duration = self.config.upstream.timeout;

        let stream = tokio::time::timeout(timeout_duration, TcpStream::connect(upstream_addr))
            .await
            .map_err(|_| {
                error!("Upstream connection timeout: {}", upstream_addr);
                ProxyError::UpstreamTimeout
            })?
            .map_err(|e| {
                error!("Failed to connect to upstream {}: {}", upstream_addr, e);
                ProxyError::UpstreamConnection(e.to_string())
            })?;

        let io = TokioIo::new(stream);

        // Create HTTP/1.1 connection
        let (mut sender, conn) = Http1Builder::new()
            .preserve_header_case(true)
            .title_case_headers(false)
            .handshake(io)
            .await
            .map_err(|e| ProxyError::UpstreamConnection(e.to_string()))?;

        // Spawn connection handler
        tokio::spawn(async move {
            if let Err(e) = conn.await {
                debug!("Upstream connection closed: {}", e);
            }
        });

        // Forward request
        let response = sender
            .send_request(req)
            .await
            .map_err(|e| ProxyError::UpstreamConnection(e.to_string()))?;

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
