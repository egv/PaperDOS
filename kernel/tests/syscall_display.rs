mod common;

use common::{RecordedOp, RecordingTransport};
use kernel::abi::{PD_SCREEN_HEIGHT, PD_SCREEN_WIDTH};
use kernel::display::ssd1677::{STRIP_COUNT, WRITE_RAM_BW};
use kernel::syscall::build_syscall_table;
use kernel::syscall::display::{
    FRAME_BYTES, display_clear_to, display_refresh_flush, pd_display_clear, pd_display_height,
    pd_display_refresh, pd_display_width,
};

#[test]
fn display_clear_fills_frame_buffer_syscall_display() {
    let mut buf = vec![0u8; FRAME_BYTES];
    display_clear_to(buf.as_mut_slice().try_into().unwrap(), 0xFF);
    assert!(buf.iter().all(|&b| b == 0xFF), "buffer must be all 0xFF after clear");
}

#[test]
fn display_refresh_sends_strip_sequence_syscall_display() {
    let buf = vec![0xFFu8; FRAME_BYTES];
    let buf_ref: &[u8; FRAME_BYTES] = buf.as_slice().try_into().unwrap();
    let mut transport = RecordingTransport::default();
    display_refresh_flush(&mut transport, buf_ref).unwrap();
    let write_ram_count = transport
        .ops
        .iter()
        .filter(|op| **op == RecordedOp::Command(WRITE_RAM_BW))
        .count();
    assert_eq!(write_ram_count, STRIP_COUNT, "must emit one WRITE_RAM_BW per strip");
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
