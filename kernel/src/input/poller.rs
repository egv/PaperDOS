use crate::input::adc::AdcSource;
use crate::input::decoder::{decode_gpio1, decode_gpio2};
use crate::input::{ButtonEvent, ButtonId};

const REPEAT_TICKS: u32 = 15;

#[derive(Clone, Copy)]
struct EventQueue {
    buf: [Option<ButtonEvent>; 4],
}

impl EventQueue {
    const fn new() -> Self {
        Self { buf: [None; 4] }
    }

    fn push(&mut self, event: ButtonEvent) {
        for slot in &mut self.buf {
            if slot.is_none() {
                *slot = Some(event);
                return;
            }
        }
    }

    fn pop(&mut self) -> Option<ButtonEvent> {
        for slot in &mut self.buf {
            if let Some(event) = slot.take() {
                return Some(event);
            }
        }
        None
    }
}

/// Pulp-style button poller.
///
/// One ladder button can be active at a time. GPIO1 has priority over GPIO2,
/// matching the `pulp-os` input driver.
pub struct InputPoller<A: AdcSource> {
    adc: A,
    last_gpio1: u16,
    last_gpio2: u16,
    last_raw1: Option<ButtonId>,
    last_raw2: Option<ButtonId>,
    stable: Option<ButtonId>,
    candidate: Option<ButtonId>,
    candidate_ticks: u32,
    press_ticks: u32,
    long_press_fired: bool,
    repeat_ticks: u32,
    queue: EventQueue,
    debounce_ticks: u32,
    longpress_ticks: u32,
}

impl<A: AdcSource> InputPoller<A> {
    pub fn new(adc: A, debounce_ticks: u32, longpress_ticks: u32) -> Self {
        Self {
            adc,
            last_gpio1: 0,
            last_gpio2: 0,
            last_raw1: None,
            last_raw2: None,
            stable: None,
            candidate: None,
            candidate_ticks: 0,
            press_ticks: 0,
            long_press_fired: false,
            repeat_ticks: 0,
            queue: EventQueue::new(),
            debounce_ticks,
            longpress_ticks,
        }
    }

    pub fn current_button(&self) -> Option<ButtonId> {
        self.stable
    }

    pub fn last_samples(&self) -> (u16, u16) {
        (self.last_gpio1, self.last_gpio2)
    }

    pub fn last_decoded(&self) -> (Option<ButtonId>, Option<ButtonId>) {
        (self.last_raw1, self.last_raw2)
    }

    pub fn poll(&mut self) -> Result<Option<ButtonEvent>, A::Error> {
        if let Some(event) = self.queue.pop() {
            return Ok(Some(event));
        }

        let raw = self.read_raw()?;

        if raw != self.candidate {
            if self.stable.is_some() && raw != self.stable {
                self.press_ticks = 0;
                self.long_press_fired = false;
                self.repeat_ticks = 0;
            }
            self.candidate = raw;
            self.candidate_ticks = 0;
        }
        self.candidate_ticks = self.candidate_ticks.saturating_add(1);

        let debounced = if self.candidate_ticks >= self.debounce_ticks {
            self.candidate
        } else {
            self.stable
        };

        if debounced != self.stable {
            if let Some(old) = self.stable {
                self.queue.push(ButtonEvent::Release(old));
            }
            if let Some(new) = debounced {
                self.queue.push(ButtonEvent::Press(new));
                self.press_ticks = 0;
                self.long_press_fired = false;
                self.repeat_ticks = 0;
            }
            self.stable = debounced;
            return Ok(self.queue.pop());
        }

        if let Some(button) = self.stable {
            self.press_ticks = self.press_ticks.saturating_add(1);

            if !self.long_press_fired && self.press_ticks >= self.longpress_ticks {
                self.long_press_fired = true;
                self.repeat_ticks = 0;
                return Ok(Some(ButtonEvent::LongPress(button)));
            }

            if self.long_press_fired {
                self.repeat_ticks = self.repeat_ticks.saturating_add(1);
                if self.repeat_ticks >= REPEAT_TICKS {
                    self.repeat_ticks = 0;
                    return Ok(Some(ButtonEvent::Repeat(button)));
                }
            }
        }

        Ok(None)
    }

    fn read_raw(&mut self) -> Result<Option<ButtonId>, A::Error> {
        let gpio1 = self.adc.read_gpio1()?;
        let gpio2 = self.adc.read_gpio2()?;
        let raw1 = decode_gpio1(gpio1);
        let raw2 = decode_gpio2(gpio2);
        self.last_gpio1 = gpio1;
        self.last_gpio2 = gpio2;
        self.last_raw1 = raw1;
        self.last_raw2 = raw2;
        Ok(raw1.or(raw2))
    }
}
