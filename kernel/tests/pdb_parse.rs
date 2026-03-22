use core::mem::{offset_of, size_of};

use kernel::pdb::{
    parse_fixed_header, payload_views, validate_header_identity, validate_payload_integrity,
    PdbError, PdbHeader, PdbPayloadView, PdbRawHeader, PDB_FLAG_NEEDS_BT, PDB_FLAG_NEEDS_WIFI,
    PDB_FLAG_STORE_APP, PDB_FORMAT_VERSION, PDB_HEADER_SIZE, PDB_MAGIC,
};

fn sample_header_bytes() -> [u8; PDB_HEADER_SIZE] {
    let mut bytes = [0u8; PDB_HEADER_SIZE];

    bytes[0x00..0x04].copy_from_slice(&PDB_MAGIC.to_le_bytes());
    bytes[0x04..0x06].copy_from_slice(&PDB_FORMAT_VERSION.to_le_bytes());
    bytes[0x06..0x08].copy_from_slice(&1u16.to_le_bytes());
    bytes[0x08..0x0C].copy_from_slice(&0x1234u32.to_le_bytes());
    bytes[0x0C..0x10].copy_from_slice(&0x200u32.to_le_bytes());
    bytes[0x10..0x14].copy_from_slice(&0x80u32.to_le_bytes());
    bytes[0x14..0x18].copy_from_slice(&0x40u32.to_le_bytes());
    bytes[0x18..0x1C].copy_from_slice(&3u32.to_le_bytes());
    bytes[0x1C..0x20].copy_from_slice(&PDB_FLAG_NEEDS_WIFI.to_le_bytes());
    bytes[0x20..0x2A].copy_from_slice(b"Hello PDB\0");
    bytes[0x40..0x46].copy_from_slice(b"1.2.3\0");
    bytes[0x60..0x64].copy_from_slice(&4096u32.to_le_bytes());
    bytes[0x64..0x68].copy_from_slice(&0xDEADBEEFu32.to_le_bytes());

    bytes
}

fn sample_pdb_bytes() -> Vec<u8> {
    let payload = [
        0x04, 0x00, 0x00, 0x00, 0x08, 0x00, 0x00, 0x00, 0x01, 0x02, 0x03, 0x04, 0x05, 0x06, 0x07,
        0x08,
    ];

    let mut bytes = sample_header_bytes().to_vec();
    bytes[0x0C..0x10].copy_from_slice(&4u32.to_le_bytes());
    bytes[0x10..0x14].copy_from_slice(&4u32.to_le_bytes());
    bytes[0x18..0x1C].copy_from_slice(&2u32.to_le_bytes());
    bytes[0x64..0x68].copy_from_slice(&0x646E2680u32.to_le_bytes());
    bytes.extend_from_slice(&payload);
    bytes
}

#[test]
fn pdb_header_layout() {
    assert_eq!(PDB_MAGIC, 0x534F4450);
    assert_eq!(PDB_FORMAT_VERSION, 1);
    assert_eq!(PDB_FLAG_NEEDS_WIFI, 1 << 0);
    assert_eq!(PDB_FLAG_NEEDS_BT, 1 << 1);
    assert_eq!(PDB_FLAG_STORE_APP, 1 << 2);
    assert_eq!(PDB_HEADER_SIZE, 104);

    assert_eq!(size_of::<PdbRawHeader>(), PDB_HEADER_SIZE);
    assert_eq!(offset_of!(PdbRawHeader, magic), 0x00);
    assert_eq!(offset_of!(PdbRawHeader, format_version), 0x04);
    assert_eq!(offset_of!(PdbRawHeader, abi_version), 0x06);
    assert_eq!(offset_of!(PdbRawHeader, entry_offset), 0x08);
    assert_eq!(offset_of!(PdbRawHeader, text_size), 0x0C);
    assert_eq!(offset_of!(PdbRawHeader, data_size), 0x10);
    assert_eq!(offset_of!(PdbRawHeader, bss_size), 0x14);
    assert_eq!(offset_of!(PdbRawHeader, reloc_count), 0x18);
    assert_eq!(offset_of!(PdbRawHeader, flags), 0x1C);
    assert_eq!(offset_of!(PdbRawHeader, app_name), 0x20);
    assert_eq!(offset_of!(PdbRawHeader, app_version), 0x40);
    assert_eq!(offset_of!(PdbRawHeader, min_heap), 0x60);
    assert_eq!(offset_of!(PdbRawHeader, checksum), 0x64);
}

#[test]
fn pdb_parse_fixed_header() {
    let header = parse_fixed_header(&sample_header_bytes()).expect("header should parse");

    assert_eq!(
        header,
        PdbHeader {
            magic: PDB_MAGIC,
            format_version: PDB_FORMAT_VERSION,
            abi_version: 1,
            entry_offset: 0x1234,
            text_size: 0x200,
            data_size: 0x80,
            bss_size: 0x40,
            reloc_count: 3,
            flags: PDB_FLAG_NEEDS_WIFI,
            app_name: {
                let mut bytes = [0u8; 32];
                bytes[..10].copy_from_slice(b"Hello PDB\0");
                bytes
            },
            app_version: {
                let mut bytes = [0u8; 32];
                bytes[..6].copy_from_slice(b"1.2.3\0");
                bytes
            },
            min_heap: 4096,
            checksum: 0xDEADBEEF,
        }
    );
}

#[test]
fn pdb_parse_fixed_header_rejects_short_input() {
    let err = parse_fixed_header(&sample_header_bytes()[..32]).expect_err("short header must fail");
    assert_eq!(err, PdbError::HeaderTooShort { found: 32 });
}

#[test]
fn pdb_validate_identity() {
    let header = parse_fixed_header(&sample_header_bytes()).expect("header should parse");
    validate_header_identity(&header).expect("header identity should validate");

    let mut bad_magic = sample_header_bytes();
    bad_magic[0x00..0x04].copy_from_slice(&0x12345678u32.to_le_bytes());
    let bad_magic_header = parse_fixed_header(&bad_magic).expect("header should still parse");
    assert_eq!(
        validate_header_identity(&bad_magic_header),
        Err(PdbError::BadMagic { found: 0x12345678 })
    );

    let mut bad_format = sample_header_bytes();
    bad_format[0x04..0x06].copy_from_slice(&7u16.to_le_bytes());
    let bad_format_header = parse_fixed_header(&bad_format).expect("header should still parse");
    assert_eq!(
        validate_header_identity(&bad_format_header),
        Err(PdbError::UnsupportedFormatVersion { found: 7 })
    );

    let mut bad_abi = sample_header_bytes();
    bad_abi[0x06..0x08].copy_from_slice(&2u16.to_le_bytes());
    let bad_abi_header = parse_fixed_header(&bad_abi).expect("header should still parse");
    assert_eq!(
        validate_header_identity(&bad_abi_header),
        Err(PdbError::UnsupportedAbiVersion { found: 2 })
    );
}

#[test]
fn pdb_validate_payload_integrity() {
    let pdb = sample_pdb_bytes();
    let header = parse_fixed_header(&pdb).expect("header should parse");

    validate_header_identity(&header).expect("header identity should validate");
    validate_payload_integrity(&header, &pdb).expect("payload should validate");

    let truncated = &pdb[..pdb.len() - 1];
    assert_eq!(
        validate_payload_integrity(&header, truncated),
        Err(PdbError::PayloadSizeMismatch {
            expected: 16,
            found: 15,
        })
    );

    let mut corrupted = pdb.clone();
    let last = corrupted.len() - 1;
    corrupted[last] ^= 0xFF;
    assert_eq!(
        validate_payload_integrity(&header, &corrupted),
        Err(PdbError::ChecksumMismatch {
            expected: 0x646E2680,
            found: 0x496CC90D,
        })
    );

    let mut overflow_bytes = sample_header_bytes();
    overflow_bytes[0x18..0x1C].copy_from_slice(&u32::MAX.to_le_bytes());
    let overflow_header = parse_fixed_header(&overflow_bytes).expect("header should still parse");
    assert_eq!(
        validate_payload_integrity(&overflow_header, &overflow_bytes),
        Err(PdbError::PayloadLengthOverflow)
    );
}

#[test]
fn pdb_payload_views() {
    let pdb = sample_pdb_bytes();
    let header = parse_fixed_header(&pdb).expect("header should parse");

    assert_eq!(
        payload_views(&header, &pdb),
        Ok(PdbPayloadView {
            reloc_table: &pdb[0x68..0x70],
            image: &pdb[0x70..0x78],
        })
    );

    let mut corrupted = pdb.clone();
    let last = corrupted.len() - 1;
    corrupted[last] ^= 0xFF;
    assert_eq!(
        payload_views(&header, &corrupted),
        Err(PdbError::ChecksumMismatch {
            expected: 0x646E2680,
            found: 0x496CC90D,
        })
    );
}
