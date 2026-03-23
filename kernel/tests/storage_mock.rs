mod common;

use common::MockSpi;
use embedded_hal::spi::{Operation, SpiDevice};

#[test]
fn mock_spi_transfer_records_sent_storage_mock() {
    let mut spi = MockSpi::new(&[&[0xAA, 0xBB]]);
    let mut read_buf = [0u8; 2];
    let write_buf = [0x01u8, 0x02];
    spi.transaction(&mut [Operation::Transfer(&mut read_buf, &write_buf)])
        .unwrap();
    assert_eq!(read_buf, [0xAA, 0xBB]);
    assert_eq!(&spi.sent, &[0x01, 0x02]);
}

#[test]
fn mock_spi_write_records_sent_storage_mock() {
    let mut spi = MockSpi::new(&[]);
    spi.transaction(&mut [Operation::Write(&[0xCC, 0xDD])])
        .unwrap();
    assert_eq!(&spi.sent, &[0xCC, 0xDD]);
}

#[test]
fn mock_spi_read_fills_from_reply_storage_mock() {
    let mut spi = MockSpi::new(&[&[0x01, 0x02, 0x03]]);
    let mut buf = [0u8; 3];
    spi.transaction(&mut [Operation::Read(&mut buf)]).unwrap();
    assert_eq!(buf, [0x01, 0x02, 0x03]);
}

#[test]
fn mock_spi_multiple_transactions_storage_mock() {
    let mut spi = MockSpi::new(&[&[0x11], &[0x22]]);
    let mut r1 = [0u8; 1];
    let mut r2 = [0u8; 1];
    spi.transaction(&mut [Operation::Read(&mut r1)]).unwrap();
    spi.transaction(&mut [Operation::Read(&mut r2)]).unwrap();
    assert_eq!(r1, [0x11]);
    assert_eq!(r2, [0x22]);
}
