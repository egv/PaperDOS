use core::cell::RefCell;

use embedded_hal::spi::SpiDevice;
use embedded_sdmmc::{Block, BlockCount, BlockDevice, BlockIdx};

use crate::storage::sd::SdCard;
use crate::storage::StorageError;

/// Adapts [`SdCard`] to the [`embedded_sdmmc::BlockDevice`] trait.
///
/// Uses `RefCell` for interior mutability because `BlockDevice::read` and
/// `BlockDevice::write` take `&self`.
pub struct SdBlockDevice<SPI> {
    sd: RefCell<SdCard<SPI>>,
    block_count: u32,
}

impl<SPI: SpiDevice> SdBlockDevice<SPI> {
    pub fn new(sd: SdCard<SPI>, block_count: u32) -> Self {
        Self {
            sd: RefCell::new(sd),
            block_count,
        }
    }

    /// Consume the adapter and return the inner [`SdCard`].
    pub fn into_sd(self) -> SdCard<SPI> {
        self.sd.into_inner()
    }
}

impl<SPI: SpiDevice> BlockDevice for SdBlockDevice<SPI> {
    type Error = StorageError;

    fn read(
        &self,
        blocks: &mut [Block],
        start_block_idx: BlockIdx,
        _reason: &str,
    ) -> Result<(), Self::Error> {
        let mut sd = self.sd.borrow_mut();
        for (i, block) in blocks.iter_mut().enumerate() {
            sd.read_block(start_block_idx.0 + i as u32, &mut block.contents)?;
        }
        Ok(())
    }

    fn write(&self, blocks: &[Block], start_block_idx: BlockIdx) -> Result<(), Self::Error> {
        let mut sd = self.sd.borrow_mut();
        for (i, block) in blocks.iter().enumerate() {
            sd.write_block(start_block_idx.0 + i as u32, &block.contents)?;
        }
        Ok(())
    }

    fn num_blocks(&self) -> Result<BlockCount, Self::Error> {
        Ok(BlockCount(self.block_count))
    }
}
