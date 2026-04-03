// ── Device serial console (USB Serial/JTAG) ──────────────────────────────────
//
// A thin fn-pointer shim so the device `main.rs` can wire the real USB
// Serial/JTAG peripheral without pulling hardware dependencies into the
// kernel library.  Before `set_serial_write_fn` is called the default
// implementation is a silent no-op, which is safe on both host and device.

use core::fmt;

static mut SERIAL_WRITE_FN: fn(&[u8]) = |_| {};

struct SerialWriter;

impl fmt::Write for SerialWriter {
    fn write_str(&mut self, s: &str) -> fmt::Result {
        serial_write_bytes(s.as_bytes());
        Ok(())
    }
}

/// Write a formatted message to the serial console via the installed callback.
///
/// Each segment of `args` is streamed through `serial_write_bytes`; no heap
/// allocation is required.
pub fn serial_write_fmt(args: fmt::Arguments<'_>) {
    use fmt::Write;
    let _ = SerialWriter.write_fmt(args);
}

/// Emit a deterministic panic report to the serial console.
///
/// The output format is stable and line-oriented:
/// - `!!! PANIC !!!`
/// - `LOC:<file>:<line>:<column>` (or `LOC:<unknown>`)
/// - `MSG:<message>`
pub fn serial_write_panic_report(location: Option<(&str, u32, u32)>, message: fmt::Arguments<'_>) {
    serial_write_bytes(b"\r\n!!! PANIC !!!\r\n");
    match location {
        Some((file, line, column)) => {
            serial_write_fmt(format_args!("LOC:{}:{}:{}\r\n", file, line, column));
        }
        None => serial_write_bytes(b"LOC:<unknown>\r\n"),
    }
    serial_write_fmt(format_args!("MSG:{}\r\n", message));
}

/// Format a panic using [`core::panic::PanicInfo`] and emit it to serial.
pub fn serial_write_panic_info(info: &core::panic::PanicInfo<'_>) {
    let location = info
        .location()
        .map(|loc| (loc.file(), loc.line(), loc.column()));
    serial_write_panic_report(location, format_args!("{}", info.message()));
}

/// Register the byte-write callback for the serial console.
///
/// # Safety
/// Must be called exactly once at init, before the first `serial_write_bytes`
/// call.  No concurrent callers allowed.
pub unsafe fn set_serial_write_fn(f: fn(&[u8])) {
    // SAFETY: caller guarantees exclusive access at init time.
    (&raw mut SERIAL_WRITE_FN).write(f);
}

/// Write `bytes` to the serial console via the installed callback.
///
/// Safe to call before `set_serial_write_fn`; bytes are silently dropped.
pub fn serial_write_bytes(bytes: &[u8]) {
    // SAFETY: SERIAL_WRITE_FN is written once at init; no concurrent mutation.
    unsafe { (*(&raw const SERIAL_WRITE_FN))(bytes) }
}
