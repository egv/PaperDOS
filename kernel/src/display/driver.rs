use crate::display::refresh::NormalizedRegion;
use crate::display::render::strip_geometry;
use crate::display::ssd1677::{
    emit_strip_window_and_cursor, PANEL_HEIGHT, ROW_BYTES, SET_RAM_X_COUNTER, SET_RAM_X_RANGE,
    SET_RAM_Y_COUNTER, SET_RAM_Y_RANGE, WRITE_RAM_BW,
};
use crate::display::transport::DisplayTransport;

/// Write one pre-packed strip through the transport.
///
/// `row_start`   — first row of the strip (0-based).
/// `row_count`   — number of rows in the strip.
/// `packed_rows` — 1-bit-per-pixel data produced by [`crate::display::render::pack_strip`];
///                 must be exactly `row_count * ROW_BYTES` bytes.
///
/// Issues the Y window, Y cursor, and X cursor commands via
/// [`emit_strip_window_and_cursor`], then streams `packed_rows` into BW RAM.
pub fn write_strip<T>(
    transport: &mut T,
    row_start: u16,
    row_count: u16,
    packed_rows: &[u8],
) -> Result<(), T::Error>
where
    T: DisplayTransport,
{
    debug_assert_eq!(
        packed_rows.len(),
        row_count as usize * ROW_BYTES,
        "packed_rows length must equal row_count * ROW_BYTES"
    );
    emit_strip_window_and_cursor(transport, row_start, row_count)?;
    transport.write_command(WRITE_RAM_BW)?;
    transport.write_data(packed_rows)?;
    Ok(())
}

/// Write a pre-packed partial-update region through the transport.
///
/// Sets the X and Y address windows to `region`, resets both counters, then
/// streams `packed_rows` into BW RAM via `WRITE_RAM_BW`.
///
/// `packed_rows` must be exactly
/// `(region.x_byte_end - region.x_byte_start + 1) * (region.y_end - region.y_start + 1)` bytes.
pub fn write_partial<T>(
    transport: &mut T,
    region: &NormalizedRegion,
    packed_rows: &[u8],
) -> Result<(), T::Error>
where
    T: DisplayTransport,
{
    debug_assert!(
        region.x_byte_end >= region.x_byte_start,
        "x_byte_end must be >= x_byte_start"
    );
    debug_assert!(region.y_end >= region.y_start, "y_end must be >= y_start");
    let row_bytes = (region.x_byte_end - region.x_byte_start + 1) as usize;
    let row_count = (region.y_end - region.y_start + 1) as usize;
    debug_assert_eq!(
        packed_rows.len(),
        row_bytes * row_count,
        "packed_rows length must equal region row_bytes * row_count"
    );
    transport.write_command(SET_RAM_X_RANGE)?;
    transport.write_data(&[region.x_byte_start, region.x_byte_end])?;
    transport.write_command(SET_RAM_Y_RANGE)?;
    transport.write_data(&[
        region.y_start as u8,
        (region.y_start >> 8) as u8,
        region.y_end as u8,
        (region.y_end >> 8) as u8,
    ])?;
    transport.write_command(SET_RAM_X_COUNTER)?;
    transport.write_data(&[region.x_byte_start])?;
    transport.write_command(SET_RAM_Y_COUNTER)?;
    transport.write_data(&[region.y_start as u8, (region.y_start >> 8) as u8])?;
    transport.write_command(WRITE_RAM_BW)?;
    transport.write_data(packed_rows)?;
    Ok(())
}

/// Fill the entire display with a constant byte value by writing all strips in order.
///
/// `fill`      — byte value to broadcast: `0xFF` = white, `0x00` = black.
/// `strip_buf` — caller-supplied scratch buffer; must be at least `STRIP_ROWS * ROW_BYTES` bytes.
pub fn clear_screen<T>(transport: &mut T, fill: u8, strip_buf: &mut [u8]) -> Result<(), T::Error>
where
    T: DisplayTransport,
{
    let geo = strip_geometry(PANEL_HEIGHT as usize);
    for strip_idx in 0..geo.strip_count {
        let row_start = (strip_idx * geo.rows_per_strip) as u16;
        let row_count = if strip_idx + 1 == geo.strip_count {
            geo.last_strip_rows
        } else {
            geo.rows_per_strip
        };
        let byte_count = row_count * ROW_BYTES;
        let buf = &mut strip_buf[..byte_count];
        buf.fill(fill);
        write_strip(transport, row_start, row_count as u16, buf)?;
    }
    Ok(())
}
