use crate::abi::PdSyscalls;

/// Type of the application entry point called by the kernel.
///
/// The kernel passes a pointer to the immutable syscall table; the app
/// receives it in the first argument register per the C calling convention.
pub type AppEntry = unsafe extern "C" fn(*const PdSyscalls);

/// Invoke the application entry point at `entry`.
///
/// On a riscv32 device the caller should arm the watchdog before this call
/// and disarm it on return.  On host targets this compiles to a plain
/// indirect function call, which lets unit tests verify the dispatch logic
/// without real hardware or inline assembly.
///
/// # Safety
/// `entry` must be a valid function pointer for the current execution
/// environment cast to `*const u8`.  `syscalls` may be null (stub/test)
/// or a valid `*const PdSyscalls` that remains live for the duration of the
/// call.
pub unsafe fn jump_to_app(entry: *const u8, syscalls: *const PdSyscalls) {
    let fn_ptr: AppEntry = core::mem::transmute(entry);
    fn_ptr(syscalls);
}
