use kernel::device::serial::{serial_write_panic_report, set_serial_write_fn};
use std::sync::Mutex;

static BYTES: Mutex<Vec<u8>> = Mutex::new(Vec::new());

fn capture_write(bytes: &[u8]) {
    BYTES.lock().unwrap().extend_from_slice(bytes);
}

#[test]
fn panic_report_is_deterministic_and_structured_panic_serial() {
    // SAFETY: test installs a single callback and does not mutate it concurrently.
    unsafe { set_serial_write_fn(capture_write) };

    {
        let mut out = BYTES.lock().unwrap();
        out.clear();
    }

    serial_write_panic_report(
        Some(("kernel/src/main.rs", 123, 7)),
        format_args!("launch returned unexpectedly"),
    );

    let out = BYTES.lock().unwrap();
    let text = core::str::from_utf8(out.as_slice()).expect("serial bytes must be utf-8");

    assert_eq!(
        text,
        "\r\n!!! PANIC !!!\r\nLOC:kernel/src/main.rs:123:7\r\nMSG:launch returned unexpectedly\r\n"
    );
}
