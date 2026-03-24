mod common;

use common::MockSpi;
use embedded_sdmmc::{Block, BlockCount, BlockDevice, BlockIdx};
use kernel::storage::block::SdBlockDevice;
use kernel::storage::sd::SdCard;

fn make_read_spi(data: &[u8; 512]) -> MockSpi {
    let mut reply = Vec::with_capacity(520);
    reply.push(0xFF);
    reply.push(0x00); // R1 = ready
    reply.push(0xFE); // data token
    reply.extend_from_slice(data);
    reply.push(0x00);
    reply.push(0x00); // CRC
    MockSpi::new(&[&reply])
}

fn make_write_spi() -> MockSpi {
    MockSpi::new(&[
        &[0xFF, 0x00], // CMD24 R1
        &[0xE5, 0xFF], // data response + not busy
    ])
}

#[test]
fn block_device_read_returns_expected_data_storage_block_device() {
    let mut expected = [0u8; 512];
    for (i, b) in expected.iter_mut().enumerate() {
        *b = (i & 0xFF) as u8;
    }
    let spi = make_read_spi(&expected);
    let bd = SdBlockDevice::new(SdCard::new(spi), 2048);
    let mut blocks = [Block::new()];
    bd.read(&mut blocks, BlockIdx(0), "test").unwrap();
    assert_eq!(blocks[0].contents, expected);
}

#[test]
fn block_device_num_blocks_returns_constructor_value_storage_block_device() {
    let spi = MockSpi::new(&[]);
    let bd = SdBlockDevice::new(SdCard::new(spi), 4096);
    assert_eq!(bd.num_blocks().unwrap(), BlockCount(4096));
}

#[test]
fn block_device_write_sends_correct_payload_storage_block_device() {
    let spi = make_write_spi();
    let payload = [0xBEu8; 512];
    let mut block = Block::new();
    block.contents.copy_from_slice(&payload);
    let bd = SdBlockDevice::new(SdCard::new(spi), 2048);
    bd.write(&[block], BlockIdx(7)).unwrap();
    let sent = bd.into_sd().into_spi().sent;
    // Find data token 0xFE and verify 512 bytes follow it
    let token_pos = sent
        .iter()
        .position(|&b| b == 0xFE)
        .expect("data token 0xFE not in sent");
    assert_eq!(&sent[token_pos + 1..token_pos + 513], &payload[..]);
}
