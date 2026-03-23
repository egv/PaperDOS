mod common;

use common::{make_test_fat16_image, InMemoryBlockDevice};
use kernel::storage::fs::{FsState, SeekFrom};

#[test]
fn fs_read_returns_correct_bytes_storage_fs_rw() {
    let bd = InMemoryBlockDevice::new(make_test_fat16_image());
    let mut fs = FsState::new(bd);
    let handle = fs.fs_open("README.TXT", false).unwrap();
    let mut buf = [0u8; 6];
    let n = fs.fs_read(handle, &mut buf).unwrap();
    assert_eq!(n, 6);
    assert_eq!(&buf, b"Hello!");
    fs.fs_close(handle).unwrap();
}

#[test]
fn fs_seek_tell_round_trip_storage_fs_rw() {
    let bd = InMemoryBlockDevice::new(make_test_fat16_image());
    let mut fs = FsState::new(bd);
    let handle = fs.fs_open("README.TXT", false).unwrap();
    fs.fs_seek(handle, SeekFrom::Start(3)).unwrap();
    let pos = fs.fs_tell(handle).unwrap();
    assert_eq!(pos, 3);
    fs.fs_close(handle).unwrap();
}

#[test]
fn fs_eof_after_reading_all_bytes_storage_fs_rw() {
    let bd = InMemoryBlockDevice::new(make_test_fat16_image());
    let mut fs = FsState::new(bd);
    let handle = fs.fs_open("README.TXT", false).unwrap();
    assert!(!fs.fs_eof(handle).unwrap());
    let mut buf = [0u8; 6];
    fs.fs_read(handle, &mut buf).unwrap();
    assert!(fs.fs_eof(handle).unwrap());
    fs.fs_close(handle).unwrap();
}

#[test]
fn fs_write_read_round_trip_storage_fs_rw() {
    let bd = InMemoryBlockDevice::new(make_test_fat16_image());
    let mut fs = FsState::new(bd);
    let wh = fs.fs_open("README.TXT", true).unwrap();
    fs.fs_write(wh, b"WORLD!").unwrap();
    fs.fs_close(wh).unwrap();
    let rh = fs.fs_open("README.TXT", false).unwrap();
    let mut buf = [0u8; 6];
    let n = fs.fs_read(rh, &mut buf).unwrap();
    assert_eq!(n, 6);
    assert_eq!(&buf, b"WORLD!");
    fs.fs_close(rh).unwrap();
}
