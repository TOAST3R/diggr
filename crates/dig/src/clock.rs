//! Time for the workers, injectable so tests can run minutes of rate limiting and retries in
//! milliseconds.

use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{Duration, Instant};

pub trait Clock: Send + Sync {
    /// Time since an arbitrary, fixed start.
    fn now(&self) -> Duration;
    fn sleep(&self, d: Duration);
}

pub struct RealClock {
    start: Instant,
}

impl Default for RealClock {
    fn default() -> Self {
        Self {
            start: Instant::now(),
        }
    }
}

impl Clock for RealClock {
    fn now(&self) -> Duration {
        self.start.elapsed()
    }

    fn sleep(&self, d: Duration) {
        std::thread::sleep(d);
    }
}

/// Time that moves only when someone sleeps (or [`FakeClock::advance`]s): sleeping returns at
/// once, having moved the time forward.
#[derive(Default)]
pub struct FakeClock {
    nanos: AtomicU64,
}

impl FakeClock {
    pub fn advance(&self, d: Duration) {
        self.nanos.fetch_add(d.as_nanos() as u64, Ordering::SeqCst);
    }
}

impl Clock for FakeClock {
    fn now(&self) -> Duration {
        Duration::from_nanos(self.nanos.load(Ordering::SeqCst))
    }

    fn sleep(&self, d: Duration) {
        self.advance(d);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fake_time_moves_only_when_asked() {
        let c = FakeClock::default();
        assert_eq!(c.now(), Duration::ZERO);
        c.sleep(Duration::from_secs(60));
        c.advance(Duration::from_millis(5));
        assert_eq!(c.now(), Duration::from_millis(60_005));
    }
}
