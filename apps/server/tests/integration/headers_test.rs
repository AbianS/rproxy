//! Integration tests for header handling

use hyper::HeaderMap;
use hyper::header::HeaderValue;
use pretty_assertions::assert_eq;
use std::net::IpAddr;

/// Strip hop-by-hop headers (mimics proxy.rs logic)
fn strip_hop_by_hop_headers(headers: &mut HeaderMap) {
    const HOP_BY_HOP: &[&str] = &[
        "connection",
        "keep-alive",
        "proxy-authenticate",
        "proxy-authorization",
        "te",
        "trailer",
        "transfer-encoding",
    ];

    for header in HOP_BY_HOP {
        headers.remove(*header);
    }
}

/// Add forwarding headers (mimics proxy.rs logic)
fn add_forwarding_headers(headers: &mut HeaderMap, client_ip: IpAddr, is_tls: bool) {
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
        let proto = if is_tls { "https" } else { "http" };
        headers.insert("x-forwarded-proto", proto.parse().unwrap());
    }
}

#[test]
fn test_strip_connection_header() {
    let mut headers = HeaderMap::new();
    headers.insert("connection", HeaderValue::from_static("keep-alive"));
    headers.insert("content-type", HeaderValue::from_static("application/json"));

    strip_hop_by_hop_headers(&mut headers);

    assert!(!headers.contains_key("connection"));
    assert!(headers.contains_key("content-type"));
}

#[test]
fn test_strip_keep_alive_header() {
    let mut headers = HeaderMap::new();
    headers.insert("keep-alive", HeaderValue::from_static("timeout=5"));

    strip_hop_by_hop_headers(&mut headers);

    assert!(!headers.contains_key("keep-alive"));
}

#[test]
fn test_strip_proxy_headers() {
    let mut headers = HeaderMap::new();
    headers.insert("proxy-authenticate", HeaderValue::from_static("Basic"));
    headers.insert(
        "proxy-authorization",
        HeaderValue::from_static("Basic abc123"),
    );

    strip_hop_by_hop_headers(&mut headers);

    assert!(!headers.contains_key("proxy-authenticate"));
    assert!(!headers.contains_key("proxy-authorization"));
}

#[test]
fn test_strip_transfer_encoding_header() {
    let mut headers = HeaderMap::new();
    headers.insert("transfer-encoding", HeaderValue::from_static("chunked"));

    strip_hop_by_hop_headers(&mut headers);

    assert!(!headers.contains_key("transfer-encoding"));
}

#[test]
fn test_strip_te_header() {
    let mut headers = HeaderMap::new();
    headers.insert("te", HeaderValue::from_static("trailers"));

    strip_hop_by_hop_headers(&mut headers);

    assert!(!headers.contains_key("te"));
}

#[test]
fn test_strip_trailer_header() {
    let mut headers = HeaderMap::new();
    headers.insert("trailer", HeaderValue::from_static("Expires"));

    strip_hop_by_hop_headers(&mut headers);

    assert!(!headers.contains_key("trailer"));
}

#[test]
fn test_strip_all_hop_by_hop_headers() {
    let mut headers = HeaderMap::new();
    headers.insert("connection", HeaderValue::from_static("keep-alive"));
    headers.insert("keep-alive", HeaderValue::from_static("timeout=5"));
    headers.insert("proxy-authenticate", HeaderValue::from_static("Basic"));
    headers.insert("proxy-authorization", HeaderValue::from_static("Basic abc"));
    headers.insert("te", HeaderValue::from_static("trailers"));
    headers.insert("trailer", HeaderValue::from_static("Expires"));
    headers.insert("transfer-encoding", HeaderValue::from_static("chunked"));
    // Non hop-by-hop headers
    headers.insert("content-type", HeaderValue::from_static("application/json"));
    headers.insert("accept", HeaderValue::from_static("*/*"));

    strip_hop_by_hop_headers(&mut headers);

    // Only non hop-by-hop headers should remain
    assert_eq!(headers.len(), 2);
    assert!(headers.contains_key("content-type"));
    assert!(headers.contains_key("accept"));
}

#[test]
fn test_add_x_forwarded_for_new() {
    let mut headers = HeaderMap::new();
    let client_ip: IpAddr = "192.168.1.100".parse().unwrap();

    add_forwarding_headers(&mut headers, client_ip, false);

    assert_eq!(headers.get("x-forwarded-for").unwrap(), "192.168.1.100");
}

#[test]
fn test_add_x_forwarded_for_append() {
    let mut headers = HeaderMap::new();
    headers.insert("x-forwarded-for", HeaderValue::from_static("10.0.0.1"));
    let client_ip: IpAddr = "192.168.1.100".parse().unwrap();

    add_forwarding_headers(&mut headers, client_ip, false);

    assert_eq!(
        headers.get("x-forwarded-for").unwrap(),
        "10.0.0.1, 192.168.1.100"
    );
}

#[test]
fn test_add_x_forwarded_for_chain() {
    let mut headers = HeaderMap::new();
    headers.insert(
        "x-forwarded-for",
        HeaderValue::from_static("10.0.0.1, 10.0.0.2"),
    );
    let client_ip: IpAddr = "192.168.1.100".parse().unwrap();

    add_forwarding_headers(&mut headers, client_ip, false);

    assert_eq!(
        headers.get("x-forwarded-for").unwrap(),
        "10.0.0.1, 10.0.0.2, 192.168.1.100"
    );
}

#[test]
fn test_add_x_real_ip_new() {
    let mut headers = HeaderMap::new();
    let client_ip: IpAddr = "192.168.1.100".parse().unwrap();

    add_forwarding_headers(&mut headers, client_ip, false);

    assert_eq!(headers.get("x-real-ip").unwrap(), "192.168.1.100");
}

#[test]
fn test_add_x_real_ip_preserves_existing() {
    let mut headers = HeaderMap::new();
    headers.insert("x-real-ip", HeaderValue::from_static("10.0.0.1"));
    let client_ip: IpAddr = "192.168.1.100".parse().unwrap();

    add_forwarding_headers(&mut headers, client_ip, false);

    // Should not overwrite existing X-Real-IP
    assert_eq!(headers.get("x-real-ip").unwrap(), "10.0.0.1");
}

#[test]
fn test_add_x_forwarded_proto_http() {
    let mut headers = HeaderMap::new();
    let client_ip: IpAddr = "192.168.1.100".parse().unwrap();

    add_forwarding_headers(&mut headers, client_ip, false);

    assert_eq!(headers.get("x-forwarded-proto").unwrap(), "http");
}

#[test]
fn test_add_x_forwarded_proto_https() {
    let mut headers = HeaderMap::new();
    let client_ip: IpAddr = "192.168.1.100".parse().unwrap();

    add_forwarding_headers(&mut headers, client_ip, true);

    assert_eq!(headers.get("x-forwarded-proto").unwrap(), "https");
}

#[test]
fn test_add_x_forwarded_proto_preserves_existing() {
    let mut headers = HeaderMap::new();
    headers.insert("x-forwarded-proto", HeaderValue::from_static("https"));
    let client_ip: IpAddr = "192.168.1.100".parse().unwrap();

    add_forwarding_headers(&mut headers, client_ip, false);

    // Should not overwrite existing X-Forwarded-Proto
    assert_eq!(headers.get("x-forwarded-proto").unwrap(), "https");
}

#[test]
fn test_forwarding_headers_ipv6() {
    let mut headers = HeaderMap::new();
    let client_ip: IpAddr = "2001:db8::1".parse().unwrap();

    add_forwarding_headers(&mut headers, client_ip, true);

    assert_eq!(headers.get("x-forwarded-for").unwrap(), "2001:db8::1");
    assert_eq!(headers.get("x-real-ip").unwrap(), "2001:db8::1");
}

#[test]
fn test_forwarding_headers_localhost() {
    let mut headers = HeaderMap::new();
    let client_ip: IpAddr = "127.0.0.1".parse().unwrap();

    add_forwarding_headers(&mut headers, client_ip, false);

    assert_eq!(headers.get("x-forwarded-for").unwrap(), "127.0.0.1");
    assert_eq!(headers.get("x-real-ip").unwrap(), "127.0.0.1");
}

#[test]
fn test_full_header_processing() {
    let mut headers = HeaderMap::new();
    // Add some hop-by-hop headers
    headers.insert("connection", HeaderValue::from_static("keep-alive"));
    headers.insert("transfer-encoding", HeaderValue::from_static("chunked"));
    // Add some regular headers
    headers.insert("content-type", HeaderValue::from_static("application/json"));
    headers.insert("accept", HeaderValue::from_static("*/*"));
    headers.insert("host", HeaderValue::from_static("api.example.com"));

    // Strip hop-by-hop
    strip_hop_by_hop_headers(&mut headers);

    // Add forwarding headers
    let client_ip: IpAddr = "203.0.113.50".parse().unwrap();
    add_forwarding_headers(&mut headers, client_ip, true);

    // Verify final state
    assert!(!headers.contains_key("connection"));
    assert!(!headers.contains_key("transfer-encoding"));
    assert!(headers.contains_key("content-type"));
    assert!(headers.contains_key("accept"));
    assert!(headers.contains_key("host"));
    assert_eq!(headers.get("x-forwarded-for").unwrap(), "203.0.113.50");
    assert_eq!(headers.get("x-real-ip").unwrap(), "203.0.113.50");
    assert_eq!(headers.get("x-forwarded-proto").unwrap(), "https");
}
