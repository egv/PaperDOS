use kernel::syscall::build_syscall_table;
use kernel::syscall::fs::{
    pd_fs_close, pd_fs_closedir, pd_fs_eof, pd_fs_mkdir, pd_fs_open, pd_fs_opendir, pd_fs_read,
    pd_fs_readdir, pd_fs_remove, pd_fs_seek, pd_fs_stat, pd_fs_tell, pd_fs_write,
};

#[test]
fn syscall_table_fs_fields_populated_syscall_fs() {
    let t = build_syscall_table(0, 0);
    assert_ne!(t.fs_open, 0, "fs_open must be wired");
    assert_ne!(t.fs_close, 0, "fs_close must be wired");
    assert_ne!(t.fs_read, 0, "fs_read must be wired");
    assert_ne!(t.fs_write, 0, "fs_write must be wired");
    assert_ne!(t.fs_seek, 0, "fs_seek must be wired");
    assert_ne!(t.fs_tell, 0, "fs_tell must be wired");
    assert_ne!(t.fs_eof, 0, "fs_eof must be wired");
    assert_ne!(t.fs_mkdir, 0, "fs_mkdir must be wired");
    assert_ne!(t.fs_remove, 0, "fs_remove must be wired");
    assert_ne!(t.fs_opendir, 0, "fs_opendir must be wired");
    assert_ne!(t.fs_readdir, 0, "fs_readdir must be wired");
    assert_ne!(t.fs_closedir, 0, "fs_closedir must be wired");
    assert_ne!(t.fs_stat, 0, "fs_stat must be wired");
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
    assert_eq!(unsafe { pd_fs_stat(core::ptr::null(), 0, core::ptr::null_mut()) }, -1);
}
