mod common;

use common::{make_test_fat16_image, InMemoryBlockDevice};
use kernel::storage::fs::{EntryType, FsState};
use kernel::storage::StorageError;

#[test]
fn fs_stat_file_returns_correct_size_and_type_storage_fs_stat() {
    let bd = InMemoryBlockDevice::new(make_test_fat16_image());
    let mut fs = FsState::new(bd);
    let stat = fs.fs_stat("README.TXT").expect("stat README.TXT should succeed");
    assert_eq!(stat.entry_type, EntryType::File);
    assert_eq!(stat.size, 6);
}

#[test]
fn fs_stat_directory_returns_dir_type_storage_fs_stat() {
    let bd = InMemoryBlockDevice::new(make_test_fat16_image());
    let mut fs = FsState::new(bd);
    let stat = fs.fs_stat("TESTDIR").expect("stat TESTDIR should succeed");
    assert_eq!(stat.entry_type, EntryType::Directory);
}

#[test]
fn fs_mkdir_creates_new_directory_storage_fs_stat() {
    let bd = InMemoryBlockDevice::new(make_test_fat16_image());
    let mut fs = FsState::new(bd);
    fs.fs_mkdir("NEWDIR").expect("mkdir NEWDIR should succeed");
    let stat = fs.fs_stat("NEWDIR").expect("stat NEWDIR should succeed after mkdir");
    assert_eq!(stat.entry_type, EntryType::Directory);
}

#[test]
fn fs_remove_then_stat_returns_not_found_storage_fs_stat() {
    let bd = InMemoryBlockDevice::new(make_test_fat16_image());
    let mut fs = FsState::new(bd);
    fs.fs_remove("README.TXT").expect("remove should succeed");
    let result = fs.fs_stat("README.TXT");
    assert_eq!(result, Err(StorageError::NotFound));
}
