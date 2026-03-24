use crate::abi::PdSyscalls;
use crate::device::serial::serial_write_bytes;
use crate::pdb::{
    parse_fixed_header, payload_views, validate_header_identity, PdbError, PdbHeader,
};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum LoaderError {
    RamBudgetOverflow,
    AppRegionTooSmall {
        required: u32,
        available: u32,
    },
    BssOutOfBounds {
        start: usize,
        end: usize,
        region_len: usize,
    },
    RelocationOutOfBounds {
        offset: usize,
        image_len: usize,
    },
    EntryOffsetOutOfBounds {
        entry_offset: u32,
        image_len: usize,
    },
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum PrepareImageError {
    Pdb(PdbError),
    Loader(LoaderError),
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct PreparedImage {
    pub entry_offset: u32,
    pub image_size: u32,
    pub total_ram: u32,
    pub min_heap: u32,
}

impl From<PdbError> for PrepareImageError {
    fn from(value: PdbError) -> Self {
        Self::Pdb(value)
    }
}

impl From<LoaderError> for PrepareImageError {
    fn from(value: LoaderError) -> Self {
        Self::Loader(value)
    }
}

pub fn ram_budget_bytes(header: &PdbHeader) -> Result<u32, LoaderError> {
    header
        .text_size
        .checked_add(header.data_size)
        .and_then(|total| total.checked_add(header.bss_size))
        .and_then(|total| total.checked_add(header.min_heap))
        .ok_or(LoaderError::RamBudgetOverflow)
}

pub fn ensure_region_fit(required: u32, available: u32) -> Result<(), LoaderError> {
    if required > available {
        return Err(LoaderError::AppRegionTooSmall {
            required,
            available,
        });
    }

    Ok(())
}

pub fn zero_bss_tail(
    region: &mut [u8],
    initialized_size: u32,
    bss_size: u32,
) -> Result<(), LoaderError> {
    let start = initialized_size as usize;
    let end = start.saturating_add(bss_size as usize);

    if end > region.len() {
        return Err(LoaderError::BssOutOfBounds {
            start,
            end,
            region_len: region.len(),
        });
    }

    region[start..end].fill(0);
    Ok(())
}

pub fn apply_relocations(
    image: &mut [u8],
    reloc_table: &[u8],
    load_address: u32,
) -> Result<(), LoaderError> {
    for entry in reloc_table.chunks_exact(4) {
        let offset = u32::from_le_bytes(entry.try_into().unwrap()) as usize;
        let end = offset + 4;

        if end > image.len() {
            return Err(LoaderError::RelocationOutOfBounds {
                offset,
                image_len: image.len(),
            });
        }

        let value = u32::from_le_bytes(image[offset..end].try_into().unwrap());
        let relocated = value.wrapping_add(load_address);
        image[offset..end].copy_from_slice(&relocated.to_le_bytes());
    }

    Ok(())
}

/// Error returned by [`load_and_run`].
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum LoadAndRunError {
    PrepareImage(PrepareImageError),
}

impl From<PrepareImageError> for LoadAndRunError {
    fn from(e: PrepareImageError) -> Self {
        Self::PrepareImage(e)
    }
}

/// Parse and validate `pdb`, copy the image into `region`, apply relocations,
/// and call `jump_fn` with the resolved entry pointer.
///
/// `jump_fn` defaults to [`crate::jump::jump_to_app`] in production; pass
/// a mock during tests to avoid executing arbitrary bytes on the host.
///
/// # Safety
/// `jump_fn` must be safe to call with the entry pointer derived from `region`
/// and the provided `syscalls` pointer.
pub unsafe fn load_and_run(
    pdb: &[u8],
    region: &mut [u8],
    syscalls: *const PdSyscalls,
    jump_fn: unsafe fn(*const u8, *const PdSyscalls),
) -> Result<(), LoadAndRunError> {
    let load_addr = region.as_ptr() as u32;
    let prepared = prepare_image(pdb, region, load_addr)?;
    serial_write_bytes(b"LAUNCH:prepare\n");
    let entry = unsafe { region.as_ptr().add(prepared.entry_offset as usize) };
    serial_write_bytes(b"LAUNCH:jump\n");
    unsafe { jump_fn(entry, syscalls) };
    serial_write_bytes(b"LAUNCH:returned\n");
    Ok(())
}

pub fn prepare_image(
    bytes: &[u8],
    region: &mut [u8],
    load_address: u32,
) -> Result<PreparedImage, PrepareImageError> {
    let header = parse_fixed_header(bytes)?;
    validate_header_identity(&header)?;

    let views = payload_views(&header, bytes)?;
    let total_ram = ram_budget_bytes(&header)?;
    let available = u32::try_from(region.len()).unwrap_or(u32::MAX);
    ensure_region_fit(total_ram, available)?;

    let image_size = views.image.len();
    if header.entry_offset >= header.text_size {
        return Err(LoaderError::EntryOffsetOutOfBounds {
            entry_offset: header.entry_offset,
            image_len: header.text_size as usize,
        }
        .into());
    }

    region[..image_size].copy_from_slice(views.image);
    zero_bss_tail(region, image_size as u32, header.bss_size)?;
    apply_relocations(&mut region[..image_size], views.reloc_table, load_address)?;

    Ok(PreparedImage {
        entry_offset: header.entry_offset,
        image_size: image_size as u32,
        total_ram,
        min_heap: header.min_heap,
    })
}
