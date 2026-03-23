use embedded_hal::spi::{Operation, SpiDevice};

use crate::storage::StorageError;

/// Number of ACMD41 polling attempts before giving up.
const ACMD41_MAX_RETRIES: u32 = 10;

/// SD card variant, determined during initialisation.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CardKind {
    /// Standard Capacity card (≤ 2 GB, byte-addressed).
    Sdsc,
    /// High/Extended Capacity card (> 2 GB, sector-addressed).
    Sdhc,
}

/// Thin SD card SPI driver.
///
/// Owns an `SpiDevice` and exposes `init`, `read_block`, and `write_block`.
pub struct SdCard<SPI> {
    spi: SPI,
}

impl<SPI: SpiDevice> SdCard<SPI> {
    pub fn new(spi: SPI) -> Self {
        Self { spi }
    }

    /// Run the SD SPI initialisation sequence.
    ///
    /// Returns the [`CardKind`] on success.
    pub fn init(&mut self) -> Result<CardKind, StorageError> {
        // Step 1: 80+ clock cycles with CS (SpiDevice asserts CS; acceptable in practice)
        self.spi
            .transaction(&mut [Operation::Write(&[0xFF; 10])])
            .map_err(|_| StorageError::IoError)?;

        // Step 2: CMD0 — GO_IDLE_STATE
        let r1 = self.cmd_r1(0, 0x00000000, 0x95)?;
        if r1 != 0x01 {
            return Err(StorageError::NotReady);
        }

        // Step 3: CMD8 — SEND_IF_COND
        let r7 = self.cmd_r7(8, 0x000001AA, 0x87)?;
        if r7[0] & 0x04 != 0 {
            // Illegal command bit set — old SD v1 card not supported
            return Err(StorageError::NotReady);
        }

        // Step 4: ACMD41 — poll until card is ready
        let mut ready = false;
        for _ in 0..ACMD41_MAX_RETRIES {
            let r1 = self.cmd_r1(55, 0x00000000, 0x65)?; // CMD55
            if r1 & 0xFE != 0x00 {
                return Err(StorageError::IoError);
            }
            let r1 = self.cmd_r1(41, 0x40000000, 0x77)?; // ACMD41 with HCS
            if r1 == 0x00 {
                ready = true;
                break;
            }
        }
        if !ready {
            return Err(StorageError::NotReady);
        }

        // Step 5: CMD58 — READ_OCR
        let r3 = self.cmd_r7(58, 0x00000000, 0xFD)?;
        let ccs = r3[1] & 0x40 != 0;
        Ok(if ccs { CardKind::Sdhc } else { CardKind::Sdsc })
    }

    /// Consume the driver and return the underlying SPI device.
    pub fn into_spi(self) -> SPI {
        self.spi
    }

    /// Read a 512-byte block at `lba` into `buf`.
    pub fn read_block(&mut self, lba: u32, buf: &mut [u8; 512]) -> Result<(), StorageError> {
        let cmd = build_cmd(17, lba, 0xFF);
        // Read enough bytes to cover: up to 8 leading 0xFF, R1, 0xFF, data token, 512 data, 2 CRC
        let mut resp = [0xFFu8; 520];
        self.spi
            .transaction(&mut [Operation::Write(&cmd), Operation::Read(&mut resp)])
            .map_err(|_| StorageError::IoError)?;

        let r1_pos = resp.iter().position(|&b| b != 0xFF).ok_or(StorageError::IoError)?;
        if resp[r1_pos] != 0x00 {
            return Err(StorageError::IoError);
        }
        let token_pos = resp[r1_pos + 1..]
            .iter()
            .position(|&b| b == 0xFE)
            .ok_or(StorageError::IoError)?
            + r1_pos
            + 1;
        let data_start = token_pos + 1;
        if data_start + 512 > resp.len() {
            return Err(StorageError::IoError);
        }
        buf.copy_from_slice(&resp[data_start..data_start + 512]);
        Ok(())
    }

    /// Write a 512-byte block at `lba` from `data`.
    pub fn write_block(&mut self, lba: u32, data: &[u8; 512]) -> Result<(), StorageError> {
        let r1 = self.cmd_r1(24, lba, 0xFF)?;
        if r1 != 0x00 {
            return Err(StorageError::IoError);
        }
        // data token + 512 bytes + 2 dummy CRC bytes
        let mut send_buf = [0xFFu8; 1 + 512 + 2];
        send_buf[0] = 0xFE;
        send_buf[1..513].copy_from_slice(data);
        // CRC bytes stay 0xFF (don't-care in SPI mode)
        let mut resp = [0xFFu8; 3];
        self.spi
            .transaction(&mut [Operation::Write(&send_buf), Operation::Read(&mut resp)])
            .map_err(|_| StorageError::IoError)?;
        let token = resp.iter().find(|&&b| b != 0xFF).copied().ok_or(StorageError::IoError)?;
        if (token & 0x0F) != 0x05 {
            return Err(StorageError::IoError);
        }
        Ok(())
    }

    /// Send a command and return R1 (first non-0xFF byte within `resp_len` read bytes).
    fn cmd_r1(&mut self, cmd: u8, arg: u32, crc: u8) -> Result<u8, StorageError> {
        let cmd_bytes = build_cmd(cmd, arg, crc);
        let mut resp = [0xFFu8; 3];
        self.spi
            .transaction(&mut [
                Operation::Write(&cmd_bytes),
                Operation::Read(&mut resp),
            ])
            .map_err(|_| StorageError::IoError)?;
        resp.iter()
            .find(|&&b| b != 0xFF)
            .copied()
            .ok_or(StorageError::IoError)
    }

    /// Send a command and return R7/R3 response as [R1, byte1, byte2, byte3, byte4].
    fn cmd_r7(&mut self, cmd: u8, arg: u32, crc: u8) -> Result<[u8; 5], StorageError> {
        let cmd_bytes = build_cmd(cmd, arg, crc);
        let mut resp = [0xFFu8; 7];
        self.spi
            .transaction(&mut [
                Operation::Write(&cmd_bytes),
                Operation::Read(&mut resp),
            ])
            .map_err(|_| StorageError::IoError)?;
        let pos = resp
            .iter()
            .position(|&b| b != 0xFF)
            .ok_or(StorageError::IoError)?;
        if pos + 4 >= resp.len() {
            return Err(StorageError::IoError);
        }
        Ok([resp[pos], resp[pos + 1], resp[pos + 2], resp[pos + 3], resp[pos + 4]])
    }
}

fn build_cmd(cmd: u8, arg: u32, crc: u8) -> [u8; 6] {
    [
        0x40 | cmd,
        (arg >> 24) as u8,
        (arg >> 16) as u8,
        (arg >> 8) as u8,
        arg as u8,
        crc,
    ]
}
