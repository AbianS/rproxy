use std::sync::Arc;

use bytes::Bytes;
use futures_util::StreamExt;
use http_body_util::BodyExt;
use http_body_util::combinators::BoxBody;
use hyper::body::Incoming;
use hyper::{Request, Response, StatusCode};
use tokio::net::TcpStream;
use tracing::{debug, error, info};

use crate::config::Config;
use crate::error::{ProxyError, Result};
use crate::proxy::empty_response;

pub fn is_websocket_upgrade(req: &Request<Incoming>) -> bool {
    let get_header = |key: &str| {
        req.headers()
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

pub async fn handle_websocket(
    req: Request<Incoming>,
    config: Arc<Config>,
) -> Result<Response<BoxBody<Bytes, hyper::Error>>> {
    // Clone values we need for the spawned task
    let upstream_addr = config.upstream.address.clone();
    let path = req.uri().path().to_string();

    info!("WebSocket upgrade request for path: {}", path);

    // Get the websocket key for the response
    let ws_key = req
        .headers()
        .get("sec-websocket-key")
        .and_then(|v| v.to_str().ok())
        .map(|s| s.to_string());

    let ws_key = match ws_key {
        Some(key) => key,
        None => {
            error!("Missing Sec-WebSocket-Key header");
            return Ok(empty_response(StatusCode::BAD_REQUEST));
        }
    };

    // Connect to upstream
    let upstream_stream = TcpStream::connect(&upstream_addr).await.map_err(|e| {
        error!("Failed to connect to upstream for WebSocket: {}", e);
        ProxyError::UpstreamConnection(e.to_string())
    })?;

    // Build WebSocket upgrade response
    let accept_key = generate_accept_key(&ws_key);

    let response = Response::builder()
        .status(StatusCode::SWITCHING_PROTOCOLS)
        .header("Upgrade", "websocket")
        .header("Connection", "Upgrade")
        .header("Sec-WebSocket-Accept", accept_key)
        .body(
            http_body_util::Empty::<Bytes>::new()
                .map_err(|never| match never {})
                .boxed(),
        )
        .unwrap();

    // Spawn the WebSocket proxy task
    let path_clone = path.clone();
    tokio::spawn(async move {
        // Small delay for the upgrade to complete
        tokio::time::sleep(tokio::time::Duration::from_millis(10)).await;

        // Connect to upstream as WebSocket
        let upstream_ws = match tokio_tungstenite::client_async(
            format!("ws://{}{}", upstream_addr, path_clone),
            upstream_stream,
        )
        .await
        {
            Ok((ws, _)) => ws,
            Err(e) => {
                error!("Failed to upgrade upstream to WebSocket: {}", e);
                return;
            }
        };

        debug!("WebSocket connection established to upstream");

        // Split the stream for bidirectional communication
        let (_upstream_write, mut upstream_read) = upstream_ws.split();

        // Keep the upstream connection alive by reading messages
        while let Some(msg) = upstream_read.next().await {
            match msg {
                Ok(_) => {}
                Err(e) => {
                    debug!("Upstream WebSocket closed: {}", e);
                    break;
                }
            }
        }
    });

    Ok(response)
}

fn generate_accept_key(key: &str) -> String {
    const GUID: &str = "258EAFA5-E914-47DA-95CA-C5AB0DC85B11";

    let mut hasher = sha1_smol::Sha1::new();
    hasher.update(key.as_bytes());
    hasher.update(GUID.as_bytes());

    base64_encode(&hasher.digest().bytes())
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

// Minimal SHA1 implementation for WebSocket handshake
mod sha1_smol {
    pub struct Sha1 {
        state: [u32; 5],
        buffer: Vec<u8>,
        length: u64,
    }

    impl Sha1 {
        pub fn new() -> Self {
            Self {
                state: [0x67452301, 0xEFCDAB89, 0x98BADCFE, 0x10325476, 0xC3D2E1F0],
                buffer: Vec::new(),
                length: 0,
            }
        }

        pub fn update(&mut self, data: &[u8]) {
            self.buffer.extend_from_slice(data);
            self.length += data.len() as u64;

            while self.buffer.len() >= 64 {
                let block: [u8; 64] = self.buffer[..64].try_into().unwrap();
                self.process_block(&block);
                self.buffer.drain(..64);
            }
        }

        pub fn digest(mut self) -> Digest {
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

            Digest { state: self.state }
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

            for (i, &w_i) in w.iter().enumerate() {
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
                    .wrapping_add(w_i);

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

    pub struct Digest {
        state: [u32; 5],
    }

    impl Digest {
        pub fn bytes(&self) -> [u8; 20] {
            let mut result = [0u8; 20];
            for (i, &word) in self.state.iter().enumerate() {
                result[i * 4..(i + 1) * 4].copy_from_slice(&word.to_be_bytes());
            }
            result
        }
    }
}
