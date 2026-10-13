//! Retrying a handler with exponential backoff.

use std::time::Duration;

#[derive(Debug, Clone, Copy)]
pub struct Retry {
    /// At least 1.
    pub attempts: u32,
    /// Delay before the first retry. Subsequent retries have delay double the one before.
    pub first_delay: Duration,
}

impl Retry {
    /// Try once, and don't retry.
    pub const NEVER: Self = Self {
        attempts: 1,
        first_delay: Duration::ZERO,
    };

    /// The wait after 1-indexed failed attempt `attempt`.
    pub fn delay(&self, attempt: u32) -> Duration {
        self.first_delay
            .saturating_mul(2u32.saturating_pow(attempt.saturating_sub(1)))
    }
}
