use embedded_hal::spi::{Operation, SpiDevice};

use crate::storage::StorageError;

/// Number of ACMD41 polling attempts before giving up (~1 s at 50 ms per retry).
const ACMD41_MAX_RETRIES: u32 = 20;

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
/// Owns an `SpiDevice` and exposes `init`, `read_block`, `write_block`, and
/// `read_capacity`.  After a successful [`init`][SdCard::init] the driver
/// stores the detected [`CardKind`] and translates block addresses correctly:
/// SDSC cards use byte addresses (LBA × 512) while SDHC/SDXC use LBA directly.
pub struct SdCard<SPI> {
    spi: SPI,
    /// Detected card variant; `None` until `init()` succeeds.
    kind: Option<CardKind>,
}

impl<SPI: SpiDevice> SdCard<SPI> {
    pub fn new(spi: SPI) -> Self {
        Self { spi, kind: None }
    }

    /// Run the SD SPI initialisation sequence.
    ///
    /// Returns the [`CardKind`] on success and stores it internally so that
    /// subsequent block operations use the correct address scheme.
    ///
    /// # Note on power-up clocks
    ///
    /// The SD SPI spec requires CS deasserted (high) during the ≥74 power-up
    /// clock cycles.  `SpiDevice::transaction` holds CS low during the initial
    /// 0xFF burst, which is a protocol deviation.  Most cards tolerate this in
    /// practice; use hardware with a pull-up on CS if strict compliance is needed.
    pub fn init(&mut self) -> Result<CardKind, StorageError> {
        // Step 1: 80+ clock cycles — see note above about CS polarity.
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

        // Step 4: ACMD41 — poll until card is ready.
        // Each iteration waits 50 ms; ACMD41_MAX_RETRIES × 50 ms ≈ 1 s total.
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
            // 50 ms inter-poll delay — keeps total init time within the 1 s
            // allowed by the SD spec for ACMD41 to complete.
            self.spi
                .transaction(&mut [Operation::DelayNs(50_000_000)])
                .map_err(|_| StorageError::IoError)?;
        }
        if !ready {
            return Err(StorageError::NotReady);
        }

        // Step 5: CMD58 — READ_OCR
        let r3 = self.cmd_r7(58, 0x00000000, 0xFD)?;
        let ccs = r3[1] & 0x40 != 0;
        let kind = if ccs { CardKind::Sdhc } else { CardKind::Sdsc };
        self.kind = Some(kind);
        Ok(kind)
    }

    /// Read the card capacity in 512-byte sectors using CMD9 (SEND_CSD).
    ///
    /// Supports both CSD v1 (SDSC) and CSD v2 (SDHC/SDXC) register formats.
    /// Must be called after a successful [`init`][SdCard::init].
    pub fn read_capacity(&mut self) -> Result<u32, StorageError> {
        let cmd_bytes = build_cmd(9, 0x00000000, 0xFF);
        // Buffer: up to 4 leading 0xFF + R1 + up to 4 more 0xFF + token + 16 CSD + 2 CRC
        let mut resp = [0xFFu8; 30];
        self.spi
            .transaction(&mut [Operation::Write(&cmd_bytes), Operation::Read(&mut resp)])
            .map_err(|_| StorageError::IoError)?;

        // Locate R1 (first non-0xFF byte).
        let r1_pos =
            resp.iter().position(|&b| b != 0xFF).ok_or(StorageError::IoError)?;
        if resp[r1_pos] != 0x00 {
            return Err(StorageError::IoError);
        }

        // Locate data token 0xFE after R1.
        let token_pos = resp[r1_pos + 1..]
            .iter()
            .position(|&b| b == 0xFE)
            .ok_or(StorageError::IoError)?
            + r1_pos
            + 1;

        let data_start = token_pos + 1;
        if data_start + 16 > resp.len() {
            return Err(StorageError::IoError);
        }
        let csd = &resp[data_start..data_start + 16];

        let csd_structure = (csd[0] >> 6) & 0x03;
        if csd_structure == 1 {
            // CSD v2 (SDHC/SDXC): C_SIZE at bits [69:48].
            // Byte 7 bits [5:0] = C_SIZE[21:16], byte 8 = C_SIZE[15:8], byte 9 = C_SIZE[7:0].
            let c_size = ((csd[7] as u32 & 0x3F) << 16)
                | ((csd[8] as u32) << 8)
                | (csd[9] as u32);
            Ok((c_size + 1) * 1024)
        } else {
            // CSD v1 (SDSC): classic formula.
            let read_bl_len = csd[5] & 0x0F;
            let c_size = (((csd[6] & 0x03) as u32) << 10)
                | ((csd[7] as u32) << 2)
                | ((csd[8] >> 6) as u32);
            let c_size_mult =
                (((csd[9] & 0x03) as u32) << 1) | ((csd[10] >> 7) as u32);
            let block_len = 1u32 << read_bl_len;
            let mult = 1u32 << (c_size_mult + 2);
            let blocknr = (c_size + 1) * mult;
            // Normalise to 512-byte sectors.
            Ok(blocknr * (block_len / 512))
        }
    }

    /// Consume the driver and return the underlying SPI device.
    pub fn into_spi(self) -> SPI {
        self.spi
    }

    /// Read a 512-byte block at `lba` into `buf`.
    ///
    /// Automatically translates `lba` to a byte address for SDSC cards.
    pub fn read_block(&mut self, lba: u32, buf: &mut [u8; 512]) -> Result<(), StorageError> {
        let addr = self.block_addr(lba);
        let cmd = build_cmd(17, addr, 0xFF);
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
    ///
    /// Automatically translates `lba` to a byte address for SDSC cards.
    pub fn write_block(&mut self, lba: u32, data: &[u8; 512]) -> Result<(), StorageError> {
        let addr = self.block_addr(lba);
        let r1 = self.cmd_r1(24, addr, 0xFF)?;
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

    /// Translate an LBA to the address expected by CMD17/CMD24/CMD9.
    ///
    /// SDHC/SDXC use sector (LBA) addresses; SDSC use byte addresses (LBA × 512).
    /// Before `init()` succeeds, defaults to SDHC addressing (no multiplication).
    fn block_addr(&self, lba: u32) -> u32 {
        match self.kind {
            Some(CardKind::Sdsc) => lba * 512,
            _ => lba,
        }
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
