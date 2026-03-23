use crate::storage::fs::{DirHandle, EntryType, FsState};
use crate::storage::StorageError;
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
        Self { filename: [b' '; 11] }
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
        let Some(entry) = fs.fs_readdir(dir)? else { break };
        if entry.entry_type != EntryType::File {
            continue;
        }
        if &entry.name[8..11] != b"PDB" {
            continue;
        }
        if count >= MAX_APPS {
            break;
        }
        apps[count] = AppInfo { filename: entry.name };
        count += 1;
    }
    Ok(count)
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
        self.selected = if self.selected == 0 { self.count - 1 } else { self.selected - 1 };
    }

    /// Move selection down; wraps from `count - 1` to 0.
    pub fn move_down(&mut self) {
        if self.count == 0 {
            return;
        }
        self.selected = (self.selected + 1) % self.count;
    }
}
