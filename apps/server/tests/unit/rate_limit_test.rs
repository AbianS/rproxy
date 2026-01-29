//! Unit tests for rate limiting

use governor::{Quota, RateLimiter};
use pretty_assertions::assert_eq;
use rstest::rstest;
use std::net::IpAddr;
use std::num::NonZeroU32;
use std::sync::Arc;
use std::time::Duration;

type KeyedRateLimiter = RateLimiter<
    IpAddr,
    governor::state::keyed::DefaultKeyedStateStore<IpAddr>,
    governor::clock::DefaultClock,
>;

/// Create a rate limiter for testing
fn create_rate_limiter(rps: u32, burst: u32) -> Arc<KeyedRateLimiter> {
    Arc::new(RateLimiter::keyed(
        Quota::per_second(NonZeroU32::new(rps).unwrap())
            .allow_burst(NonZeroU32::new(burst).unwrap()),
    ))
}

#[test]
fn test_rate_limiter_allows_within_limit() {
    let limiter = create_rate_limiter(10, 10);
    let ip: IpAddr = "192.168.1.1".parse().unwrap();

    // First 10 requests should be allowed (burst)
    for i in 0..10 {
        assert!(
            limiter.check_key(&ip).is_ok(),
            "Request {} should be allowed",
            i + 1
        );
    }
}

#[test]
fn test_rate_limiter_blocks_over_limit() {
    let limiter = create_rate_limiter(10, 10);
    let ip: IpAddr = "192.168.1.1".parse().unwrap();

    // Exhaust the burst
    for _ in 0..10 {
        let _ = limiter.check_key(&ip);
    }

    // Next request should be blocked
    assert!(limiter.check_key(&ip).is_err());
}

#[test]
fn test_rate_limiter_different_ips_independent() {
    let limiter = create_rate_limiter(5, 5);
    let ip1: IpAddr = "192.168.1.1".parse().unwrap();
    let ip2: IpAddr = "192.168.1.2".parse().unwrap();

    // Exhaust ip1's quota
    for _ in 0..5 {
        let _ = limiter.check_key(&ip1);
    }

    // ip1 should be blocked
    assert!(limiter.check_key(&ip1).is_err());

    // ip2 should still be allowed
    assert!(limiter.check_key(&ip2).is_ok());
}

#[test]
fn test_rate_limiter_ipv6() {
    let limiter = create_rate_limiter(10, 10);
    let ip: IpAddr = "2001:db8::1".parse().unwrap();

    // Should work with IPv6
    assert!(limiter.check_key(&ip).is_ok());
}

#[test]
fn test_rate_limiter_localhost() {
    let limiter = create_rate_limiter(10, 10);
    let ip: IpAddr = "127.0.0.1".parse().unwrap();

    // Should work with localhost
    assert!(limiter.check_key(&ip).is_ok());
}

#[rstest]
#[case(1, 1)]
#[case(10, 10)]
#[case(100, 200)]
#[case(1000, 2000)]
fn test_rate_limiter_various_configs(#[case] rps: u32, #[case] burst: u32) {
    let limiter = create_rate_limiter(rps, burst);
    let ip: IpAddr = "10.0.0.1".parse().unwrap();

    // Should allow up to burst requests
    for i in 0..burst {
        assert!(
            limiter.check_key(&ip).is_ok(),
            "Request {} of {} burst should be allowed",
            i + 1,
            burst
        );
    }

    // Next request should be blocked (unless refilled)
    assert!(
        limiter.check_key(&ip).is_err(),
        "Request {} should be blocked (over burst)",
        burst + 1
    );
}

#[test]
fn test_rate_limiter_burst_larger_than_rps() {
    // Burst can be larger than per-second rate
    let limiter = create_rate_limiter(10, 50);
    let ip: IpAddr = "192.168.1.1".parse().unwrap();

    // Should allow 50 requests initially (burst)
    for i in 0..50 {
        assert!(
            limiter.check_key(&ip).is_ok(),
            "Request {} should be allowed",
            i + 1
        );
    }

    // 51st should be blocked
    assert!(limiter.check_key(&ip).is_err());
}

#[tokio::test]
async fn test_rate_limiter_refills_over_time() {
    let limiter = create_rate_limiter(100, 1); // 100 per second, burst of 1
    let ip: IpAddr = "192.168.1.1".parse().unwrap();

    // Use the one allowed request
    assert!(limiter.check_key(&ip).is_ok());

    // Should be blocked now
    assert!(limiter.check_key(&ip).is_err());

    // Wait for refill (10ms = 1 token at 100/s)
    tokio::time::sleep(Duration::from_millis(15)).await;

    // Should be allowed again
    assert!(limiter.check_key(&ip).is_ok());
}

#[test]
fn test_quota_per_second_calculation() {
    // Test that quota is correctly calculated
    let quota = Quota::per_second(NonZeroU32::new(100).unwrap());

    // Replenish interval should be 10ms for 100/s
    let period = quota.replenish_interval();
    assert_eq!(period, Duration::from_millis(10));
}

#[test]
fn test_quota_with_burst() {
    let quota =
        Quota::per_second(NonZeroU32::new(10).unwrap()).allow_burst(NonZeroU32::new(20).unwrap());

    // Burst should be 20
    assert_eq!(quota.burst_size().get(), 20);
}

#[test]
fn test_multiple_ips_high_load() {
    let limiter = create_rate_limiter(10, 10);

    // Simulate 100 different IPs
    for i in 0..100u8 {
        let ip: IpAddr = format!("192.168.1.{}", i).parse().unwrap();
        assert!(
            limiter.check_key(&ip).is_ok(),
            "First request from IP {} should be allowed",
            i
        );
    }
}

#[test]
fn test_rate_limiter_thread_safety() {
    use std::thread;

    let limiter = create_rate_limiter(1000, 1000);
    let limiter = Arc::new(limiter);

    let handles: Vec<_> = (0..10)
        .map(|i| {
            let limiter = limiter.clone();
            thread::spawn(move || {
                let ip: IpAddr = format!("10.0.0.{}", i).parse().unwrap();
                for _ in 0..100 {
                    let _ = limiter.check_key(&ip);
                }
            })
        })
        .collect();

    for handle in handles {
        handle.join().unwrap();
    }
}
