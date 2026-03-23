use crate::input::{ButtonEvent, ButtonId};

/// Classifies a debounced button state stream into `Tap` and `LongPress` events.
pub struct LongPressDetector {
    held: Option<ButtonId>,
    held_ticks: u32,
    threshold_ticks: u32,
    /// True after a LongPress has been emitted for the current hold; suppresses re-fire.
    fired: bool,
}

impl LongPressDetector {
    pub fn new(threshold_ticks: u32) -> Self {
        Self { held: None, held_ticks: 0, threshold_ticks, fired: false }
    }

    /// Feed one stable (debounced) state. Returns an event when one is ready.
    pub fn update(&mut self, stable: Option<ButtonId>) -> Option<ButtonEvent> {
        match (self.held, stable) {
            (None, Some(b)) => {
                // Start of a new press.
                self.held = Some(b);
                self.held_ticks = 1;
                self.fired = false;
                None
            }
            (Some(b), Some(c)) if b == c => {
                // Continued hold.
                self.held_ticks += 1;
                if !self.fired && self.held_ticks >= self.threshold_ticks {
                    self.fired = true;
                    Some(ButtonEvent::LongPress(b))
                } else {
                    None
                }
            }
            (Some(_), Some(c)) => {
                // Different button while held — treat as a new press.
                self.held = Some(c);
                self.held_ticks = 1;
                self.fired = false;
                None
            }
            (Some(b), None) => {
                // Release.
                let event = if !self.fired { Some(ButtonEvent::Tap(b)) } else { None };
                self.held = None;
                self.held_ticks = 0;
                self.fired = false;
                event
            }
            (None, None) => None,
        }
    }
}
