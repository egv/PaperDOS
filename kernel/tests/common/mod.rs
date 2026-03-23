use kernel::display::transport::DisplayTransport;
use kernel::input::adc::AdcSource;

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

/// Test double: replays scripted ADC readings for each channel.
///
/// When the scripted sequence is exhausted, repeats the last value indefinitely.
/// Both slices must be non-empty.
pub struct ScriptedAdc<'a> {
    gpio1: &'a [u16],
    gpio2: &'a [u16],
    idx1: usize,
    idx2: usize,
}

impl<'a> ScriptedAdc<'a> {
    pub fn new(gpio1: &'a [u16], gpio2: &'a [u16]) -> Self {
        debug_assert!(!gpio1.is_empty(), "ScriptedAdc: gpio1 slice must not be empty");
        debug_assert!(!gpio2.is_empty(), "ScriptedAdc: gpio2 slice must not be empty");
        Self { gpio1, gpio2, idx1: 0, idx2: 0 }
    }
}

impl<'a> AdcSource for ScriptedAdc<'a> {
    type Error = core::convert::Infallible;

    fn read_gpio1(&mut self) -> Result<u16, Self::Error> {
        let val = self.gpio1[self.idx1];
        if self.idx1 + 1 < self.gpio1.len() {
            self.idx1 += 1;
        }
        Ok(val)
    }

    fn read_gpio2(&mut self) -> Result<u16, Self::Error> {
        let val = self.gpio2[self.idx2];
        if self.idx2 + 1 < self.gpio2.len() {
            self.idx2 += 1;
        }
        Ok(val)
    }
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
