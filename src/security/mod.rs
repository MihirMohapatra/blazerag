use axum::http::HeaderMap;
use std::collections::{HashMap, HashSet, VecDeque};
use std::time::{Duration, Instant};
use tokio::sync::Mutex;

#[derive(Debug)]
pub struct SecurityState {
    pub require_auth: bool,
    api_keys: HashSet<String>,
    pub rate_limit_per_minute: usize,
    limiter: Mutex<HashMap<String, VecDeque<Instant>>>,
}

#[derive(Debug)]
pub enum SecurityError {
    Unauthorized(String),
    RateLimited(String),
}

impl SecurityState {
    pub fn from_env() -> Self {
        let require_auth = parse_bool_env("REQUIRE_AUTH", false);
        let api_keys = std::env::var("API_KEYS")
            .unwrap_or_default()
            .split(',')
            .map(|key| key.trim().to_string())
            .filter(|key| !key.is_empty())
            .collect::<HashSet<_>>();
        let rate_limit_per_minute = std::env::var("RATE_LIMIT_PER_MINUTE")
            .unwrap_or_else(|_| "120".to_string())
            .parse::<usize>()
            .ok()
            .filter(|v| *v > 0)
            .unwrap_or(120);

        Self {
            require_auth,
            api_keys,
            rate_limit_per_minute,
            limiter: Mutex::new(HashMap::new()),
        }
    }

    pub async fn authorize_and_check_rate_limit(
        &self,
        headers: &HeaderMap,
        tenant_id: &str,
    ) -> Result<(), SecurityError> {
        let provided_key = extract_api_key(headers);
        let identity = if let Some(key) = provided_key.as_ref() {
            if !self.api_keys.is_empty() && !self.api_keys.contains(key) {
                return Err(SecurityError::Unauthorized("Invalid API key".to_string()));
            }
            format!("api_key:{key}")
        } else {
            if self.require_auth {
                return Err(SecurityError::Unauthorized("Missing API key".to_string()));
            }
            format!("tenant:{tenant_id}")
        };

        self.check_rate_limit(&identity).await
    }

    async fn check_rate_limit(&self, identity: &str) -> Result<(), SecurityError> {
        let now = Instant::now();
        let cutoff = now - Duration::from_secs(60);
        let mut guard = self.limiter.lock().await;
        let entries = guard.entry(identity.to_string()).or_default();

        while let Some(ts) = entries.front() {
            if *ts < cutoff {
                entries.pop_front();
            } else {
                break;
            }
        }

        if entries.len() >= self.rate_limit_per_minute {
            return Err(SecurityError::RateLimited(format!(
                "Rate limit exceeded: max {} requests per minute",
                self.rate_limit_per_minute
            )));
        }

        entries.push_back(now);
        Ok(())
    }
}

fn parse_bool_env(key: &str, default: bool) -> bool {
    std::env::var(key)
        .ok()
        .and_then(|v| match v.to_ascii_lowercase().as_str() {
            "1" | "true" | "yes" | "on" => Some(true),
            "0" | "false" | "no" | "off" => Some(false),
            _ => None,
        })
        .unwrap_or(default)
}

pub fn extract_api_key(headers: &HeaderMap) -> Option<String> {
    if let Some(raw) = headers.get("x-api-key").and_then(|v| v.to_str().ok()) {
        let key = raw.trim();
        if !key.is_empty() {
            return Some(key.to_string());
        }
    }

    headers
        .get("authorization")
        .and_then(|v| v.to_str().ok())
        .and_then(|raw| {
            raw.strip_prefix("Bearer ")
                .or_else(|| raw.strip_prefix("bearer "))
        })
        .map(str::trim)
        .filter(|v| !v.is_empty())
        .map(str::to_string)
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::http::HeaderValue;

    #[tokio::test]
    async fn rate_limit_blocks_when_capacity_reached() {
        let state = SecurityState {
            require_auth: false,
            api_keys: HashSet::new(),
            rate_limit_per_minute: 2,
            limiter: Mutex::new(HashMap::new()),
        };

        assert!(state.check_rate_limit("anon").await.is_ok());
        assert!(state.check_rate_limit("anon").await.is_ok());
        assert!(matches!(
            state.check_rate_limit("anon").await,
            Err(SecurityError::RateLimited(_))
        ));
    }

    #[test]
    fn extracts_key_from_headers() {
        let mut headers = HeaderMap::new();
        headers.insert("x-api-key", HeaderValue::from_static("test-key"));
        assert_eq!(extract_api_key(&headers).as_deref(), Some("test-key"));

        headers.clear();
        headers.insert(
            "authorization",
            HeaderValue::from_static("Bearer another-key"),
        );
        assert_eq!(extract_api_key(&headers).as_deref(), Some("another-key"));
    }
}
