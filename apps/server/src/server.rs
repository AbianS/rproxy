use std::net::SocketAddr;
use std::sync::Arc;

use futures_util::StreamExt;
use hyper::server::conn::http1;
use hyper::service::service_fn;
use hyper_util::rt::{TokioExecutor, TokioIo};
use hyper_util::server::conn::auto::Builder as AutoBuilder;
use tokio::net::TcpListener;
use tokio_rustls_acme::AcmeConfig;
use tokio_rustls_acme::caches::DirCache;
use tokio_stream::wrappers::TcpListenerStream;
use tracing::{error, info, warn};

use crate::config::Config;
use crate::error::Result;
use crate::middleware::MiddlewareStack;
use crate::proxy::ProxyHandler;

pub async fn run(config: Config) -> Result<()> {
    let config = Arc::new(config);

    // Create the proxy handler
    let proxy = Arc::new(ProxyHandler::new(config.clone())?);

    // Create middleware stack
    let middleware = Arc::new(MiddlewareStack::new(config.clone()));

    if config.tls.enabled {
        run_with_tls(config, proxy, middleware).await
    } else {
        run_without_tls(config, proxy, middleware).await
    }
}

async fn run_with_tls(
    config: Arc<Config>,
    proxy: Arc<ProxyHandler>,
    middleware: Arc<MiddlewareStack>,
) -> Result<()> {
    // Ensure cache directory exists
    let cache_dir = &config.tls.cache_dir;
    if !cache_dir.exists() {
        std::fs::create_dir_all(cache_dir).map_err(|e| {
            crate::error::ProxyError::Config(format!("Failed to create TLS cache directory: {}", e))
        })?;
    }

    // Spawn HTTP redirect server
    if config.server.http_port > 0 {
        let http_addr: SocketAddr = format!("0.0.0.0:{}", config.server.http_port)
            .parse()
            .unwrap();
        let config_clone = config.clone();
        tokio::spawn(async move {
            if let Err(e) = run_http_redirect(http_addr, config_clone).await {
                error!("HTTP redirect server error: {}", e);
            }
        });
    }

    // Bind HTTPS listener
    let tcp_listener = TcpListener::bind(config.server.listen_addr).await?;
    let tcp_incoming = TcpListenerStream::new(tcp_listener);

    info!(
        "HTTPS server listening on {} (TLS + HTTP/1.1 + HTTP/2)",
        config.server.listen_addr
    );
    info!("Domains: {:?}", config.tls.domains);

    // Configure ACME and create TLS incoming stream
    let mut acme_config = AcmeConfig::new(&config.tls.domains)
        .cache(DirCache::new(cache_dir.clone()))
        .directory_lets_encrypt(true);

    if !config.tls.email.is_empty() {
        acme_config = acme_config.contact_push(format!("mailto:{}", config.tls.email));
    }

    let mut tls_incoming = acme_config.incoming(tcp_incoming, Vec::new());

    // Accept TLS connections with HTTP/1.1 and HTTP/2 support
    while let Some(tls_result) = tls_incoming.next().await {
        let tls_stream = match tls_result {
            Ok(stream) => stream,
            Err(e) => {
                warn!("TLS accept error: {}", e);
                continue;
            }
        };

        // Get peer address before moving the stream
        let remote_addr = tls_stream
            .get_ref()
            .0
            .peer_addr()
            .unwrap_or_else(|_| "0.0.0.0:0".parse().unwrap());

        let proxy = proxy.clone();
        let middleware = middleware.clone();

        tokio::spawn(async move {
            let io = TokioIo::new(tls_stream);

            let service = service_fn(move |req| {
                let proxy = proxy.clone();
                let middleware = middleware.clone();
                async move { middleware.handle(req, remote_addr, &proxy).await }
            });

            // Auto-detect HTTP/1.1 or HTTP/2
            if let Err(e) = AutoBuilder::new(TokioExecutor::new())
                .serve_connection_with_upgrades(io, service)
                .await
            {
                if !is_connection_closed_error(&e) {
                    error!("Connection error from {}: {}", remote_addr, e);
                }
            }
        });
    }

    Ok(())
}

async fn run_without_tls(
    config: Arc<Config>,
    proxy: Arc<ProxyHandler>,
    middleware: Arc<MiddlewareStack>,
) -> Result<()> {
    let listener = TcpListener::bind(config.server.listen_addr).await?;
    info!(
        "HTTP server listening on {} (HTTP/1.1 only, TLS disabled)",
        config.server.listen_addr
    );

    loop {
        let (stream, remote_addr) = match listener.accept().await {
            Ok(conn) => conn,
            Err(e) => {
                error!("Failed to accept connection: {}", e);
                continue;
            }
        };

        let proxy = proxy.clone();
        let middleware = middleware.clone();

        tokio::spawn(async move {
            let io = TokioIo::new(stream);

            let service = service_fn(move |req| {
                let proxy = proxy.clone();
                let middleware = middleware.clone();
                async move { middleware.handle(req, remote_addr, &proxy).await }
            });

            // HTTP/1.1 only for non-TLS (HTTP/2 requires TLS in practice)
            if let Err(e) = http1::Builder::new()
                .preserve_header_case(true)
                .title_case_headers(false)
                .serve_connection(io, service)
                .with_upgrades()
                .await
            {
                if !is_connection_closed_error(&e) {
                    error!("Connection error from {}: {}", remote_addr, e);
                }
            }
        });
    }
}

async fn run_http_redirect(addr: SocketAddr, config: Arc<Config>) -> Result<()> {
    let listener = TcpListener::bind(addr).await?;
    info!("HTTP redirect server listening on {}", addr);

    loop {
        let (stream, _) = listener.accept().await?;
        let config = config.clone();

        tokio::spawn(async move {
            let io = TokioIo::new(stream);

            let service = service_fn(move |req| {
                let host = req
                    .headers()
                    .get("host")
                    .and_then(|h| h.to_str().ok())
                    .unwrap_or_else(|| {
                        config
                            .tls
                            .domains
                            .first()
                            .map(|s| s.as_str())
                            .unwrap_or("localhost")
                    });

                let uri = req.uri();
                let path = uri.path_and_query().map(|p| p.as_str()).unwrap_or("/");

                let redirect_url = format!("https://{}{}", host, path);

                async move {
                    Ok::<_, hyper::Error>(
                        hyper::Response::builder()
                            .status(301)
                            .header("Location", redirect_url)
                            .header("Content-Length", "0")
                            .body(http_body_util::Empty::<bytes::Bytes>::new())
                            .unwrap(),
                    )
                }
            });

            let _ = http1::Builder::new().serve_connection(io, service).await;
        });
    }
}

fn is_connection_closed_error<E: std::fmt::Display>(e: &E) -> bool {
    let msg = e.to_string();
    msg.contains("connection closed")
        || msg.contains("reset by peer")
        || msg.contains("broken pipe")
        || msg.contains("connection reset")
}
