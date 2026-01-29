use hyper::header::HeaderMap;

use crate::config::SecurityConfig;

pub struct SecurityMiddleware {
    hsts_enabled: bool,
    headers: Vec<(String, String)>,
}

impl SecurityMiddleware {
    pub fn new(config: &SecurityConfig) -> Self {
        Self {
            hsts_enabled: config.hsts_enabled,
            headers: config.headers.clone(),
        }
    }

    pub fn add_headers(&self, headers: &mut HeaderMap) {
        // Add HSTS header
        if self.hsts_enabled {
            if let Ok(value) = "max-age=31536000; includeSubDomains".parse() {
                headers.insert("strict-transport-security", value);
            }
        }

        // Add custom security headers
        for (name, value) in &self.headers {
            if let (Ok(header_name), Ok(header_value)) = (
                name.parse::<hyper::header::HeaderName>(),
                value.parse::<hyper::header::HeaderValue>(),
            ) {
                headers.insert(header_name, header_value);
            }
        }
    }
}
