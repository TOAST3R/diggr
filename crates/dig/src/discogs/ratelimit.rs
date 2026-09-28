//! Discogs' rate limit: 60 requests a minute with a token, 25 without, counted over a moving
//! 60-second window. We keep our own sliding window so we never hit the limit, slow down when
//! Discogs says little is left (another tool may share it), and back off on a 429.

use std::collections::VecDeque;
use std::time::Duration;

pub const WINDOW: Duration = Duration::from_secs(60);
/// A little past the window, so clock rounding can never squeeze in one request too many.
const MARGIN: Duration = Duration::from_millis(50);
pub const WITH_TOKEN: usize = 60;
pub const WITHOUT_TOKEN: usize = 25;
/// When Discogs reports this many requests left or fewer, the next one waits.
const LOW_REMAINING: u32 = 2;
const LOW_WAIT: Duration = Duration::from_secs(10);
/// Waits after successive 429s.
const BACKOFF_SECS: [u64; 4] = [10, 20, 40, 60];

#[derive(Debug, Clone)]
pub struct RateLimiter {
    limit: usize,
    /// When the last `limit` requests were sent.
    sent: VecDeque<Duration>,
    /// No request before this time (low remaining, or a 429).
    hold_until: Duration,
    backoffs: usize,
}

impl RateLimiter {
    pub fn new(limit: usize) -> Self {
        Self {
            limit: limit.max(1),
            sent: VecDeque::new(),
            hold_until: Duration::ZERO,
            backoffs: 0,
        }
    }

    pub fn for_token(has_token: bool) -> Self {
        Self::new(if has_token { WITH_TOKEN } else { WITHOUT_TOKEN })
    }

    pub fn set_limit(&mut self, limit: usize) {
        self.limit = limit.max(1);
    }

    pub fn limit(&self) -> usize {
        self.limit
    }

    /// How long to wait at `now` before the next request may be sent.
    pub fn delay(&self, now: Duration) -> Duration {
        let window = if self.sent.len() >= self.limit {
            let oldest = self.sent[self.sent.len() - self.limit];
            (oldest + WINDOW + MARGIN).saturating_sub(now)
        } else {
            Duration::ZERO
        };
        window.max(self.hold_until.saturating_sub(now))
    }

    /// A request was sent at `now`, and Discogs answered with `remaining` left.
    pub fn record(&mut self, now: Duration, remaining: Option<u32>) {
        self.sent.push_back(now);
        while self.sent.len() > self.limit {
            self.sent.pop_front();
        }
        if remaining.is_some_and(|r| r <= LOW_REMAINING) {
            self.hold_until = self.hold_until.max(now + LOW_WAIT);
        }
    }

    /// Discogs said 429: wait 10, 20, 40, then 60 s before the retry. Returns the wait.
    pub fn too_many(&mut self, now: Duration) -> Duration {
        let wait = Duration::from_secs(BACKOFF_SECS[self.backoffs.min(BACKOFF_SECS.len() - 1)]);
        self.backoffs += 1;
        self.hold_until = self.hold_until.max(now + wait);
        wait
    }

    /// A request went through: the next 429 starts the backoff again.
    pub fn succeeded(&mut self) {
        self.backoffs = 0;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::clock::{Clock, FakeClock};

    /// Sends `n` requests as fast as the limiter allows, each taking `took`; returns send times.
    fn run(l: &mut RateLimiter, clock: &FakeClock, n: usize, took: Duration) -> Vec<Duration> {
        let mut times = Vec::new();
        for _ in 0..n {
            clock.sleep(l.delay(clock.now()));
            let t = clock.now();
            times.push(t);
            clock.sleep(took);
            l.record(t, Some(100));
            l.succeeded();
        }
        times
    }

    fn max_in_any_window(times: &[Duration]) -> usize {
        (0..times.len())
            .map(|i| {
                times[i..]
                    .iter()
                    .take_while(|&&t| t < times[i] + WINDOW)
                    .count()
            })
            .max()
            .unwrap_or(0)
    }

    #[test]
    fn eight_hundred_requests_never_exceed_sixty_a_minute() {
        let clock = FakeClock::default();
        let mut l = RateLimiter::for_token(true);
        let times = run(&mut l, &clock, 800, Duration::from_millis(120));
        assert_eq!(max_in_any_window(&times), 60);
        // And it isn't needlessly slow: 800 requests fit in about 14 minutes.
        assert!(
            clock.now() < Duration::from_secs(14 * 60),
            "{:?}",
            clock.now()
        );
    }

    #[test]
    fn without_a_token_it_is_twenty_five() {
        let clock = FakeClock::default();
        let mut l = RateLimiter::for_token(false);
        let times = run(&mut l, &clock, 200, Duration::ZERO);
        assert_eq!(max_in_any_window(&times), 25);
    }

    #[test]
    fn low_remaining_slows_down_and_429_backs_off() {
        let clock = FakeClock::default();
        let mut l = RateLimiter::for_token(true);
        l.record(clock.now(), Some(2));
        assert_eq!(l.delay(clock.now()), LOW_WAIT);
        let secs: Vec<u64> = (0..5)
            .map(|_| {
                let w = l.too_many(clock.now());
                clock.sleep(l.delay(clock.now()));
                w.as_secs()
            })
            .collect();
        assert_eq!(secs, [10, 20, 40, 60, 60]);
        l.succeeded();
        assert_eq!(l.too_many(clock.now()).as_secs(), 10, "starts again");
    }
}
