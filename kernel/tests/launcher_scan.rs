mod common;
use common::InMemoryBlockDevice;

use embedded_sdmmc::Block;
use kernel::launcher::{scan_apps, AppInfo, LauncherState, MAX_APPS};
use kernel::storage::fs::FsState;

// ── FAT16 image with two .PDB files in root ───────────────────────────────────
//
// Layout mirrors make_test_fat16_image() but with HELLO.PDB and WORLD.PDB in
// the root directory instead of README.TXT + TESTDIR.
//
// Global blocks:
//   0  : MBR (partition 1: FAT16, LBA start=1, size=4200)
//   1  : Boot sector (FAT16 BPB)
//   2-18: FAT (17 sectors); entries 0-3 set (end-of-chain for clusters 2+3)
//   19 : Root directory — HELLO.PDB (cluster 2, size 104) + WORLD.PDB (cluster 3, size 104)
//   20 : Cluster 2 — 104 bytes of HELLO.PDB data (valid PDB magic at offset 0)
//   21 : Cluster 3 — 104 bytes of WORLD.PDB data (valid PDB magic at offset 0)
fn make_apps_fat16_image() -> Vec<Block> {
    const TOTAL: usize = 4201;
    let mut blocks = vec![Block::new(); TOTAL];

    // Block 0: MBR
    {
        let b = &mut blocks[0].contents;
        b[446] = 0x00;
        b[447] = 0x00; b[448] = 0x01; b[449] = 0x00;
        b[450] = 0x06; // FAT16
        b[451] = 0x00; b[452] = 0x00; b[453] = 0x00;
        b[454] = 0x01; b[455] = 0x00; b[456] = 0x00; b[457] = 0x00; // LBA start = 1
        b[458] = 0x68; b[459] = 0x10; b[460] = 0x00; b[461] = 0x00; // size = 4200
        b[510] = 0x55; b[511] = 0xAA;
    }

    // Block 1: Boot sector (FAT16 BPB)
    {
        let b = &mut blocks[1].contents;
        b[0] = 0xEB; b[1] = 0x3C; b[2] = 0x90;
        b[3..11].copy_from_slice(b"PAPERDOS");
        b[11] = 0x00; b[12] = 0x02; // bytes/sector = 512
        b[13] = 0x01; // sectors/cluster = 1
        b[14] = 0x01; b[15] = 0x00; // reserved sectors = 1
        b[16] = 0x01; // num FATs = 1
        b[17] = 0x10; b[18] = 0x00; // root entries = 16
        b[19] = 0x68; b[20] = 0x10; // total sectors = 4200
        b[21] = 0xF8; // media type
        b[22] = 0x11; b[23] = 0x00; // FAT size = 17 sectors
        b[24] = 0x01; b[25] = 0x00; // sectors/track
        b[26] = 0x01; b[27] = 0x00; // num heads
        b[28] = 0x00; b[29] = 0x00; b[30] = 0x00; b[31] = 0x00; // hidden sectors
        b[32] = 0x00; b[33] = 0x00; b[34] = 0x00; b[35] = 0x00; // large total sectors
        b[510] = 0x55; b[511] = 0xAA;
    }

    // Block 2: FAT sector 0 — clusters 0-3 all end-of-chain
    {
        let b = &mut blocks[2].contents;
        b[0] = 0xF8; b[1] = 0xFF; // FAT ID
        b[2] = 0xFF; b[3] = 0xFF; // entry 1 (reserved)
        b[4] = 0xFF; b[5] = 0xFF; // cluster 2 = EOF
        b[6] = 0xFF; b[7] = 0xFF; // cluster 3 = EOF
    }

    // Block 19: Root directory — 2 entries × 32 bytes
    // Entry 0: HELLO.PDB  cluster 2  size 104
    // Entry 1: WORLD.PDB  cluster 3  size 104
    {
        let b = &mut blocks[19].contents;

        // Entry 0: HELLO.PDB
        b[0x00..0x08].copy_from_slice(b"HELLO   ");
        b[0x08..0x0B].copy_from_slice(b"PDB");
        b[0x0B] = 0x20; // archive attribute
        // cluster high = 0 (FAT16)
        b[0x1A] = 0x02; b[0x1B] = 0x00; // first cluster = 2
        b[0x1C] = 104;  b[0x1D] = 0x00; b[0x1E] = 0x00; b[0x1F] = 0x00; // size = 104

        // Entry 1: WORLD.PDB
        b[0x20..0x28].copy_from_slice(b"WORLD   ");
        b[0x28..0x2B].copy_from_slice(b"PDB");
        b[0x2B] = 0x20;
        b[0x3A] = 0x03; b[0x3B] = 0x00; // first cluster = 3
        b[0x3C] = 104;  b[0x3D] = 0x00; b[0x3E] = 0x00; b[0x3F] = 0x00;
    }

    // Block 20: Cluster 2 — HELLO.PDB content (minimal valid PDB magic)
    {
        let b = &mut blocks[20].contents;
        b[0x00..0x04].copy_from_slice(&0x534F4450u32.to_le_bytes()); // "PDOS" magic
    }

    // Block 21: Cluster 3 — WORLD.PDB content
    {
        let b = &mut blocks[21].contents;
        b[0x00..0x04].copy_from_slice(&0x534F4450u32.to_le_bytes());
    }

    blocks
}

// ── E5 tests ──────────────────────────────────────────────────────────────────

/// scan_apps must find both .PDB files in root.
#[test]
fn scan_apps_finds_pdb_files_launcher_scan() {
    let bd = InMemoryBlockDevice::new(make_apps_fat16_image());
    let mut fs = FsState::new(bd);
    let dir = fs.fs_opendir("").expect("root dir must open");
    let mut apps = [AppInfo::default(); MAX_APPS];
    let count = scan_apps(&mut fs, dir, &mut apps).expect("scan must succeed");
    assert_eq!(count, 2, "must find exactly 2 .PDB files");
    assert_eq!(&apps[0].filename[8..11], b"PDB");
    assert_eq!(&apps[1].filename[8..11], b"PDB");
}

/// scan_apps must ignore entries that are not .PDB files.
#[test]
fn scan_apps_ignores_non_pdb_files_launcher_scan() {
    // Use the standard test image which has README.TXT + TESTDIR — no .PDB files.
    let bd = InMemoryBlockDevice::new(common::make_test_fat16_image());
    let mut fs = FsState::new(bd);
    let dir = fs.fs_opendir("").expect("root dir must open");
    let mut apps = [AppInfo::default(); MAX_APPS];
    let count = scan_apps(&mut fs, dir, &mut apps).expect("scan must succeed");
    assert_eq!(count, 0, "no .PDB files means count must be 0");
}

/// LauncherState::move_down wraps at the last entry.
#[test]
fn launcher_state_move_down_wraps_launcher_scan() {
    let mut state = LauncherState::new(3);
    assert_eq!(state.selected, 0);
    state.move_down();
    assert_eq!(state.selected, 1);
    state.move_down();
    assert_eq!(state.selected, 2);
    state.move_down(); // wrap
    assert_eq!(state.selected, 0);
}

/// LauncherState::move_up wraps at the first entry.
#[test]
fn launcher_state_move_up_wraps_launcher_scan() {
    let mut state = LauncherState::new(3);
    state.move_up(); // wrap from 0 → 2
    assert_eq!(state.selected, 2);
    state.move_up();
    assert_eq!(state.selected, 1);
    state.move_up();
    assert_eq!(state.selected, 0);
}

/// LauncherState with zero apps must not panic on navigation.
#[test]
fn launcher_state_empty_no_panic_launcher_scan() {
    let mut state = LauncherState::new(0);
    state.move_down();
    state.move_up();
    assert_eq!(state.selected, 0);
}
