mod common;

use core::sync::atomic::{AtomicBool, Ordering};

use common::InMemoryBlockDevice;
use embedded_sdmmc::Block;
use kernel::abi::{PdSyscalls, PD_ABI_VERSION};
use kernel::boot_app::{load_and_run, LoadAndRunError};
use kernel::storage::fs::FsState;
use kernel::syscall::build_syscall_table;

static JUMP_CALLED: AtomicBool = AtomicBool::new(false);

unsafe fn mock_jump(_entry: *const u8, _syscalls: *const PdSyscalls) {
    JUMP_CALLED.store(true, Ordering::SeqCst);
}

fn make_valid_apps_fat16_image() -> Vec<Block> {
    let hello = common::make_min_pdb(&[0u8; 4]);
    let world = common::make_min_pdb(&[1u8, 2, 3, 4]);
    assert!(hello.len() <= 512);
    assert!(world.len() <= 512);

    const TOTAL: usize = 4201;
    let mut blocks = vec![Block::new(); TOTAL];

    {
        let b = &mut blocks[0].contents;
        b[446] = 0x00;
        b[447] = 0x00;
        b[448] = 0x01;
        b[449] = 0x00;
        b[450] = 0x06;
        b[454] = 0x01;
        b[458] = 0x68;
        b[459] = 0x10;
        b[510] = 0x55;
        b[511] = 0xAA;
    }

    {
        let b = &mut blocks[1].contents;
        b[0] = 0xEB;
        b[1] = 0x3C;
        b[2] = 0x90;
        b[3..11].copy_from_slice(b"PAPERDOS");
        b[11] = 0x00;
        b[12] = 0x02;
        b[13] = 0x01;
        b[14] = 0x01;
        b[16] = 0x01;
        b[17] = 0x10;
        b[19] = 0x68;
        b[20] = 0x10;
        b[21] = 0xF8;
        b[22] = 0x11;
        b[24] = 0x01;
        b[26] = 0x01;
        b[510] = 0x55;
        b[511] = 0xAA;
    }

    {
        let b = &mut blocks[2].contents;
        b[0] = 0xF8;
        b[1] = 0xFF;
        b[2] = 0xFF;
        b[3] = 0xFF;
        b[4] = 0xFF;
        b[5] = 0xFF;
        b[6] = 0xFF;
        b[7] = 0xFF;
    }

    {
        let b = &mut blocks[19].contents;
        b[0x00..0x08].copy_from_slice(b"HELLO   ");
        b[0x08..0x0B].copy_from_slice(b"PDB");
        b[0x0B] = 0x20;
        b[0x1A] = 0x02;
        b[0x1C..0x20].copy_from_slice(&(hello.len() as u32).to_le_bytes());

        b[0x20..0x28].copy_from_slice(b"WORLD   ");
        b[0x28..0x2B].copy_from_slice(b"PDB");
        b[0x2B] = 0x20;
        b[0x3A] = 0x03;
        b[0x3C..0x40].copy_from_slice(&(world.len() as u32).to_le_bytes());
    }

    blocks[20].contents[..hello.len()].copy_from_slice(&hello);
    blocks[21].contents[..world.len()].copy_from_slice(&world);
    blocks
}

#[test]
fn boot_app_load_and_run_reads_named_pdb_and_jumps() {
    JUMP_CALLED.store(false, Ordering::SeqCst);
    let bd = InMemoryBlockDevice::new(make_valid_apps_fat16_image());
    let mut fs = FsState::new(bd);
    let mut pdb_buf = [0u8; 512];
    let mut app_region = [0u8; 256];
    let syscalls = build_syscall_table(0, 0);

    let result = unsafe {
        load_and_run(
            &mut fs,
            b"HELLO   PDB",
            &mut pdb_buf,
            &mut app_region,
            &syscalls,
            mock_jump,
        )
    };

    assert!(
        result.is_ok(),
        "valid PDB on FAT image must load: {result:?}"
    );
    assert!(JUMP_CALLED.load(Ordering::SeqCst), "jump must be invoked");
}

#[test]
fn boot_app_load_and_run_reports_small_scratch_buffer() {
    let bd = InMemoryBlockDevice::new(make_valid_apps_fat16_image());
    let mut fs = FsState::new(bd);
    let mut pdb_buf = [0u8; 32];
    let mut app_region = [0u8; 256];
    let syscalls = build_syscall_table(0, 0);

    let result = unsafe {
        load_and_run(
            &mut fs,
            b"HELLO   PDB",
            &mut pdb_buf,
            &mut app_region,
            &syscalls,
            mock_jump,
        )
    };

    assert!(matches!(result, Err(LoadAndRunError::FileTooLarge { .. })));
}

#[test]
fn boot_app_test_data_matches_kernel_abi() {
    assert_eq!(PD_ABI_VERSION, 1);
}
