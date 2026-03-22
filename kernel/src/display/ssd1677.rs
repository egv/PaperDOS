use crate::display::transport::DisplayTransport;

pub const PANEL_WIDTH: u16 = 800;
pub const PANEL_HEIGHT: u16 = 480;
pub const ROW_BYTES: usize = PANEL_WIDTH as usize / 8;
pub const STRIP_BUFFER_BYTES: usize = 4096;
pub const STRIP_ROWS: usize = STRIP_BUFFER_BYTES / ROW_BYTES;
pub const STRIP_COUNT: usize = PANEL_HEIGHT as usize / STRIP_ROWS;

pub const DRIVER_OUTPUT_CONTROL: u8 = 0x01;
pub const BOOSTER_SOFT_START: u8 = 0x0C;
pub const DEEP_SLEEP: u8 = 0x10;
pub const DATA_ENTRY_MODE: u8 = 0x11;
pub const SOFT_RESET: u8 = 0x12;
pub const TEMP_SENSOR_CONTROL: u8 = 0x18;
pub const WRITE_TEMP: u8 = 0x1A;
pub const MASTER_ACTIVATION: u8 = 0x20;
pub const DISPLAY_UPDATE_CTRL1: u8 = 0x21;
pub const DISPLAY_UPDATE_CTRL2: u8 = 0x22;
pub const WRITE_RAM_BW: u8 = 0x24;
pub const WRITE_RAM_RED: u8 = 0x26;
pub const WRITE_VCOM: u8 = 0x2C;
pub const WRITE_LUT: u8 = 0x32;
pub const BORDER_WAVEFORM: u8 = 0x3C;
pub const SET_RAM_X_RANGE: u8 = 0x44;
pub const SET_RAM_Y_RANGE: u8 = 0x45;
pub const AUTO_WRITE_BW_RAM: u8 = 0x46;
pub const AUTO_WRITE_RED_RAM: u8 = 0x47;
pub const SET_RAM_X_COUNTER: u8 = 0x4E;
pub const SET_RAM_Y_COUNTER: u8 = 0x4F;

pub fn emit_reset_preamble<T>(transport: &mut T) -> Result<(), T::Error>
where
    T: DisplayTransport,
{
    transport.reset()?;
    transport.wait_while_busy()?;
    Ok(())
}

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
