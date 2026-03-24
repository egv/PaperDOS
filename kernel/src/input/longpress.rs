use crate::input::{ButtonEvent, ButtonId};

/// Classifies a debounced button state stream into press/release/long-press events.
pub struct LongPressDetector {
    held: Option<ButtonId>,
    held_ticks: u32,
    threshold_ticks: u32,
    /// True after a LongPress has been emitted for the current hold; suppresses re-fire.
    fired: bool,
}

impl LongPressDetector {
    pub fn new(threshold_ticks: u32) -> Self {
        Self {
            held: None,
            held_ticks: 0,
            threshold_ticks,
            fired: false,
        }
    }

    /// Feed one stable (debounced) state. Returns an event when one is ready.
    pub fn update(&mut self, stable: Option<ButtonId>) -> Option<ButtonEvent> {
        match (self.held, stable) {
            (None, Some(b)) => {
                self.held = Some(b);
                self.held_ticks = 1;
                self.fired = false;
                Some(ButtonEvent::Press(b))
            }
            (Some(b), Some(c)) if b == c => {
                self.held_ticks += 1;
                if !self.fired && self.held_ticks >= self.threshold_ticks {
                    self.fired = true;
                    Some(ButtonEvent::LongPress(b))
                } else {
                    None
                }
            }
            (Some(_), Some(c)) => {
                self.held = Some(c);
                self.held_ticks = 1;
                self.fired = false;
                Some(ButtonEvent::Press(c))
            }
            (Some(b), None) => {
                self.held = None;
                self.held_ticks = 0;
                self.fired = false;
                Some(ButtonEvent::Release(b))
            }
            (None, None) => None,
        }
    }
}
