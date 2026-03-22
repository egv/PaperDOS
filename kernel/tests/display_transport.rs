use kernel::display::ssd1677::{
    emit_addressing_init_block, emit_power_init_block, emit_reset_preamble,
    emit_strip_window_and_cursor, AUTO_WRITE_BW_RAM,
    AUTO_WRITE_RED_RAM, BOOSTER_SOFT_START, BORDER_WAVEFORM, DATA_ENTRY_MODE, DEEP_SLEEP,
    DISPLAY_UPDATE_CTRL1, DISPLAY_UPDATE_CTRL2, DRIVER_OUTPUT_CONTROL, MASTER_ACTIVATION,
    PANEL_HEIGHT, PANEL_WIDTH, ROW_BYTES, SET_RAM_X_COUNTER, SET_RAM_X_RANGE, SET_RAM_Y_COUNTER,
    SET_RAM_Y_RANGE, SOFT_RESET, STRIP_BUFFER_BYTES, STRIP_COUNT, STRIP_ROWS, TEMP_SENSOR_CONTROL,
    WRITE_LUT, WRITE_RAM_BW, WRITE_RAM_RED, WRITE_TEMP, WRITE_VCOM,
};
use kernel::display::transport::DisplayTransport;

#[derive(Debug, Eq, PartialEq)]
enum RecordedOp {
    Reset,
    WaitWhileBusy,
    Command(u8),
    Data(Vec<u8>),
}

#[derive(Default)]
struct RecordingTransport {
    ops: Vec<RecordedOp>,
}

impl DisplayTransport for RecordingTransport {
    type Error = ();

    fn reset(&mut self) -> Result<(), Self::Error> {
        self.ops.push(RecordedOp::Reset);
        Ok(())
    }

    fn wait_while_busy(&mut self) -> Result<(), Self::Error> {
        self.ops.push(RecordedOp::WaitWhileBusy);
        Ok(())
    }

    fn write_command(&mut self, command: u8) -> Result<(), Self::Error> {
        self.ops.push(RecordedOp::Command(command));
        Ok(())
    }

    fn write_data(&mut self, data: &[u8]) -> Result<(), Self::Error> {
        self.ops.push(RecordedOp::Data(data.to_vec()));
        Ok(())
    }
}

#[test]
fn ssd1677_constants_display_transport() {
    assert_eq!(PANEL_WIDTH, 800);
    assert_eq!(PANEL_HEIGHT, 480);
    assert_eq!(ROW_BYTES, 100);
    assert_eq!(STRIP_BUFFER_BYTES, 4096);
    assert_eq!(STRIP_ROWS, 40);
    assert_eq!(STRIP_COUNT, 12);

    assert_eq!(SOFT_RESET, 0x12);
    assert_eq!(BOOSTER_SOFT_START, 0x0C);
    assert_eq!(DRIVER_OUTPUT_CONTROL, 0x01);
    assert_eq!(BORDER_WAVEFORM, 0x3C);
    assert_eq!(TEMP_SENSOR_CONTROL, 0x18);
    assert_eq!(DATA_ENTRY_MODE, 0x11);
    assert_eq!(SET_RAM_X_RANGE, 0x44);
    assert_eq!(SET_RAM_Y_RANGE, 0x45);
    assert_eq!(SET_RAM_X_COUNTER, 0x4E);
    assert_eq!(SET_RAM_Y_COUNTER, 0x4F);
    assert_eq!(WRITE_RAM_BW, 0x24);
    assert_eq!(WRITE_RAM_RED, 0x26);
    assert_eq!(AUTO_WRITE_BW_RAM, 0x46);
    assert_eq!(AUTO_WRITE_RED_RAM, 0x47);
    assert_eq!(DISPLAY_UPDATE_CTRL1, 0x21);
    assert_eq!(DISPLAY_UPDATE_CTRL2, 0x22);
    assert_eq!(MASTER_ACTIVATION, 0x20);
    assert_eq!(WRITE_LUT, 0x32);
    assert_eq!(WRITE_VCOM, 0x2C);
    assert_eq!(WRITE_TEMP, 0x1A);
    assert_eq!(DEEP_SLEEP, 0x10);
}

#[test]
fn display_transport_trait_display_transport() {
    let mut transport = RecordingTransport::default();

    transport.reset().unwrap();
    transport.wait_while_busy().unwrap();
    transport.write_command(SOFT_RESET).unwrap();
    transport.write_data(&[0xAA, 0x55]).unwrap();

    assert_eq!(
        transport.ops,
        vec![
            RecordedOp::Reset,
            RecordedOp::WaitWhileBusy,
            RecordedOp::Command(SOFT_RESET),
            RecordedOp::Data(vec![0xAA, 0x55]),
        ]
    );
}

#[test]
fn display_reset_preamble_display_transport() {
    let mut transport = RecordingTransport::default();

    emit_reset_preamble(&mut transport).unwrap();

    assert_eq!(
        transport.ops,
        vec![RecordedOp::Reset, RecordedOp::WaitWhileBusy,]
    );
}

#[test]
fn display_power_init_block_display_transport() {
    let mut transport = RecordingTransport::default();

    emit_power_init_block(&mut transport).unwrap();

    assert_eq!(
        transport.ops,
        vec![
            RecordedOp::Command(BOOSTER_SOFT_START),
            RecordedOp::Data(vec![0xAE, 0xC7, 0xC3, 0xC0, 0x40]),
            RecordedOp::Command(BORDER_WAVEFORM),
            RecordedOp::Data(vec![0x01]),
            RecordedOp::Command(WRITE_VCOM),
            RecordedOp::Data(vec![0x3C]),
            RecordedOp::Command(TEMP_SENSOR_CONTROL),
            RecordedOp::Data(vec![0x80]),
        ]
    );
}

#[test]
fn display_addressing_init_block_display_transport() {
    let mut transport = RecordingTransport::default();

    emit_addressing_init_block(&mut transport).unwrap();

    assert_eq!(
        transport.ops,
        vec![
            RecordedOp::Command(DRIVER_OUTPUT_CONTROL),
            RecordedOp::Data(vec![0xDF, 0x01, 0x02]),
            RecordedOp::Command(DATA_ENTRY_MODE),
            RecordedOp::Data(vec![0x01]),
            RecordedOp::Command(SET_RAM_X_RANGE),
            RecordedOp::Data(vec![0x00, 0x63]),
            RecordedOp::Command(SET_RAM_Y_RANGE),
            RecordedOp::Data(vec![0x00, 0x00, 0xDF, 0x01]),
        ]
    );
}

#[test]
fn strip_window_cursor_first_strip() {
    // Strip 0: rows 0..39.  Y window [0x0000, 0x0027], Y cursor 0x0000, X cursor 0x00.
    let mut transport = RecordingTransport::default();

    emit_strip_window_and_cursor(&mut transport, 0, 40).unwrap();

    assert_eq!(
        transport.ops,
        vec![
            RecordedOp::Command(SET_RAM_Y_RANGE),
            RecordedOp::Data(vec![0x00, 0x00, 0x27, 0x00]),
            RecordedOp::Command(SET_RAM_Y_COUNTER),
            RecordedOp::Data(vec![0x00, 0x00]),
            RecordedOp::Command(SET_RAM_X_COUNTER),
            RecordedOp::Data(vec![0x00]),
        ]
    );
}

#[test]
fn strip_window_cursor_mid_strip() {
    // Strip 1: rows 40..79.  Y window [0x0028, 0x004F], Y cursor 0x0028, X cursor 0x00.
    let mut transport = RecordingTransport::default();

    emit_strip_window_and_cursor(&mut transport, 40, 40).unwrap();

    assert_eq!(
        transport.ops,
        vec![
            RecordedOp::Command(SET_RAM_Y_RANGE),
            RecordedOp::Data(vec![0x28, 0x00, 0x4F, 0x00]),
            RecordedOp::Command(SET_RAM_Y_COUNTER),
            RecordedOp::Data(vec![0x28, 0x00]),
            RecordedOp::Command(SET_RAM_X_COUNTER),
            RecordedOp::Data(vec![0x00]),
        ]
    );
}
