use std::sync::Mutex;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::Duration;

use tokio::time::Instant;

/// Token bucket shared by any number of tasks. Rate 0 means unlimited.
///
/// Callers take tokens first and wait out the debt, so the long-run rate holds even when a
/// single chunk is larger than the bucket.
pub struct SpeedLimiter {
    rate: AtomicU64,
    bucket: Mutex<Bucket>,
}

impl std::fmt::Debug for SpeedLimiter {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("SpeedLimiter").field("rate", &self.rate.load(Ordering::Relaxed)).finish()
    }
}

struct Bucket {
    tokens: f64,
    last: Instant,
}

/// Longest sleep before the debt is checked again, so a rate change lands quickly.
const SLICE: Duration = Duration::from_millis(100);

fn capacity(rate: u64) -> f64 {
    (rate as f64 / 10.0).max(16.0 * 1024.0)
}

impl Bucket {
    fn refill(&mut self, rate: u64) {
        let now = Instant::now();
        let dt = now.duration_since(self.last).as_secs_f64();
        self.last = now;
        self.tokens = (self.tokens + dt * rate as f64).min(capacity(rate));
    }
}

impl SpeedLimiter {
    pub fn new(bytes_per_sec: u64) -> Self {
        Self {
            rate: AtomicU64::new(bytes_per_sec),
            bucket: Mutex::new(Bucket {
                tokens: 0.0,
                last: Instant::now(),
            }),
        }
    }

    pub fn rate(&self) -> u64 {
        self.rate.load(Ordering::Relaxed)
    }

    pub fn set_rate(&self, bytes_per_sec: u64) {
        let mut b = self.lock();
        let old = self.rate();
        if old != 0 {
            b.refill(old);
        } else {
            b.last = Instant::now();
            b.tokens = 0.0;
        }
        self.rate.store(bytes_per_sec, Ordering::Relaxed);
        if bytes_per_sec != 0 {
            b.tokens = b.tokens.min(capacity(bytes_per_sec));
        }
    }

    /// Wait until `n` bytes may be used.
    pub async fn acquire(&self, n: usize) {
        if n == 0 || self.rate() == 0 {
            return;
        }
        {
            let mut b = self.lock();
            let rate = self.rate();
            if rate == 0 {
                return;
            }
            b.refill(rate);
            b.tokens -= n as f64;
        }
        loop {
            let wait = {
                let mut b = self.lock();
                let rate = self.rate();
                if rate == 0 {
                    return;
                }
                b.refill(rate);
                if b.tokens >= 0.0 {
                    return;
                }
                Duration::from_secs_f64(-b.tokens / rate as f64)
            };
            tokio::time::sleep(wait.min(SLICE)).await;
        }
    }

    fn lock(&self) -> std::sync::MutexGuard<'_, Bucket> {
        self.bucket.lock().unwrap_or_else(|e| e.into_inner())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Arc;

    #[tokio::test(start_paused = true)]
    async fn zero_is_unlimited() {
        let l = SpeedLimiter::new(0);
        let t = Instant::now();
        l.acquire(100_000_000).await;
        assert!(t.elapsed() < Duration::from_millis(1));
    }

    #[tokio::test(start_paused = true)]
    async fn holds_the_rate() {
        let l = SpeedLimiter::new(100_000);
        let t = Instant::now();
        for _ in 0..10 {
            l.acquire(50_000).await;
        }
        let e = t.elapsed().as_secs_f64();
        assert!((4.5..=5.2).contains(&e), "took {e}");
    }

    #[tokio::test(start_paused = true)]
    async fn shared_between_tasks() {
        let l = Arc::new(SpeedLimiter::new(100_000));
        let t = Instant::now();
        let mut set = tokio::task::JoinSet::new();
        for _ in 0..4 {
            let l = l.clone();
            set.spawn(async move {
                for _ in 0..5 {
                    l.acquire(10_000).await;
                }
            });
        }
        while set.join_next().await.is_some() {}
        let e = t.elapsed().as_secs_f64();
        assert!((1.5..=2.2).contains(&e), "took {e}");
    }

    #[tokio::test(start_paused = true)]
    async fn rate_change_applies_at_once() {
        let l = Arc::new(SpeedLimiter::new(1000));
        let l2 = l.clone();
        let waiter = tokio::spawn(async move { l2.acquire(100_000).await });
        tokio::time::sleep(Duration::from_millis(300)).await;
        l.set_rate(0);
        let t = Instant::now();
        waiter.await.unwrap();
        assert!(t.elapsed() < Duration::from_millis(200));
        assert_eq!(l.rate(), 0);
    }
}
