// F6: run_launcher orchestrates scan_apps + render_launcher + button input.
//
// Uses InMemoryBlockDevice with two .PDB files in the root.
// Button callbacks are registered via set_input_wait_button_fn to simulate
// user input without real hardware.
//
// GLOBAL_LOCK serialises tests that write to WAIT_BUTTON_FN to prevent
// races between concurrent test threads within this binary.

mod common;
use common::InMemoryBlockDevice;

use std::sync::{Mutex, MutexGuard};
use std::sync::atomic::{AtomicUsize, Ordering};

use kernel::abi::{PD_BTN_DOWN, PD_BTN_OK};
use kernel::launcher::run_launcher;
use kernel::storage::fs::FsState;
use kernel::syscall::display::{FrameBuffer, FRAME_BYTES};
use kernel::syscall::input::set_input_wait_button_fn;

static GLOBAL_LOCK: Mutex<()> = Mutex::new(());

/// Allocate a zeroed FrameBuffer on the heap (48 KB).
fn zero_buf() -> Box<FrameBuffer> {
    vec![0u8; FRAME_BYTES]
        .into_boxed_slice()
        .try_into()
        .unwrap()
}

fn new_pdb_fs() -> FsState<InMemoryBlockDevice> {
    FsState::new(InMemoryBlockDevice::new(common::make_apps_fat16_image()))
}

fn select_immediately() -> u32 { PD_BTN_OK }

static DOWN_THEN_OK_IDX: AtomicUsize = AtomicUsize::new(0);
fn down_then_ok() -> u32 {
    let n = DOWN_THEN_OK_IDX.fetch_add(1, Ordering::SeqCst);
    if n == 0 { PD_BTN_DOWN } else { PD_BTN_OK }
}

#[test]
fn run_launcher_returns_pdb_filename_on_select_launcher_loop() {
    let _g: MutexGuard<'_, ()> = GLOBAL_LOCK.lock().unwrap();
    let mut fs = new_pdb_fs();
    let mut buf = zero_buf();
    // SAFETY: single-threaded region guarded by GLOBAL_LOCK.
    unsafe { set_input_wait_button_fn(select_immediately) };
    let filename = run_launcher(&mut fs, &mut *buf);
    assert_eq!(&filename[8..11], b"PDB", "selected filename must have PDB extension");
}

#[test]
fn run_launcher_down_then_select_picks_second_app_launcher_loop() {
    let _g: MutexGuard<'_, ()> = GLOBAL_LOCK.lock().unwrap();
    DOWN_THEN_OK_IDX.store(0, Ordering::SeqCst);
    let mut fs = new_pdb_fs();
    let mut buf = zero_buf();
    unsafe { set_input_wait_button_fn(down_then_ok) };
    let filename = run_launcher(&mut fs, &mut *buf);
    // After pressing DOWN once, the second app (WORLD.PDB) should be selected.
    assert_eq!(&filename[0..5], b"WORLD", "DOWN then SELECT must pick second app");
}
