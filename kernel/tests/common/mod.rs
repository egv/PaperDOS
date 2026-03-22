use kernel::display::transport::DisplayTransport;

#[derive(Debug, Eq, PartialEq)]
pub enum RecordedOp {
    Reset,
    WaitWhileBusy,
    Command(u8),
    Data(Vec<u8>),
}

#[derive(Default)]
pub struct RecordingTransport {
    pub ops: Vec<RecordedOp>,
}

impl DisplayTransport for RecordingTransport {
    type Error = ();

    fn reset(&mut self) -> Result<(), ()> {
        self.ops.push(RecordedOp::Reset);
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
