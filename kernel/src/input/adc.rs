/// ADC abstraction for reading the two resistor-ladder button channels.
pub trait AdcSource {
    type Error;
    /// Read the GPIO1 ADC channel (4-button ladder) as a raw 12-bit ADC count.
    fn read_gpio1(&mut self) -> Result<u16, Self::Error>;
    /// Read the GPIO2 ADC channel (2-button ladder) as a raw 12-bit ADC count.
    fn read_gpio2(&mut self) -> Result<u16, Self::Error>;
}

/// Return the trimmed mean of four ADC samples.
///
/// This drops the minimum and maximum sample before averaging the middle two,
/// which rejects the first stale conversion after a ladder button is released.
pub fn trimmed_mean4(samples: [u16; 4]) -> u16 {
    let sum = samples.iter().map(|&v| v as u32).sum::<u32>();
    let min = *samples.iter().min().unwrap_or(&0) as u32;
    let max = *samples.iter().max().unwrap_or(&0) as u32;
    ((sum - min - max) / 2) as u16
}
