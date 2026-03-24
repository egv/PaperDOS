// BR-P1-G2: Instrument launcher→loader→jump path and add no-jump validation mode.
//
// Tests verify that each launch stage emits its serial tag, and that DryRun
// mode loads and prepares the image without invoking the real jump entry point.

mod common;

use core::sync::atomic::{AtomicBool, Ordering};

use common::InMemoryBlockDevice;
use kernel::abi::PdSyscalls;
use kernel::boot_app::{load_and_run, JumpMode};
use kernel::device::serial::set_serial_write_fn;
use kernel::storage::fs::FsState;
use kernel::syscall::build_syscall_table;

// ── Stage-detection globals ───────────────────────────────────────────────────

static SAW_SELECT: AtomicBool = AtomicBool::new(false);
static SAW_OPEN: AtomicBool = AtomicBool::new(false);
static SAW_READ: AtomicBool = AtomicBool::new(false);
static SAW_PREPARE: AtomicBool = AtomicBool::new(false);
static SAW_JUMP: AtomicBool = AtomicBool::new(false);
static JUMP_FN_CALLED: AtomicBool = AtomicBool::new(false);


fn stage_recording_write(bytes: &[u8]) {
    if bytes == b"LAUNCH:select\n" {
        SAW_SELECT.store(true, Ordering::SeqCst);
    } else if bytes == b"LAUNCH:open\n" {
        SAW_OPEN.store(true, Ordering::SeqCst);
    } else if bytes == b"LAUNCH:read\n" {
        SAW_READ.store(true, Ordering::SeqCst);
    } else if bytes == b"LAUNCH:prepare\n" {
        SAW_PREPARE.store(true, Ordering::SeqCst);
    } else if bytes == b"LAUNCH:jump\n" {
        SAW_JUMP.store(true, Ordering::SeqCst);
    }
}

unsafe fn mock_jump_record(_entry: *const u8, _syscalls: *const PdSyscalls) {
    JUMP_FN_CALLED.store(true, Ordering::SeqCst);
}

// ── Tests ─────────────────────────────────────────────────────────────────────

/// All six stage tags must be written to serial for a successful load+jump.
#[test]
fn all_stages_logged_for_valid_pdb_launcher_stages() {
    // SAFETY: called once per test binary; no concurrent writer.
    unsafe { set_serial_write_fn(stage_recording_write) };

    SAW_SELECT.store(false, Ordering::SeqCst);
    SAW_OPEN.store(false, Ordering::SeqCst);
    SAW_READ.store(false, Ordering::SeqCst);
    SAW_PREPARE.store(false, Ordering::SeqCst);
    SAW_JUMP.store(false, Ordering::SeqCst);
    JUMP_FN_CALLED.store(false, Ordering::SeqCst);

    let bd = InMemoryBlockDevice::new(common::make_valid_apps_fat16_image());
    let mut fs = FsState::new(bd);
    let mut pdb_buf = [0u8; 512];
    let mut app_region = [0u8; 256];
    let syscalls = build_syscall_table(0, 0);

    let result = unsafe {
        load_and_run(
            &mut fs,
            b"HELLO   PDB",
            &mut pdb_buf,
            &mut app_region,
            &syscalls,
            JumpMode::Jump(mock_jump_record),
        )
    };

    assert!(result.is_ok(), "valid PDB must load: {result:?}");
    assert!(JUMP_FN_CALLED.load(Ordering::SeqCst), "jump function must be called");
    assert!(SAW_SELECT.load(Ordering::SeqCst), "LAUNCH:select must be logged");
    assert!(SAW_OPEN.load(Ordering::SeqCst), "LAUNCH:open must be logged");
    assert!(SAW_READ.load(Ordering::SeqCst), "LAUNCH:read must be logged");
    assert!(SAW_PREPARE.load(Ordering::SeqCst), "LAUNCH:prepare must be logged");
    assert!(SAW_JUMP.load(Ordering::SeqCst), "LAUNCH:jump must be logged");
}

/// DryRun mode must load and prepare the image and return `Ok(())` without
/// panicking or executing any user-provided entry point.
///
/// The LAUNCH:jump tag is still emitted (so the host sees the full serial
/// trace up to the branch point), but the entry function is not invoked.
/// Correctness is demonstrated by the successful return value and the presence
/// of the prepare stage tag.
#[test]
fn dry_run_loads_without_calling_jump_launcher_stages() {
    // SAFETY: idempotent; same fn as the other test.
    unsafe { set_serial_write_fn(stage_recording_write) };

    SAW_SELECT.store(false, Ordering::SeqCst);
    SAW_PREPARE.store(false, Ordering::SeqCst);
    SAW_JUMP.store(false, Ordering::SeqCst);

    let bd = InMemoryBlockDevice::new(common::make_valid_apps_fat16_image());
    let mut fs = FsState::new(bd);
    let mut pdb_buf = [0u8; 512];
    let mut app_region = [0u8; 256];
    let syscalls = build_syscall_table(0, 0);

    let result = unsafe {
        load_and_run(
            &mut fs,
            b"HELLO   PDB",
            &mut pdb_buf,
            &mut app_region,
            &syscalls,
            JumpMode::DryRun,
        )
    };

    assert!(result.is_ok(), "dry-run must succeed: {result:?}");
    assert!(SAW_SELECT.load(Ordering::SeqCst), "LAUNCH:select must be logged in dry-run");
    assert!(SAW_PREPARE.load(Ordering::SeqCst), "LAUNCH:prepare must be logged in dry-run");
    assert!(SAW_JUMP.load(Ordering::SeqCst), "LAUNCH:jump must be logged in dry-run");
}
