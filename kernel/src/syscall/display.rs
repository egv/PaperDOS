use crate::display::refresh::trigger_full_refresh;
use crate::display::render::strip_geometry;
use crate::display::scene::{logical_to_physical, render_scene_strip, DisplayScene};
use crate::display::ssd1677::{
    emit_full_window_and_cursor, PANEL_HEIGHT, ROW_BYTES, STRIP_BUFFER_BYTES,
};
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

/// Push `buf` to both full-frame RAM planes,
/// then trigger a full refresh cycle.
///
/// The framebuffer is already in the packed 1-bit SSD1677 format, so this
/// function slices it directly without repacking.
pub fn display_refresh_flush<T: DisplayTransport>(
    transport: &mut T,
    buf: &FrameBuffer,
) -> Result<(), T::Error> {
    use crate::display::ssd1677::{WRITE_RAM_BW, WRITE_RAM_RED};
    let geo = strip_geometry(PANEL_HEIGHT as usize);

    // Match pulp-os: program the full window per RAM plane, then stream the
    // frame in 40-row strips so each SPI write stays under the DMA limit.
    for ram_command in [WRITE_RAM_RED, WRITE_RAM_BW] {
        emit_full_window_and_cursor(transport)?;
        transport.write_command(ram_command)?;
        for strip_idx in 0..geo.strip_count {
            let row_count = if strip_idx + 1 == geo.strip_count {
                geo.last_strip_rows
            } else {
                geo.rows_per_strip
            };
            let byte_start = strip_idx * geo.rows_per_strip * ROW_BYTES;
            let byte_end = byte_start + row_count * ROW_BYTES;
            transport.write_data(&buf[byte_start..byte_end])?;
        }
    }
    trigger_full_refresh(transport)
}

/// Replay a retained scene into a strip buffer and flush it to the display.
pub fn display_scene_flush<T: DisplayTransport>(
    transport: &mut T,
    scene: &DisplayScene,
    strip_buf: &mut [u8],
) -> Result<(), T::Error> {
    use crate::display::ssd1677::{WRITE_RAM_BW, WRITE_RAM_RED};

    let geo = strip_geometry(PANEL_HEIGHT as usize);
    debug_assert!(
        strip_buf.len() >= geo.rows_per_strip * ROW_BYTES,
        "strip buffer must hold one full strip"
    );

    for ram_command in [WRITE_RAM_RED, WRITE_RAM_BW] {
        emit_full_window_and_cursor(transport)?;
        transport.write_command(ram_command)?;
        for strip_idx in 0..geo.strip_count {
            let row_start = (strip_idx * geo.rows_per_strip) as u16;
            let row_count = if strip_idx + 1 == geo.strip_count {
                geo.last_strip_rows
            } else {
                geo.rows_per_strip
            } as u16;
            let byte_count = row_count as usize * ROW_BYTES;
            let buf = &mut strip_buf[..byte_count];
            render_scene_strip(scene, row_start, row_count, buf);
            transport.write_data(buf)?;
        }
    }
    trigger_full_refresh(transport)
}

/// Flush the retained global display scene using the global strip scratch buffer.
pub fn display_scene_flush_current<T: DisplayTransport>(transport: &mut T) -> Result<(), T::Error> {
    // SAFETY: single-core kernel; scene and scratch buffer are only accessed from
    // display syscalls / refresh entry points.
    unsafe {
        display_scene_flush(
            transport,
            &*(&raw const DISPLAY_SCENE),
            &mut *(&raw mut DISPLAY_STRIP_BUF),
        )
    }
}

// ── Pure pixel helpers (testable without global state) ────────────────────────

/// Set or clear one pixel in a packed 1-bit framebuffer.
///
/// Logical coordinates use the portrait orientation exposed by the syscall API
/// (`480×800`, matching pulp-os). They are rotated into the panel's physical
/// `800×480` packed framebuffer before the bit is updated.
pub fn set_pixel_in(buf: &mut FrameBuffer, x: i32, y: i32, color: u8) {
    let Some((px, py)) = logical_to_physical(x, y) else {
        return;
    };
    let byte_idx = py * ROW_BYTES + px / 8;
    let bit_mask = 0x80u8 >> (px % 8);
    if color != 0 {
        buf[byte_idx] |= bit_mask;
    } else {
        buf[byte_idx] &= !bit_mask;
    }
}

/// Fill a rectangular region with `color` in portrait logical coordinates.
pub fn fill_rect_in(buf: &mut FrameBuffer, x: i32, y: i32, w: i32, h: i32, color: u8) {
    let x0 = x.max(0) as usize;
    let y0 = y.max(0) as usize;
    let x1 = (x + w).min(crate::abi::PD_SCREEN_WIDTH).max(0) as usize;
    let y1 = (y + h).min(crate::abi::PD_SCREEN_HEIGHT).max(0) as usize;
    for row in y0..y1 {
        for col in x0..x1 {
            set_pixel_in(buf, col as i32, row as i32, color);
        }
    }
}

/// Draw a 1-pixel-wide outline rectangle in portrait logical coordinates.
pub fn draw_rect_in(buf: &mut FrameBuffer, x: i32, y: i32, w: i32, h: i32, color: u8) {
    if w <= 0 || h <= 0 {
        return;
    }
    for col in x..x + w {
        set_pixel_in(buf, col, y, color);
        set_pixel_in(buf, col, y + h - 1, color);
    }
    for row in y + 1..y + h - 1 {
        set_pixel_in(buf, x, row, color);
        set_pixel_in(buf, x + w - 1, row, color);
    }
}

// ── Global flush function ─────────────────────────────────────────────────────
//
// Decouples pd_display_refresh from the concrete X4DisplayTransport (device-only).
// On device, main.rs calls set_display_flush_fn() once during init with a function
// that drives display_refresh_flush through the real transport.

static mut FLUSH_FN: fn(i32) = |_| {};

/// Register the display flush function.
///
/// # Safety
/// Must be called at most once, before the first `pd_display_refresh` call.
/// Not safe to call concurrently.
pub unsafe fn set_display_flush_fn(f: fn(i32)) {
    (&raw mut FLUSH_FN).write(f);
}

// ── Retained display scene ────────────────────────────────────────────────────
//
// The device runtime keeps a compact retained op list plus a 4 KB strip scratch
// buffer instead of a full 48 KB framebuffer.

static mut DISPLAY_SCENE: DisplayScene = DisplayScene::new();
static mut DISPLAY_STRIP_BUF: [u8; STRIP_BUFFER_BYTES] = [0u8; STRIP_BUFFER_BYTES];

// ── Syscall stubs ─────────────────────────────────────────────────────────────

/// Reset the retained scene to a solid background color.
pub extern "C" fn pd_display_clear(color: u8) {
    // SAFETY: single-core, no concurrent kernel access.
    unsafe { (*(&raw mut DISPLAY_SCENE)).clear_to(color) };
}

/// Flush the framebuffer to the display and trigger a refresh.
///
/// Calls the function registered via `set_display_flush_fn`.
/// Default: no-op until main.rs registers the real device flush.
pub extern "C" fn pd_display_refresh(mode: i32) {
    // SAFETY: FLUSH_FN is written once at init; no concurrent modification.
    unsafe { (*(&raw const FLUSH_FN))(mode) };
}

/// Return the panel width in pixels.
pub extern "C" fn pd_display_width() -> i32 {
    crate::abi::PD_SCREEN_WIDTH
}

/// Return the panel height in pixels.
pub extern "C" fn pd_display_height() -> i32 {
    crate::abi::PD_SCREEN_HEIGHT
}

/// Append one pixel draw op to the retained display scene.
pub extern "C" fn pd_display_set_pixel(x: i32, y: i32, color: u8) {
    unsafe { (*(&raw mut DISPLAY_SCENE)).set_pixel(x, y, color) };
}

/// Append an outline rectangle op to the retained display scene.
pub extern "C" fn pd_display_draw_rect(x: i32, y: i32, w: i32, h: i32, color: u8) {
    unsafe { (*(&raw mut DISPLAY_SCENE)).draw_rect(x, y, w, h, color) };
}

/// Append a filled rectangle op to the retained display scene.
pub extern "C" fn pd_display_fill_rect(x: i32, y: i32, w: i32, h: i32, color: u8) {
    unsafe { (*(&raw mut DISPLAY_SCENE)).fill_rect(x, y, w, h, color) };
}

pub extern "C" fn pd_display_draw_bitmap(_x: i32, _y: i32, _w: i32, _h: i32, _data: *const u8) {}
pub extern "C" fn pd_display_draw_text(
    _x: i32,
    _y: i32,
    _text: *const u8,
    _len: usize,
    _color: u8,
) {
}
pub extern "C" fn pd_display_set_rotation(_rotation: i32) {}
