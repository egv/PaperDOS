mod common;

use common::MockSpi;
use kernel::storage::block::SdBlockDevice;
use kernel::storage::fs::FsState;
use kernel::storage::sd::SdCard;

#[test]
fn fs_state_constructs_without_panic_storage_fs_state() {
    let spi = MockSpi::new(&[]);
    let bd = SdBlockDevice::new(SdCard::new(spi), 4096);
    // Construction should not panic or read from the device.
    let _fs = FsState::new(bd);
}

#[test]
fn noop_time_source_returns_zero_timestamp_storage_fs_state() {
    use embedded_sdmmc::TimeSource;
    use kernel::storage::fs::NoopTimeSource;
    let ts = NoopTimeSource;
    let t = ts.get_timestamp();
    // NoopTimeSource returns the epoch / zeroed timestamp
    assert_eq!(t.year_since_1970, 0);
    assert_eq!(t.hours, 0);
}
