use crate::input::ButtonId;

/// Debounce filter: stable state only changes after `threshold_ticks` consecutive
/// identical readings.
pub struct DebounceFilter {
    stable: Option<ButtonId>,
    candidate: Option<ButtonId>,
    ticks: u32,
    threshold_ticks: u32,
}

impl DebounceFilter {
    pub fn new(threshold_ticks: u32) -> Self {
        Self {
            stable: None,
            candidate: None,
            ticks: 0,
            threshold_ticks,
        }
    }

    /// Feed one raw reading. Returns the current stable state after this tick.
    pub fn update(&mut self, raw: Option<ButtonId>) -> Option<ButtonId> {
        if raw == self.candidate {
            self.ticks += 1;
        } else {
            self.candidate = raw;
            self.ticks = 1;
        }
        if self.ticks >= self.threshold_ticks {
            self.stable = self.candidate;
        }
        self.stable
    }
}
