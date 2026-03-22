mod common;

use common::{RecordedOp, RecordingTransport};
use kernel::display::refresh::trigger_full_refresh;
use kernel::display::ssd1677::{DISPLAY_UPDATE_CTRL2, FULL_UPDATE_SEQUENCE, MASTER_ACTIVATION};

#[test]
fn full_refresh_trigger_emits_update_ctrl2_activation_then_busy_wait() {
    let mut transport = RecordingTransport::default();

    trigger_full_refresh(&mut transport).unwrap();

    assert_eq!(
        transport.ops,
        vec![
            RecordedOp::Command(DISPLAY_UPDATE_CTRL2),
            RecordedOp::Data(vec![FULL_UPDATE_SEQUENCE]),
            RecordedOp::Command(MASTER_ACTIVATION),
            RecordedOp::WaitWhileBusy,
        ]
    );
}
