# PaperDOS Web Flasher — Product Requirements Document

**Version:** 0.1 · **Date:** March 2026 · **Status:** Draft

---

## 1. Overview

The PaperDOS Web Flasher is a browser-based tool that lets users install and update the PaperDOS kernel firmware on an Xteink X4 device — no toolchain installation required. Plug in the device via USB-C, open a web page, click "Flash," and the firmware is installed.

This is the primary onboarding path for non-developer users and the recommended recovery tool when a firmware update goes wrong. It also serves as a one-stop hub for converting EPUBs and images into PaperDOS-optimized formats (.trbk books, .tri images).

## 2. Problem Statement

Today, flashing Rust firmware to an ESP32-C3 device requires installing either ESP-IDF + esptool.py or the Rust espflash toolchain. This is a significant barrier for non-developers who just want to try PaperDOS on their X4. TernReader demonstrated that a web-based flasher dramatically increases adoption — users flash firmware in under 60 seconds without touching a terminal.

## 3. Goals

- **Zero-install flashing.** Users should go from "I want PaperDOS" to "PaperDOS is running" in under 2 minutes, with no software installation.
- **One-click updates.** When a new PaperDOS kernel is released, the web flasher should detect the current version and offer a one-click update.
- **Recovery mode.** If the device is bricked (stuck in bootloader), the flasher should detect this and offer recovery.
- **Content conversion.** Provide EPUB-to-.trbk and image-to-.tri conversion in the same web app, so users can prepare content for their device in one place.
- **Developer-friendly.** Developers building PaperDOS apps should be able to use the flasher to push custom kernel builds during development.

## 4. Technical Feasibility

### 4.1 WebSerial API

The Web Serial API (available in Chrome 89+, Edge 89+, Opera 76+) provides direct serial port access from the browser. This is the same API used by:

- **esptool-js** — Espressif's official browser-based flash tool
- **ESP Web Tools** — ESPHome's widely-used web flasher
- **TernReader's web flasher** at ternreader.org

The ESP32-C3's USB-JTAG/Serial peripheral exposes a CDC-ACM serial interface over USB-C, which WebSerial can access directly — no USB driver installation needed on modern operating systems.

### 4.2 ESP32-C3 Flash Protocol

The ESP32-C3 ROM bootloader speaks the SLIP-framed esptool serial protocol. esptool-js implements this protocol entirely in JavaScript, including:

- Chip detection and identification
- Flash size detection
- Compressed (deflate) upload for faster transfers
- MD5 verification after flashing
- Bootloader/partition-table/app binary writing at correct offsets

The ESP32-C3 flash layout for a Rust (no_std) firmware:

| Offset | Content | Size (typical) |
|--------|---------|----------------|
| 0x0000 | Bootloader (2nd stage) | ~16 KB |
| 0x8000 | Partition table | 3 KB |
| 0x10000 | Application firmware | 1–4 MB |

For PaperDOS, we'll distribute a single merged binary (bootloader + partition table + app) created via `espflash save-image` or `esptool.py merge-bin`. This simplifies the flasher to a single-file write starting at offset 0x0.

### 4.3 Entering Bootloader Mode

The Xteink X4 can enter the ESP32-C3's download mode (bootloader) in two ways:

1. **Automatic (via RTS/DTR).** esptool-js toggles the serial control lines to reset the chip into download mode automatically. This works if the USB-Serial bridge supports RTS/DTR — the ESP32-C3's built-in USB-JTAG/Serial does support this.
2. **Manual.** Hold the BOOT button (if accessible) while power-cycling. On the X4, this may require holding a specific button combination during power-on (to be verified on hardware).

The flasher should attempt automatic entry first, and fall back to guided manual instructions with illustrations if auto-entry fails.

### 4.4 Rust Firmware Compatibility

Rust firmware compiled for ESP32-C3 via `cargo build --release --target riscv32imc-unknown-none-elf` produces a standard ELF binary. The `espflash save-image` command converts this to a flashable binary containing the bootloader, partition table, and app image. This merged .bin file is exactly what esptool-js expects.

The build pipeline for release firmware:

```
cargo build --release
espflash save-image --chip esp32c3 --merge \
    target/riscv32imc-unknown-none-elf/release/paperdos \
    paperdos-firmware-vX.Y.Z.bin
```

The resulting .bin file is hosted on the web flasher's server and downloaded by the browser during the flash process.

### 4.5 Browser Compatibility

| Browser | WebSerial Support | Status |
|---------|-------------------|--------|
| Chrome (desktop) | Yes (v89+) | Primary target |
| Edge (desktop) | Yes (v89+) | Supported |
| Opera (desktop) | Yes (v76+) | Supported |
| Chrome (Android) | Partial (USB OTG) | Best-effort |
| Firefox | No | Not supported (display message) |
| Safari | No | Not supported (display message) |

The flasher must detect unsupported browsers and display a clear message directing users to Chrome or Edge.

## 5. User Experience

### 5.1 Flash Flow

**Step 1: Landing page.** User arrives at flash.paperdos.org (or similar). Sees the PaperDOS logo, a brief description, and a prominent "Flash PaperDOS" button. Current firmware version is displayed. Below the fold: "Convert Books" and "Convert Images" sections.

**Step 2: Connect device.** User clicks "Flash PaperDOS." Browser shows the native WebSerial port picker. User selects the X4's serial port and clicks "Connect."

**Step 3: Device detection.** The flasher reads the ESP32-C3 chip ID and flash size. If PaperDOS is already installed, it reads the current version from a known flash offset. Displays: chip info, flash size, current firmware version (if any).

**Step 4: Version selection.** Shows the latest stable release (pre-selected) and optionally a list of recent versions. For developers, a "Custom .bin" option allows uploading a local file.

**Step 5: Flashing.** Progress bar with percentage, transfer speed, and ETA. The merged binary is downloaded from the server (or used from the local upload), then streamed to the device. Typical flash time: 30–60 seconds for a 2 MB image at 460800 baud.

**Step 6: Verification.** MD5 checksum verification after flashing. Display green checkmark on success.

**Step 7: Reboot.** Automatic hard reset via RTS/DTR. The device boots into PaperDOS. "Done! Your X4 is now running PaperDOS vX.Y.Z." with a link to the getting-started guide.

### 5.2 Recovery Flow

If the flasher detects the device is in bootloader mode (no running firmware responds), it enters recovery mode:

1. Skip version detection (no firmware to read).
2. Offer to flash the latest stable release.
3. After flashing, verify and reboot as normal.

### 5.3 Content Conversion

#### EPUB to .trbk

Runs entirely in-browser using WebAssembly (the same Rust `core` crate compiled to WASM):

1. User drags an .epub file onto the page (or clicks "Choose file").
2. The WASM module parses the EPUB, performs text layout at the X4's exact 800×480 resolution, and outputs a .trbk file.
3. User downloads the .trbk and copies it to the X4's SD card (or, if the device is connected, pushes it via the PaperDOS Wi-Fi upload feature).

This is directly inspired by TernReader's ternreader.org conversion flow.

#### Image to .tri

1. User selects image files (PNG, JPEG).
2. Browser-side JavaScript/WASM converts to 1-bit or 4-level grayscale, applying Floyd-Steinberg dithering.
3. Output .tri files are downloaded for SD card transfer.

### 5.4 Error Handling

| Scenario | Behavior |
|----------|----------|
| Unsupported browser | Banner: "Web Serial requires Chrome or Edge. Please switch browsers." |
| No device detected | "No device found. Make sure your X4 is plugged in via USB-C." + link to troubleshooting guide |
| Auto-bootloader fails | Step-by-step visual guide for manual boot mode entry |
| Flash write fails mid-transfer | "Flash failed at X%. Your device is safe — it will boot the previous firmware via the backup OTA partition. Click Retry." |
| Checksum mismatch | "Verification failed. Please retry. If this persists, try a different USB cable." |
| Device disconnected mid-flash | "Connection lost. Reconnect and retry — the device has a backup partition and is not bricked." |

## 6. Architecture

### 6.1 Frontend

A single-page app. Minimal dependencies to keep load time fast.

| Component | Technology | Rationale |
|-----------|------------|-----------|
| Framework | Vanilla JS or Preact | Minimal bundle; no heavy framework needed |
| Serial comms | esptool-js | Official Espressif library; maintained, battle-tested |
| EPUB conversion | Rust core crate → WASM | Code sharing with device firmware; pixel-perfect parity |
| Image conversion | Canvas API + JS/WASM | Floyd-Steinberg dithering for 1-bit/4-gray output |
| Styling | Tailwind CSS (CDN) | Fast development; responsive by default |
| Hosting | GitHub Pages or Cloudflare Pages | Free; fast CDN; custom domain support |

### 6.2 Firmware Distribution

Firmware binaries are stored as GitHub Release assets in the PaperDOS repository. The flasher fetches a manifest.json from a known URL:

```json
{
  "latest": "0.3.0",
  "releases": [
    {
      "version": "0.3.0",
      "date": "2026-06-15",
      "url": "https://github.com/.../releases/download/v0.3.0/paperdos-v0.3.0.bin",
      "size": 1843200,
      "sha256": "abc123...",
      "changelog": "Added Store app, improved partial refresh"
    },
    {
      "version": "0.2.0",
      "date": "2026-05-01",
      "url": "https://github.com/.../releases/download/v0.2.0/paperdos-v0.2.0.bin",
      "size": 1536000,
      "sha256": "def456...",
      "changelog": "Initial EPUB reader, launcher UI"
    }
  ]
}
```

### 6.3 WASM Module

The EPUB-to-.trbk converter and image-to-.tri converter share Rust code with the device firmware via a `paperdos-core` crate compiled to `wasm32-unknown-unknown`. The crate exposes:

```rust
// paperdos-core/src/lib.rs (simplified)

#[cfg(target_arch = "wasm32")]
use wasm_bindgen::prelude::*;

#[wasm_bindgen]
pub fn convert_epub(epub_bytes: &[u8], screen_width: u32, screen_height: u32) -> Vec<u8> {
    // Returns .trbk bytes
}

#[wasm_bindgen]
pub fn convert_image(image_bytes: &[u8], target_width: u32, target_height: u32, grayscale_levels: u8) -> Vec<u8> {
    // Returns .tri bytes
}
```

This ensures that content converted on the desktop renders identically on the device.

## 7. Security

### 7.1 Firmware Integrity

- All firmware binaries include SHA-256 hashes in the manifest.
- The flasher verifies the hash after downloading and before flashing.
- Future: Ed25519 signatures on firmware binaries, with the public key embedded in the flasher. This prevents MITM attacks even if the CDN is compromised.

### 7.2 WebSerial Permissions

- The browser's native permission prompt ensures users explicitly grant serial port access.
- The flasher never auto-connects; the user must click "Connect" and select the device.
- No serial port access is retained between page loads (permission is per-session unless the user grants persistent access).

### 7.3 Supply Chain

- The flasher is open source and auditable.
- Firmware binaries are built via GitHub Actions CI from tagged commits, ensuring reproducible builds.
- The manifest.json is served from the same origin as the flasher (no cross-origin firmware fetches except to GitHub Releases).

## 8. Scope & Milestones

### v1.0 — Minimum Viable Flasher

- [ ] Flash merged .bin firmware to ESP32-C3 via esptool-js
- [ ] Automatic bootloader entry via RTS/DTR
- [ ] Manual bootloader mode instructions with illustrations
- [ ] Progress bar with percentage and ETA
- [ ] MD5 post-flash verification
- [ ] Automatic hard-reset after flashing
- [ ] Version manifest fetching from GitHub Releases
- [ ] "Custom .bin" upload for developers
- [ ] Unsupported browser detection
- [ ] Error handling for all failure modes
- [ ] Responsive design (works on mobile Chrome with USB OTG, best-effort)
- [ ] Deploy to flash.paperdos.org (or paperdos.github.io/flash)

### v1.1 — Content Conversion

- [ ] EPUB-to-.trbk conversion via WASM module
- [ ] Drag-and-drop file input
- [ ] Image-to-.tri conversion (1-bit and 4-gray modes)
- [ ] Batch conversion for multiple files
- [ ] Preview rendered output at 800×480 in-browser

### v1.2 — Device Integration

- [ ] Read current firmware version from connected device
- [ ] Show "Update Available" when newer version exists
- [ ] Wi-Fi push: after conversion, send .trbk/.tri directly to device over local network (if device has PaperDOS Wi-Fi upload server running)
- [ ] OTA update support (flash app partition only, preserve bootloader)

### v2.0 — Signed Firmware

- [ ] Ed25519 firmware signature verification
- [ ] Signed manifest with update channel support (stable / beta / nightly)
- [ ] Rollback protection (prevent flashing older vulnerable versions)

## 9. Open Questions

1. **Domain & hosting.** Should the flasher live at a custom domain (flash.paperdos.org), a GitHub Pages subdirectory (paperdos.github.io/flash), or embedded in the main project documentation site?

2. **Offline support.** Should the flasher work offline (service worker caches the last firmware)? Useful for flashing at events or in areas without internet.

3. **Multiple device support.** Should the flasher support batch-flashing multiple X4 devices in sequence? Relevant for workshops or group purchases.

4. **Bootloader backup.** Should the flasher offer to back up the stock Xteink firmware before flashing PaperDOS? This would require reading the full flash contents first (~30 seconds for 16 MB).

5. **Partition scheme.** Should PaperDOS use the ESP32-C3's dual OTA partition scheme (app0 + app1)? This would allow safe updates — if a new firmware fails to boot, the device falls back to the previous partition. The tradeoff is halving the available app flash space (6.5 MB → 3.25 MB per slot).

---

*This PRD is part of the PaperDOS project documentation. See also: PaperDOS Technical Design Document v0.1.*
