use embedded_sdmmc::filesystem::ToShortFileName;
use embedded_sdmmc::{
    BlockDevice, Mode, RawDirectory, RawFile, RawVolume, ShortFileName, TimeSource, Timestamp,
    VolumeIdx, VolumeManager,
};

/// Seek origin, mirroring `std::io::SeekFrom`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SeekFrom {
    /// Offset from the beginning of the file.
    Start(u32),
    /// Offset from the current position (may be negative).
    Current(i32),
    /// Offset backwards from the end of the file.
    ///
    /// Unlike `std::io::SeekFrom::End` (which takes `i64` and can seek past
    /// EOF), this variant is unsigned and only supports seeking backwards.
    End(u32),
}

/// Whether a directory entry is a regular file or a subdirectory.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EntryType {
    File,
    Directory,
}

/// Metadata returned by [`FsState::fs_stat`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PdStat {
    pub entry_type: EntryType,
    /// Size of the file in bytes (0 for directories).
    pub size: u32,
}

use crate::storage::StorageError;

/// A [`TimeSource`] that always returns the epoch (all zeros).
///
/// Used when no RTC is present or when timestamps are not required.
pub struct NoopTimeSource;

impl TimeSource for NoopTimeSource {
    fn get_timestamp(&self) -> Timestamp {
        Timestamp {
            year_since_1970: 0,
            zero_indexed_month: 0,
            zero_indexed_day: 0,
            hours: 0,
            minutes: 0,
            seconds: 0,
        }
    }
}

/// Opaque index into the open-file slot table.
///
/// The inner slot index is intentionally private; callers should treat this as
/// an opaque token and not inspect or construct it directly.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FileHandle(pub(crate) u8);

impl FileHandle {
    pub fn from_raw(raw: i32) -> Option<Self> {
        if raw < 0 || raw > u8::MAX as i32 {
            return None;
        }
        Some(Self(raw as u8))
    }

    pub fn to_raw(self) -> i32 {
        self.0 as i32
    }
}

/// Opaque index into the open-directory slot table.
///
/// The inner slot index is intentionally private; callers should treat this as
/// an opaque token and not inspect or construct it directly.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DirHandle(pub(crate) u8);

impl DirHandle {
    pub fn from_raw(raw: i32) -> Option<Self> {
        if raw < 0 || raw > u8::MAX as i32 {
            return None;
        }
        Some(Self(raw as u8))
    }

    pub fn to_raw(self) -> i32 {
        self.0 as i32
    }
}

/// A single directory entry returned by [`FsState::fs_readdir`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PdDirEntry {
    /// Raw 8.3 name: bytes 0–7 = base name, bytes 8–10 = extension (space-padded).
    pub name: [u8; 11],
    pub entry_type: EntryType,
    pub size: u32,
}

const MAX_OPEN_FILES: usize = 8;
const MAX_OPEN_DIRS: usize = 4;

/// FAT filesystem state.
///
/// Wraps a [`VolumeManager`] over any [`BlockDevice`] with [`NoopTimeSource`].
/// Keeps one volume and one root-directory handle open lazily on first use.
/// Supports up to [`MAX_OPEN_FILES`] simultaneously open files and
/// [`MAX_OPEN_DIRS`] simultaneously open user directories.
pub struct FsState<D: BlockDevice> {
    pub(crate) vm: VolumeManager<D, NoopTimeSource, 8, 8, 1>,
    volume: Option<RawVolume>,
    root_dir: Option<RawDirectory>,
    file_slots: [Option<RawFile>; MAX_OPEN_FILES],
    /// Each slot: (RawDirectory, entry_index consumed so far).
    dir_slots: [Option<(RawDirectory, usize)>; MAX_OPEN_DIRS],
}

impl<D: BlockDevice> FsState<D>
where
    D::Error: core::fmt::Debug,
{
    /// Construct a new `FsState` wrapping the given block device.
    pub fn new(bd: D) -> Self {
        Self {
            vm: VolumeManager::new_with_limits(bd, NoopTimeSource, 0),
            volume: None,
            root_dir: None,
            file_slots: Default::default(),
            dir_slots: Default::default(),
        }
    }

    fn ensure_volume(&mut self) -> Result<RawVolume, StorageError> {
        if let Some(v) = self.volume {
            return Ok(v);
        }
        let v = self
            .vm
            .open_raw_volume(VolumeIdx(0))
            .map_err(|_| StorageError::NotReady)?;
        self.volume = Some(v);
        Ok(v)
    }

    fn ensure_root_dir(&mut self) -> Result<RawDirectory, StorageError> {
        if let Some(d) = self.root_dir {
            return Ok(d);
        }
        let vol = self.ensure_volume()?;
        let d = self
            .vm
            .open_root_dir(vol)
            .map_err(|_| StorageError::NotReady)?;
        self.root_dir = Some(d);
        Ok(d)
    }

    /// Open a file by 8.3 name in the root directory.
    ///
    /// Returns a [`FileHandle`] on success.  Returns [`StorageError::NotFound`]
    /// if the file does not exist, or [`StorageError::NoSpace`] if all 8 slots
    /// are already occupied.
    ///
    /// When `write` is `true` the file is opened with
    /// `ReadWriteCreateOrTruncate`: the file is created if absent and truncated
    /// to zero length if it exists.  There is no append mode.
    pub fn fs_open(&mut self, path: &str, write: bool) -> Result<FileHandle, StorageError> {
        let dir = self.ensure_root_dir()?;
        let mode = if write {
            Mode::ReadWriteCreateOrTruncate
        } else {
            Mode::ReadOnly
        };
        let raw_file = self.vm.open_file_in_dir(dir, path, mode).map_err(|e| {
            use embedded_sdmmc::Error as E;
            match e {
                E::NotFound => StorageError::NotFound,
                _ => StorageError::IoError,
            }
        })?;
        for (i, slot) in self.file_slots.iter_mut().enumerate() {
            if slot.is_none() {
                *slot = Some(raw_file);
                return Ok(FileHandle(i as u8));
            }
        }
        self.vm.close_file(raw_file).ok();
        Err(StorageError::NoSpace)
    }

    /// Close a previously opened file and free its slot.
    ///
    /// Returns [`StorageError::NotFound`] if the handle is not valid.
    pub fn fs_close(&mut self, handle: FileHandle) -> Result<(), StorageError> {
        let raw_file = self
            .file_slots
            .get_mut(handle.0 as usize)
            .and_then(|s| s.take())
            .ok_or(StorageError::NotFound)?;
        self.vm
            .close_file(raw_file)
            .map_err(|_| StorageError::IoError)
    }

    fn raw_file(&self, handle: FileHandle) -> Result<RawFile, StorageError> {
        self.file_slots
            .get(handle.0 as usize)
            .and_then(|s| *s)
            .ok_or(StorageError::NotFound)
    }

    /// Read up to `buf.len()` bytes from the current position.
    pub fn fs_read(&mut self, handle: FileHandle, buf: &mut [u8]) -> Result<usize, StorageError> {
        let f = self.raw_file(handle)?;
        self.vm.read(f, buf).map_err(|_| StorageError::IoError)
    }

    /// Write `buf` to the current position.
    pub fn fs_write(&mut self, handle: FileHandle, buf: &[u8]) -> Result<(), StorageError> {
        let f = self.raw_file(handle)?;
        self.vm.write(f, buf).map_err(|_| StorageError::IoError)
    }

    /// Seek within the file.
    pub fn fs_seek(&mut self, handle: FileHandle, whence: SeekFrom) -> Result<(), StorageError> {
        let f = self.raw_file(handle)?;
        match whence {
            SeekFrom::Start(n) => self.vm.file_seek_from_start(f, n),
            SeekFrom::Current(n) => self.vm.file_seek_from_current(f, n),
            SeekFrom::End(n) => self.vm.file_seek_from_end(f, n),
        }
        .map(|_| ())
        .map_err(|_| StorageError::IoError)
    }

    /// Return the current byte offset within the file.
    pub fn fs_tell(&self, handle: FileHandle) -> Result<u32, StorageError> {
        let f = self.raw_file(handle)?;
        self.vm.file_offset(f).map_err(|_| StorageError::IoError)
    }

    /// Return `true` if the current position is at or past end of file.
    pub fn fs_eof(&self, handle: FileHandle) -> Result<bool, StorageError> {
        let f = self.raw_file(handle)?;
        self.vm.file_eof(f).map_err(|_| StorageError::IoError)
    }

    /// Return metadata for the named entry in the root directory.
    pub fn fs_stat(&mut self, path: &str) -> Result<PdStat, StorageError> {
        let target: ShortFileName = path
            .to_short_filename()
            .map_err(|_| StorageError::InvalidFormat)?;
        let dir = self.ensure_root_dir()?;
        let mut found: Option<PdStat> = None;
        self.vm
            .iterate_dir(dir, |entry| {
                if entry.name == target {
                    let entry_type = if entry.attributes.is_directory() {
                        EntryType::Directory
                    } else {
                        EntryType::File
                    };
                    found = Some(PdStat {
                        entry_type,
                        size: entry.size,
                    });
                }
            })
            .map_err(|_| StorageError::IoError)?;
        found.ok_or(StorageError::NotFound)
    }

    /// Create a new subdirectory in the root directory.
    pub fn fs_mkdir(&mut self, name: &str) -> Result<(), StorageError> {
        let dir = self.ensure_root_dir()?;
        self.vm
            .make_dir_in_dir(dir, name)
            .map_err(|_| StorageError::IoError)
    }

    /// Delete a file from the root directory.
    pub fn fs_remove(&mut self, name: &str) -> Result<(), StorageError> {
        let dir = self.ensure_root_dir()?;
        self.vm.delete_file_in_dir(dir, name).map_err(|e| {
            use embedded_sdmmc::Error as E;
            match e {
                E::NotFound => StorageError::NotFound,
                _ => StorageError::IoError,
            }
        })
    }

    /// Open a directory for iteration.
    ///
    /// Pass `""` to open the root directory, or an 8.3 name to open a
    /// subdirectory of root.  Returns a [`DirHandle`] referencing a slot in
    /// the 4-entry open-directory table.
    pub fn fs_opendir(&mut self, path: &str) -> Result<DirHandle, StorageError> {
        let raw_dir = if path.is_empty() {
            let vol = self.ensure_volume()?;
            self.vm
                .open_root_dir(vol)
                .map_err(|_| StorageError::NotReady)?
        } else {
            let root = self.ensure_root_dir()?;
            self.vm.open_dir(root, path).map_err(|e| {
                use embedded_sdmmc::Error as E;
                match e {
                    E::NotFound => StorageError::NotFound,
                    _ => StorageError::IoError,
                }
            })?
        };
        for (i, slot) in self.dir_slots.iter_mut().enumerate() {
            if slot.is_none() {
                *slot = Some((raw_dir, 0));
                return Ok(DirHandle(i as u8));
            }
        }
        self.vm.close_dir(raw_dir).ok();
        Err(StorageError::NoSpace)
    }

    /// Read the next entry from an open directory.
    ///
    /// Returns `Ok(Some(entry))` while entries remain, `Ok(None)` when
    /// exhausted.  The internal counter advances on each successful call.
    ///
    /// # Performance
    ///
    /// `embedded-sdmmc` 0.7 provides no incremental cursor API.  Each call
    /// re-iterates the directory from the beginning and skips previously-seen
    /// entries, giving O(n²) block reads for a directory with n entries.  For
    /// the small root directories expected on PaperDOS this is acceptable.
    pub fn fs_readdir(&mut self, handle: DirHandle) -> Result<Option<PdDirEntry>, StorageError> {
        let idx = handle.0 as usize;
        let (raw_dir, entry_idx) = self
            .dir_slots
            .get(idx)
            .and_then(|s| *s)
            .ok_or(StorageError::NotFound)?;

        let mut count = 0usize;
        let mut found: Option<PdDirEntry> = None;

        self.vm
            .iterate_dir(raw_dir, |entry| {
                if count == entry_idx {
                    let entry_type = if entry.attributes.is_directory() {
                        EntryType::Directory
                    } else {
                        EntryType::File
                    };
                    let mut name = [b' '; 11];
                    let base = entry.name.base_name();
                    let ext = entry.name.extension();
                    name[..base.len()].copy_from_slice(base);
                    name[8..8 + ext.len()].copy_from_slice(ext);
                    found = Some(PdDirEntry {
                        name,
                        entry_type,
                        size: entry.size,
                    });
                }
                count += 1;
            })
            .map_err(|_| StorageError::IoError)?;

        if found.is_some() {
            if let Some(slot) = self.dir_slots.get_mut(idx) {
                if let Some((_, ref mut ei)) = slot {
                    *ei += 1;
                }
            }
        }
        Ok(found)
    }

    /// Close an open directory and free its slot.
    pub fn fs_closedir(&mut self, handle: DirHandle) -> Result<(), StorageError> {
        let (raw_dir, _) = self
            .dir_slots
            .get_mut(handle.0 as usize)
            .and_then(|s| s.take())
            .ok_or(StorageError::NotFound)?;
        self.vm
            .close_dir(raw_dir)
            .map_err(|_| StorageError::IoError)
    }
}
