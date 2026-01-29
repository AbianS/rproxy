mod config;
mod error;
mod middleware;
mod proxy;
mod server;
mod websocket;

use tracing::info;
use tracing_subscriber::EnvFilter;

use crate::config::Config;
use crate::error::Result;

#[tokio::main]
async fn main() -> Result<()> {
    // Load configuration from environment
    let config = Config::from_env()?;

    // Initialize tracing
    let filter =
        EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new(&config.log_level));

    tracing_subscriber::fmt()
        .with_env_filter(filter)
        .with_target(false)
        .compact()
        .init();

    info!("Starting rproxy v{}", env!("CARGO_PKG_VERSION"));
    info!("Listening on {}", config.server.listen_addr);
    info!("Upstream: {}", config.upstream.address);

    if config.tls.enabled {
        info!("TLS enabled for domains: {:?}", config.tls.domains);
    } else {
        info!("TLS disabled (no domains configured)");
    }

    if config.rate_limit.enabled {
        info!(
            "Rate limiting: {} req/s, burst {}",
            config.rate_limit.requests_per_second, config.rate_limit.burst_size
        );
    }

    if config.cache.enabled {
        info!("Cache enabled, max {} entries", config.cache.max_size);
    }

    // Start the server
    server::run(config).await
}
