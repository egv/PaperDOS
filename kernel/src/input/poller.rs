use crate::input::adc::AdcSource;
use crate::input::debounce::DebounceFilter;
use crate::input::decoder::{decode_gpio1, decode_gpio2};
use crate::input::longpress::LongPressDetector;
use crate::input::ButtonEvent;

/// Composes ADC sampling, threshold decoding, per-channel debouncing, and long-press
/// classification into a single `poll()` call.
///
/// GPIO1 (4-button) has priority: when both channels report a button simultaneously,
/// the GPIO1 result is used.
pub struct InputPoller<A: AdcSource> {
    adc: A,
    db1: DebounceFilter,
    db2: DebounceFilter,
    lp: LongPressDetector,
}

impl<A: AdcSource> InputPoller<A> {
    pub fn new(adc: A, debounce_ticks: u32, longpress_ticks: u32) -> Self {
        Self {
            adc,
            db1: DebounceFilter::new(debounce_ticks),
            db2: DebounceFilter::new(debounce_ticks),
            lp: LongPressDetector::new(longpress_ticks),
        }
    }

    /// Sample both ADC channels, decode, debounce, and classify.
    ///
    /// Returns `Ok(Some(event))` when an event is ready, `Ok(None)` otherwise.
    pub fn poll(&mut self) -> Result<Option<ButtonEvent>, A::Error> {
        let mv1 = self.adc.read_gpio1()?;
        let mv2 = self.adc.read_gpio2()?;

        let raw1 = decode_gpio1(mv1);
        let raw2 = decode_gpio2(mv2);

        let stable1 = self.db1.update(raw1);
        let stable2 = self.db2.update(raw2);

        // GPIO1 has priority when both channels are active.
        let combined = stable1.or(stable2);

        Ok(self.lp.update(combined))
    }
}
