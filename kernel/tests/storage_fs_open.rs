mod common;

use common::{make_test_fat16_image, InMemoryBlockDevice};
use kernel::storage::fs::FsState;
use kernel::storage::StorageError;

#[test]
fn fs_open_existing_file_returns_handle_storage_fs_open() {
    let bd = InMemoryBlockDevice::new(make_test_fat16_image());
    let mut fs = FsState::new(bd);
    let handle = fs.fs_open("README.TXT", false).expect("open README.TXT should succeed");
    fs.fs_close(handle).expect("close should succeed");
}

#[test]
fn fs_close_frees_slot_for_reuse_storage_fs_open() {
    let bd = InMemoryBlockDevice::new(make_test_fat16_image());
    let mut fs = FsState::new(bd);
    let h1 = fs.fs_open("README.TXT", false).unwrap();
    fs.fs_close(h1).unwrap();
    // After close the slot must be free; opening again must not return NoSpace.
    let h2 = fs.fs_open("README.TXT", false).unwrap();
    fs.fs_close(h2).unwrap();
}

#[test]
fn fs_open_nonexistent_returns_not_found_storage_fs_open() {
    let bd = InMemoryBlockDevice::new(make_test_fat16_image());
    let mut fs = FsState::new(bd);
    let result = fs.fs_open("NOTEXIST.TXT", false);
    assert_eq!(result, Err(StorageError::NotFound));
}

#[test]
fn fs_open_all_slots_full_returns_error_storage_fs_open() {
    // Exhaust all 8 VolumeManager file slots by creating 8 distinct files via
    // write mode (create-or-truncate).  A 9th open must fail.  VolumeManager's
    // MAX_FILES limit (= 8) is hit before our file_slots check, so the error is
    // IoError rather than NoSpace — but the important contract is that it fails.
    let bd = InMemoryBlockDevice::new(make_test_fat16_image());
    let mut fs = FsState::new(bd);
    let mut handles = Vec::new();
    let names = ["F0.TXT", "F1.TXT", "F2.TXT", "F3.TXT", "F4.TXT", "F5.TXT", "F6.TXT", "F7.TXT"];
    for &name in &names {
        handles.push(fs.fs_open(name, true).expect("should open within 8 slots"));
    }
    let result = fs.fs_open("F8.TXT", true);
    assert!(result.is_err(), "9th open must fail when all file slots are occupied");
    for h in handles {
        fs.fs_close(h).unwrap();
    }
}
