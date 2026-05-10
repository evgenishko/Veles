use std::time::{Duration, Instant};

use tokio::sync::Mutex;

#[derive(Debug)]
pub struct RateLimiter {
    min_interval: Duration,
    next_allowed: Mutex<Instant>,
}

impl RateLimiter {
    pub fn new(requests_per_second: u32) -> Self {
        let min_interval = Duration::from_secs_f64(1.0 / f64::from(requests_per_second));
        Self {
            min_interval,
            next_allowed: Mutex::new(Instant::now()),
        }
    }

    pub async fn wait(&self) {
        let mut next_allowed = self.next_allowed.lock().await;
        let now = Instant::now();

        if *next_allowed > now {
            tokio::time::sleep(*next_allowed - now).await;
        }

        *next_allowed = Instant::now() + self.min_interval;
    }
}
