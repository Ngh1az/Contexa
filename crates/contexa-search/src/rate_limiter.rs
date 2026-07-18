//! `RateLimiter` — `docs/09_Search_Engine.md` §10.1 sliding-window limits.
//! Max concurrent searches (2) is enforced separately via a `Semaphore` in
//! `engine.rs` — this only tracks call rate over time.

use std::collections::VecDeque;
use std::sync::{Mutex, PoisonError};
use std::time::{Duration, Instant};

use contexa_core::{ContexaError, Result};

const DEFAULT_MAX_PER_MINUTE: usize = 10;
const DEFAULT_MAX_PER_HOUR: usize = 100;

pub struct RateLimiter {
    per_minute: Mutex<VecDeque<Instant>>,
    per_hour: Mutex<VecDeque<Instant>>,
    max_per_minute: usize,
    max_per_hour: usize,
}

impl RateLimiter {
    #[must_use]
    pub fn new(max_per_minute: usize, max_per_hour: usize) -> Self {
        Self {
            per_minute: Mutex::new(VecDeque::new()),
            per_hour: Mutex::new(VecDeque::new()),
            max_per_minute,
            max_per_hour,
        }
    }

    /// Records a call attempt now, or rejects it if either window is full.
    /// # Errors
    /// Returns `ContexaError::RateLimited` if either the per-minute or
    /// per-hour limit has been reached.
    pub fn check(&self) -> Result<()> {
        let now = Instant::now();
        check_window(
            &self.per_minute,
            now,
            Duration::from_secs(60),
            self.max_per_minute,
        )?;
        check_window(
            &self.per_hour,
            now,
            Duration::from_secs(3600),
            self.max_per_hour,
        )?;
        Ok(())
    }
}

impl Default for RateLimiter {
    fn default() -> Self {
        Self::new(DEFAULT_MAX_PER_MINUTE, DEFAULT_MAX_PER_HOUR)
    }
}

fn check_window(
    window: &Mutex<VecDeque<Instant>>,
    now: Instant,
    span: Duration,
    max: usize,
) -> Result<()> {
    let mut window = window.lock().unwrap_or_else(PoisonError::into_inner);
    while window.front().is_some_and(|t| now.duration_since(*t) > span) {
        window.pop_front();
    }
    if window.len() >= max {
        return Err(ContexaError::RateLimited);
    }
    window.push_back(now);
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn allows_calls_under_the_limit() {
        let limiter = RateLimiter::new(3, 100);
        assert!(limiter.check().is_ok());
        assert!(limiter.check().is_ok());
        assert!(limiter.check().is_ok());
    }

    #[test]
    fn rejects_calls_over_the_per_minute_limit() {
        let limiter = RateLimiter::new(2, 100);
        assert!(limiter.check().is_ok());
        assert!(limiter.check().is_ok());
        assert!(limiter.check().is_err());
    }

    #[test]
    fn per_hour_limit_is_independent_and_also_enforced() {
        let limiter = RateLimiter::new(100, 1);
        assert!(limiter.check().is_ok());
        assert!(limiter.check().is_err());
    }
}
