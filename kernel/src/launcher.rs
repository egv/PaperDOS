use crate::abi::{PD_BTN_DOWN, PD_BTN_OK, PD_BTN_UP};
use crate::device::serial::serial_write_bytes;
use crate::storage::fs::{DirHandle, EntryType, FsState};
use crate::storage::StorageError;
use crate::syscall::display::pd_display_refresh;
use crate::syscall::display::{display_clear_to, fill_rect_in, set_pixel_in, FrameBuffer};
use crate::syscall::input::pd_input_wait_button;
use embedded_sdmmc::BlockDevice;

/// Maximum number of apps the launcher will track in a single scan.
pub const MAX_APPS: usize = 16;

/// Metadata for one discovered application.
#[derive(Clone, Copy, Debug)]
pub struct AppInfo {
    /// Raw FAT 8.3 directory name: bytes 0–7 = base, 8–10 = extension (space-padded).
    pub filename: [u8; 11],
}

impl AppInfo {
    const fn zeroed() -> Self {
        Self {
            filename: [b' '; 11],
        }
    }
}

impl Default for AppInfo {
    fn default() -> Self {
        Self::zeroed()
    }
}

/// Scan the open directory `dir` for files whose 8.3 extension is `PDB`.
///
/// Populates `apps[0..count]` and returns `count` (≤ `MAX_APPS`).
/// Directories and non-PDB files are silently skipped.
pub fn scan_apps<D: BlockDevice>(
    fs: &mut FsState<D>,
    dir: DirHandle,
    apps: &mut [AppInfo; MAX_APPS],
) -> Result<usize, StorageError>
where
    D::Error: core::fmt::Debug,
{
    let mut count = 0usize;
    loop {
        let Some(entry) = fs.fs_readdir(dir)? else {
            break;
        };
        if entry.entry_type != EntryType::File {
            continue;
        }
        if &entry.name[8..11] != b"PDB" {
            continue;
        }
        if count >= MAX_APPS {
            break;
        }
        apps[count] = AppInfo {
            filename: entry.name,
        };
        count += 1;
    }
    Ok(count)
}

/// Pixel height of each app row in the launcher list.
pub const LAUNCHER_ROW_HEIGHT: i32 = 48;

const GLYPH_WIDTH: i32 = 5;
const GLYPH_HEIGHT: i32 = 7;
const GLYPH_SCALE: i32 = 4;
const GLYPH_ADVANCE: i32 = 24;
const LAUNCHER_TEXT_X: i32 = 16;
const LAUNCHER_TEXT_Y_PADDING: i32 = 10;

/// Format a raw FAT 8.3 filename as `NAME.EXT` into `out`.
///
/// Returns the number of bytes written. `out` must be large enough for the
/// formatted name (13 bytes is enough for all 8.3 names plus NUL if needed).
pub fn format_app_name(filename: &[u8; 11], out: &mut [u8]) -> usize {
    debug_assert!(out.len() >= 12, "out must hold an 8.3 filename");

    let mut len = 0usize;
    for &byte in &filename[..8] {
        if byte == b' ' {
            break;
        }
        out[len] = byte;
        len += 1;
    }

    let ext = &filename[8..11];
    if ext.iter().any(|&b| b != b' ') {
        out[len] = b'.';
        len += 1;
        for &byte in ext {
            if byte == b' ' {
                break;
            }
            out[len] = byte;
            len += 1;
        }
    }

    len
}

/// Render the launcher app list into `buf`.
///
/// Fills the background white, draws a solid black highlight bar over the
/// selected row, and renders each 8.3 filename in a scaled built-in bitmap font.
pub fn render_launcher(buf: &mut FrameBuffer, apps: &[AppInfo], state: &LauncherState) {
    use crate::abi::PD_SCREEN_WIDTH;

    display_clear_to(buf, 0xFF);

    if state.count > 0 {
        let sel_y = (state.selected as i32) * LAUNCHER_ROW_HEIGHT;
        fill_rect_in(buf, 0, sel_y, PD_SCREEN_WIDTH, LAUNCHER_ROW_HEIGHT, 0x00);
    }

    let mut name = [0u8; 13];
    for (idx, app) in apps.iter().enumerate() {
        let len = format_app_name(&app.filename, &mut name);
        let y = (idx as i32) * LAUNCHER_ROW_HEIGHT + LAUNCHER_TEXT_Y_PADDING;
        let color = if idx == state.selected { 0xFF } else { 0x00 };
        draw_text(buf, LAUNCHER_TEXT_X, y, &name[..len], color);
    }
}

pub fn draw_text(buf: &mut FrameBuffer, x: i32, y: i32, text: &[u8], color: u8) {
    let mut cursor_x = x;
    for &byte in text {
        draw_glyph(buf, cursor_x, y, byte, color);
        cursor_x += GLYPH_ADVANCE;
    }
}

fn draw_glyph(buf: &mut FrameBuffer, x: i32, y: i32, ch: u8, color: u8) {
    let glyph = glyph_rows(ch);
    for (row, bits) in glyph.iter().copied().enumerate() {
        for col in 0..GLYPH_WIDTH {
            if bits & (1 << (GLYPH_WIDTH - 1 - col)) != 0 {
                let px = x + col * GLYPH_SCALE;
                let py = y + row as i32 * GLYPH_SCALE;
                for dy in 0..GLYPH_SCALE {
                    for dx in 0..GLYPH_SCALE {
                        set_pixel_in(buf, px + dx, py + dy, color);
                    }
                }
            }
        }
    }
}

fn glyph_rows(ch: u8) -> [u8; GLYPH_HEIGHT as usize] {
    match ch.to_ascii_uppercase() {
        b'A' => [0x0E, 0x11, 0x11, 0x1F, 0x11, 0x11, 0x11],
        b'B' => [0x1E, 0x11, 0x11, 0x1E, 0x11, 0x11, 0x1E],
        b'C' => [0x0E, 0x11, 0x10, 0x10, 0x10, 0x11, 0x0E],
        b'D' => [0x1C, 0x12, 0x11, 0x11, 0x11, 0x12, 0x1C],
        b'E' => [0x1F, 0x10, 0x10, 0x1E, 0x10, 0x10, 0x1F],
        b'F' => [0x1F, 0x10, 0x10, 0x1E, 0x10, 0x10, 0x10],
        b'G' => [0x0E, 0x11, 0x10, 0x10, 0x13, 0x11, 0x0E],
        b'H' => [0x11, 0x11, 0x11, 0x1F, 0x11, 0x11, 0x11],
        b'I' => [0x1F, 0x04, 0x04, 0x04, 0x04, 0x04, 0x1F],
        b'J' => [0x01, 0x01, 0x01, 0x01, 0x11, 0x11, 0x0E],
        b'K' => [0x11, 0x12, 0x14, 0x18, 0x14, 0x12, 0x11],
        b'L' => [0x10, 0x10, 0x10, 0x10, 0x10, 0x10, 0x1F],
        b'M' => [0x11, 0x1B, 0x15, 0x15, 0x11, 0x11, 0x11],
        b'N' => [0x11, 0x11, 0x19, 0x15, 0x13, 0x11, 0x11],
        b'O' => [0x0E, 0x11, 0x11, 0x11, 0x11, 0x11, 0x0E],
        b'P' => [0x1E, 0x11, 0x11, 0x1E, 0x10, 0x10, 0x10],
        b'Q' => [0x0E, 0x11, 0x11, 0x11, 0x15, 0x12, 0x0D],
        b'R' => [0x1E, 0x11, 0x11, 0x1E, 0x14, 0x12, 0x11],
        b'S' => [0x0F, 0x10, 0x10, 0x0E, 0x01, 0x01, 0x1E],
        b'T' => [0x1F, 0x04, 0x04, 0x04, 0x04, 0x04, 0x04],
        b'U' => [0x11, 0x11, 0x11, 0x11, 0x11, 0x11, 0x0E],
        b'V' => [0x11, 0x11, 0x11, 0x11, 0x11, 0x0A, 0x04],
        b'W' => [0x11, 0x11, 0x11, 0x15, 0x15, 0x15, 0x0A],
        b'X' => [0x11, 0x11, 0x0A, 0x04, 0x0A, 0x11, 0x11],
        b'Y' => [0x11, 0x11, 0x0A, 0x04, 0x04, 0x04, 0x04],
        b'Z' => [0x1F, 0x01, 0x02, 0x04, 0x08, 0x10, 0x1F],
        b'0' => [0x0E, 0x11, 0x13, 0x15, 0x19, 0x11, 0x0E],
        b'1' => [0x04, 0x0C, 0x14, 0x04, 0x04, 0x04, 0x1F],
        b'2' => [0x0E, 0x11, 0x01, 0x02, 0x04, 0x08, 0x1F],
        b'3' => [0x1E, 0x01, 0x01, 0x0E, 0x01, 0x01, 0x1E],
        b'4' => [0x02, 0x06, 0x0A, 0x12, 0x1F, 0x02, 0x02],
        b'5' => [0x1F, 0x10, 0x10, 0x1E, 0x01, 0x01, 0x1E],
        b'6' => [0x0E, 0x10, 0x10, 0x1E, 0x11, 0x11, 0x0E],
        b'7' => [0x1F, 0x01, 0x02, 0x04, 0x08, 0x08, 0x08],
        b'8' => [0x0E, 0x11, 0x11, 0x0E, 0x11, 0x11, 0x0E],
        b'9' => [0x0E, 0x11, 0x11, 0x0F, 0x01, 0x01, 0x0E],
        b'.' => [0x00, 0x00, 0x00, 0x00, 0x00, 0x0C, 0x0C],
        b'-' => [0x00, 0x00, 0x00, 0x1F, 0x00, 0x00, 0x00],
        b'_' => [0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x1F],
        b' ' => [0x00; GLYPH_HEIGHT as usize],
        _ => [0x1F, 0x11, 0x02, 0x04, 0x04, 0x00, 0x04],
    }
}

/// Run the launcher loop on the given filesystem and framebuffer.
///
/// Scans the root directory for `.PDB` files, renders the selection list,
/// and blocks on button input until the user selects an app.
///
/// Returns the raw FAT 8.3 filename (`[u8; 11]`) of the chosen app.
/// If no apps are found, blocks indefinitely on button input until one
/// appears — in practice the caller should ensure the SD is populated.
pub fn run_launcher<D: BlockDevice>(fs: &mut FsState<D>, buf: &mut FrameBuffer) -> [u8; 11]
where
    D::Error: core::fmt::Debug,
{
    run_launcher_with_refresh(fs, buf, |_| pd_display_refresh(0))
}

/// Run the launcher loop and invoke `refresh` after each redraw.
pub fn run_launcher_with_refresh<D, F>(
    fs: &mut FsState<D>,
    buf: &mut FrameBuffer,
    mut refresh: F,
) -> [u8; 11]
where
    D: BlockDevice,
    D::Error: core::fmt::Debug,
    F: FnMut(&FrameBuffer),
{
    let mut apps = [AppInfo::default(); MAX_APPS];
    let count = match fs.fs_opendir("") {
        Ok(dir) => scan_apps(fs, dir, &mut apps).unwrap_or(0),
        Err(_) => 0,
    };
    let mut state = LauncherState::new(count);

    // Initial render — paint once before blocking on input.
    render_launcher(buf, &apps[..count], &state);
    refresh(buf);

    loop {
        let buttons = pd_input_wait_button();
        let prev = state.selected;

        if buttons & PD_BTN_DOWN != 0 {
            state.move_down();
        } else if buttons & PD_BTN_UP != 0 {
            state.move_up();
        } else if buttons & PD_BTN_OK != 0 && count > 0 {
            let mut name = [0u8; 13];
            let len = format_app_name(&apps[state.selected].filename, &mut name);
            serial_write_bytes(b"APP:");
            serial_write_bytes(&name[..len]);
            serial_write_bytes(b"\n");
            return apps[state.selected].filename;
        }

        // Re-render only when the selection actually moved.
        if state.selected != prev {
            render_launcher(buf, &apps[..count], &state);
            refresh(buf);
        }
    }
}

/// Cursor state for the launcher selection list.
pub struct LauncherState {
    /// Total number of apps available.
    pub count: usize,
    /// Index of the currently highlighted app (0-based).
    pub selected: usize,
}

impl LauncherState {
    /// Create a new state for `count` apps; selection starts at 0.
    pub fn new(count: usize) -> Self {
        Self { count, selected: 0 }
    }

    /// Move selection up; wraps from 0 to `count - 1`.
    pub fn move_up(&mut self) {
        if self.count == 0 {
            return;
        }
        self.selected = if self.selected == 0 {
            self.count - 1
        } else {
            self.selected - 1
        };
    }

    /// Move selection down; wraps from `count - 1` to 0.
    pub fn move_down(&mut self) {
        if self.count == 0 {
            return;
        }
        self.selected = (self.selected + 1) % self.count;
    }
}
