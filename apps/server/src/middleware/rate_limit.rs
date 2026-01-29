use std::net::IpAddr;
use std::num::NonZeroU32;
use std::sync::Arc;

use governor::clock::DefaultClock;
use governor::state::{InMemoryState, NotKeyed};
use governor::{Quota, RateLimiter};

use crate::config::RateLimitConfig;

pub struct RateLimitMiddleware {
    limiter: Option<Arc<RateLimiter<NotKeyed, InMemoryState, DefaultClock>>>,
}

impl RateLimitMiddleware {
    pub fn new(config: &RateLimitConfig) -> Self {
        if !config.enabled {
            return Self { limiter: None };
        }

        let quota = Quota::per_second(NonZeroU32::new(config.requests_per_second).unwrap())
            .allow_burst(NonZeroU32::new(config.burst_size).unwrap());

        let limiter = RateLimiter::direct(quota);

        Self {
            limiter: Some(Arc::new(limiter)),
        }
    }

    pub fn check(&self, _client_ip: IpAddr) -> bool {
        match &self.limiter {
            Some(limiter) => limiter.check().is_ok(),
            None => true,
        }
    }
}
