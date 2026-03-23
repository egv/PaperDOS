mod common;

use common::MockSpi;
use kernel::storage::sd::{CardKind, SdCard};

/// Build a MockSpi for a single CMD17 (read block) exchange.
///
/// Reply sequence:
/// 0: CMD17 response — R1=0x00 then data token 0xFE then 512 data bytes then 2 CRC bytes
fn make_read_spi(data: &[u8; 512]) -> MockSpi {
    let mut reply = Vec::with_capacity(3 + 512 + 2);
    reply.push(0xFF); // R1 leading dummy
    reply.push(0x00); // R1 = ready
    reply.push(0xFE); // data token
    reply.extend_from_slice(data);
    reply.push(0x00); // CRC high
    reply.push(0x00); // CRC low
    MockSpi::new(&[&reply])
}

/// Build a MockSpi for a single CMD24 (write block) exchange.
///
/// Reply: R1=0x00, then data response token 0xE5 (accepted), then busy (0x00 × N, 0xFF ready).
fn make_write_spi() -> MockSpi {
    MockSpi::new(&[
        &[0xFF, 0x00],           // CMD24 R1
        &[0xE5, 0xFF],           // data response token + not busy
    ])
}

#[test]
fn read_block_fills_buffer_storage_sd_rw() {
    let mut expected = [0u8; 512];
    for (i, b) in expected.iter_mut().enumerate() {
        *b = (i & 0xFF) as u8;
    }
    let mut spi = make_read_spi(&expected);
    let mut card = SdCard::new(spi);
    let mut buf = [0u8; 512];
    card.read_block(0, &mut buf).unwrap();
    assert_eq!(buf, expected);
}

#[test]
fn write_block_sends_data_token_and_payload_storage_sd_rw() {
    let mut spi = make_write_spi();
    let mut card = SdCard::new(spi);
    let data = [0xABu8; 512];
    card.write_block(0, &data).unwrap();
    // Verify that sent bytes include the data token 0xFE followed by the 512 data bytes
    let sent = card.into_spi().sent;
    // CMD24 = 6 bytes, then data token 0xFE, then 512 bytes, then 2 CRC bytes
    let token_pos = sent.iter().position(|&b| b == 0xFE).expect("data token not found");
    assert_eq!(&sent[token_pos + 1..token_pos + 513], &data[..]);
}

// ── SDSC address-translation tests ─────────────────────────────────────────
//
// After init with CCS=0 (SDSC), read_block/write_block must multiply the LBA
// by 512 before encoding it in the command argument.

/// Returns a MockSpi loaded with an SDSC init sequence (CCS=0) followed by
/// a CMD17 read reply for `data`.
fn make_sdsc_read_spi(data: &[u8; 512]) -> MockSpi {
    let mut read_reply = Vec::with_capacity(3 + 512 + 2);
    read_reply.push(0xFF); // leading dummy
    read_reply.push(0x00); // R1 = ready
    read_reply.push(0xFE); // data token
    read_reply.extend_from_slice(data);
    read_reply.extend_from_slice(&[0x00, 0x00]); // CRC
    MockSpi::new(&[
        &[],                                          // preamble
        &[0xFF, 0xFF, 0x01],                          // CMD0
        &[0xFF, 0xFF, 0x01, 0x00, 0x00, 0x01, 0xAA], // CMD8
        &[0xFF, 0xFF, 0x01],                          // CMD55
        &[0xFF, 0xFF, 0x00],                          // ACMD41 ready
        &[0xFF, 0xFF, 0x00, 0x80, 0x00, 0x00, 0x00], // CMD58 CCS=0 → SDSC
        &read_reply,
    ])
}

/// Returns a MockSpi loaded with an SDSC init sequence followed by a CMD24
/// write reply.
fn make_sdsc_write_spi() -> MockSpi {
    MockSpi::new(&[
        &[],
        &[0xFF, 0xFF, 0x01],
        &[0xFF, 0xFF, 0x01, 0x00, 0x00, 0x01, 0xAA],
        &[0xFF, 0xFF, 0x01],
        &[0xFF, 0xFF, 0x00],
        &[0xFF, 0xFF, 0x00, 0x80, 0x00, 0x00, 0x00],
        &[0xFF, 0x00],   // CMD24 R1
        &[0xE5, 0xFF],   // data response token
    ])
}

#[test]
fn sdsc_read_block_uses_byte_address_storage_sd_rw() {
    // Init bytes sent: preamble(10) + CMD0(6) + CMD8(6) + CMD55(6) + ACMD41(6) + CMD58(6) = 40
    // Then CMD17 starts at sent[40].  For LBA=1, byte address = 512 = 0x00000200.
    // build_cmd(17, 512, 0xFF) = [0x51, 0x00, 0x00, 0x02, 0x00, 0xFF]
    let expected_data = [0xCCu8; 512];
    let spi = make_sdsc_read_spi(&expected_data);
    let mut card = SdCard::new(spi);
    assert_eq!(card.init().unwrap(), CardKind::Sdsc);
    let mut buf = [0u8; 512];
    card.read_block(1, &mut buf).unwrap();
    assert_eq!(buf, expected_data);
    // Verify the CMD17 argument encodes byte address 512, not LBA 1.
    let sent = card.into_spi().sent;
    let cmd17_start = 40; // 10 + 5×6
    assert_eq!(sent[cmd17_start], 0x40 | 17, "CMD17 opcode");
    // Argument bytes: 0x00 0x00 0x02 0x00  (512 = 0x00000200, big-endian)
    assert_eq!(&sent[cmd17_start + 1..cmd17_start + 5], &[0x00, 0x00, 0x02, 0x00],
        "SDSC CMD17 arg must be byte address (LBA×512)");
}

#[test]
fn sdsc_write_block_uses_byte_address_storage_sd_rw() {
    // CMD24 for LBA=2 with SDSC → byte address = 1024 = 0x00000400.
    // build_cmd(24, 1024, 0xFF) = [0x58, 0x00, 0x00, 0x04, 0x00, 0xFF]
    let spi = make_sdsc_write_spi();
    let mut card = SdCard::new(spi);
    assert_eq!(card.init().unwrap(), CardKind::Sdsc);
    let data = [0xBBu8; 512];
    card.write_block(2, &data).unwrap();
    let sent = card.into_spi().sent;
    let cmd24_start = 40; // 10 + 5×6
    assert_eq!(sent[cmd24_start], 0x40 | 24, "CMD24 opcode");
    // Byte address = 2×512 = 1024 = 0x00000400
    assert_eq!(&sent[cmd24_start + 1..cmd24_start + 5], &[0x00, 0x00, 0x04, 0x00],
        "SDSC CMD24 arg must be byte address (LBA×512)");
}
