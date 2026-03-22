use kernel::display::refresh::trigger_full_refresh;
use kernel::display::ssd1677::{DISPLAY_UPDATE_CTRL2, MASTER_ACTIVATION};
use kernel::display::transport::DisplayTransport;

#[derive(Debug, Eq, PartialEq)]
enum RecordedOp {
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

    fn reset(&mut self) -> Result<(), ()> {
        Ok(())
    }
    fn wait_while_busy(&mut self) -> Result<(), ()> {
        self.ops.push(RecordedOp::WaitWhileBusy);
        Ok(())
    }
    fn write_command(&mut self, cmd: u8) -> Result<(), ()> {
        self.ops.push(RecordedOp::Command(cmd));
        Ok(())
    }
    fn write_data(&mut self, data: &[u8]) -> Result<(), ()> {
        self.ops.push(RecordedOp::Data(data.to_vec()));
        Ok(())
    }
}

#[test]
fn full_refresh_trigger_emits_update_ctrl2_activation_then_busy_wait() {
    let mut transport = RecordingTransport::default();

    trigger_full_refresh(&mut transport).unwrap();

    assert_eq!(
        transport.ops,
        vec![
            RecordedOp::Command(DISPLAY_UPDATE_CTRL2),
            RecordedOp::Data(vec![0xF7]),
            RecordedOp::Command(MASTER_ACTIVATION),
            RecordedOp::WaitWhileBusy,
        ]
    );
}
