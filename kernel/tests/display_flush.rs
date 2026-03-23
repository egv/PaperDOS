// F2: pd_display_refresh routes to the registered flush function.
//
// The global FLUSH_FN slot decouples the syscall entry point from the
// concrete X4DisplayTransport (device-only) so the mechanism can be
// tested on the host without real SPI hardware.

use std::sync::atomic::{AtomicI32, Ordering};

use kernel::syscall::display::{pd_display_refresh, set_display_flush_fn};

static LAST_MODE: AtomicI32 = AtomicI32::new(-1);

fn record_mode(mode: i32) {
    LAST_MODE.store(mode, Ordering::SeqCst);
}

#[test]
fn pd_display_refresh_routes_to_registered_fn_display_flush() {
    LAST_MODE.store(-1, Ordering::SeqCst);
    // SAFETY: single-threaded test binary; no concurrent access to FLUSH_FN.
    unsafe { set_display_flush_fn(record_mode) };
    pd_display_refresh(1);
    assert_eq!(
        LAST_MODE.load(Ordering::SeqCst),
        1,
        "pd_display_refresh must call the registered flush fn with mode"
    );
}

#[test]
fn pd_display_refresh_passes_mode_arg_display_flush() {
    LAST_MODE.store(-1, Ordering::SeqCst);
    unsafe { set_display_flush_fn(record_mode) };
    pd_display_refresh(42);
    assert_eq!(LAST_MODE.load(Ordering::SeqCst), 42);
}
