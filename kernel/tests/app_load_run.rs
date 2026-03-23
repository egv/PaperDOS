// F7: load_and_run prepares a PDB image and invokes the jump function.
//
// The `jump_fn` parameter makes the call site testable without executing
// RISC-V machine code on the host — a mock records the call.

mod common;

use core::sync::atomic::{AtomicBool, Ordering};

use kernel::abi::PdSyscalls;
use kernel::loader::{load_and_run, LoadAndRunError};

static JUMP_CALLED: AtomicBool = AtomicBool::new(false);

unsafe fn mock_jump(_entry: *const u8, _syscalls: *const PdSyscalls) {
    JUMP_CALLED.store(true, Ordering::SeqCst);
}

unsafe fn unreachable_jump(_: *const u8, _: *const PdSyscalls) {
    panic!("jump must not be called on error paths");
}

#[test]
fn load_and_run_calls_jump_on_valid_pdb_app_load_run() {
    JUMP_CALLED.store(false, Ordering::SeqCst);
    let pdb = common::make_min_pdb(&[0u8; 4]);
    let mut region = vec![0u8; 256];
    // SAFETY: mock_jump does not execute any code.
    let result = unsafe {
        load_and_run(&pdb, &mut region, core::ptr::null(), mock_jump)
    };
    assert!(result.is_ok(), "valid PDB must succeed: {result:?}");
    assert!(JUMP_CALLED.load(Ordering::SeqCst), "jump must be called for a valid PDB");
}

#[test]
fn load_and_run_returns_error_for_bad_magic_app_load_run() {
    let mut pdb = common::make_min_pdb(&[0u8; 4]);
    pdb[0] = 0x00; // corrupt magic
    let mut region = vec![0u8; 256];
    let result = unsafe {
        load_and_run(&pdb, &mut region, core::ptr::null(), unreachable_jump)
    };
    assert!(
        matches!(result, Err(LoadAndRunError::PrepareImage(_))),
        "bad magic must return PrepareImage error"
    );
}

#[test]
fn load_and_run_returns_error_when_region_too_small_app_load_run() {
    let pdb = common::make_min_pdb(&[0u8; 64]); // image = 64 bytes
    let mut region = vec![0u8; 8];               // region only 8 bytes — too small
    let result = unsafe {
        load_and_run(&pdb, &mut region, core::ptr::null(), unreachable_jump)
    };
    assert!(
        matches!(result, Err(LoadAndRunError::PrepareImage(_))),
        "too-small region must return PrepareImage error"
    );
}
