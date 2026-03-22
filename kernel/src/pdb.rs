use crate::abi::PD_ABI_VERSION;

pub const PDB_MAGIC: u32 = 0x534F4450;
pub const PDB_FORMAT_VERSION: u16 = 1;
pub const PDB_HEADER_SIZE: usize = 104;

pub const PDB_FLAG_NEEDS_WIFI: u32 = 1 << 0;
pub const PDB_FLAG_NEEDS_BT: u32 = 1 << 1;
pub const PDB_FLAG_STORE_APP: u32 = 1 << 2;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum PdbError {
    HeaderTooShort { found: usize },
    BadMagic { found: u32 },
    UnsupportedFormatVersion { found: u16 },
    UnsupportedAbiVersion { found: u16 },
    PayloadLengthOverflow,
    PayloadSizeMismatch { expected: usize, found: usize },
    ChecksumMismatch { expected: u32, found: u32 },
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct PdbHeader {
    pub magic: u32,
    pub format_version: u16,
    pub abi_version: u16,
    pub entry_offset: u32,
    pub text_size: u32,
    pub data_size: u32,
    pub bss_size: u32,
    pub reloc_count: u32,
    pub flags: u32,
    pub app_name: [u8; 32],
    pub app_version: [u8; 32],
    pub min_heap: u32,
    pub checksum: u32,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct PdbPayloadView<'a> {
    pub reloc_table: &'a [u8],
    pub image: &'a [u8],
}

#[repr(C)]
pub struct PdbRawHeader {
    pub magic: u32,
    pub format_version: u16,
    pub abi_version: u16,
    pub entry_offset: u32,
    pub text_size: u32,
    pub data_size: u32,
    pub bss_size: u32,
    pub reloc_count: u32,
    pub flags: u32,
    pub app_name: [u8; 32],
    pub app_version: [u8; 32],
    pub min_heap: u32,
    pub checksum: u32,
}

pub fn parse_fixed_header(bytes: &[u8]) -> Result<PdbHeader, PdbError> {
    if bytes.len() < PDB_HEADER_SIZE {
        return Err(PdbError::HeaderTooShort { found: bytes.len() });
    }

    let mut app_name = [0u8; 32];
    app_name.copy_from_slice(&bytes[0x20..0x40]);

    let mut app_version = [0u8; 32];
    app_version.copy_from_slice(&bytes[0x40..0x60]);

    Ok(PdbHeader {
        magic: u32::from_le_bytes(bytes[0x00..0x04].try_into().unwrap()),
        format_version: u16::from_le_bytes(bytes[0x04..0x06].try_into().unwrap()),
        abi_version: u16::from_le_bytes(bytes[0x06..0x08].try_into().unwrap()),
        entry_offset: u32::from_le_bytes(bytes[0x08..0x0C].try_into().unwrap()),
        text_size: u32::from_le_bytes(bytes[0x0C..0x10].try_into().unwrap()),
        data_size: u32::from_le_bytes(bytes[0x10..0x14].try_into().unwrap()),
        bss_size: u32::from_le_bytes(bytes[0x14..0x18].try_into().unwrap()),
        reloc_count: u32::from_le_bytes(bytes[0x18..0x1C].try_into().unwrap()),
        flags: u32::from_le_bytes(bytes[0x1C..0x20].try_into().unwrap()),
        app_name,
        app_version,
        min_heap: u32::from_le_bytes(bytes[0x60..0x64].try_into().unwrap()),
        checksum: u32::from_le_bytes(bytes[0x64..0x68].try_into().unwrap()),
    })
}

pub fn validate_header_identity(header: &PdbHeader) -> Result<(), PdbError> {
    if header.magic != PDB_MAGIC {
        return Err(PdbError::BadMagic {
            found: header.magic,
        });
    }

    if header.format_version != PDB_FORMAT_VERSION {
        return Err(PdbError::UnsupportedFormatVersion {
            found: header.format_version,
        });
    }

    if header.abi_version != PD_ABI_VERSION as u16 {
        return Err(PdbError::UnsupportedAbiVersion {
            found: header.abi_version,
        });
    }

    Ok(())
}

pub fn validate_payload_integrity(header: &PdbHeader, bytes: &[u8]) -> Result<(), PdbError> {
    let (_, expected_payload_len) = payload_layout(header)?;

    let payload = bytes.get(PDB_HEADER_SIZE..).unwrap_or(&[]);
    if payload.len() != expected_payload_len {
        return Err(PdbError::PayloadSizeMismatch {
            expected: expected_payload_len,
            found: payload.len(),
        });
    }

    let found = crc32(payload);
    if found != header.checksum {
        return Err(PdbError::ChecksumMismatch {
            expected: header.checksum,
            found,
        });
    }

    Ok(())
}

pub fn payload_views<'a>(
    header: &PdbHeader,
    bytes: &'a [u8],
) -> Result<PdbPayloadView<'a>, PdbError> {
    validate_payload_integrity(header, bytes)?;

    let payload = &bytes[PDB_HEADER_SIZE..];
    let (reloc_size, _) = payload_layout(header)?;

    Ok(PdbPayloadView {
        reloc_table: &payload[..reloc_size],
        image: &payload[reloc_size..],
    })
}

fn payload_layout(header: &PdbHeader) -> Result<(usize, usize), PdbError> {
    let reloc_size = header
        .reloc_count
        .checked_mul(4)
        .ok_or(PdbError::PayloadLengthOverflow)?;
    let expected_payload_len = reloc_size
        .checked_add(header.text_size)
        .and_then(|total| total.checked_add(header.data_size))
        .ok_or(PdbError::PayloadLengthOverflow)?;

    Ok((reloc_size as usize, expected_payload_len as usize))
}

fn crc32(bytes: &[u8]) -> u32 {
    let mut crc = 0xFFFF_FFFFu32;

    for &byte in bytes {
        crc ^= byte as u32;
        for _ in 0..8 {
            let mask = (crc & 1).wrapping_neg() & 0xEDB8_8320;
            crc = (crc >> 1) ^ mask;
        }
    }

    !crc
}
