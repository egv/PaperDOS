// ── Memory syscall stubs ──────────────────────────────────────────────────────
//
// Device impls will delegate to the global allocator (esp-alloc) via
// the `alloc` crate behind a `#[cfg(all(target_arch = "riscv32", ...))]` guard.

/// Allocate `size` bytes.
///
/// Returns a pointer to the allocation, or null on failure.
///
/// Stub: returns null.  Device impl: calls `alloc::alloc::alloc`.
pub extern "C" fn pd_mem_alloc(_size: usize) -> *mut u8 {
    core::ptr::null_mut()
}

/// Free a pointer previously returned by [`pd_mem_alloc`] or [`pd_mem_realloc`].
///
/// `size` must match the original allocation size.
///
/// Stub: no-op.  Device impl: calls `alloc::alloc::dealloc`.
///
/// # Safety
/// `ptr` must be null or a valid allocation of `size` bytes.
pub unsafe extern "C" fn pd_mem_free(_ptr: *mut u8, _size: usize) {}

/// Resize an allocation.
///
/// `ptr` may be null (acts as alloc); `new_size` may be 0 (acts as free, returns null).
///
/// Returns a pointer to the resized allocation, or null on failure.
///
/// Stub: returns null.  Device impl: calls `alloc::alloc::realloc`.
///
/// # Safety
/// `ptr` must be null or a valid allocation of `old_size` bytes.
pub unsafe extern "C" fn pd_mem_realloc(
    _ptr: *mut u8,
    _old_size: usize,
    _new_size: usize,
) -> *mut u8 {
    core::ptr::null_mut()
}
