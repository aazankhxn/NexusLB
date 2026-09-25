use std::time::Duration;
use tokio::time::sleep;

#[derive(Debug, Clone)]
pub struct RetryPolicy {
    pub max_retries: usize,
    pub initial_backoff: Duration,
    pub max_backoff: Duration,
    pub retry_statuses: Vec<u16>,
}

impl Default for RetryPolicy {
    fn default() -> Self {
        Self {
            max_retries: 2,
            initial_backoff: Duration::from_millis(25),
            max_backoff: Duration::from_millis(250),
            retry_statuses: vec![502, 503, 504],
        }
    }
}

impl RetryPolicy {
    pub fn is_retryable_method(method: &str) -> bool {
        matches!(
            method.to_ascii_uppercase().as_str(),
            "GET" | "HEAD" | "OPTIONS"
        )
    }

    pub fn is_retryable_status(&self, status: u16) -> bool {
        self.retry_statuses.contains(&status)
    }

    pub async fn backoff(&self, attempt: usize) {
        if attempt == 0 {
            return;
        }
        let backoff_factor = 1 << attempt.min(6);
        let backoff = (self.initial_backoff * backoff_factor).min(self.max_backoff);
        sleep(backoff).await;
    }
}
