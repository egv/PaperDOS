use crate::display::render::strip_geometry;
use crate::display::ssd1677::{emit_strip_window_and_cursor, PANEL_HEIGHT, ROW_BYTES, WRITE_RAM_BW};
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
    emit_strip_window_and_cursor(transport, row_start, row_count)?;
    transport.write_command(WRITE_RAM_BW)?;
    transport.write_data(packed_rows)?;
    Ok(())
}

/// Fill the entire display with a constant byte value by writing all strips in order.
///
/// `fill`      — byte value to broadcast: `0xFF` = white, `0x00` = black.
/// `strip_buf` — caller-supplied scratch buffer; must be at least `STRIP_BUFFER_BYTES` bytes.
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
