use crate::abi::PdSyscalls;
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

/// Open a PDB by FAT 8.3 filename, copy it into `pdb_buf`, and jump to it.
///
/// `pdb_buf` is temporary scratch that holds the on-disk `.pdb` bytes; `app_region`
/// receives the relocated image. `syscalls` stays live for the duration of the jump.
pub unsafe fn load_and_run<D: BlockDevice>(
    fs: &mut FsState<D>,
    filename: &[u8; 11],
    pdb_buf: &mut [u8],
    app_region: &mut [u8],
    syscalls: &PdSyscalls,
    jump_fn: unsafe fn(*const u8, *const PdSyscalls),
) -> Result<(), LoadAndRunError>
where
    D::Error: core::fmt::Debug,
{
    let size = load_pdb(fs, filename, pdb_buf)?;
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
        Ok(read)
    })();
    let _ = fs.fs_close(handle);
    result
}
