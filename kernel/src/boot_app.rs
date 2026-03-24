use crate::abi::PdSyscalls;
use crate::device::serial::serial_write_bytes;
use crate::launcher::format_app_name;
use crate::loader;
use crate::storage::fs::FsState;
use crate::storage::StorageError;
use embedded_sdmmc::BlockDevice;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum LoadAndRunError {
    Storage(StorageError),
    FileTooLarge { size: usize, capacity: usize },
    ShortRead { expected: usize, read: usize },
    Loader(loader::LoadAndRunError),
}

impl From<StorageError> for LoadAndRunError {
    fn from(value: StorageError) -> Self {
        Self::Storage(value)
    }
}

impl From<loader::LoadAndRunError> for LoadAndRunError {
    fn from(value: loader::LoadAndRunError) -> Self {
        Self::Loader(value)
    }
}

/// Controls whether the kernel actually jumps to the loaded application.
///
/// `Jump` runs the full path: load → prepare → jump.  `DryRun` completes every
/// stage up to and including the pre-jump serial tag, then returns without
/// calling the entry point — useful for isolating crashes in the jump path.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum JumpMode {
    /// Load, prepare, and invoke the entry point via `jump_fn`.
    Jump(unsafe fn(*const u8, *const PdSyscalls)),
    /// Load and prepare but do not invoke the entry point.
    DryRun,
}

unsafe fn noop_jump(_entry: *const u8, _syscalls: *const PdSyscalls) {}

/// Open a PDB by FAT 8.3 filename, copy it into `pdb_buf`, and jump to it
/// according to `mode`.
///
/// Each stage of the launch path emits a `LAUNCH:<stage>` tag via
/// `serial_write_bytes` so the host can observe how far the kernel
/// progressed before a crash.
///
/// `pdb_buf` is temporary scratch that holds the on-disk `.pdb` bytes; `app_region`
/// receives the relocated image. `syscalls` stays live for the duration of the jump.
pub unsafe fn load_and_run<D: BlockDevice>(
    fs: &mut FsState<D>,
    filename: &[u8; 11],
    pdb_buf: &mut [u8],
    app_region: &mut [u8],
    syscalls: &PdSyscalls,
    mode: JumpMode,
) -> Result<(), LoadAndRunError>
where
    D::Error: core::fmt::Debug,
{
    serial_write_bytes(b"LAUNCH:select\n");
    let size = load_pdb(fs, filename, pdb_buf)?;
    let jump_fn: unsafe fn(*const u8, *const PdSyscalls) = match mode {
        JumpMode::Jump(f) => f,
        JumpMode::DryRun => noop_jump,
    };
    unsafe { loader::load_and_run(&pdb_buf[..size], app_region, syscalls as *const _, jump_fn) }?;
    Ok(())
}

fn load_pdb<D: BlockDevice>(
    fs: &mut FsState<D>,
    filename: &[u8; 11],
    pdb_buf: &mut [u8],
) -> Result<usize, LoadAndRunError>
where
    D::Error: core::fmt::Debug,
{
    serial_write_bytes(b"LAUNCH:open\n");
    let mut name = [0u8; 13];
    let len = format_app_name(filename, &mut name);
    let path = core::str::from_utf8(&name[..len]).map_err(|_| StorageError::InvalidFormat)?;

    let stat = fs.fs_stat(path)?;
    let size = stat.size as usize;
    if size > pdb_buf.len() {
        return Err(LoadAndRunError::FileTooLarge {
            size,
            capacity: pdb_buf.len(),
        });
    }

    let handle = fs.fs_open(path, false)?;
    let result = (|| {
        let mut read = 0usize;
        while read < size {
            let n = fs.fs_read(handle, &mut pdb_buf[read..size])?;
            if n == 0 {
                return Err(LoadAndRunError::ShortRead {
                    expected: size,
                    read,
                });
            }
            read += n;
        }
        serial_write_bytes(b"LAUNCH:read\n");
        Ok(read)
    })();
    let _ = fs.fs_close(handle);
    result
}
