mod common;

use common::{RecordedOp, RecordingTransport};
use kernel::display::driver::{clear_screen, write_strip};
use kernel::display::render::{pack_strip, strip_geometry, StripGeometry};
use kernel::display::ssd1677::{
    PANEL_HEIGHT, ROW_BYTES, SET_RAM_X_COUNTER, SET_RAM_Y_COUNTER, SET_RAM_Y_RANGE,
    STRIP_BUFFER_BYTES, STRIP_COUNT, STRIP_ROWS, WRITE_RAM_BW,
};

#[test]
fn strip_geometry_display_render() {
    assert_eq!(
        strip_geometry(PANEL_HEIGHT as usize),
        StripGeometry {
            rows_per_strip: 40,
            strip_count: 12,
            last_strip_rows: 40,
        }
    );

    assert_eq!(
        strip_geometry(PANEL_HEIGHT as usize + 1),
        StripGeometry {
            rows_per_strip: 40,
            strip_count: 13,
            last_strip_rows: 1,
        }
    );

    assert!(40 * ROW_BYTES <= STRIP_BUFFER_BYTES);
    assert!(41 * ROW_BYTES > STRIP_BUFFER_BYTES);
}

#[test]
fn strip_packer_byte_order() {
    // MSB first: pixel 0 maps to bit 7 of byte 0.
    // Alternating white(1)/black(0): 1 0 1 0 1 0 1 0 → 0b10101010 = 0xAA
    let pixels = [1u8, 0, 1, 0, 1, 0, 1, 0];
    let mut dst = [0u8; 1];
    pack_strip(&pixels, 8, 1, &mut dst);
    assert_eq!(dst[0], 0xAA);
}

#[test]
fn strip_packer_row_boundary() {
    // 2 rows × 8 pixels wide.  Row 0 all-white → 0xFF, row 1 all-black → 0x00.
    let mut pixels = [0u8; 16];
    for p in &mut pixels[..8] {
        *p = 1;
    }
    let mut dst = [0u8; 2];
    pack_strip(&pixels, 8, 2, &mut dst);
    assert_eq!(dst[0], 0xFF);
    assert_eq!(dst[1], 0x00);
}

#[test]
fn strip_packer_full_row_width() {
    // Verify packing works at the real panel width (ROW_BYTES bytes per row).
    // All-white single row must produce ROW_BYTES 0xFF bytes.
    let width = ROW_BYTES * 8; // = PANEL_WIDTH = 800
    let pixels = vec![1u8; width];
    let mut dst = vec![0u8; ROW_BYTES];
    pack_strip(&pixels, width, 1, &mut dst);
    assert!(dst.iter().all(|&b| b == 0xFF));
}

#[test]
fn single_strip_write_emits_window_cursor_then_data() {
    // Strip at row 0 (1 row, ROW_BYTES = 100 bytes of pre-packed pixel data).
    // Expected sequence: Y range, Y cursor, X cursor reset, WRITE_RAM_BW + payload.
    let mut transport = RecordingTransport::default();
    let packed = vec![0xA5u8; ROW_BYTES];

    write_strip(&mut transport, 0, 1, &packed).unwrap();

    assert_eq!(
        transport.ops,
        vec![
            RecordedOp::Command(SET_RAM_Y_RANGE),
            RecordedOp::Data(vec![0x00, 0x00, 0x00, 0x00]),
            RecordedOp::Command(SET_RAM_Y_COUNTER),
            RecordedOp::Data(vec![0x00, 0x00]),
            RecordedOp::Command(SET_RAM_X_COUNTER),
            RecordedOp::Data(vec![0x00]),
            RecordedOp::Command(WRITE_RAM_BW),
            RecordedOp::Data(vec![0xA5; ROW_BYTES]),
        ]
    );
}

#[test]
fn full_clear_writes_all_strips_in_order() {
    // 12 strips × 8 ops each (Y range, Y cursor, X cursor, WRITE_RAM_BW × command+data) = 96 ops.
    let mut transport = RecordingTransport::default();
    let mut buf = vec![0u8; STRIP_BUFFER_BYTES];

    clear_screen(&mut transport, 0xFF, &mut buf).unwrap();

    // Correct strip count
    assert_eq!(transport.ops.len(), STRIP_COUNT * 8);

    // First strip: Y window rows 0..39 = [0x00,0x00, 0x27,0x00]
    assert_eq!(transport.ops[0], RecordedOp::Command(SET_RAM_Y_RANGE));
    assert_eq!(transport.ops[1], RecordedOp::Data(vec![0x00, 0x00, 0x27, 0x00]));

    // Last strip: Y window rows 440..479 = [0xB8,0x01, 0xDF,0x01]
    let last = (STRIP_COUNT - 1) * 8;
    assert_eq!(transport.ops[last], RecordedOp::Command(SET_RAM_Y_RANGE));
    assert_eq!(
        transport.ops[last + 1],
        RecordedOp::Data(vec![0xB8, 0x01, 0xDF, 0x01])
    );

    // Every WRITE_RAM_BW payload must be all 0xFF (white fill) and correct size
    for strip in 0..STRIP_COUNT {
        let rows = if strip + 1 == STRIP_COUNT {
            // last strip
            PANEL_HEIGHT as usize - strip * STRIP_ROWS
        } else {
            STRIP_ROWS
        };
        if let RecordedOp::Data(data) = &transport.ops[strip * 8 + 7] {
            assert_eq!(data.len(), rows * ROW_BYTES);
            assert!(data.iter().all(|&b| b == 0xFF));
        } else {
            panic!("expected Data op for WRITE_RAM_BW payload");
        }
    }
}
