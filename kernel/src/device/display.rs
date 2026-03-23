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
    use esp_hal::dma::{DmaChannelFor, DmaDescriptor, DmaRxBuf, DmaTxBuf};
    use esp_hal::gpio::{Input, InputConfig, Level, Output, OutputConfig};
    use esp_hal::spi::master::{AnySpi, Config, Instance, Spi, SpiDmaBus};
    use esp_hal::spi::Mode;
    use esp_hal::time::Rate;
    use esp_hal::Blocking;
    use static_cell::StaticCell;

    static TX_DESC: StaticCell<[DmaDescriptor; 2]> = StaticCell::new();
    static RX_DESC: StaticCell<[DmaDescriptor; 2]> = StaticCell::new();
    static TX_BUF: StaticCell<[u8; 4096]> = StaticCell::new();
    static RX_BUF: StaticCell<[u8; 4096]> = StaticCell::new();
    const DMA_CHUNK_BYTES: usize = 4096;

    /// SSD1677 display transport backed by DMA-accelerated SPI2 and four GPIO lines.
    pub struct X4DisplayTransport {
        spi: SpiDmaBus<'static, Blocking>,
        cs: Output<'static>,
        dc: Output<'static>,
        rst: Output<'static>,
        busy: Input<'static>,
    }

    impl X4DisplayTransport {
        /// Construct the display transport from raw peripherals.
        ///
        /// `spi2`     — SPI2 peripheral.
        /// `dma_ch0`  — DMA channel for SPI transfers.
        /// `sclk/mosi/cs` — SPI signal pins.
        /// `dc_pin`   — data/command select (high = data).
        /// `rst_pin`  — active-low hardware reset.
        /// `busy_pin` — BUSY signal from the controller (high = busy).
        pub fn new(
            spi2: impl Instance + 'static,
            dma_ch0: impl DmaChannelFor<AnySpi<'static>> + 'static,
            sclk: impl esp_hal::gpio::OutputPin + 'static,
            mosi: impl esp_hal::gpio::OutputPin + 'static,
            cs: impl esp_hal::gpio::OutputPin + 'static,
            dc_pin: impl esp_hal::gpio::OutputPin + 'static,
            rst_pin: impl esp_hal::gpio::OutputPin + 'static,
            busy_pin: impl esp_hal::gpio::InputPin + 'static,
        ) -> Self {
            let tx_buf = DmaTxBuf::new(
                TX_DESC.init([DmaDescriptor::EMPTY; 2]),
                TX_BUF.init([0u8; 4096]),
            )
            .unwrap();
            let rx_buf = DmaRxBuf::new(
                RX_DESC.init([DmaDescriptor::EMPTY; 2]),
                RX_BUF.init([0u8; 4096]),
            )
            .unwrap();

            let spi = Spi::new(
                spi2,
                Config::default()
                    .with_frequency(Rate::from_mhz(10))
                    .with_mode(Mode::_0),
            )
            .unwrap()
            .with_sck(sclk)
            .with_mosi(mosi)
            .with_dma(dma_ch0)
            .with_buffers(rx_buf, tx_buf);

            let cs = Output::new(cs, Level::High, OutputConfig::default());
            let dc = Output::new(dc_pin, Level::High, OutputConfig::default());
            let rst = Output::new(rst_pin, Level::High, OutputConfig::default());
            let busy = Input::new(busy_pin, InputConfig::default());

            Self {
                spi,
                cs,
                dc,
                rst,
                busy,
            }
        }
    }

    impl DisplayTransport for X4DisplayTransport {
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
            self.dc.set_low();
            self.cs.set_low();
            let result = self.spi.write(&[command]).map_err(|_| ());
            self.cs.set_high();
            result
        }

        fn write_data(&mut self, data: &[u8]) -> Result<(), ()> {
            self.dc.set_high();
            self.cs.set_low();
            for chunk in data.chunks(DMA_CHUNK_BYTES) {
                if self.spi.write(chunk).is_err() {
                    self.cs.set_high();
                    return Err(());
                }
            }
            self.cs.set_high();
            Ok(())
        }
    }
}

#[cfg(all(target_arch = "riscv32", target_os = "none"))]
pub use imp::X4DisplayTransport;
