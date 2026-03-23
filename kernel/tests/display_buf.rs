// F1: Display pixel buffer — pure pixel/rect helpers + global framebuffer.
//
// Tests exercise set_pixel_in, fill_rect_in, and draw_rect_in in isolation
// (no transport, no global state mutations visible across tests).
//
// RISK: DISPLAY_BUF is packed 1-bit (48 KB). DO NOT change to byte-per-pixel
//       (384 KB) — that exceeds ESP32-C3 free RAM (~108–172 KB for the app region).

use kernel::abi::{PD_SCREEN_HEIGHT, PD_SCREEN_WIDTH};
use kernel::display::ssd1677::{PANEL_HEIGHT, ROW_BYTES};
use kernel::syscall::display::{
    draw_rect_in, fill_rect_in, set_pixel_in, FrameBuffer, FRAME_BYTES,
};

/// Allocate a zeroed framebuffer on the heap (48 KB — too large for stack).
fn zero_buf() -> Box<FrameBuffer> {
    vec![0u8; FRAME_BYTES]
        .into_boxed_slice()
        .try_into()
        .unwrap()
}

/// Allocate an all-ones (white) framebuffer on the heap.
fn white_buf() -> Box<FrameBuffer> {
    vec![0xFFu8; FRAME_BYTES]
        .into_boxed_slice()
        .try_into()
        .unwrap()
}

fn physical_bit(buf: &FrameBuffer, x: usize, y: usize) -> bool {
    let byte_idx = y * ROW_BYTES + x / 8;
    let bit_mask = 0x80u8 >> (x % 8);
    buf[byte_idx] & bit_mask != 0
}

fn logical_bit(buf: &FrameBuffer, x: usize, y: usize) -> bool {
    let px = y;
    let py = PANEL_HEIGHT as usize - 1 - x;
    physical_bit(buf, px, py)
}

// ── set_pixel_in ──────────────────────────────────────────────────────────────

#[test]
fn set_pixel_white_sets_top_left_logical_pixel_display_buf() {
    let mut buf = zero_buf();
    set_pixel_in(&mut *buf, 0, 0, 0xFF);
    let last_row = PANEL_HEIGHT as usize - 1;
    assert!(
        physical_bit(&buf, 0, last_row),
        "logical pixel (0,0) must map to the last physical row, first bit"
    );
}

#[test]
fn set_pixel_black_clears_top_left_logical_pixel_display_buf() {
    let mut buf = white_buf();
    set_pixel_in(&mut *buf, 0, 0, 0x00);
    let last_row = PANEL_HEIGHT as usize - 1;
    assert!(
        !physical_bit(&buf, 0, last_row),
        "logical pixel (0,0) black must clear the mapped physical bit"
    );
}

#[test]
fn set_pixel_second_logical_column_moves_up_one_physical_row_display_buf() {
    let mut buf = zero_buf();
    set_pixel_in(&mut *buf, 1, 0, 0xFF);
    let last_row = PANEL_HEIGHT as usize - 1;
    assert!(
        physical_bit(&buf, 0, last_row - 1),
        "logical pixel (1,0) must map one physical row above (0,0)"
    );
}

#[test]
fn set_pixel_second_logical_row_moves_right_one_physical_bit_display_buf() {
    let mut buf = zero_buf();
    set_pixel_in(&mut *buf, 0, 1, 0xFF);
    let last_row = PANEL_HEIGHT as usize - 1;
    assert!(
        physical_bit(&buf, 1, last_row),
        "logical pixel (0,1) must map to the second physical bit in the last row"
    );
}

#[test]
fn set_pixel_eighth_logical_row_sets_lsb_of_last_physical_row_display_buf() {
    let mut buf = zero_buf();
    set_pixel_in(&mut *buf, 0, 7, 0xFF);
    let last_row = PANEL_HEIGHT as usize - 1;
    assert!(
        physical_bit(&buf, 7, last_row),
        "logical pixel (0,7) must set the LSB of the last physical-row byte"
    );
}

#[test]
fn set_pixel_out_of_bounds_does_not_panic_display_buf() {
    let mut buf = zero_buf();
    // These must silently clamp, not panic.
    set_pixel_in(&mut *buf, -1, 0, 0xFF);
    set_pixel_in(&mut *buf, 0, -1, 0xFF);
    set_pixel_in(&mut *buf, PD_SCREEN_WIDTH, 0, 0xFF);
    set_pixel_in(&mut *buf, 0, PD_SCREEN_HEIGHT, 0xFF);
}

// ── fill_rect_in ──────────────────────────────────────────────────────────────

#[test]
fn fill_rect_1x8_fills_one_byte_white_display_buf() {
    let mut buf = zero_buf();
    fill_rect_in(&mut *buf, 0, 0, 1, 8, 0xFF);
    let last_row = PANEL_HEIGHT as usize - 1;
    assert_eq!(
        buf[last_row * ROW_BYTES],
        0xFF,
        "1x8 logical rect at origin must fill one physical byte"
    );
}

#[test]
fn fill_rect_1x8_fills_one_byte_black_display_buf() {
    let mut buf = white_buf();
    fill_rect_in(&mut *buf, 0, 0, 1, 8, 0x00);
    let last_row = PANEL_HEIGHT as usize - 1;
    assert_eq!(
        buf[last_row * ROW_BYTES],
        0x00,
        "clearing 1x8 logical rect must zero one physical byte"
    );
}

#[test]
fn fill_rect_full_logical_row_fills_all_logical_pixels_display_buf() {
    let mut buf = zero_buf();
    fill_rect_in(&mut *buf, 0, 0, PD_SCREEN_WIDTH, 1, 0xFF);
    assert!((0..PD_SCREEN_WIDTH as usize).all(|x| logical_bit(&buf, x, 0)));
}

#[test]
fn fill_rect_out_of_bounds_does_not_panic_display_buf() {
    let mut buf = zero_buf();
    // Huge rect that exceeds all panel dimensions.
    fill_rect_in(&mut *buf, -100, -100, 9999, 9999, 0xFF);
}

// ── draw_rect_in (outline) ────────────────────────────────────────────────────

#[test]
fn draw_rect_sets_top_left_corner_display_buf() {
    let mut buf = zero_buf();
    draw_rect_in(&mut *buf, 0, 0, 16, 8, 0xFF);
    assert!(
        logical_bit(&buf, 0, 0),
        "top-left logical corner pixel must be set"
    );
}

#[test]
fn draw_rect_interior_not_filled_display_buf() {
    let mut buf = zero_buf();
    draw_rect_in(&mut *buf, 0, 0, 24, 8, 0xFF);
    assert!(
        !logical_bit(&buf, 8, 1),
        "interior logical pixel must remain clear"
    );
}
