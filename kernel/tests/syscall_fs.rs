use kernel::syscall::build_syscall_table;
use kernel::syscall::fs::{
    pd_fs_close, pd_fs_closedir, pd_fs_eof, pd_fs_mkdir, pd_fs_open, pd_fs_opendir, pd_fs_read,
    pd_fs_readdir, pd_fs_remove, pd_fs_seek, pd_fs_stat, pd_fs_tell, pd_fs_write,
};

#[test]
fn syscall_table_fs_fields_populated_syscall_fs() {
    let t = build_syscall_table(0, 0);
    assert_eq!(t.fs_open, pd_fs_open as usize as u32);
    assert_eq!(t.fs_close, pd_fs_close as usize as u32);
    assert_eq!(t.fs_read, pd_fs_read as usize as u32);
    assert_eq!(t.fs_write, pd_fs_write as usize as u32);
    assert_eq!(t.fs_seek, pd_fs_seek as usize as u32);
    assert_eq!(t.fs_tell, pd_fs_tell as usize as u32);
    assert_eq!(t.fs_eof, pd_fs_eof as usize as u32);
    assert_eq!(t.fs_mkdir, pd_fs_mkdir as usize as u32);
    assert_eq!(t.fs_remove, pd_fs_remove as usize as u32);
    assert_eq!(t.fs_opendir, pd_fs_opendir as usize as u32);
    assert_eq!(t.fs_readdir, pd_fs_readdir as usize as u32);
    assert_eq!(t.fs_closedir, pd_fs_closedir as usize as u32);
    assert_eq!(t.fs_stat, pd_fs_stat as usize as u32);
}

#[test]
fn fs_stubs_return_error_codes_syscall_fs() {
    // Stubs return -1 (error); on device the global FsState is used instead.
    assert_eq!(unsafe { pd_fs_open(core::ptr::null(), 0, 0) }, -1);
    assert_eq!(pd_fs_close(-1), -1);
    assert_eq!(unsafe { pd_fs_read(-1, core::ptr::null_mut(), 0) }, -1);
    assert_eq!(unsafe { pd_fs_write(-1, core::ptr::null(), 0) }, -1);
    assert_eq!(pd_fs_seek(-1, 0, 0), -1);
    assert_eq!(pd_fs_tell(-1), -1);
    assert_eq!(pd_fs_eof(-1), -1);
    assert_eq!(unsafe { pd_fs_mkdir(core::ptr::null(), 0) }, -1);
    assert_eq!(unsafe { pd_fs_remove(core::ptr::null(), 0) }, -1);
    assert_eq!(unsafe { pd_fs_opendir(core::ptr::null(), 0) }, -1);
    assert_eq!(unsafe { pd_fs_readdir(-1, core::ptr::null_mut()) }, -1);
    assert_eq!(pd_fs_closedir(-1), -1);
    assert_eq!(
        unsafe { pd_fs_stat(core::ptr::null(), 0, core::ptr::null_mut()) },
        -1
    );
}
