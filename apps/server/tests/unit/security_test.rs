//! Unit tests for security headers

use hyper::HeaderMap;
use hyper::header::HeaderValue;
use pretty_assertions::assert_eq;
use rstest::rstest;

/// Add security headers (mimics SecurityMiddleware logic)
fn add_security_headers(headers: &mut HeaderMap, hsts_enabled: bool) {
    // HSTS
    if hsts_enabled {
        headers.insert(
            "strict-transport-security",
            HeaderValue::from_static("max-age=31536000; includeSubDomains"),
        );
    }

    // Other security headers
    headers.insert(
        "x-content-type-options",
        HeaderValue::from_static("nosniff"),
    );
    headers.insert("x-frame-options", HeaderValue::from_static("DENY"));
    headers.insert(
        "x-xss-protection",
        HeaderValue::from_static("1; mode=block"),
    );
    headers.insert(
        "referrer-policy",
        HeaderValue::from_static("strict-origin-when-cross-origin"),
    );
}

#[test]
fn test_hsts_header_when_enabled() {
    let mut headers = HeaderMap::new();
    add_security_headers(&mut headers, true);

    let hsts = headers.get("strict-transport-security").unwrap();
    assert_eq!(hsts, "max-age=31536000; includeSubDomains");
}

#[test]
fn test_hsts_header_when_disabled() {
    let mut headers = HeaderMap::new();
    add_security_headers(&mut headers, false);

    assert!(headers.get("strict-transport-security").is_none());
}

#[test]
fn test_x_content_type_options_header() {
    let mut headers = HeaderMap::new();
    add_security_headers(&mut headers, true);

    let header = headers.get("x-content-type-options").unwrap();
    assert_eq!(header, "nosniff");
}

#[test]
fn test_x_frame_options_header() {
    let mut headers = HeaderMap::new();
    add_security_headers(&mut headers, true);

    let header = headers.get("x-frame-options").unwrap();
    assert_eq!(header, "DENY");
}

#[test]
fn test_x_xss_protection_header() {
    let mut headers = HeaderMap::new();
    add_security_headers(&mut headers, true);

    let header = headers.get("x-xss-protection").unwrap();
    assert_eq!(header, "1; mode=block");
}

#[test]
fn test_referrer_policy_header() {
    let mut headers = HeaderMap::new();
    add_security_headers(&mut headers, true);

    let header = headers.get("referrer-policy").unwrap();
    assert_eq!(header, "strict-origin-when-cross-origin");
}

#[test]
fn test_all_headers_present() {
    let mut headers = HeaderMap::new();
    add_security_headers(&mut headers, true);

    // Should have 5 headers
    assert_eq!(headers.len(), 5);
    assert!(headers.contains_key("strict-transport-security"));
    assert!(headers.contains_key("x-content-type-options"));
    assert!(headers.contains_key("x-frame-options"));
    assert!(headers.contains_key("x-xss-protection"));
    assert!(headers.contains_key("referrer-policy"));
}

#[test]
fn test_headers_without_hsts() {
    let mut headers = HeaderMap::new();
    add_security_headers(&mut headers, false);

    // Should have 4 headers (no HSTS)
    assert_eq!(headers.len(), 4);
    assert!(!headers.contains_key("strict-transport-security"));
}

#[test]
fn test_headers_do_not_overwrite_existing() {
    let mut headers = HeaderMap::new();
    headers.insert("x-custom-header", HeaderValue::from_static("custom"));

    add_security_headers(&mut headers, true);

    // Custom header should still be there
    assert_eq!(headers.get("x-custom-header").unwrap(), "custom");
    // Security headers should be added
    assert_eq!(headers.len(), 6);
}

#[rstest]
#[case("strict-transport-security", "max-age=31536000; includeSubDomains")]
#[case("x-content-type-options", "nosniff")]
#[case("x-frame-options", "DENY")]
#[case("x-xss-protection", "1; mode=block")]
#[case("referrer-policy", "strict-origin-when-cross-origin")]
fn test_header_values(#[case] header_name: &str, #[case] expected_value: &str) {
    let mut headers = HeaderMap::new();
    add_security_headers(&mut headers, true);

    let header = headers.get(header_name).unwrap();
    assert_eq!(header, expected_value);
}

#[test]
fn test_hsts_max_age_is_one_year() {
    let mut headers = HeaderMap::new();
    add_security_headers(&mut headers, true);

    let hsts = headers
        .get("strict-transport-security")
        .unwrap()
        .to_str()
        .unwrap();

    // 31536000 seconds = 1 year
    assert!(hsts.contains("max-age=31536000"));
}

#[test]
fn test_hsts_includes_subdomains() {
    let mut headers = HeaderMap::new();
    add_security_headers(&mut headers, true);

    let hsts = headers
        .get("strict-transport-security")
        .unwrap()
        .to_str()
        .unwrap();
    assert!(hsts.contains("includeSubDomains"));
}

#[test]
fn test_x_frame_options_deny_prevents_clickjacking() {
    let mut headers = HeaderMap::new();
    add_security_headers(&mut headers, true);

    let xfo = headers.get("x-frame-options").unwrap();
    // DENY is the strictest option - prevents all framing
    assert_eq!(xfo, "DENY");
}

#[test]
fn test_content_type_options_nosniff() {
    let mut headers = HeaderMap::new();
    add_security_headers(&mut headers, true);

    let xcto = headers.get("x-content-type-options").unwrap();
    // nosniff prevents MIME type sniffing
    assert_eq!(xcto, "nosniff");
}
