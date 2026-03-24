// F5: pd_fs_opendir and pd_fs_readdir delegate to registered fn pointers.

use std::sync::atomic::{AtomicI32, Ordering};

use kernel::syscall::fs::{
    pd_fs_closedir, pd_fs_opendir, pd_fs_readdir, set_fs_closedir_fn, set_fs_opendir_fn,
    set_fs_readdir_fn,
};

static LAST_OPENDIR_RESULT: AtomicI32 = AtomicI32::new(-1);

unsafe fn mock_opendir(_path: *const u8, _len: usize) -> i32 {
    7 // fixed handle
}

unsafe fn mock_readdir(_handle: i32, _buf: *mut u8) -> i32 {
    1 // end of directory
}

fn mock_closedir(_handle: i32) -> i32 {
    0
}

#[test]
fn pd_fs_opendir_delegates_to_registered_fn_fs_global() {
    let path = b"apps\0";
    // SAFETY: single-threaded test binary; path is valid.
    unsafe { set_fs_opendir_fn(mock_opendir) };
    let handle = unsafe { pd_fs_opendir(path.as_ptr(), 4) };
    assert_eq!(handle, 7, "must return value from registered opendir fn");
}

#[test]
fn pd_fs_readdir_delegates_to_registered_fn_fs_global() {
    let mut dirent = [0u8; 264];
    unsafe { set_fs_readdir_fn(mock_readdir) };
    let result = unsafe { pd_fs_readdir(7, dirent.as_mut_ptr()) };
    assert_eq!(
        result, 1,
        "must return end-of-dir from registered readdir fn"
    );
}

#[test]
fn pd_fs_closedir_delegates_to_registered_fn_fs_global() {
    unsafe { set_fs_closedir_fn(mock_closedir) };
    let result = pd_fs_closedir(7);
    assert_eq!(result, 0, "must return 0 from registered closedir fn");
}
