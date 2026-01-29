//! Unit tests for WebSocket handling

use hyper::HeaderMap;
use hyper::header::HeaderValue;
use pretty_assertions::assert_eq;
use rstest::rstest;

/// Check if a request is a WebSocket upgrade (mimics is_websocket_upgrade logic)
fn is_websocket_upgrade(headers: &HeaderMap) -> bool {
    let get_header = |key: &str| {
        headers
            .get(key)
            .and_then(|v| v.to_str().ok())
            .map(|s| s.to_lowercase())
    };

    get_header("connection")
        .map(|v| v.contains("upgrade"))
        .unwrap_or(false)
        && get_header("upgrade")
            .map(|v| v.contains("websocket"))
            .unwrap_or(false)
}

/// Generate WebSocket accept key (mimics generate_accept_key logic)
fn generate_accept_key(key: &str) -> String {
    const GUID: &str = "258EAFA5-E914-47DA-95CA-C5AB0DC85B11";

    let mut hasher = Sha1::new();
    hasher.update(key.as_bytes());
    hasher.update(GUID.as_bytes());

    base64_encode(&hasher.digest())
}

// Minimal SHA1 implementation for testing
struct Sha1 {
    state: [u32; 5],
    buffer: Vec<u8>,
    length: u64,
}

impl Sha1 {
    fn new() -> Self {
        Self {
            state: [0x67452301, 0xEFCDAB89, 0x98BADCFE, 0x10325476, 0xC3D2E1F0],
            buffer: Vec::new(),
            length: 0,
        }
    }

    fn update(&mut self, data: &[u8]) {
        self.buffer.extend_from_slice(data);
        self.length += data.len() as u64;

        while self.buffer.len() >= 64 {
            let block: [u8; 64] = self.buffer[..64].try_into().unwrap();
            self.process_block(&block);
            self.buffer.drain(..64);
        }
    }

    fn digest(mut self) -> [u8; 20] {
        let bit_length = self.length * 8;

        self.buffer.push(0x80);
        while (self.buffer.len() % 64) != 56 {
            self.buffer.push(0x00);
        }
        self.buffer.extend_from_slice(&bit_length.to_be_bytes());

        while self.buffer.len() >= 64 {
            let block: [u8; 64] = self.buffer[..64].try_into().unwrap();
            self.process_block(&block);
            self.buffer.drain(..64);
        }

        let mut result = [0u8; 20];
        for (i, &word) in self.state.iter().enumerate() {
            result[i * 4..(i + 1) * 4].copy_from_slice(&word.to_be_bytes());
        }
        result
    }

    fn process_block(&mut self, block: &[u8; 64]) {
        let mut w = [0u32; 80];

        for i in 0..16 {
            w[i] = u32::from_be_bytes(block[i * 4..(i + 1) * 4].try_into().unwrap());
        }

        for i in 16..80 {
            w[i] = (w[i - 3] ^ w[i - 8] ^ w[i - 14] ^ w[i - 16]).rotate_left(1);
        }

        let mut a = self.state[0];
        let mut b = self.state[1];
        let mut c = self.state[2];
        let mut d = self.state[3];
        let mut e = self.state[4];

        for i in 0..80 {
            let (f, k) = match i {
                0..=19 => ((b & c) | ((!b) & d), 0x5A827999u32),
                20..=39 => (b ^ c ^ d, 0x6ED9EBA1u32),
                40..=59 => ((b & c) | (b & d) | (c & d), 0x8F1BBCDCu32),
                _ => (b ^ c ^ d, 0xCA62C1D6u32),
            };

            let temp = a
                .rotate_left(5)
                .wrapping_add(f)
                .wrapping_add(e)
                .wrapping_add(k)
                .wrapping_add(w[i]);

            e = d;
            d = c;
            c = b.rotate_left(30);
            b = a;
            a = temp;
        }

        self.state[0] = self.state[0].wrapping_add(a);
        self.state[1] = self.state[1].wrapping_add(b);
        self.state[2] = self.state[2].wrapping_add(c);
        self.state[3] = self.state[3].wrapping_add(d);
        self.state[4] = self.state[4].wrapping_add(e);
    }
}

fn base64_encode(data: &[u8]) -> String {
    const ALPHABET: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";

    let mut result = String::new();

    for chunk in data.chunks(3) {
        let b0 = chunk[0] as usize;
        let b1 = chunk.get(1).copied().unwrap_or(0) as usize;
        let b2 = chunk.get(2).copied().unwrap_or(0) as usize;

        result.push(ALPHABET[b0 >> 2] as char);
        result.push(ALPHABET[((b0 & 0x03) << 4) | (b1 >> 4)] as char);

        if chunk.len() > 1 {
            result.push(ALPHABET[((b1 & 0x0f) << 2) | (b2 >> 6)] as char);
        } else {
            result.push('=');
        }

        if chunk.len() > 2 {
            result.push(ALPHABET[b2 & 0x3f] as char);
        } else {
            result.push('=');
        }
    }

    result
}

#[test]
fn test_websocket_upgrade_detection_valid() {
    let mut headers = HeaderMap::new();
    headers.insert("connection", HeaderValue::from_static("Upgrade"));
    headers.insert("upgrade", HeaderValue::from_static("websocket"));

    assert!(is_websocket_upgrade(&headers));
}

#[test]
fn test_websocket_upgrade_case_insensitive() {
    let mut headers = HeaderMap::new();
    headers.insert("connection", HeaderValue::from_static("UPGRADE"));
    headers.insert("upgrade", HeaderValue::from_static("WEBSOCKET"));

    assert!(is_websocket_upgrade(&headers));
}

#[test]
fn test_websocket_upgrade_mixed_case() {
    let mut headers = HeaderMap::new();
    headers.insert("connection", HeaderValue::from_static("Upgrade"));
    headers.insert("upgrade", HeaderValue::from_static("WebSocket"));

    assert!(is_websocket_upgrade(&headers));
}

#[test]
fn test_websocket_upgrade_with_keep_alive() {
    let mut headers = HeaderMap::new();
    headers.insert(
        "connection",
        HeaderValue::from_static("keep-alive, Upgrade"),
    );
    headers.insert("upgrade", HeaderValue::from_static("websocket"));

    assert!(is_websocket_upgrade(&headers));
}

#[test]
fn test_not_websocket_missing_connection() {
    let mut headers = HeaderMap::new();
    headers.insert("upgrade", HeaderValue::from_static("websocket"));

    assert!(!is_websocket_upgrade(&headers));
}

#[test]
fn test_not_websocket_missing_upgrade() {
    let mut headers = HeaderMap::new();
    headers.insert("connection", HeaderValue::from_static("Upgrade"));

    assert!(!is_websocket_upgrade(&headers));
}

#[test]
fn test_not_websocket_wrong_upgrade_value() {
    let mut headers = HeaderMap::new();
    headers.insert("connection", HeaderValue::from_static("Upgrade"));
    headers.insert("upgrade", HeaderValue::from_static("h2c"));

    assert!(!is_websocket_upgrade(&headers));
}

#[test]
fn test_not_websocket_wrong_connection_value() {
    let mut headers = HeaderMap::new();
    headers.insert("connection", HeaderValue::from_static("keep-alive"));
    headers.insert("upgrade", HeaderValue::from_static("websocket"));

    assert!(!is_websocket_upgrade(&headers));
}

#[test]
fn test_not_websocket_empty_headers() {
    let headers = HeaderMap::new();
    assert!(!is_websocket_upgrade(&headers));
}

#[test]
fn test_websocket_accept_key_generation() {
    // RFC 6455 example
    let key = "dGhlIHNhbXBsZSBub25jZQ==";
    let expected = "s3pPLMBiTxaQ9kYGzzhZRbK+xOo=";

    let accept = generate_accept_key(key);
    assert_eq!(accept, expected);
}

#[rstest]
#[case("dGhlIHNhbXBsZSBub25jZQ==", "s3pPLMBiTxaQ9kYGzzhZRbK+xOo=")]
#[case("x3JJHMbDL1EzLkh9GBhXDw==", "HSmrc0sMlYUkAGmm5OPpG2HaGWk=")]
fn test_websocket_accept_key_known_values(#[case] key: &str, #[case] expected: &str) {
    let accept = generate_accept_key(key);
    assert_eq!(accept, expected);
}

#[test]
fn test_base64_encoding() {
    // Test empty
    assert_eq!(base64_encode(&[]), "");

    // Test single byte
    assert_eq!(base64_encode(&[0x4d]), "TQ==");

    // Test two bytes
    assert_eq!(base64_encode(&[0x4d, 0x61]), "TWE=");

    // Test three bytes (no padding)
    assert_eq!(base64_encode(&[0x4d, 0x61, 0x6e]), "TWFu");
}

#[test]
fn test_sha1_empty_string() {
    let mut hasher = Sha1::new();
    hasher.update(b"");
    let digest = hasher.digest();

    // SHA1 of empty string
    let expected: [u8; 20] = [
        0xda, 0x39, 0xa3, 0xee, 0x5e, 0x6b, 0x4b, 0x0d, 0x32, 0x55, 0xbf, 0xef, 0x95, 0x60, 0x18,
        0x90, 0xaf, 0xd8, 0x07, 0x09,
    ];
    assert_eq!(digest, expected);
}

#[test]
fn test_sha1_abc() {
    let mut hasher = Sha1::new();
    hasher.update(b"abc");
    let digest = hasher.digest();

    // SHA1 of "abc"
    let expected: [u8; 20] = [
        0xa9, 0x99, 0x3e, 0x36, 0x47, 0x06, 0x81, 0x6a, 0xba, 0x3e, 0x25, 0x71, 0x78, 0x50, 0xc2,
        0x6c, 0x9c, 0xd0, 0xd8, 0x9d,
    ];
    assert_eq!(digest, expected);
}

/// Test WebSocket path matching patterns
fn is_websocket_path(path: &str, patterns: &[&str]) -> bool {
    patterns.iter().any(|pattern| {
        if pattern.ends_with("/*") {
            let prefix = &pattern[..pattern.len() - 2];
            path.starts_with(prefix)
        } else if pattern.ends_with("*") {
            let prefix = &pattern[..pattern.len() - 1];
            path.starts_with(prefix)
        } else {
            path == *pattern
        }
    })
}

#[rstest]
#[case("/_next/webpack-hmr", &["/_next/webpack-hmr"], true)]
#[case("/_next/webpack-hmr", &["/ws/*"], false)]
#[case("/ws/chat", &["/ws/*"], true)]
#[case("/ws/notifications", &["/ws/*"], true)]
#[case("/socket.io/123", &["/socket.io/*"], true)]
#[case("/api/data", &["/_next/webpack-hmr", "/ws/*"], false)]
fn test_websocket_path_matching(
    #[case] path: &str,
    #[case] patterns: &[&str],
    #[case] expected: bool,
) {
    assert_eq!(
        is_websocket_path(path, patterns),
        expected,
        "Path '{}' with patterns {:?}",
        path,
        patterns
    );
}

#[test]
fn test_websocket_default_paths() {
    let default_patterns = ["/_next/webpack-hmr", "/ws", "/ws/*", "/socket.io/*"];

    // Should match
    assert!(is_websocket_path("/_next/webpack-hmr", &default_patterns));
    assert!(is_websocket_path("/ws", &default_patterns));
    assert!(is_websocket_path("/ws/chat", &default_patterns));
    assert!(is_websocket_path("/socket.io/123", &default_patterns));

    // Should not match
    assert!(!is_websocket_path("/api/data", &default_patterns));
    assert!(!is_websocket_path("/", &default_patterns));
}
