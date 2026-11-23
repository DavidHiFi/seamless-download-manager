use std::collections::VecDeque;
use std::time::{Duration, Instant};

const WINDOW: Duration = Duration::from_secs(3);

/// Bytes per second over the last few seconds.
pub(crate) struct SpeedMeter {
    samples: VecDeque<(Instant, u64)>,
}

impl SpeedMeter {
    pub(crate) fn new(start_bytes: u64) -> Self {
        let mut samples = VecDeque::new();
        samples.push_back((Instant::now(), start_bytes));
        Self { samples }
    }

    pub(crate) fn sample(&mut self, bytes: u64) -> u64 {
        let now = Instant::now();
        self.samples.push_back((now, bytes));
        while self.samples.len() > 2 && now.duration_since(self.samples[1].0) >= WINDOW {
            self.samples.pop_front();
        }
        let (t0, b0) = self.samples[0];
        let dt = now.duration_since(t0).as_secs_f64();
        if dt < 0.05 {
            return 0;
        }
        (bytes.saturating_sub(b0) as f64 / dt) as u64
    }
}
