/// SPI2 clock pin (GPIO8).
pub const DISPLAY_SCLK_PIN: u8 = 8;
/// SPI2 MOSI pin (GPIO10).
pub const DISPLAY_MOSI_PIN: u8 = 10;
/// Display SPI chip-select pin (GPIO21).
pub const DISPLAY_CS_PIN: u8 = 21;
/// Display data/command select pin (GPIO4, high = data, low = command).
pub const DISPLAY_DC_PIN: u8 = 4;
/// Display hardware reset pin (GPIO5, active-low).
pub const DISPLAY_RST_PIN: u8 = 5;
/// Display BUSY signal pin (GPIO6, high while controller is busy).
pub const DISPLAY_BUSY_PIN: u8 = 6;

#[cfg(all(target_arch = "riscv32", target_os = "none"))]
mod imp {
    use crate::display::transport::DisplayTransport;
    use embedded_hal::spi::{Operation, SpiDevice};
    use esp_hal::gpio::{Input, InputConfig, Level, Output, OutputConfig};

    /// SSD1677 display transport backed by an SPI device and three GPIO lines.
    pub struct X4DisplayTransport<SPI> {
        spi: SPI,
        dc: Output<'static>,
        rst: Output<'static>,
        busy: Input<'static>,
    }

    impl<SPI> X4DisplayTransport<SPI> {
        /// Construct the display transport from a configured SPI device.
        pub fn new(
            spi: SPI,
            dc_pin: impl esp_hal::gpio::OutputPin + 'static,
            rst_pin: impl esp_hal::gpio::OutputPin + 'static,
            busy_pin: impl esp_hal::gpio::InputPin + 'static,
        ) -> Self {
            let dc = Output::new(dc_pin, Level::High, OutputConfig::default());
            let rst = Output::new(rst_pin, Level::High, OutputConfig::default());
            let busy = Input::new(busy_pin, InputConfig::default());

            Self { spi, dc, rst, busy }
        }
    }

    impl<SPI> DisplayTransport for X4DisplayTransport<SPI>
    where
        SPI: SpiDevice,
    {
        type Error = ();

        fn reset(&mut self) -> Result<(), ()> {
            // pulp-os reference timing: high 20 ms → low 2 ms → high 20 ms.
            // A shorter low pulse leaves the SSD1677 in an undefined state.
            let delay = esp_hal::delay::Delay::new();
            self.rst.set_high();
            delay.delay_millis(20);
            self.rst.set_low();
            delay.delay_millis(2);
            self.rst.set_high();
            delay.delay_millis(20);
            Ok(())
        }

        fn delay_ms(&mut self, ms: u32) {
            esp_hal::delay::Delay::new().delay_millis(ms);
        }

        fn wait_while_busy(&mut self) -> Result<(), ()> {
            // SSD1677 BUSY is high while the controller is executing a command.
            // Bound the spin to ~10 s worth of iterations to avoid an infinite loop
            // in debug builds if the display is absent or misbehaving.
            for _ in 0u32..800_000_000 {
                if self.busy.is_low() {
                    return Ok(());
                }
                core::hint::spin_loop();
            }
            Err(())
        }

        fn write_command(&mut self, command: u8) -> Result<(), ()> {
            let cmd = [command];
            let mut ops = [Operation::Write(&cmd)];
            self.dc.set_low();
            self.spi.transaction(&mut ops).map_err(|_| ())
        }

        fn write_data(&mut self, data: &[u8]) -> Result<(), ()> {
            let mut ops = [Operation::Write(data)];
            self.dc.set_high();
            self.spi.transaction(&mut ops).map_err(|_| ())
        }
    }
}

#[cfg(all(target_arch = "riscv32", target_os = "none"))]
pub use imp::X4DisplayTransport;
