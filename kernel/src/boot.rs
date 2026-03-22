use crate::abi::PD_ABI_VERSION;
use crate::pdb::{parse_fixed_header, validate_header_identity, PDB_HEADER_SIZE};
use crate::platform::{KernelSupport, LogLevel, StorageError, StorageReader};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct AbiMetadata {
    pub abi_version: u32,
    pub kernel_version: u32,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct BootState {
    pub boot_millis: u32,
    pub storage_probe_len: usize,
    pub abi: AbiMetadata,
}

pub fn boot(
    storage: &impl StorageReader,
    support: &impl KernelSupport,
    probe_path: &str,
) -> Result<BootState, StorageError> {
    let mut probe = [0u8; PDB_HEADER_SIZE];
    let storage_probe_len = storage.read(probe_path, &mut probe)?;
    if storage_probe_len != PDB_HEADER_SIZE {
        return Err(StorageError::InvalidData);
    }

    let header = parse_fixed_header(&probe).map_err(|_| StorageError::InvalidData)?;
    validate_header_identity(&header).map_err(|_| StorageError::InvalidData)?;

    support.feed_watchdog();
    support.log(LogLevel::Info, "boot");

    Ok(BootState {
        boot_millis: support.millis(),
        storage_probe_len,
        abi: AbiMetadata {
            abi_version: PD_ABI_VERSION,
            kernel_version: 1,
        },
    })
}
