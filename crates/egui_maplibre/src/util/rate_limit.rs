use std::time::Duration;

use instant::Instant;

#[derive(Clone, Debug, PartialEq)]
pub struct RateLimitState {
    min_interval: Duration,
    last_allowed: Option<Instant>,
}

impl RateLimitState {
    pub fn new(max_fps: f32) -> Self {
        let mut state = Self::default();
        state.set_max_fps(max_fps);
        state
    }

    pub fn set_max_fps(&mut self, max_fps: f32) {
        let fps = if max_fps.is_finite() {
            max_fps.max(1.0)
        } else {
            1.0
        };
        self.min_interval = Duration::from_secs_f64(1.0 / fps as f64);
    }

    pub fn reset(&mut self) {
        self.last_allowed = None;
    }

    pub fn allow_now(&mut self, now: Instant) -> bool {
        match self.last_allowed {
            Some(last) if now.duration_since(last) < self.min_interval => false,
            _ => {
                self.last_allowed = Some(now);
                true
            }
        }
    }
}

impl Default for RateLimitState {
    fn default() -> Self {
        Self {
            min_interval: Duration::from_millis(50),
            last_allowed: None,
        }
    }
}

#[cfg(test)]
mod tests {
    use instant::Instant;

    use super::RateLimitState;

    #[test]
    fn rate_limit_blocks_immediate_followup() {
        let now = Instant::now();
        let mut limiter = RateLimitState::new(20.0);
        assert!(limiter.allow_now(now));
        assert!(!limiter.allow_now(now));
    }

    #[test]
    fn rate_limit_allows_after_interval() {
        let now = Instant::now();
        let mut limiter = RateLimitState::new(10.0);
        assert!(limiter.allow_now(now));
        assert!(limiter.allow_now(now + std::time::Duration::from_millis(150)));
    }
}
