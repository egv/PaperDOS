use kernel::display::render::{pack_strip, strip_geometry, StripGeometry};
use kernel::display::ssd1677::{PANEL_HEIGHT, ROW_BYTES, STRIP_BUFFER_BYTES};

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
