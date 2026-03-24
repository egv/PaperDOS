use core::sync::atomic::{AtomicUsize, Ordering};
use kernel::device::serial::{serial_write_bytes, serial_write_fmt, set_serial_write_fn};

static CALL_COUNT: AtomicUsize = AtomicUsize::new(0);
static BYTE_SUM: AtomicUsize = AtomicUsize::new(0);

fn recording_write(bytes: &[u8]) {
    CALL_COUNT.fetch_add(1, Ordering::SeqCst);
    let sum: usize = bytes.iter().map(|&b| b as usize).sum();
    BYTE_SUM.fetch_add(sum, Ordering::SeqCst);
}

#[test]
fn serial_write_bytes_default_noop_serial_write() {
    // With no custom fn installed the default must be a no-op (no crash, no side-effects).
    serial_write_bytes(b"boot\n");
}

#[test]
fn serial_write_routes_through_installed_fn_serial_write() {
    // SAFETY: called once in this test binary; no concurrent writer.
    unsafe { set_serial_write_fn(recording_write) };

    let before = CALL_COUNT.load(Ordering::SeqCst);
    serial_write_bytes(b"PANIC\n");
    assert_eq!(
        CALL_COUNT.load(Ordering::SeqCst),
        before + 1,
        "serial_write_bytes must invoke the installed fn exactly once"
    );
}

#[test]
fn serial_write_fmt_routes_formatted_serial_write() {
    // SAFETY: same binary; fn is idempotent.
    unsafe { set_serial_write_fn(recording_write) };

    let before = CALL_COUNT.load(Ordering::SeqCst);
    serial_write_fmt(format_args!("panic at {}:{}", "src/main.rs", 42u32));
    assert!(
        CALL_COUNT.load(Ordering::SeqCst) > before,
        "serial_write_fmt must route through the installed fn"
    );
}

#[test]
fn serial_write_passes_bytes_verbatim_serial_write() {
    // SAFETY: same binary; fn is idempotent.
    unsafe { set_serial_write_fn(recording_write) };

    // Reset counters for a predictable check.
    CALL_COUNT.store(0, Ordering::SeqCst);
    BYTE_SUM.store(0, Ordering::SeqCst);

    serial_write_bytes(b"AB"); // 65 + 66 = 131
    assert_eq!(BYTE_SUM.load(Ordering::SeqCst), 131);
}
