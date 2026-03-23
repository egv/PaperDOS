/// Hardware abstraction for the display SPI/GPIO interface.
///
/// Implementors drive the physical reset, busy, DC, and SPI signals for the
/// connected e-paper controller.
pub trait DisplayTransport {
    type Error;

    /// Assert the hardware reset line and release it.
    fn reset(&mut self) -> Result<(), Self::Error>;

    /// Block until the controller's BUSY signal deasserts.
    fn wait_while_busy(&mut self) -> Result<(), Self::Error>;

    /// Assert DC low (command mode) and transfer one command byte over SPI.
    fn write_command(&mut self, command: u8) -> Result<(), Self::Error>;

    /// Assert DC high (data mode) and transfer `data` bytes over SPI.
    fn write_data(&mut self, data: &[u8]) -> Result<(), Self::Error>;

    /// Busy-wait for at least `ms` milliseconds.
    ///
    /// The default implementation is a no-op, suitable for host-side test
    /// transports.  Device transports must override this with a real delay.
    fn delay_ms(&mut self, _ms: u32) {}
}
