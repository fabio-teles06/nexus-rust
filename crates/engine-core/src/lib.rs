use serde::{Deserialize, Serialize};
use std::{thread, time::{Duration, Instant}};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct ClientId(pub u64);

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Default, Serialize, Deserialize)]
pub struct Tick(pub u64);

pub struct FixedTicker {
    interval: Duration,
    next_tick: Instant,
    current_tick: Tick,
}

impl FixedTicker {
    pub fn new(ticks_per_second: u32) -> Self {
        assert!(ticks_per_second > 0);
        Self {
            interval: Duration::from_secs_f64(1.0 / ticks_per_second as f64),
            next_tick: Instant::now(),
            current_tick: Tick(0),
        }
    }

    pub fn current_tick(&self) -> Tick { self.current_tick }

    pub fn wait_for_next_tick(&mut self) {
        self.next_tick += self.interval;
        let now = Instant::now();
        if let Some(remaining) = self.next_tick.checked_duration_since(now) {
            thread::sleep(remaining);
        } else {
            self.next_tick = now;
        }
        self.current_tick.0 = self.current_tick.0.wrapping_add(1);
    }
}

pub fn sequence_is_newer(sequence: u32, previous: u32) -> bool {
    const HALF_RANGE: u32 = 1 << 31;
    sequence != previous && sequence.wrapping_sub(previous) < HALF_RANGE
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sequence_comparison_handles_overflow() {
        assert!(sequence_is_newer(0, u32::MAX));
        assert!(!sequence_is_newer(u32::MAX, 0));
    }
}
