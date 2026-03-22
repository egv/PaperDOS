mod common;

use common::{RecordedOp, RecordingTransport};
use kernel::display::driver::write_partial;
use kernel::display::refresh::{
    normalize_partial_region, trigger_full_refresh, NormalizedRegion, PartialRegion,
};
use kernel::display::ssd1677::{
    DISPLAY_UPDATE_CTRL2, FULL_UPDATE_SEQUENCE, MASTER_ACTIVATION, PANEL_HEIGHT, PANEL_WIDTH,
    ROW_BYTES, SET_RAM_X_COUNTER, SET_RAM_X_RANGE, SET_RAM_Y_COUNTER, SET_RAM_Y_RANGE,
    WRITE_RAM_BW,
};

#[test]
fn partial_region_normalize_full_panel() {
    // Full 800×480 panel normalizes to byte columns [0, 99] and rows [0, 479].
    let r = normalize_partial_region(PartialRegion {
        x: 0,
        y: 0,
        width: PANEL_WIDTH,
        height: PANEL_HEIGHT,
    })
    .unwrap();
    assert_eq!(r.x_byte_start, 0);
    assert_eq!(r.x_byte_end, (ROW_BYTES - 1) as u8);
    assert_eq!(r.y_start, 0);
    assert_eq!(r.y_end, PANEL_HEIGHT - 1);
}

#[test]
fn partial_region_normalize_inner_rect() {
    // x=8, y=10, w=16, h=5 → byte cols [1, 2], rows [10, 14].
    let r = normalize_partial_region(PartialRegion {
        x: 8,
        y: 10,
        width: 16,
        height: 5,
    })
    .unwrap();
    assert_eq!(r.x_byte_start, 1);
    assert_eq!(r.x_byte_end, 2);
    assert_eq!(r.y_start, 10);
    assert_eq!(r.y_end, 14);
}

#[test]
fn partial_region_normalize_clamps_right_edge() {
    // x=792, w=16 extends past panel width 800; clamped to byte col 99.
    let r = normalize_partial_region(PartialRegion {
        x: 792,
        y: 0,
        width: 16,
        height: 1,
    })
    .unwrap();
    assert_eq!(r.x_byte_end, (ROW_BYTES - 1) as u8);
}

#[test]
fn partial_region_normalize_clamps_bottom_edge() {
    // y=476, h=8 extends past panel height 480; clamped to row 479.
    let r = normalize_partial_region(PartialRegion {
        x: 0,
        y: 476,
        width: 8,
        height: 8,
    })
    .unwrap();
    assert_eq!(r.y_end, PANEL_HEIGHT - 1);
}

#[test]
fn partial_region_normalize_off_screen_returns_none() {
    // x=800 is entirely off-panel — must return None.
    assert!(normalize_partial_region(PartialRegion {
        x: PANEL_WIDTH,
        y: 0,
        width: 8,
        height: 1,
    })
    .is_none());
    // y=480 is entirely off-panel — must return None.
    assert!(normalize_partial_region(PartialRegion {
        x: 0,
        y: PANEL_HEIGHT,
        width: 8,
        height: 1,
    })
    .is_none());
}

#[test]
fn partial_region_normalize_degenerate_zero_size_returns_none() {
    assert!(normalize_partial_region(PartialRegion {
        x: 0,
        y: 0,
        width: 0,
        height: 1,
    })
    .is_none());
    assert!(normalize_partial_region(PartialRegion {
        x: 0,
        y: 0,
        width: 8,
        height: 0,
    })
    .is_none());
}

#[test]
fn partial_region_normalize_aligns_x_to_byte_boundary() {
    // x=4, w=4 → pixel span [4,7] fits in byte column 0.
    let r = normalize_partial_region(PartialRegion {
        x: 4,
        y: 0,
        width: 4,
        height: 1,
    })
    .unwrap();
    assert_eq!(r.x_byte_start, 0);
    assert_eq!(r.x_byte_end, 0);
}

#[test]
fn partial_window_write_emits_x_y_window_cursor_then_data() {
    // Region: byte-cols [1,2], rows [10,11] → 2 rows × 2 byte-cols = 4 bytes payload.
    // Expected: SET_RAM_X_RANGE, SET_RAM_Y_RANGE, SET_RAM_X_COUNTER, SET_RAM_Y_COUNTER,
    //           WRITE_RAM_BW + payload.
    let region = NormalizedRegion {
        x_byte_start: 1,
        x_byte_end: 2,
        y_start: 10,
        y_end: 11,
    };
    let payload = [0xAAu8, 0xBB, 0xCC, 0xDD];
    let mut transport = RecordingTransport::default();

    write_partial(&mut transport, &region, &payload).unwrap();

    assert_eq!(
        transport.ops,
        vec![
            RecordedOp::Command(SET_RAM_X_RANGE),
            RecordedOp::Data(vec![1, 2]),
            RecordedOp::Command(SET_RAM_Y_RANGE),
            RecordedOp::Data(vec![10, 0, 11, 0]),
            RecordedOp::Command(SET_RAM_X_COUNTER),
            RecordedOp::Data(vec![1]),
            RecordedOp::Command(SET_RAM_Y_COUNTER),
            RecordedOp::Data(vec![10, 0]),
            RecordedOp::Command(WRITE_RAM_BW),
            RecordedOp::Data(vec![0xAA, 0xBB, 0xCC, 0xDD]),
        ]
    );
}

#[test]
fn partial_window_write_encodes_high_row_y_address() {
    // y_start=300 (0x012C), y_end=301 (0x012D) — both > 255, verifying the high byte
    // of the 16-bit little-endian Y address is encoded correctly.
    // 300 = 0x012C → [0x2C, 0x01]; 301 = 0x012D → [0x2D, 0x01]
    let region = NormalizedRegion {
        x_byte_start: 0,
        x_byte_end: 0,
        y_start: 300,
        y_end: 301,
    };
    let payload = [0xFFu8, 0xFF];
    let mut transport = RecordingTransport::default();

    write_partial(&mut transport, &region, &payload).unwrap();

    assert_eq!(transport.ops[3], RecordedOp::Data(vec![0x2C, 0x01, 0x2D, 0x01])); // SET_RAM_Y_RANGE
    assert_eq!(transport.ops[7], RecordedOp::Data(vec![0x2C, 0x01])); // SET_RAM_Y_COUNTER
}

#[test]
fn partial_du_trigger_emits_update_ctrl2_with_du_sequence_then_busy_wait() {
    use kernel::display::refresh::trigger_partial_refresh;
    use kernel::display::ssd1677::PARTIAL_UPDATE_SEQUENCE;
    let mut transport = RecordingTransport::default();

    trigger_partial_refresh(&mut transport).unwrap();

    assert_eq!(
        transport.ops,
        vec![
            RecordedOp::Command(DISPLAY_UPDATE_CTRL2),
            RecordedOp::Data(vec![PARTIAL_UPDATE_SEQUENCE]),
            RecordedOp::Command(MASTER_ACTIVATION),
            RecordedOp::WaitWhileBusy,
        ]
    );
}

#[test]
fn partial_refresh_promotion_returns_partial_below_threshold() {
    use kernel::display::refresh::PartialRefreshCounter;
    let mut counter = PartialRefreshCounter::new(4);
    assert!(!counter.record_partial()); // 1 → partial
    assert!(!counter.record_partial()); // 2 → partial
    assert!(!counter.record_partial()); // 3 → partial
}

#[test]
fn partial_refresh_promotion_returns_full_at_threshold_and_resets() {
    use kernel::display::refresh::PartialRefreshCounter;
    let mut counter = PartialRefreshCounter::new(4);
    counter.record_partial();
    counter.record_partial();
    counter.record_partial();
    assert!(counter.record_partial()); // 4th → promote to full, counter resets
    assert!(!counter.record_partial()); // 1 again → partial
}

#[test]
fn partial_refresh_promotion_reset_clears_count() {
    use kernel::display::refresh::PartialRefreshCounter;
    let mut counter = PartialRefreshCounter::new(4);
    counter.record_partial();
    counter.record_partial();
    counter.reset();
    assert!(!counter.record_partial()); // count was cleared, back to 1
    assert!(!counter.record_partial()); // 2 → partial
}

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
