use crate::display::ssd1677::{ROW_BYTES, STRIP_BUFFER_BYTES};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct StripGeometry {
    pub rows_per_strip: usize,
    pub strip_count: usize,
    pub last_strip_rows: usize,
}

pub fn strip_geometry(total_rows: usize) -> StripGeometry {
    let rows_per_strip = STRIP_BUFFER_BYTES / ROW_BYTES;
    let strip_count = total_rows.div_ceil(rows_per_strip);
    let last_strip_rows = if strip_count == 0 {
        0
    } else {
        total_rows - (rows_per_strip * (strip_count - 1))
    };

    StripGeometry {
        rows_per_strip,
        strip_count,
        last_strip_rows,
    }
}

/// Pack monochrome pixels into 1-bit-per-pixel wire format.
///
/// `pixels` — one byte per pixel, 0 = black, non-zero = white.
/// `width`  — pixels per row; must be a multiple of 8.
/// `rows`   — number of rows to pack.
/// `dst`    — output buffer; must hold at least `rows * (width / 8)` bytes.
///
/// Bit order: MSB first — pixel 0 maps to bit 7 of the first output byte.
pub fn pack_strip(pixels: &[u8], width: usize, rows: usize, dst: &mut [u8]) {
    let row_bytes = width / 8;
    for row in 0..rows {
        for col_byte in 0..row_bytes {
            let mut byte = 0u8;
            for bit in 0..8u8 {
                if pixels[row * width + col_byte * 8 + bit as usize] != 0 {
                    byte |= 1 << (7 - bit);
                }
            }
            dst[row * row_bytes + col_byte] = byte;
        }
    }
}
