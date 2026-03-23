use crate::abi::{PD_BTN_DOWN, PD_BTN_OK, PD_BTN_UP};
use crate::storage::fs::{DirHandle, EntryType, FsState};
use crate::storage::StorageError;
use crate::syscall::display::pd_display_refresh;
use crate::syscall::display::{display_clear_to, fill_rect_in, FrameBuffer};
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
pub const LAUNCHER_ROW_HEIGHT: i32 = 32;

/// Render the launcher app list into `buf`.
///
/// Fills the background white, then draws a solid black highlight bar over
/// the selected row.  Text rendering is deferred to a later phase (font
/// syscalls); this pass produces a structurally correct pixel layout that
/// the display can show immediately.
pub fn render_launcher(buf: &mut FrameBuffer, _apps: &[AppInfo], state: &LauncherState) {
    use crate::abi::PD_SCREEN_WIDTH;

    display_clear_to(buf, 0xFF);

    if state.count > 0 {
        let sel_y = (state.selected as i32) * LAUNCHER_ROW_HEIGHT;
        fill_rect_in(buf, 0, sel_y, PD_SCREEN_WIDTH, LAUNCHER_ROW_HEIGHT, 0x00);
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
    let mut apps = [AppInfo::default(); MAX_APPS];
    let count = match fs.fs_opendir("") {
        Ok(dir) => scan_apps(fs, dir, &mut apps).unwrap_or(0),
        Err(_) => 0,
    };
    let mut state = LauncherState::new(count);

    // Initial render — paint once before blocking on input.
    render_launcher(buf, &apps[..count], &state);
    pd_display_refresh(0);

    loop {
        let buttons = pd_input_wait_button();
        let prev = state.selected;

        if buttons & PD_BTN_DOWN != 0 {
            state.move_down();
        } else if buttons & PD_BTN_UP != 0 {
            state.move_up();
        } else if buttons & PD_BTN_OK != 0 && count > 0 {
            return apps[state.selected].filename;
        }

        // Re-render only when the selection actually moved.
        if state.selected != prev {
            render_launcher(buf, &apps[..count], &state);
            pd_display_refresh(0);
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
