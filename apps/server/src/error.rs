use std::io;

use thiserror::Error;

#[derive(Error, Debug)]
#[allow(dead_code)]
pub enum ProxyError {
    #[error("Configuration error: {0}")]
    Config(String),

    #[error("IO error: {0}")]
    Io(#[from] io::Error),

    #[error("HTTP error: {0}")]
    Http(#[from] http::Error),

    #[error("Hyper error: {0}")]
    Hyper(#[from] hyper::Error),

    #[error("TLS error: {0}")]
    Tls(String),

    #[error("Upstream connection failed: {0}")]
    UpstreamConnection(String),

    #[error("Upstream timeout")]
    UpstreamTimeout,

    #[error("WebSocket error: {0}")]
    WebSocket(String),

    #[error("Rate limited")]
    RateLimited,

    #[error("Path blocked: {0}")]
    PathBlocked(String),
}

pub type Result<T> = std::result::Result<T, ProxyError>;
