// adapted from aisix/crates/aisix-ratelimit/src/window.rs (Apache-2.0)

//! Fixed-window counter.

#[derive(Debug, PartialEq, Eq)]
pub enum WindowCheck {
    Ok,
    Full { retry_after_secs: u64 },
}

#[derive(Debug)]
pub struct FixedWindowCounter {
    window_secs: u64,
    window_start: u64,
    count: u64,
}

impl FixedWindowCounter {
    pub fn new(window_secs: u64) -> Self {
        assert!(window_secs > 0, "window_secs must be positive");
        Self {
            window_secs,
            window_start: 0,
            count: 0,
        }
    }

    pub fn window_secs(&self) -> u64 {
        self.window_secs
    }

    fn roll_if_stale(&mut self, now_secs: u64) {
        let bucket_start = (now_secs / self.window_secs) * self.window_secs;
        if bucket_start != self.window_start {
            self.window_start = bucket_start;
            self.count = 0;
        }
    }

    pub fn check_and_increment(&mut self, now_secs: u64, delta: u64, limit: u64) -> WindowCheck {
        self.roll_if_stale(now_secs);
        let would_be = self.count.saturating_add(delta);
        if would_be > limit {
            let remainder = self
                .window_secs
                .saturating_sub(now_secs.saturating_sub(self.window_start));
            return WindowCheck::Full {
                retry_after_secs: remainder.max(1),
            };
        }
        self.count = would_be;
        WindowCheck::Ok
    }

    pub fn add(&mut self, now_secs: u64, delta: u64) {
        self.roll_if_stale(now_secs);
        self.count = self.count.saturating_add(delta);
    }

    pub fn decrement(&mut self, now_secs: u64, delta: u64) {
        let bucket_start = (now_secs / self.window_secs) * self.window_secs;
        if bucket_start == self.window_start {
            self.count = self.count.saturating_sub(delta);
        }
    }

    pub fn current(&mut self, now_secs: u64) -> u64 {
        self.roll_if_stale(now_secs);
        self.count
    }

    pub fn is_exceeded(&mut self, now_secs: u64, limit: u64) -> Option<u64> {
        self.roll_if_stale(now_secs);
        if self.count > limit {
            let remainder = self
                .window_secs
                .saturating_sub(now_secs.saturating_sub(self.window_start));
            Some(remainder.max(1))
        } else {
            None
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn first_increments_fit_then_subsequent_block() {
        let mut w = FixedWindowCounter::new(60);
        assert_eq!(w.check_and_increment(100, 1, 3), WindowCheck::Ok);
        assert_eq!(w.check_and_increment(100, 1, 3), WindowCheck::Ok);
        assert_eq!(w.check_and_increment(100, 1, 3), WindowCheck::Ok);
        match w.check_and_increment(100, 1, 3) {
            WindowCheck::Full { retry_after_secs } => {
                assert!(retry_after_secs > 0);
                assert!(retry_after_secs <= 60);
            }
            _ => panic!("expected Full"),
        }
    }

    #[test]
    fn counter_rolls_over_at_window_boundary() {
        let mut w = FixedWindowCounter::new(60);
        for _ in 0..3 {
            w.check_and_increment(100, 1, 3);
        }
        assert_eq!(w.check_and_increment(161, 1, 3), WindowCheck::Ok);
        assert_eq!(w.current(161), 1);
    }

    #[test]
    fn retry_after_reflects_time_remaining_in_window() {
        let mut w = FixedWindowCounter::new(60);
        for _ in 0..3 {
            w.check_and_increment(100, 1, 3);
        }
        match w.check_and_increment(110, 1, 3) {
            WindowCheck::Full { retry_after_secs } => {
                assert_eq!(retry_after_secs, 10);
            }
            _ => panic!("expected Full"),
        }
    }

    #[test]
    fn add_records_post_deduct_usage_and_is_checkable() {
        let mut w = FixedWindowCounter::new(60);
        w.add(100, 1_000);
        w.add(101, 500);
        assert_eq!(w.current(101), 1_500);

        assert!(w.is_exceeded(101, 2_000).is_none());
        assert!(w.is_exceeded(101, 1_000).is_some());
    }

    #[test]
    fn check_with_zero_delta_is_a_read_only_peek_that_succeeds() {
        let mut w = FixedWindowCounter::new(60);
        assert_eq!(w.check_and_increment(100, 0, 5), WindowCheck::Ok);
        assert_eq!(w.current(100), 0);
    }

    #[test]
    fn decrement_reduces_current_window_count_only() {
        let mut w = FixedWindowCounter::new(60);
        w.check_and_increment(100, 1, 100);
        w.check_and_increment(100, 1, 100);
        assert_eq!(w.current(100), 2);

        w.decrement(100, 1);
        assert_eq!(w.current(100), 1);

        w.decrement(100, 100);
        assert_eq!(w.current(100), 0);
    }

    #[test]
    fn decrement_is_noop_after_window_rollover() {
        let mut w = FixedWindowCounter::new(60);
        for _ in 0..5 {
            w.check_and_increment(100, 1, 100);
        }
        w.decrement(200, 1);
        assert_eq!(w.current(200), 0);
    }

    #[test]
    fn retry_after_is_at_least_one_second() {
        let mut w = FixedWindowCounter::new(60);
        w.add(100, 1_000);
        let hint = w.is_exceeded(119, 100).unwrap();
        assert!(hint >= 1);
    }
}
