mod common;

use core::sync::atomic::{AtomicBool, Ordering};

use common::InMemoryBlockDevice;
use kernel::abi::{PdSyscalls, PD_ABI_VERSION};
use kernel::boot_app::{load_and_run, JumpMode, LoadAndRunError};
use kernel::storage::fs::FsState;
use kernel::syscall::build_syscall_table;

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
