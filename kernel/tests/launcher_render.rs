// F3: render_launcher produces a correct pixel layout in the framebuffer.
//
// Layout contract:
//  - Background is white (0xFF bytes).
//  - The selected row is highlighted with a filled black bar:
//      y = selected * LAUNCHER_ROW_HEIGHT, height = LAUNCHER_ROW_HEIGHT.
//  - With zero apps, the buffer is entirely white.

use kernel::abi::PD_SCREEN_WIDTH;
use kernel::display::ssd1677::{PANEL_HEIGHT, ROW_BYTES};
use kernel::launcher::{render_launcher, AppInfo, LauncherState, LAUNCHER_ROW_HEIGHT, MAX_APPS};
use kernel::syscall::display::{FrameBuffer, FRAME_BYTES};

/// Allocate a zeroed FrameBuffer on the heap to avoid stack overflow (48 KB).
fn zero_buf() -> Box<FrameBuffer> {
    vec![0u8; FRAME_BYTES]
        .into_boxed_slice()
        .try_into()
        .unwrap()
}

fn logical_pixel_is(buf: &FrameBuffer, x: usize, y: usize, expected: u8) -> bool {
    let px = y;
    let py = PANEL_HEIGHT as usize - 1 - x;
    let byte_idx = py * ROW_BYTES + px / 8;
    let bit_mask = 0x80u8 >> (px % 8);
    let on = buf[byte_idx] & bit_mask != 0;
    on == (expected != 0)
}

fn logical_row_is(buf: &FrameBuffer, y: usize, expected: u8) -> bool {
    (0..PD_SCREEN_WIDTH as usize).all(|x| logical_pixel_is(buf, x, y, expected))
}

fn logical_row_sample(buf: &FrameBuffer, y: usize) -> Vec<bool> {
    (0..PD_SCREEN_WIDTH as usize)
        .map(|x| logical_pixel_is(buf, x, y, 0xFF))
        .collect()
}

#[test]
fn render_launcher_zero_apps_all_white_launcher_render() {
    let mut buf = zero_buf();
    let state = LauncherState::new(0);
    render_launcher(&mut *buf, &[], &state);
    assert!(
        buf.iter().all(|&b| b == 0xFF),
        "zero apps → entire buffer must be white"
    );
}

#[test]
fn render_launcher_one_app_selected_row_is_black_launcher_render() {
    let mut buf = zero_buf();
    let apps = [AppInfo::default(); MAX_APPS];
    let state = LauncherState::new(1);
    render_launcher(&mut *buf, &apps[..1], &state);
    // Logical row 0 must be all black (0x00).
    assert!(
        logical_row_is(&buf, 0, 0x00),
        "selected row 0 must be entirely black"
    );
}

#[test]
fn render_launcher_second_app_selected_correct_row_launcher_render() {
    let mut buf = zero_buf();
    let apps = [AppInfo::default(); MAX_APPS];
    let mut state = LauncherState::new(2);
    state.selected = 1;
    render_launcher(&mut *buf, &apps[..2], &state);
    // Logical row 0 must be white, logical row 32 must be black.
    assert!(
        logical_row_is(&buf, 0, 0xFF),
        "unselected row 0 must be white"
    );
    assert!(
        logical_row_is(&buf, LAUNCHER_ROW_HEIGHT as usize, 0x00),
        "selected row 1 must be black"
    );
}

#[test]
fn render_launcher_highlight_height_matches_constant_launcher_render() {
    let mut buf = zero_buf();
    let apps = [AppInfo::default(); MAX_APPS];
    let state = LauncherState::new(1);
    render_launcher(&mut *buf, &apps[..1], &state);
    // Exactly LAUNCHER_ROW_HEIGHT logical rows are black; the row after must be white.
    let last_black = LAUNCHER_ROW_HEIGHT as usize - 1;
    let first_white = LAUNCHER_ROW_HEIGHT as usize;
    assert!(
        logical_row_is(&buf, last_black, 0x00),
        "last row of highlight must be black"
    );
    assert!(
        logical_row_is(&buf, first_white, 0xFF),
        "row after highlight must be white"
    );
}

#[test]
fn render_launcher_draws_distinct_filename_rows_launcher_render() {
    let mut buf = zero_buf();
    let apps = [
        AppInfo {
            filename: *b"HELLO   PDB",
        },
        AppInfo {
            filename: *b"WORLD   PDB",
        },
        AppInfo {
            filename: *b"THIRD   PDB",
        },
        AppInfo::default(),
        AppInfo::default(),
        AppInfo::default(),
        AppInfo::default(),
        AppInfo::default(),
        AppInfo::default(),
        AppInfo::default(),
        AppInfo::default(),
        AppInfo::default(),
        AppInfo::default(),
        AppInfo::default(),
        AppInfo::default(),
        AppInfo::default(),
    ];
    let state = LauncherState::new(3);

    render_launcher(&mut *buf, &apps[..3], &state);

    let y0 = 12usize;
    let y1 = y0 + LAUNCHER_ROW_HEIGHT as usize;
    let y2 = y1 + LAUNCHER_ROW_HEIGHT as usize;
    assert_ne!(logical_row_sample(&buf, y1), logical_row_sample(&buf, y2));
    assert_ne!(logical_row_sample(&buf, y0), logical_row_sample(&buf, y1));
}
