mod common;

use core::sync::atomic::{AtomicBool, Ordering};

use common::InMemoryBlockDevice;
use kernel::abi::{PdSyscalls, PD_ABI_VERSION};
use kernel::boot_app::{load_and_run, JumpMode, LoadAndRunError};
use kernel::device::serial::set_serial_write_fn;
use kernel::storage::fs::FsState;
use kernel::syscall::build_syscall_table;

static SAW_SELECT: AtomicBool = AtomicBool::new(false);
static SAW_OPEN: AtomicBool = AtomicBool::new(false);
static SAW_STAT: AtomicBool = AtomicBool::new(false);
static SAW_READ: AtomicBool = AtomicBool::new(false);

fn stage_write(bytes: &[u8]) {
    if bytes == b"LAUNCH:select\n" {
        SAW_SELECT.store(true, Ordering::SeqCst);
    } else if bytes == b"LAUNCH:stat\n" {
        SAW_STAT.store(true, Ordering::SeqCst);
    } else if bytes == b"LAUNCH:open\n" {
        SAW_OPEN.store(true, Ordering::SeqCst);
    } else if bytes == b"LAUNCH:read\n" {
        SAW_READ.store(true, Ordering::SeqCst);
    }
}

/// All four file-load stage tags must be emitted for a successful load.
#[test]
fn pdb_file_load_stages_emitted_boot_app() {
    // SAFETY: called once per test binary; no concurrent writer.
    unsafe { set_serial_write_fn(stage_write) };

    SAW_SELECT.store(false, Ordering::SeqCst);
    SAW_STAT.store(false, Ordering::SeqCst);
    SAW_OPEN.store(false, Ordering::SeqCst);
    SAW_READ.store(false, Ordering::SeqCst);

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

    assert!(result.is_ok(), "valid PDB must load: {result:?}");
    assert!(
        SAW_SELECT.load(Ordering::SeqCst),
        "LAUNCH:select must be logged"
    );
    assert!(
        SAW_STAT.load(Ordering::SeqCst),
        "LAUNCH:stat must be logged"
    );
    assert!(
        SAW_OPEN.load(Ordering::SeqCst),
        "LAUNCH:open must be logged"
    );
    assert!(
        SAW_READ.load(Ordering::SeqCst),
        "LAUNCH:read must be logged"
    );
}

static JUMP_CALLED: AtomicBool = AtomicBool::new(false);

unsafe fn mock_jump(_entry: *const u8, _syscalls: *const PdSyscalls) {
    JUMP_CALLED.store(true, Ordering::SeqCst);
}

#[test]
fn boot_app_load_and_run_reads_named_pdb_and_jumps() {
    JUMP_CALLED.store(false, Ordering::SeqCst);
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
            JumpMode::Jump(mock_jump),
        )
    };

    assert!(
        result.is_ok(),
        "valid PDB on FAT image must load: {result:?}"
    );
    assert!(JUMP_CALLED.load(Ordering::SeqCst), "jump must be invoked");
}

#[test]
fn boot_app_load_and_run_reports_small_scratch_buffer() {
    let bd = InMemoryBlockDevice::new(common::make_valid_apps_fat16_image());
    let mut fs = FsState::new(bd);
    let mut pdb_buf = [0u8; 32];
    let mut app_region = [0u8; 256];
    let syscalls = build_syscall_table(0, 0);

    let result = unsafe {
        load_and_run(
            &mut fs,
            b"HELLO   PDB",
            &mut pdb_buf,
            &mut app_region,
            &syscalls,
            JumpMode::Jump(mock_jump),
        )
    };

    assert!(matches!(result, Err(LoadAndRunError::FileTooLarge { .. })));
}

#[test]
fn boot_app_test_data_matches_kernel_abi() {
    assert_eq!(PD_ABI_VERSION, 1);
}
