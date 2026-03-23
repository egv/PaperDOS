mod common;

use common::MockSpi;
use kernel::storage::sd::{CardKind, SdCard};

/// Build a MockSpi pre-loaded with a minimal happy-path SDHC init exchange.
///
/// Transaction sequence:
/// 0: 80-clock preamble (10x 0xFF write)                     — no reply needed
/// 1: CMD0 → R1 = 0x01 (idle)
/// 2: CMD8 → R7 = [0x01, 0x00, 0x00, 0x01, 0xAA]
/// 3: CMD55 (APP_CMD) → R1 = 0x01
/// 4: ACMD41 (HCS) → R1 = 0x00  (card ready)
/// 5: CMD58 (READ_OCR) → R3 = [0x00, 0xC0, 0x00, 0x00, 0x00] (CCS=1 → SDHC)
fn make_sdhc_spi() -> MockSpi {
    MockSpi::new(&[
        &[],                                          // preamble — no reply
        &[0xFF, 0xFF, 0x01],                          // CMD0 response (0x01 = idle)
        &[0xFF, 0xFF, 0x01, 0x00, 0x00, 0x01, 0xAA], // CMD8 R7
        &[0xFF, 0xFF, 0x01],                          // CMD55 R1
        &[0xFF, 0xFF, 0x00],                          // ACMD41 R1 = ready
        &[0xFF, 0xFF, 0x00, 0xC0, 0x00, 0x00, 0x00], // CMD58 R3 (CCS=1)
    ])
}

#[test]
fn sd_init_sdhc_returns_ok_storage_sd_init() {
    let mut card = SdCard::new(make_sdhc_spi());
    let kind = card.init().unwrap();
    assert_eq!(kind, CardKind::Sdhc);
}

#[test]
fn sd_init_not_ready_returns_err_storage_sd_init() {
    // ACMD41 always returns 0x01 (busy) — should return NotReady after retries
    let mut spi = MockSpi::new(&[
        &[],                                          // preamble
        &[0xFF, 0xFF, 0x01],                          // CMD0
        &[0xFF, 0xFF, 0x01, 0x00, 0x00, 0x01, 0xAA], // CMD8
        // CMD55+ACMD41 always returns busy (0x01); fill 20 pairs
        &[0xFF, 0xFF, 0x01], &[0xFF, 0xFF, 0x01],
        &[0xFF, 0xFF, 0x01], &[0xFF, 0xFF, 0x01],
        &[0xFF, 0xFF, 0x01], &[0xFF, 0xFF, 0x01],
        &[0xFF, 0xFF, 0x01], &[0xFF, 0xFF, 0x01],
        &[0xFF, 0xFF, 0x01], &[0xFF, 0xFF, 0x01],
        &[0xFF, 0xFF, 0x01], &[0xFF, 0xFF, 0x01],
        &[0xFF, 0xFF, 0x01], &[0xFF, 0xFF, 0x01],
        &[0xFF, 0xFF, 0x01], &[0xFF, 0xFF, 0x01],
        &[0xFF, 0xFF, 0x01], &[0xFF, 0xFF, 0x01],
        &[0xFF, 0xFF, 0x01], &[0xFF, 0xFF, 0x01], // 10 pairs total
    ]);
    let mut card = SdCard::new(spi);
    let err = card.init().unwrap_err();
    assert_eq!(err, kernel::storage::StorageError::NotReady);
}
