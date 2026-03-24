mod common;

use common::MockSpi;
use kernel::storage::sd::SdCard;

/// Build a CSD v2 (SDHC) reply for CMD9.
///
/// For C_SIZE = 0x3FF (1023):
///   block_count = (1023 + 1) × 1024 = 1_048_576 sectors
///
/// CSD byte layout (128-bit register, MSB first):
///   byte 0 = 0x40  (CSD_STRUCTURE = 01b)
///   bytes 1-6 = don't-care for capacity
///   byte 7 bits [5:0] = C_SIZE[21:16] = 0x00
///   byte 8            = C_SIZE[15:8]  = 0x03
///   byte 9            = C_SIZE[7:0]   = 0xFF
///   bytes 10-15 = don't-care
fn make_csd_v2_reply() -> Vec<u8> {
    let mut csd = [0u8; 16];
    csd[0] = 0x40; // CSD_STRUCTURE = 01b
                   // C_SIZE = 0x3FF = 1023
    csd[7] = 0x00; // C_SIZE[21:16]
    csd[8] = 0x03; // C_SIZE[15:8]
    csd[9] = 0xFF; // C_SIZE[7:0]
                   // Response: 2 preamble 0xFF + R1=0x00 + 1 padding + token 0xFE + 16 CSD + 2 CRC
    let mut reply = vec![0xFF, 0xFF, 0x00, 0xFF, 0xFE];
    reply.extend_from_slice(&csd);
    reply.extend_from_slice(&[0x00, 0x00]); // dummy CRC
    reply
}

/// Build a CSD v1 (SDSC) reply for CMD9.
///
/// Parameters: READ_BL_LEN=9 (512 B), C_SIZE=100, C_SIZE_MULT=1
///   BLOCKNR = (100+1) × 2^(1+2) = 101 × 8 = 808
///   block_count = 808 × (512/512) = 808 sectors
fn make_csd_v1_reply() -> Vec<u8> {
    let mut csd = [0u8; 16];
    // byte 0: CSD_STRUCTURE = 00b → bits [7:6] = 0x00
    csd[5] = 0x09; // READ_BL_LEN = 9 (bits [3:0])
                   // C_SIZE = 100 = 0x64 (12-bit at bits [73:62])
                   // byte 6 bits [1:0] = C_SIZE[11:10] = (0x64 >> 10) & 0x03 = 0
                   // byte 7            = C_SIZE[9:2]   = (0x64 >> 2) & 0xFF  = 0x19
                   // byte 8 bits [7:6] = C_SIZE[1:0]   = (0x64 & 0x03) << 6  = 0x00
    csd[7] = 0x19;
    // C_SIZE_MULT = 1 (3-bit at bits [49:47])
    // byte 9 bits [1:0] = C_SIZE_MULT[2:1] = (1 >> 1) & 0x03 = 0
    // byte 10 bit  [7]  = C_SIZE_MULT[0]   = (1 & 0x01) << 7  = 0x80
    csd[10] = 0x80;
    let mut reply = vec![0xFF, 0xFF, 0x00, 0xFF, 0xFE];
    reply.extend_from_slice(&csd);
    reply.extend_from_slice(&[0x00, 0x00]);
    reply
}

/// Returns a MockSpi pre-loaded for a successful SDHC init + CMD9 exchange.
fn make_spi_with_capacity(csd_reply: Vec<u8>) -> MockSpi {
    MockSpi::new(&[
        &[],
        &[0xFF, 0xFF, 0x01],
        &[0xFF, 0xFF, 0x01, 0x00, 0x00, 0x01, 0xAA],
        &[0xFF, 0xFF, 0x01],
        &[0xFF, 0xFF, 0x00],
        &[0xFF, 0xFF, 0x00, 0xC0, 0x00, 0x00, 0x00], // CMD58 CCS=1
        &csd_reply,
    ])
}

#[test]
fn read_capacity_csd_v2_returns_correct_sector_count_storage_sd_capacity() {
    let spi = make_spi_with_capacity(make_csd_v2_reply());
    let mut card = SdCard::new(spi);
    card.init().unwrap();
    let count = card.read_capacity().unwrap();
    assert_eq!(count, 1_048_576, "CSD v2 C_SIZE=1023 → 1024×1024 sectors");
}

#[test]
fn read_capacity_csd_v1_returns_correct_sector_count_storage_sd_capacity() {
    let spi = make_spi_with_capacity(make_csd_v1_reply());
    let mut card = SdCard::new(spi);
    card.init().unwrap();
    let count = card.read_capacity().unwrap();
    assert_eq!(count, 808, "CSD v1 C_SIZE=100 C_SIZE_MULT=1 → 808 sectors");
}
