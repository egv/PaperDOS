use core::sync::atomic::{AtomicBool, AtomicUsize, Ordering};

use kernel::abi::PdSyscalls;
use kernel::jump::jump_to_app;

// ── E4 tests ──────────────────────────────────────────────────────────────────

static CALLED: AtomicBool = AtomicBool::new(false);

unsafe extern "C" fn stub_noop(_syscalls: *const PdSyscalls) {
    CALLED.store(true, Ordering::SeqCst);
}

/// jump_to_app must invoke the entry function.
#[test]
fn jump_to_app_calls_entry_jump_app() {
    CALLED.store(false, Ordering::SeqCst);
    unsafe { jump_to_app(stub_noop as *const u8, core::ptr::null()) };
    assert!(CALLED.load(Ordering::SeqCst), "entry must be called");
}

static RECEIVED: AtomicUsize = AtomicUsize::new(0);

unsafe extern "C" fn stub_capture(syscalls: *const PdSyscalls) {
    RECEIVED.store(syscalls as usize, Ordering::SeqCst);
}

/// jump_to_app must forward the syscall-table pointer unchanged.
#[test]
fn jump_to_app_passes_syscall_ptr_jump_app() {
    RECEIVED.store(0, Ordering::SeqCst);
    let fake: *const PdSyscalls = 0xDEAD_C0DE as *const PdSyscalls;
    unsafe { jump_to_app(stub_capture as *const u8, fake) };
    assert_eq!(RECEIVED.load(Ordering::SeqCst), fake as usize);
}
