use kernel::boot::{boot, AbiMetadata, BootState};
use kernel::pdb::{PDB_FORMAT_VERSION, PDB_HEADER_SIZE, PDB_MAGIC};
use kernel::platform::{HostPlatform, StorageError};

fn sample_boot_header() -> [u8; PDB_HEADER_SIZE] {
    let mut bytes = [0u8; PDB_HEADER_SIZE];
    bytes[0x00..0x04].copy_from_slice(&PDB_MAGIC.to_le_bytes());
    bytes[0x04..0x06].copy_from_slice(&PDB_FORMAT_VERSION.to_le_bytes());
    bytes[0x06..0x08].copy_from_slice(&1u16.to_le_bytes());
    bytes
}

#[test]
fn boot_smoke() {
    let header = sample_boot_header();
    let platform = HostPlatform::new("/apps/demo.pdb", &header, 99);

    assert_eq!(
        boot(&platform.storage, &platform.support, "/apps/demo.pdb"),
        Ok(BootState {
            boot_millis: 99,
            storage_probe_len: PDB_HEADER_SIZE,
            abi: AbiMetadata {
                abi_version: 1,
                kernel_version: 1,
            },
        })
    );

    assert!(platform.support.watchdog_fed());
    assert!(platform.support.logged());
}

#[test]
fn boot_rejects_invalid_probe_data() {
    let short_platform = HostPlatform::new("/apps/demo.pdb", &[0xAA; 8], 99);
    assert_eq!(
        boot(
            &short_platform.storage,
            &short_platform.support,
            "/apps/demo.pdb"
        ),
        Err(StorageError::InvalidData)
    );

    let invalid_platform = HostPlatform::new("/apps/demo.pdb", &[0xAA; PDB_HEADER_SIZE], 99);
    assert_eq!(
        boot(
            &invalid_platform.storage,
            &invalid_platform.support,
            "/apps/demo.pdb"
        ),
        Err(StorageError::InvalidData)
    );
}
