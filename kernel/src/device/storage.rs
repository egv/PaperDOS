/// SD card block-device wiring for the ESP32-C3 target.
///
/// Only compiled when targeting `riscv32-none` (the device binary).
/// Host tests use `InMemoryBlockDevice` directly and are unaffected.
#[cfg(all(target_arch = "riscv32", target_os = "none"))]
mod imp {
    use embedded_hal_bus::spi::CriticalSectionDevice;
    use esp_hal::spi::master::Spi;
    use esp_hal::Blocking;

    use crate::storage::block::SdBlockDevice;
    use crate::storage::fs::FsState;
    use crate::storage::sd::SdCard;
    use crate::storage::StorageError;

    /// SPI device type for the SD card on the shared SPI2 bus.
    ///
    /// `CS` — chip-select output pin; `D` — delay provider.
    pub type SdSpiDevice<'a, CS, D> = CriticalSectionDevice<'a, Spi<'static, Blocking>, CS, D>;

    impl<CS, D> FsState<SdBlockDevice<SdSpiDevice<'static, CS, D>>>
    where
        CS: embedded_hal::digital::OutputPin + 'static,
        D: embedded_hal::delay::DelayNs + 'static,
    {
        /// Construct an [`FsState`] from a shared SPI2 device.
        ///
        /// Runs the SD card SPI initialisation sequence; returns a ready
        /// filesystem state on success. `block_count` is the number of
        /// 512-byte sectors on the card.
        pub fn from_spi2(
            spi_dev: SdSpiDevice<'static, CS, D>,
            block_count: u32,
        ) -> Result<Self, StorageError> {
            let mut sd = SdCard::new(spi_dev);
            sd.init()?;
            let bd = SdBlockDevice::new(sd, block_count);
            Ok(Self::new(bd))
        }
    }
}

#[cfg(all(target_arch = "riscv32", target_os = "none"))]
pub use imp::SdSpiDevice;
