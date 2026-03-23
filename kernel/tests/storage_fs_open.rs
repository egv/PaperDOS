mod common;

use common::{make_test_fat16_image, InMemoryBlockDevice};
use kernel::storage::fs::{FileHandle, FsState};
use kernel::storage::StorageError;

#[test]
fn fs_open_existing_file_returns_handle_0_storage_fs_open() {
    let bd = InMemoryBlockDevice::new(make_test_fat16_image());
    let mut fs = FsState::new(bd);
    let handle = fs.fs_open("README.TXT", false).expect("open README.TXT should succeed");
    assert_eq!(handle.0, 0, "first open should occupy slot 0");
    fs.fs_close(handle).expect("close should succeed");
}

#[test]
fn fs_close_frees_slot_for_reuse_storage_fs_open() {
    let bd = InMemoryBlockDevice::new(make_test_fat16_image());
    let mut fs = FsState::new(bd);
    let h1 = fs.fs_open("README.TXT", false).unwrap();
    fs.fs_close(h1).unwrap();
    // Slot 0 should be free again
    let h2 = fs.fs_open("README.TXT", false).unwrap();
    assert_eq!(h2.0, 0, "freed slot 0 should be reused");
    fs.fs_close(h2).unwrap();
}

#[test]
fn fs_open_nonexistent_returns_not_found_storage_fs_open() {
    let bd = InMemoryBlockDevice::new(make_test_fat16_image());
    let mut fs = FsState::new(bd);
    let result = fs.fs_open("NOTEXIST.TXT", false);
    assert_eq!(result, Err(StorageError::NotFound));
}
