// ── Filesystem syscall stubs ──────────────────────────────────────────────────
//
// Each function has the C ABI signature the app expects. All stubs return -1
// (error / not-implemented). Device impls will delegate to a global `FsState`
// via critical-section mutex, wired in a later task.

/// Open a file.
///
/// `path`  — UTF-8 path bytes (not NUL-terminated).
/// `len`   — byte length of `path`.
/// `write` — 0 = read-only, 1 = read-write / create-or-truncate.
///
/// Returns a non-negative file handle, or −1 on error.
///
/// # Safety
/// `path` must point to at least `len` valid bytes.
pub unsafe extern "C" fn pd_fs_open(_path: *const u8, _len: usize, _write: i32) -> i32 {
    -1
}

/// Close a file handle previously returned by [`pd_fs_open`].
///
/// Returns 0 on success, −1 on error.
pub extern "C" fn pd_fs_close(_handle: i32) -> i32 {
    -1
}

/// Read up to `len` bytes from `handle` into `buf`.
///
/// Returns the number of bytes read, 0 at EOF, or −1 on error.
///
/// # Safety
/// `buf` must be valid for `len` bytes of writes.
pub unsafe extern "C" fn pd_fs_read(_handle: i32, _buf: *mut u8, _len: usize) -> i32 {
    -1
}

/// Write `len` bytes from `buf` to `handle`.
///
/// Returns the number of bytes written, or −1 on error.
///
/// # Safety
/// `buf` must be valid for `len` bytes of reads.
pub unsafe extern "C" fn pd_fs_write(_handle: i32, _buf: *const u8, _len: usize) -> i32 {
    -1
}

/// Seek within an open file.
///
/// `whence` follows the `PD_SEEK_*` constants (0=SET, 1=CUR, 2=END).
///
/// Returns 0 on success, −1 on error.
pub extern "C" fn pd_fs_seek(_handle: i32, _offset: i32, _whence: i32) -> i32 {
    -1
}

/// Return the current file position.
///
/// Returns the byte offset from the start, or −1 on error.
pub extern "C" fn pd_fs_tell(_handle: i32) -> i32 {
    -1
}

/// Return 1 if the file position is at EOF, 0 if not, −1 on error.
pub extern "C" fn pd_fs_eof(_handle: i32) -> i32 {
    -1
}

/// Create a directory at `path`.
///
/// Returns 0 on success, −1 on error.
///
/// # Safety
/// `path` must point to at least `len` valid bytes.
pub unsafe extern "C" fn pd_fs_mkdir(_path: *const u8, _len: usize) -> i32 {
    -1
}

/// Remove a file or empty directory at `path`.
///
/// Returns 0 on success, −1 on error.
///
/// # Safety
/// `path` must point to at least `len` valid bytes.
pub unsafe extern "C" fn pd_fs_remove(_path: *const u8, _len: usize) -> i32 {
    -1
}

/// Open a directory for iteration.
///
/// Returns a non-negative directory handle, or −1 on error.
///
/// # Safety
/// `path` must point to at least `len` valid bytes.
pub unsafe extern "C" fn pd_fs_opendir(_path: *const u8, _len: usize) -> i32 {
    -1
}

/// Read the next entry from an open directory into `dirent_buf`.
///
/// `dirent_buf` must point to a `PdDirent`-sized buffer (264 bytes).
///
/// Returns 0 if an entry was written, 1 if no more entries remain, −1 on error.
///
/// # Safety
/// `dirent_buf` must be valid for 264 bytes of writes.
pub unsafe extern "C" fn pd_fs_readdir(_handle: i32, _dirent_buf: *mut u8) -> i32 {
    -1
}

/// Close a directory handle previously returned by [`pd_fs_opendir`].
///
/// Returns 0 on success, −1 on error.
pub extern "C" fn pd_fs_closedir(_handle: i32) -> i32 {
    -1
}

/// Stat a file or directory at `path`, writing a `PdStat` into `stat_buf`.
///
/// `stat_buf` must point to a `PdStat`-sized buffer (12 bytes).
///
/// Returns 0 on success, −1 on error.
///
/// # Safety
/// `path` must point to at least `len` valid bytes;
/// `stat_buf` must be valid for 12 bytes of writes.
pub unsafe extern "C" fn pd_fs_stat(
    _path: *const u8,
    _len: usize,
    _stat_buf: *mut u8,
) -> i32 {
    -1
}
