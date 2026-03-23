/// ADC abstraction for reading the two resistor-ladder button channels.
pub trait AdcSource {
    type Error;
    /// Read the GPIO1 ADC channel (4-button ladder) in millivolts.
    fn read_gpio1(&mut self) -> Result<u16, Self::Error>;
    /// Read the GPIO2 ADC channel (2-button ladder) in millivolts.
    fn read_gpio2(&mut self) -> Result<u16, Self::Error>;
}
