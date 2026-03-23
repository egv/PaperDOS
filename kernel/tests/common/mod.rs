use core::cell::RefCell;

use embedded_hal::spi::{Operation, SpiDevice};
use embedded_sdmmc::{Block, BlockCount, BlockDevice, BlockIdx};
use kernel::display::transport::DisplayTransport;
use kernel::input::adc::AdcSource;

#[derive(Debug, Eq, PartialEq)]
pub enum RecordedOp {
    Reset,
    WaitWhileBusy,
    Command(u8),
    Data(Vec<u8>),
}

#[derive(Default)]
pub struct RecordingTransport {
    pub ops: Vec<RecordedOp>,
}

/// Test double: replays scripted ADC readings for each channel.
///
/// When the scripted sequence is exhausted, repeats the last value indefinitely.
/// Both slices must be non-empty.
pub struct ScriptedAdc<'a> {
    gpio1: &'a [u16],
    gpio2: &'a [u16],
    idx1: usize,
    idx2: usize,
}

impl<'a> ScriptedAdc<'a> {
    pub fn new(gpio1: &'a [u16], gpio2: &'a [u16]) -> Self {
        debug_assert!(!gpio1.is_empty(), "ScriptedAdc: gpio1 slice must not be empty");
        debug_assert!(!gpio2.is_empty(), "ScriptedAdc: gpio2 slice must not be empty");
        Self { gpio1, gpio2, idx1: 0, idx2: 0 }
    }
}

impl<'a> AdcSource for ScriptedAdc<'a> {
    type Error = core::convert::Infallible;

    fn read_gpio1(&mut self) -> Result<u16, Self::Error> {
        let val = self.gpio1[self.idx1];
        if self.idx1 + 1 < self.gpio1.len() {
            self.idx1 += 1;
        }
        Ok(val)
    }

    fn read_gpio2(&mut self) -> Result<u16, Self::Error> {
        let val = self.gpio2[self.idx2];
        if self.idx2 + 1 < self.gpio2.len() {
            self.idx2 += 1;
        }
        Ok(val)
    }
}

/// Test double: replays scripted SPI transaction replies and records sent bytes.
///
/// Each entry in `replies` is a byte sequence returned for one `transaction()` call.
/// When transactions exceed pre-loaded replies, subsequent reads return 0xFF.
pub struct MockSpi {
    replies: Vec<Vec<u8>>,
    reply_idx: usize,
    /// All bytes written via Write and Transfer operations, in order.
    pub sent: Vec<u8>,
}

impl MockSpi {
    /// `replies[i]` is the byte sequence returned during the i-th `transaction()` call.
    pub fn new(replies: &[&[u8]]) -> Self {
        Self {
            replies: replies.iter().map(|s| s.to_vec()).collect(),
            reply_idx: 0,
            sent: Vec::new(),
        }
    }
}

impl SpiDevice for MockSpi {
    fn transaction(&mut self, operations: &mut [Operation<'_, u8>]) -> Result<(), Self::Error> {
        let reply = if self.reply_idx < self.replies.len() {
            let r = self.replies[self.reply_idx].clone();
            self.reply_idx += 1;
            r
        } else {
            Vec::new()
        };
        let mut reply_pos = 0usize;

        for op in operations.iter_mut() {
            match op {
                Operation::Write(bytes) => {
                    self.sent.extend_from_slice(bytes);
                }
                Operation::Read(buf) => {
                    for b in buf.iter_mut() {
                        *b = if reply_pos < reply.len() {
                            let v = reply[reply_pos];
                            reply_pos += 1;
                            v
                        } else {
                            0xFF
                        };
                    }
                }
                Operation::Transfer(read, write) => {
                    self.sent.extend_from_slice(write);
                    for b in read.iter_mut() {
                        *b = if reply_pos < reply.len() {
                            let v = reply[reply_pos];
                            reply_pos += 1;
                            v
                        } else {
                            0xFF
                        };
                    }
                }
                Operation::TransferInPlace(buf) => {
                    let written: Vec<u8> = buf.to_vec();
                    self.sent.extend_from_slice(&written);
                    for b in buf.iter_mut() {
                        *b = if reply_pos < reply.len() {
                            let v = reply[reply_pos];
                            reply_pos += 1;
                            v
                        } else {
                            0xFF
                        };
                    }
                }
                Operation::DelayNs(_) => {}
            }
        }
        Ok(())
    }
}

impl embedded_hal::spi::ErrorType for MockSpi {
    type Error = core::convert::Infallible;
}

/// In-memory block device for filesystem integration tests.
///
/// Holds a pre-populated `Vec<Block>` in a `RefCell`. Reads beyond the
/// pre-populated range return zero blocks; out-of-range writes are silently
/// dropped. `num_blocks()` returns the length of the inner Vec.
pub struct InMemoryBlockDevice {
    blocks: RefCell<Vec<Block>>,
}

impl InMemoryBlockDevice {
    /// Create a device backed by `blocks`. `num_blocks()` returns `blocks.len()`.
    pub fn new(blocks: Vec<Block>) -> Self {
        Self { blocks: RefCell::new(blocks) }
    }
}

impl BlockDevice for InMemoryBlockDevice {
    type Error = ();

    fn read(
        &self,
        blocks: &mut [Block],
        start_block_idx: BlockIdx,
        _reason: &str,
    ) -> Result<(), ()> {
        let store = self.blocks.borrow();
        for (i, block) in blocks.iter_mut().enumerate() {
            let idx = start_block_idx.0 as usize + i;
            if idx < store.len() {
                block.contents.copy_from_slice(&store[idx].contents);
            } else {
                block.contents = [0u8; 512];
            }
        }
        Ok(())
    }

    fn write(&self, blocks: &[Block], start_block_idx: BlockIdx) -> Result<(), ()> {
        let mut store = self.blocks.borrow_mut();
        for (i, block) in blocks.iter().enumerate() {
            let idx = start_block_idx.0 as usize + i;
            if idx < store.len() {
                store[idx].contents.copy_from_slice(&block.contents);
            }
        }
        Ok(())
    }

    fn num_blocks(&self) -> Result<BlockCount, ()> {
        Ok(BlockCount(self.blocks.borrow().len() as u32))
    }
}

impl embedded_hal::spi::ErrorType for InMemoryBlockDevice {
    type Error = core::convert::Infallible;
}

/// Build a minimal FAT16 disk image for filesystem tests.
///
/// Layout (device LBAs):
/// - 0: MBR  — partition 1: type 0x06, LBA start=1, size=4200 sectors
/// - 1: Boot sector — BPB: 512 B/sec, 1 sec/cluster, 1 reserved, 1 FAT,
///       16 root entries, 4200 total sectors, FAT size=17 → cluster_count=4181
/// - 2-18: FAT (17 sectors); sector 0 has entries 0-3 set, rest are zero (free)
/// - 19: Root directory — README.TXT (cluster 2, size 6) + TESTDIR (cluster 3)
/// - 20: Cluster 2 = "Hello!" content
/// - 21: Cluster 3 = TESTDIR (empty, zeros)
/// - 22-4200: zeros
///
/// `num_blocks()` = 4201.
pub fn make_test_fat16_image() -> Vec<Block> {
    const TOTAL: usize = 4201; // 1 MBR + 4200 partition sectors
    let mut blocks = vec![Block::new(); TOTAL];

    // ── Block 0: MBR ──────────────────────────────────────────────────────────
    {
        let b = &mut blocks[0].contents;
        // Partition 1 entry at offset 446 (16 bytes)
        b[446] = 0x00; // status: not bootable
        b[447] = 0x00; b[448] = 0x01; b[449] = 0x00; // CHS start (ignored)
        b[450] = 0x06; // type: FAT16
        b[451] = 0x00; b[452] = 0x00; b[453] = 0x00; // CHS end (ignored)
        b[454] = 0x01; b[455] = 0x00; b[456] = 0x00; b[457] = 0x00; // LBA start = 1
        b[458] = 0x68; b[459] = 0x10; b[460] = 0x00; b[461] = 0x00; // num sectors = 4200
        b[510] = 0x55; b[511] = 0xAA; // signature
    }

    // ── Block 1: Boot sector (FAT16 BPB) ─────────────────────────────────────
    {
        let b = &mut blocks[1].contents;
        b[0] = 0xEB; b[1] = 0x3C; b[2] = 0x90; // jump boot
        b[3..11].copy_from_slice(b"PAPERDOS"); // OEM name
        b[11] = 0x00; b[12] = 0x02; // bytes per sector = 512
        b[13] = 0x01; // sectors per cluster
        b[14] = 0x01; b[15] = 0x00; // reserved sectors = 1
        b[16] = 0x01; // num FATs
        b[17] = 0x10; b[18] = 0x00; // root entries = 16
        b[19] = 0x68; b[20] = 0x10; // total sectors = 4200 (0x1068 LE)
        b[21] = 0xF8; // media = fixed
        b[22] = 0x11; b[23] = 0x00; // FAT size = 17
        b[24] = 0x3F; b[25] = 0x00; // sectors/track
        b[26] = 0xFF; b[27] = 0x00; // num heads
        b[28] = 0x01; b[29] = 0x00; b[30] = 0x00; b[31] = 0x00; // hidden sectors = 1
        // total_sectors32 at 32-35: zero (use 16-bit field above)
        b[36] = 0x80; // drive number
        b[38] = 0x29; // extended boot signature
        b[39] = 0x01; b[40] = 0x02; b[41] = 0x03; b[42] = 0x04; // volume serial
        b[43..54].copy_from_slice(b"PAPERDOS   "); // volume label (11 bytes)
        b[54..62].copy_from_slice(b"FAT16   "); // FS type
        b[510] = 0x55; b[511] = 0xAA; // signature
    }

    // ── Block 2: FAT sector 0 ─────────────────────────────────────────────────
    {
        let b = &mut blocks[2].contents;
        b[0] = 0xF8; b[1] = 0xFF; // entry 0: media marker 0xFFF8
        b[2] = 0xFF; b[3] = 0xFF; // entry 1: end-of-chain
        b[4] = 0xFF; b[5] = 0xFF; // entry 2: README.TXT (single cluster)
        b[6] = 0xFF; b[7] = 0xFF; // entry 3: TESTDIR (single cluster)
    }

    // ── Block 19: Root directory ──────────────────────────────────────────────
    {
        let b = &mut blocks[19].contents;
        // Entry 0: README.TXT — "README  TXT", ARCHIVE, cluster=2, size=6
        b[0..11].copy_from_slice(b"README  TXT");
        b[11] = 0x20; // attribute: ARCHIVE
        b[26] = 0x02; b[27] = 0x00; // first cluster = 2
        b[28] = 0x06; b[29] = 0x00; b[30] = 0x00; b[31] = 0x00; // size = 6

        // Entry 1: TESTDIR — "TESTDIR    ", DIRECTORY, cluster=3
        b[32..43].copy_from_slice(b"TESTDIR    ");
        b[43] = 0x10; // attribute: DIRECTORY
        b[58] = 0x03; b[59] = 0x00; // first cluster = 3
        // size = 0 (already zero)
    }

    // ── Block 20: Cluster 2 = README.TXT content ─────────────────────────────
    blocks[20].contents[0..6].copy_from_slice(b"Hello!");

    // Block 21: Cluster 3 = TESTDIR — all zeros (already initialized)

    blocks
}

impl DisplayTransport for RecordingTransport {
    type Error = ();

    fn reset(&mut self) -> Result<(), ()> {
        self.ops.push(RecordedOp::Reset);
        Ok(())
    }

    fn wait_while_busy(&mut self) -> Result<(), ()> {
        self.ops.push(RecordedOp::WaitWhileBusy);
        Ok(())
    }

    fn write_command(&mut self, cmd: u8) -> Result<(), ()> {
        self.ops.push(RecordedOp::Command(cmd));
        Ok(())
    }

    fn write_data(&mut self, data: &[u8]) -> Result<(), ()> {
        self.ops.push(RecordedOp::Data(data.to_vec()));
        Ok(())
    }
}
