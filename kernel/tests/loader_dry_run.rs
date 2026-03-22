use kernel::loader::{
    apply_relocations, ensure_region_fit, prepare_image, ram_budget_bytes, zero_bss_tail,
    LoaderError, PrepareImageError, PreparedImage,
};
use kernel::pdb::PdbHeader;

fn sample_header() -> PdbHeader {
    PdbHeader {
        magic: 0,
        format_version: 0,
        abi_version: 0,
        entry_offset: 0,
        text_size: 512,
        data_size: 128,
        bss_size: 64,
        reloc_count: 0,
        flags: 0,
        app_name: [0; 32],
        app_version: [0; 32],
        min_heap: 4096,
        checksum: 0,
    }
}

fn sample_loader_pdb() -> Vec<u8> {
    let mut bytes = vec![0u8; 104];
    let payload = [
        0x00, 0x00, 0x00, 0x00, 0x04, 0x00, 0x00, 0x00, 0x10, 0x00, 0x00, 0x00, 0x20, 0x00, 0x00,
        0x00,
    ];

    bytes[0x00..0x04].copy_from_slice(&0x534F4450u32.to_le_bytes());
    bytes[0x04..0x06].copy_from_slice(&1u16.to_le_bytes());
    bytes[0x06..0x08].copy_from_slice(&1u16.to_le_bytes());
    bytes[0x08..0x0C].copy_from_slice(&0x00u32.to_le_bytes());
    bytes[0x0C..0x10].copy_from_slice(&4u32.to_le_bytes());
    bytes[0x10..0x14].copy_from_slice(&4u32.to_le_bytes());
    bytes[0x14..0x18].copy_from_slice(&4u32.to_le_bytes());
    bytes[0x18..0x1C].copy_from_slice(&2u32.to_le_bytes());
    bytes[0x20..0x2C].copy_from_slice(b"Loader Test\0");
    bytes[0x40..0x46].copy_from_slice(b"0.1.0\0");
    bytes[0x64..0x68].copy_from_slice(&0x3334EE3Fu32.to_le_bytes());
    bytes.extend_from_slice(&payload);

    bytes
}

#[test]
fn loader_ram_budget_loader_dry_run() {
    let header = sample_header();
    assert_eq!(ram_budget_bytes(&header), Ok(4800));

    let mut overflow = sample_header();
    overflow.text_size = u32::MAX;
    overflow.data_size = 1;
    assert_eq!(
        ram_budget_bytes(&overflow),
        Err(LoaderError::RamBudgetOverflow)
    );
}

#[test]
fn loader_region_fit_loader_dry_run() {
    assert_eq!(ensure_region_fit(4096, 4096), Ok(()));
    assert_eq!(ensure_region_fit(4096, 8192), Ok(()));
    assert_eq!(
        ensure_region_fit(8193, 8192),
        Err(LoaderError::AppRegionTooSmall {
            required: 8193,
            available: 8192,
        })
    );
}

#[test]
fn loader_zero_bss_loader_dry_run() {
    let mut region = [0xAAu8; 12];
    zero_bss_tail(&mut region, 8, 4).expect("bss tail should zero cleanly");
    assert_eq!(&region[..8], &[0xAA; 8]);
    assert_eq!(&region[8..], &[0x00; 4]);

    let mut short_region = [0xAAu8; 11];
    assert_eq!(
        zero_bss_tail(&mut short_region, 8, 4),
        Err(LoaderError::BssOutOfBounds {
            start: 8,
            end: 12,
            region_len: 11,
        })
    );
}

#[test]
fn loader_apply_relocations_loader_dry_run() {
    let mut image = [0x10, 0x00, 0x00, 0x00, 0x20, 0x00, 0x00, 0x00];
    let reloc_table = [0x00, 0x00, 0x00, 0x00, 0x04, 0x00, 0x00, 0x00];

    apply_relocations(&mut image, &reloc_table, 0x1000).expect("relocations should apply");
    assert_eq!(image, [0x10, 0x10, 0x00, 0x00, 0x20, 0x10, 0x00, 0x00]);

    let mut bad_image = [0u8; 8];
    let bad_reloc_table = [0x05, 0x00, 0x00, 0x00];
    assert_eq!(
        apply_relocations(&mut bad_image, &bad_reloc_table, 0x1000),
        Err(LoaderError::RelocationOutOfBounds {
            offset: 5,
            image_len: 8,
        })
    );
}

#[test]
fn loader_prepare_image_loader_dry_run() {
    let pdb = sample_loader_pdb();
    let mut region = [0xAAu8; 12];

    assert_eq!(
        prepare_image(&pdb, &mut region, 0x2000),
        Ok(PreparedImage {
            entry_offset: 0,
            image_size: 8,
            total_ram: 12,
            min_heap: 0,
        })
    );
    assert_eq!(
        region,
        [0x10, 0x20, 0x00, 0x00, 0x20, 0x20, 0x00, 0x00, 0, 0, 0, 0]
    );

    let mut small_region = [0u8; 11];
    assert_eq!(
        prepare_image(&pdb, &mut small_region, 0x2000),
        Err(PrepareImageError::Loader(LoaderError::AppRegionTooSmall {
            required: 12,
            available: 11,
        }))
    );

    let mut bad_entry = sample_loader_pdb();
    bad_entry[0x08..0x0C].copy_from_slice(&8u32.to_le_bytes());
    let mut region = [0xAAu8; 12];
    assert_eq!(
        prepare_image(&bad_entry, &mut region, 0x2000),
        Err(PrepareImageError::Loader(
            LoaderError::EntryOffsetOutOfBounds {
                entry_offset: 8,
                image_len: 4,
            }
        ))
    );
}
