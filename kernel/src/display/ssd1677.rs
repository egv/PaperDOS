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
/// SSD1677 command: set X RAM address window (byte-column units, 2-byte payload).
pub const SET_RAM_X_RANGE: u8 = 0x44;
/// SSD1677 command: set Y RAM address window (row units, 4-byte little-endian payload).
pub const SET_RAM_Y_RANGE: u8 = 0x45;
/// SSD1677 command: auto-fill black/white RAM with a fixed value.
pub const AUTO_WRITE_BW_RAM: u8 = 0x46;
/// SSD1677 command: auto-fill red RAM with a fixed value.
pub const AUTO_WRITE_RED_RAM: u8 = 0x47;
/// SSD1677 command: set X RAM address counter (byte-column units).
pub const SET_RAM_X_COUNTER: u8 = 0x4E;
/// SSD1677 command: set Y RAM address counter (row units, 2-byte little-endian).
pub const SET_RAM_Y_COUNTER: u8 = 0x4F;

/// Assert hardware reset and wait for the controller to become ready.
pub fn emit_reset_preamble<T>(transport: &mut T) -> Result<(), T::Error>
where
    T: DisplayTransport,
{
    transport.reset()?;
    transport.wait_while_busy()?;
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
        &[0xAE, 0xC7, 0xC3, 0xC0, 0x40],
    )?;
    write_command_with_data(transport, BORDER_WAVEFORM, &[0x01])?;
    write_command_with_data(transport, WRITE_VCOM, &[0x3C])?;
    write_command_with_data(transport, TEMP_SENSOR_CONTROL, &[0x80])?;
    Ok(())
}

/// Write gate-driver, data-entry-mode, and RAM address-window registers.
///
/// Sets the X window as byte-column addresses `[0, ROW_BYTES-1]` and the Y window
/// as row addresses `[0, PANEL_HEIGHT-1]` in little-endian 16-bit format.
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
    write_command_with_data(transport, DATA_ENTRY_MODE, &[0x01])?;
    // X range: 2-byte payload — one byte-column address per field (0x00–0x63 for 800 px).
    write_command_with_data(
        transport,
        SET_RAM_X_RANGE,
        &[0x00, (ROW_BYTES - 1) as u8],
    )?;
    write_command_with_data(
        transport,
        SET_RAM_Y_RANGE,
        &[
            0x00,
            0x00,
            (PANEL_HEIGHT - 1) as u8,
            ((PANEL_HEIGHT - 1) >> 8) as u8,
        ],
    )?;
    Ok(())
}

fn write_command_with_data<T>(transport: &mut T, command: u8, data: &[u8]) -> Result<(), T::Error>
where
    T: DisplayTransport,
{
    transport.write_command(command)?;
    transport.write_data(data)?;
    Ok(())
}
