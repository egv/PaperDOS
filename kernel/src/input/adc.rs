/// ADC abstraction for reading the two resistor-ladder button channels.
pub trait AdcSource {
    type Error;
    /// Read the GPIO1 ADC channel (4-button ladder) in millivolts.
    fn read_gpio1(&mut self) -> Result<u16, Self::Error>;
    /// Read the GPIO2 ADC channel (2-button ladder) in millivolts.
    fn read_gpio2(&mut self) -> Result<u16, Self::Error>;
}

/// Test double: replays scripted ADC readings for each channel.
///
/// When the scripted sequence is exhausted, repeats the last value indefinitely.
pub struct ScriptedAdc<'a> {
    gpio1: &'a [u16],
    gpio2: &'a [u16],
    idx1: usize,
    idx2: usize,
}

impl<'a> ScriptedAdc<'a> {
    pub fn new(gpio1: &'a [u16], gpio2: &'a [u16]) -> Self {
        Self { gpio1, gpio2, idx1: 0, idx2: 0 }
    }
}

impl<'a> AdcSource for ScriptedAdc<'a> {
    type Error = core::convert::Infallible;

    fn read_gpio1(&mut self) -> Result<u16, Self::Error> {
        let val = self.gpio1[self.idx1];
        if self.idx1 + 1 < self.gpio1.len() {
            self.idx1 += 1;
        }
        Ok(val)
    }

    fn read_gpio2(&mut self) -> Result<u16, Self::Error> {
        let val = self.gpio2[self.idx2];
        if self.idx2 + 1 < self.gpio2.len() {
            self.idx2 += 1;
        }
        Ok(val)
    }
}
