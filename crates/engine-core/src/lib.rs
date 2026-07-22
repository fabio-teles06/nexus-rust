use serde::{Deserialize, Serialize};
use std::{
    thread,
    time::{Duration, Instant},
};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct ClientId(pub u32);

#[derive(
    Debug,
    Clone,
    Copy,
    PartialEq,
    Eq,
    PartialOrd,
    Ord,
    Serialize,
    Deserialize,
)]
pub struct Tick(pub u64);

pub struct FixedTicker {
    interval: Duration,
    next_tick: Instant,
    current_tick: Tick,
}

impl FixedTicker {
    pub fn new(ticks_per_second: u32) -> Self {
        assert!(
            ticks_per_second > 0,
            "ticks_per_second must be greater than 0"
        );

        Self {
            interval: Duration::from_secs_f64(1.0 / ticks_per_second as f64),
            next_tick: Instant::now(),
            current_tick: Tick(0),
        }
    }

    pub fn current_tick(&self) -> Tick {
        self.current_tick
    }

    pub fn wait_for_next_tick(&mut self) {
        self.next_tick += self.interval;

        let now = Instant::now();

        if let Some(remaining) = self.next_tick.checked_duration_since(now) {
            thread::sleep(remaining);
        } else {
            self.next_tick = now;
        }

        self.current_tick.0 += 1;
    }
}
