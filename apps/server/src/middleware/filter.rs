use globset::{Glob, GlobSet, GlobSetBuilder};

use crate::config::FilterConfig;

pub struct FilterMiddleware {
    blocked: Option<GlobSet>,
}

impl FilterMiddleware {
    pub fn new(config: &FilterConfig) -> Self {
        if !config.enabled || config.blocked_paths.is_empty() {
            return Self { blocked: None };
        }

        let mut builder = GlobSetBuilder::new();

        for pattern in &config.blocked_paths {
            // Convert simple glob patterns to globset format
            let glob_pattern = if pattern.starts_with('/') {
                format!("**{}", pattern)
            } else {
                format!("**/{}", pattern)
            };

            if let Ok(glob) = Glob::new(&glob_pattern) {
                builder.add(glob);
            }
        }

        match builder.build() {
            Ok(set) => Self { blocked: Some(set) },
            Err(_) => Self { blocked: None },
        }
    }

    pub fn is_blocked(&self, path: &str) -> bool {
        match &self.blocked {
            Some(set) => set.is_match(path),
            None => false,
        }
    }
}
