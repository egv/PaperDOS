mod common;

use common::{make_test_fat16_image, InMemoryBlockDevice};
use kernel::storage::fs::{EntryType, FsState};

#[test]
fn fs_readdir_root_returns_two_entries_then_none_storage_fs_dir() {
    let bd = InMemoryBlockDevice::new(make_test_fat16_image());
    let mut fs = FsState::new(bd);
    let dh = fs.fs_opendir("").unwrap();

    let e1 = fs.fs_readdir(dh).unwrap().expect("first entry should be present");
    assert_eq!(e1.entry_type, EntryType::File); // README.TXT

    let e2 = fs.fs_readdir(dh).unwrap().expect("second entry should be present");
    assert_eq!(e2.entry_type, EntryType::Directory); // TESTDIR

    let e3 = fs.fs_readdir(dh).unwrap();
    assert!(e3.is_none(), "third readdir should return None");

    fs.fs_closedir(dh).unwrap();
}

#[test]
fn fs_closedir_frees_slot_for_reuse_storage_fs_dir() {
    let bd = InMemoryBlockDevice::new(make_test_fat16_image());
    let mut fs = FsState::new(bd);
    let dh = fs.fs_opendir("").unwrap();
    assert_eq!(dh.0, 0);
    fs.fs_closedir(dh).unwrap();
    let dh2 = fs.fs_opendir("").unwrap();
    assert_eq!(dh2.0, 0, "slot 0 should be reused after close");
    fs.fs_closedir(dh2).unwrap();
}
