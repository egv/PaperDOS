mod common;

use common::{RecordedOp, RecordingTransport};
use kernel::abi::{PD_SCREEN_HEIGHT, PD_SCREEN_WIDTH};
use kernel::display::ssd1677::{WRITE_RAM_BW, WRITE_RAM_RED};
use kernel::syscall::build_syscall_table;
use kernel::syscall::display::{
    display_clear_to, display_refresh_flush, pd_display_clear, pd_display_height,
    pd_display_refresh, pd_display_width, FRAME_BYTES,
};

#[test]
fn display_clear_fills_frame_buffer_syscall_display() {
    let mut buf = vec![0u8; FRAME_BYTES];
    display_clear_to(buf.as_mut_slice().try_into().unwrap(), 0xFF);
    assert!(
        buf.iter().all(|&b| b == 0xFF),
        "buffer must be all 0xFF after clear"
    );
}

#[test]
fn display_refresh_sends_strip_sequence_syscall_display() {
    use kernel::display::ssd1677::PANEL_HEIGHT;
    use kernel::display::ssd1677::{
        DATA_ENTRY_MODE, DISPLAY_UPDATE_CTRL1, DISPLAY_UPDATE_CTRL2, FULL_UPDATE_SEQUENCE,
        MASTER_ACTIVATION, SET_RAM_X_COUNTER, SET_RAM_X_RANGE, SET_RAM_Y_COUNTER, SET_RAM_Y_RANGE,
    };

    let buf = vec![0xFFu8; FRAME_BYTES];
    let buf_ref: &[u8; FRAME_BYTES] = buf.as_slice().try_into().unwrap();
    let mut transport = RecordingTransport::default();
    display_refresh_flush(&mut transport, buf_ref).unwrap();

    let write_red_count = transport
        .ops
        .iter()
        .filter(|op| **op == RecordedOp::Command(WRITE_RAM_RED))
        .count();
    let write_bw_count = transport
        .ops
        .iter()
        .filter(|op| **op == RecordedOp::Command(WRITE_RAM_BW))
        .count();
    assert_eq!(
        write_red_count, 1,
        "must emit exactly one WRITE_RAM_RED for previous frame"
    );
    assert_eq!(
        write_bw_count, 1,
        "must emit exactly one WRITE_RAM_BW for current frame"
    );

    // First plane setup matches pulp-os full-frame rendering.
    assert_eq!(transport.ops[0], RecordedOp::Command(DATA_ENTRY_MODE));
    assert_eq!(transport.ops[1], RecordedOp::Data(vec![0x01]));
    assert_eq!(transport.ops[2], RecordedOp::Command(SET_RAM_X_RANGE));
    assert_eq!(
        transport.ops[3],
        RecordedOp::Data(vec![0x00, 0x00, 0x1F, 0x03])
    );
    assert_eq!(transport.ops[4], RecordedOp::Command(SET_RAM_Y_RANGE));
    assert_eq!(
        transport.ops[5],
        RecordedOp::Data(vec![
            (PANEL_HEIGHT - 1) as u8,
            ((PANEL_HEIGHT - 1) >> 8) as u8,
            0x00,
            0x00
        ])
    );
    assert_eq!(transport.ops[6], RecordedOp::Command(SET_RAM_X_COUNTER));
    assert_eq!(transport.ops[7], RecordedOp::Data(vec![0x00, 0x00]));
    assert_eq!(transport.ops[8], RecordedOp::Command(SET_RAM_Y_COUNTER));
    assert_eq!(
        transport.ops[9],
        RecordedOp::Data(vec![
            (PANEL_HEIGHT - 1) as u8,
            ((PANEL_HEIGHT - 1) >> 8) as u8
        ])
    );
    assert_eq!(transport.ops[10], RecordedOp::Command(WRITE_RAM_RED));

    let red_plane = &transport.ops[11..23];
    assert_eq!(red_plane.len(), 12);
    for op in red_plane {
        let RecordedOp::Data(chunk) = op else {
            panic!("red plane payload must be data-only strip writes");
        };
        assert_eq!(chunk.len(), 40 * 100);
        assert!(chunk.iter().all(|&b| b == 0xFF));
    }

    assert_eq!(transport.ops[23], RecordedOp::Command(DATA_ENTRY_MODE));
    assert_eq!(transport.ops[24], RecordedOp::Data(vec![0x01]));
    assert_eq!(transport.ops[25], RecordedOp::Command(SET_RAM_X_RANGE));
    assert_eq!(
        transport.ops[26],
        RecordedOp::Data(vec![0x00, 0x00, 0x1F, 0x03])
    );
    assert_eq!(transport.ops[27], RecordedOp::Command(SET_RAM_Y_RANGE));
    assert_eq!(
        transport.ops[28],
        RecordedOp::Data(vec![
            (PANEL_HEIGHT - 1) as u8,
            ((PANEL_HEIGHT - 1) >> 8) as u8,
            0x00,
            0x00
        ])
    );
    assert_eq!(transport.ops[29], RecordedOp::Command(SET_RAM_X_COUNTER));
    assert_eq!(transport.ops[30], RecordedOp::Data(vec![0x00, 0x00]));
    assert_eq!(transport.ops[31], RecordedOp::Command(SET_RAM_Y_COUNTER));
    assert_eq!(
        transport.ops[32],
        RecordedOp::Data(vec![
            (PANEL_HEIGHT - 1) as u8,
            ((PANEL_HEIGHT - 1) >> 8) as u8
        ])
    );
    assert_eq!(transport.ops[33], RecordedOp::Command(WRITE_RAM_BW));

    let bw_plane = &transport.ops[34..46];
    assert_eq!(bw_plane.len(), 12);
    for op in bw_plane {
        let RecordedOp::Data(chunk) = op else {
            panic!("bw plane payload must be data-only strip writes");
        };
        assert_eq!(chunk.len(), 40 * 100);
        assert!(chunk.iter().all(|&b| b == 0xFF));
    }

    assert_eq!(transport.ops[46], RecordedOp::Command(DISPLAY_UPDATE_CTRL1));
    assert_eq!(transport.ops[47], RecordedOp::Data(vec![0x40, 0x00]));
    assert_eq!(transport.ops[48], RecordedOp::Command(DISPLAY_UPDATE_CTRL2));
    assert_eq!(
        transport.ops[49],
        RecordedOp::Data(vec![FULL_UPDATE_SEQUENCE])
    );
    assert_eq!(transport.ops[50], RecordedOp::Command(MASTER_ACTIVATION));
    assert_eq!(transport.ops[51], RecordedOp::WaitWhileBusy);
}

#[test]
fn display_width_returns_panel_width_syscall_display() {
    assert_eq!(pd_display_width(), PD_SCREEN_WIDTH);
}

#[test]
fn display_height_returns_panel_height_syscall_display() {
    assert_eq!(pd_display_height(), PD_SCREEN_HEIGHT);
}

#[test]
fn syscall_table_display_fields_populated_syscall_display() {
    let t = build_syscall_table(0, 0);
    assert_eq!(t.display_clear, pd_display_clear as usize as u32);
    assert_eq!(t.display_refresh, pd_display_refresh as usize as u32);
    assert_eq!(t.display_width, pd_display_width as usize as u32);
    assert_eq!(t.display_height, pd_display_height as usize as u32);
}
