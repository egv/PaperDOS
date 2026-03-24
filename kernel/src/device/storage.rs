/// SD card block-device wiring for the ESP32-C3 target.
///
/// Only compiled when targeting `riscv32-none` (the device binary).
/// Host tests use `InMemoryBlockDevice` directly and are unaffected.
#[cfg(all(target_arch = "riscv32", target_os = "none"))]
mod imp {
    use embedded_hal_bus::spi::CriticalSectionDevice;
    use embedded_sdmmc::sdcard::DummyCsPin;
    use embedded_sdmmc::SdCard as RuntimeSdCard;
    use esp_hal::spi::master::SpiDmaBus;
    use esp_hal::Blocking;

    use crate::device::raw_gpio::RawOutputPin;
    use crate::storage::fs::FsState;
    use crate::storage::StorageError;

    /// SPI device type for the SD card on the shared SPI2 bus.
    ///
    /// Uses `DummyCsPin` because `embedded_sdmmc::SdCard` drives the real SD CS
    /// line itself to satisfy the SPI-mode card spec.
    pub type SdSpiDevice<'a, D> =
        CriticalSectionDevice<'a, SpiDmaBus<'static, Blocking>, DummyCsPin, D>;

    pub type RuntimeSdFs<D> = FsState<RuntimeSdCard<SdSpiDevice<'static, D>, RawOutputPin, D>>;

    impl<D> RuntimeSdFs<D>
    where
        D: embedded_hal::delay::DelayNs + 'static,
    {
        /// Construct an [`FsState`] from a shared SPI2 device.
        ///
        /// Uses `embedded_sdmmc`'s own SD SPI driver, which matches the
        /// working pulp-os wiring model and manages the real CS pin directly.
        pub fn from_spi2(
            spi_dev: SdSpiDevice<'static, D>,
            cs: RawOutputPin,
            delay: D,
        ) -> Result<Self, StorageError> {
            let sd = RuntimeSdCard::new(spi_dev, cs, delay);
            sd.num_bytes().map_err(|_| StorageError::IoError)?;
            Ok(Self::new(sd))
        }
    }
}

#[cfg(all(target_arch = "riscv32", target_os = "none"))]
pub use imp::{RuntimeSdFs, SdSpiDevice};
