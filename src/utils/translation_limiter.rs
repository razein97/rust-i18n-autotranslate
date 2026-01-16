use std::{
    sync::Mutex,
    time::{Duration, Instant},
};

pub struct TranslationLimiter {
    max_burst: u32,
    tokens_per_sec: f64,
    tokens: f64,
    last_update: Instant,
}

impl TranslationLimiter {
    pub fn new() -> Self {
        Self {
            max_burst: 80,
            // 20 per minute = 1 permit every 3 seconds (0.333... per second)
            tokens_per_sec: 20.0 / 60.0,
            tokens: 80.0, // Start full for the burst
            last_update: Instant::now(),
        }
    }
}

pub struct SyncRateLimiter(Mutex<TranslationLimiter>);

impl SyncRateLimiter {
    pub fn new() -> Self {
        Self(Mutex::new(TranslationLimiter::new()))
    }

    pub fn run<F, R>(&self, f: F) -> R
    where
        F: FnOnce() -> R,
    {
        let mut guard = self.0.lock().unwrap();
        loop {
            let now = Instant::now();
            let elapsed = now.duration_since(guard.last_update).as_secs_f64();

            guard.tokens =
                (guard.tokens + elapsed * guard.tokens_per_sec).min(guard.max_burst as f64);
            guard.last_update = now;

            if guard.tokens >= 1.0 {
                guard.tokens -= 1.0;
                break;
            }

            let wait_time = Duration::from_secs_f64((1.0 - guard.tokens) / guard.tokens_per_sec);
            drop(guard);
            std::thread::sleep(wait_time);
            guard = self.0.lock().unwrap();
        }

        // Execute the passed function after rate limiting
        f()
    }
}
