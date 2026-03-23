mod common;

use common::MockSpi;
use kernel::storage::sd::SdCard;

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
