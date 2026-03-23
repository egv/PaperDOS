use crate::display::driver::write_strip;
use crate::display::refresh::trigger_full_refresh;
use crate::display::render::strip_geometry;
use crate::display::ssd1677::{PANEL_HEIGHT, ROW_BYTES};
use crate::display::transport::DisplayTransport;

/// Total bytes in the packed 1-bit-per-pixel frame buffer for the full panel.
///
/// Each byte covers 8 horizontal pixels; `FRAME_BYTES = ROW_BYTES × PANEL_HEIGHT`.
pub const FRAME_BYTES: usize = ROW_BYTES * PANEL_HEIGHT as usize;

/// A packed 1-bit-per-pixel frame buffer for the full panel.
pub type FrameBuffer = [u8; FRAME_BYTES];

// ── Pure display helpers (testable without global state) ──────────────────────

/// Fill `buf` with `color` (e.g. `0xFF` = white, `0x00` = black).
///
/// `color` is a packed byte: every bit represents one pixel, so `0xFF` sets
/// all 8 pixels in a byte to white and `0x00` sets them all to black.
pub fn display_clear_to(buf: &mut FrameBuffer, color: u8) {
    buf.fill(color);
}

/// Push `buf` to the display hardware as a sequence of full-panel strips, then
/// trigger a full refresh cycle.
///
/// The framebuffer is already in the packed 1-bit SSD1677 format, so this
/// function slices it directly without repacking.
pub fn display_refresh_flush<T: DisplayTransport>(
    transport: &mut T,
    buf: &FrameBuffer,
) -> Result<(), T::Error> {
    let geo = strip_geometry(PANEL_HEIGHT as usize);
    for strip_idx in 0..geo.strip_count {
        let row_start = (strip_idx * geo.rows_per_strip) as u16;
        let row_count = if strip_idx + 1 == geo.strip_count {
            geo.last_strip_rows
        } else {
            geo.rows_per_strip
        };
        let byte_start = strip_idx * geo.rows_per_strip * ROW_BYTES;
        let byte_end = byte_start + row_count * ROW_BYTES;
        write_strip(transport, row_start, row_count as u16, &buf[byte_start..byte_end])?;
    }
    trigger_full_refresh(transport)
}

// ── Syscall stubs ─────────────────────────────────────────────────────────────
//
// These are the `extern "C"` entry points stored in `PdSyscalls`.
// Stub bodies are sufficient for Phase-1 host tests; device impls that
// operate on a global framebuffer and transport will be layered on in later
// tasks behind `cfg(all(target_arch = "riscv32", target_os = "none"))`.

/// Fill the framebuffer with `color`.
///
/// Stub: no-op.  Device impl: writes to the global framebuffer.
pub extern "C" fn pd_display_clear(_color: u8) {}

/// Flush the framebuffer to the display and trigger a refresh.
///
/// Stub: no-op.  Device impl: calls `display_refresh_flush` with the global
/// transport and framebuffer, then dispatches on `mode`.
pub extern "C" fn pd_display_refresh(_mode: i32) {}

/// Return the panel width in pixels.
pub extern "C" fn pd_display_width() -> i32 {
    crate::abi::PD_SCREEN_WIDTH
}

/// Return the panel height in pixels.
pub extern "C" fn pd_display_height() -> i32 {
    crate::abi::PD_SCREEN_HEIGHT
}

/// No-op stubs for draw operations — wired in D4.
pub extern "C" fn pd_display_set_pixel(_x: i32, _y: i32, _color: u8) {}
pub extern "C" fn pd_display_draw_rect(_x: i32, _y: i32, _w: i32, _h: i32, _color: u8) {}
pub extern "C" fn pd_display_fill_rect(_x: i32, _y: i32, _w: i32, _h: i32, _color: u8) {}
pub extern "C" fn pd_display_draw_bitmap(
    _x: i32,
    _y: i32,
    _w: i32,
    _h: i32,
    _data: *const u8,
) {
}
pub extern "C" fn pd_display_draw_text(
    _x: i32,
    _y: i32,
    _text: *const u8,
    _len: usize,
    _color: u8,
) {
}
pub extern "C" fn pd_display_set_rotation(_rotation: i32) {}
