use crate::display::transport::DisplayTransport;

/// Pixel width of the panel.
pub const PANEL_WIDTH: u16 = 800;
/// Pixel height of the panel.
pub const PANEL_HEIGHT: u16 = 480;
/// Bytes per display row (`PANEL_WIDTH / 8` — one bit per pixel).
pub const ROW_BYTES: usize = PANEL_WIDTH as usize / 8;
/// Maximum payload size in bytes for a single strip transfer (4 KB).
pub const STRIP_BUFFER_BYTES: usize = 4096;
/// Number of pixel rows that fit in one strip buffer (`STRIP_BUFFER_BYTES / ROW_BYTES`).
pub const STRIP_ROWS: usize = STRIP_BUFFER_BYTES / ROW_BYTES;
/// Number of strips required to cover the full panel height (ceiling division).
pub const STRIP_COUNT: usize = (PANEL_HEIGHT as usize + STRIP_ROWS - 1) / STRIP_ROWS;

/// SSD1677 command: set gate driver output count and scan order.
pub const DRIVER_OUTPUT_CONTROL: u8 = 0x01;
/// SSD1677 command: configure booster soft-start phases.
pub const BOOSTER_SOFT_START: u8 = 0x0C;
/// SSD1677 command: enter deep-sleep mode.
pub const DEEP_SLEEP: u8 = 0x10;
/// SSD1677 command: set RAM data-entry mode (X/Y increment direction).
pub const DATA_ENTRY_MODE: u8 = 0x11;
/// SSD1677 command: software reset — returns registers to factory defaults.
pub const SOFT_RESET: u8 = 0x12;
/// SSD1677 command: select temperature sensor source.
pub const TEMP_SENSOR_CONTROL: u8 = 0x18;
/// SSD1677 command: write a temperature value directly.
pub const WRITE_TEMP: u8 = 0x1A;
/// SSD1677 command: activate display update sequence.
pub const MASTER_ACTIVATION: u8 = 0x20;
/// SSD1677 command: display update control register 1.
pub const DISPLAY_UPDATE_CTRL1: u8 = 0x21;
/// SSD1677 command: display update control register 2 (sequence flags).
pub const DISPLAY_UPDATE_CTRL2: u8 = 0x22;
/// SSD1677 command: stream pixel data into the black/white RAM.
pub const WRITE_RAM_BW: u8 = 0x24;
/// SSD1677 command: stream pixel data into the red RAM.
pub const WRITE_RAM_RED: u8 = 0x26;
/// SSD1677 command: set VCOM voltage.
pub const WRITE_VCOM: u8 = 0x2C;
/// SSD1677 command: upload a custom waveform LUT.
pub const WRITE_LUT: u8 = 0x32;
/// SSD1677 command: configure border waveform.
pub const BORDER_WAVEFORM: u8 = 0x3C;
/// SSD1677 command: set X RAM address window (pixel units, 4-byte little-endian payload).
pub const SET_RAM_X_RANGE: u8 = 0x44;
/// SSD1677 command: set Y RAM address window (row units, 4-byte little-endian payload).
pub const SET_RAM_Y_RANGE: u8 = 0x45;
/// SSD1677 command: auto-fill black/white RAM with a fixed value.
pub const AUTO_WRITE_BW_RAM: u8 = 0x46;
/// SSD1677 command: auto-fill red RAM with a fixed value.
pub const AUTO_WRITE_RED_RAM: u8 = 0x47;
/// SSD1677 command: set X RAM address counter (pixel units, 2-byte little-endian).
pub const SET_RAM_X_COUNTER: u8 = 0x4E;
/// SSD1677 command: set Y RAM address counter (row units, 2-byte little-endian).
pub const SET_RAM_Y_COUNTER: u8 = 0x4F;
/// `DISPLAY_UPDATE_CTRL2` flag byte that selects the full-panel waveform update sequence.
///
/// Bit field (SSD1677 datasheet §7.2.17):
/// 7=enable clock, 6=enable analog, 5=load temperature, 4=load LUT,
/// 3=display update, 2=disable analog, 1=disable clock, 0=reserved → `0b1111_0111`.
pub const FULL_UPDATE_SEQUENCE: u8 = 0xF7;
/// `DISPLAY_UPDATE_CTRL2` flag byte that selects the DU (Direct Update) partial-refresh sequence.
///
/// Same as [`FULL_UPDATE_SEQUENCE`] but with temperature and LUT reload (bits 5 and 4) cleared,
/// so the previously loaded waveform LUT is reused without reloading → `0b1100_0111`.
pub const PARTIAL_UPDATE_SEQUENCE: u8 = 0xC7;

/// Assert hardware reset, issue a software reset, and wait 10 ms for the
/// controller to complete its internal startup sequence.
///
/// The SSD1677 BUSY pin goes HIGH immediately after the hardware reset pulse
/// (the controller is initialising), so polling BUSY here would time-out.
/// The pulp-os reference firmware confirms the correct sequence:
/// hardware reset → SW_RESET command (0x12) → fixed 10 ms delay.
pub fn emit_reset_preamble<T>(transport: &mut T) -> Result<(), T::Error>
where
    T: DisplayTransport,
{
    transport.reset()?;
    transport.write_command(SOFT_RESET)?;
    transport.delay_ms(10);
    Ok(())
}

/// Write the booster, border, VCOM, and temperature-sensor configuration registers.
pub fn emit_power_init_block<T>(transport: &mut T) -> Result<(), T::Error>
where
    T: DisplayTransport,
{
    write_command_with_data(
        transport,
        BOOSTER_SOFT_START,
        &[0xAE, 0xC7, 0xC3, 0xC0, 0x80],
    )?;
    write_command_with_data(transport, BORDER_WAVEFORM, &[0x01])?;
    write_command_with_data(transport, WRITE_VCOM, &[0x3C])?;
    write_command_with_data(transport, TEMP_SENSOR_CONTROL, &[0x80])?;
    Ok(())
}

/// Write gate-driver, data-entry-mode, and RAM address-window registers.
///
/// Sets the X window as pixel addresses `[0, PANEL_WIDTH-1]` and the Y window
/// as row addresses `[PANEL_HEIGHT-1, 0]` in little-endian 16-bit format.
pub fn emit_addressing_init_block<T>(transport: &mut T) -> Result<(), T::Error>
where
    T: DisplayTransport,
{
    write_command_with_data(
        transport,
        DRIVER_OUTPUT_CONTROL,
        &[
            (PANEL_HEIGHT - 1) as u8,
            ((PANEL_HEIGHT - 1) >> 8) as u8,
            0x02,
        ],
    )?;
    // DATA_ENTRY_MODE 0x01: Y-decrement, X-increment.
    // On the X4 gate 0 is at the physical bottom; gate PANEL_HEIGHT-1 is at
    // the top.  With Y-decrement the counter starts at the top gate and walks
    // toward the bottom, so logical row 0 maps to the top of the screen.
    write_command_with_data(transport, DATA_ENTRY_MODE, &[0x01])?;
    // X range: pixel addresses 0x0000 to 0x031F (0–799) covering all 800 pixels.
    write_command_with_data(
        transport,
        SET_RAM_X_RANGE,
        &[
            0x00,
            0x00,
            (PANEL_WIDTH - 1) as u8,
            ((PANEL_WIDTH - 1) >> 8) as u8,
        ],
    )?;
    // Y range: gate PANEL_HEIGHT-1 (top) down to gate 0 (bottom).
    write_command_with_data(
        transport,
        SET_RAM_Y_RANGE,
        &[
            (PANEL_HEIGHT - 1) as u8,
            ((PANEL_HEIGHT - 1) >> 8) as u8,
            0x00,
            0x00,
        ],
    )?;
    Ok(())
}

/// Program a byte-aligned RAM window and reset the SSD1677 address counters.
///
/// This mirrors the working pulp-os `set_partial_ram_area()` sequence.
pub fn emit_window_and_cursor<T>(
    transport: &mut T,
    x: u16,
    y: u16,
    width: u16,
    height: u16,
) -> Result<(), T::Error>
where
    T: DisplayTransport,
{
    debug_assert!(width > 0, "width must be at least 1");
    debug_assert!(height > 0, "height must be at least 1");
    debug_assert!(x % 8 == 0, "x must be byte aligned");
    debug_assert!(width % 8 == 0, "width must be byte aligned");

    let x_end = x + width - 1;
    let y_flipped = PANEL_HEIGHT - y - height;
    let y_top = y_flipped + height - 1;

    write_command_with_data(transport, DATA_ENTRY_MODE, &[0x01])?;
    write_command_with_data(
        transport,
        SET_RAM_X_RANGE,
        &[x as u8, (x >> 8) as u8, x_end as u8, (x_end >> 8) as u8],
    )?;
    write_command_with_data(
        transport,
        SET_RAM_Y_RANGE,
        &[
            y_top as u8,
            (y_top >> 8) as u8,
            y_flipped as u8,
            (y_flipped >> 8) as u8,
        ],
    )?;
    write_command_with_data(transport, SET_RAM_X_COUNTER, &[x as u8, (x >> 8) as u8])?;
    write_command_with_data(
        transport,
        SET_RAM_Y_COUNTER,
        &[y_top as u8, (y_top >> 8) as u8],
    )?;
    Ok(())
}

/// Program the full-panel RAM window and reset the address counters.
pub fn emit_full_window_and_cursor<T>(transport: &mut T) -> Result<(), T::Error>
where
    T: DisplayTransport,
{
    emit_window_and_cursor(transport, 0, 0, PANEL_WIDTH, PANEL_HEIGHT)
}

/// Set the Y RAM window, set the Y address cursor to `row_start`, and reset
/// the X cursor.
///
/// `row_start` — first row of the strip (0 = top of the image / top of screen).
/// `row_count` — number of rows in the strip; must be ≥ 1.
///
/// Gate 0 is at the physical top of the X4 panel.  `DATA_ENTRY_MODE = 0x03`
/// (Y-increment, X-increment) means logical row N maps directly to gate N —
/// no coordinate transformation is needed.
///
/// Issues `SET_RAM_Y_RANGE`, `SET_RAM_Y_COUNTER`, and `SET_RAM_X_COUNTER` in
/// that order, leaving the controller ready to accept strip pixel data.
pub fn emit_strip_window_and_cursor<T>(
    transport: &mut T,
    row_start: u16,
    row_count: u16,
) -> Result<(), T::Error>
where
    T: DisplayTransport,
{
    debug_assert!(row_count > 0, "row_count must be at least 1");
    emit_window_and_cursor(transport, 0, row_start, PANEL_WIDTH, row_count)
}

fn write_command_with_data<T>(transport: &mut T, command: u8, data: &[u8]) -> Result<(), T::Error>
where
    T: DisplayTransport,
{
    transport.write_command(command)?;
    transport.write_data(data)?;
    Ok(())
}
