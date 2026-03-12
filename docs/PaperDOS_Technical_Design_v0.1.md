# PaperDOS — Technical Design Document v0.1

**A Lightweight Runtime for Downloadable Applications on the Xteink X4 E-Ink Reader**

March 2026 · DRAFT — Community Discussion Document

---

## Table of Contents

1. [Executive Summary](#1-executive-summary)
2. [Hardware Reference](#2-hardware-reference)
3. [Architecture Overview](#3-architecture-overview)
4. [The PaperDOS Kernel](#4-the-paperdos-kernel)
5. [The PaperDOS Binary Format (.PDB)](#5-the-paperdos-binary-format-pdb)
6. [SDK and Toolchain](#6-sdk-and-toolchain)
7. [The PaperDOS Store](#7-the-paperdos-store)
8. [Flagship Applications](#8-flagship-applications)
9. [Security Considerations](#9-security-considerations)
10. [Development Roadmap](#10-development-roadmap)
11. [Open Questions](#11-open-questions)
12. [Appendix A: Prior Art Analysis](#appendix-a-prior-art-analysis)

---

## 1. Executive Summary

PaperDOS is a lightweight, DOS-inspired runtime environment designed for the Xteink X4, a portable 4.26-inch e-ink reader powered by an ESP32-C3 (RISC-V) microcontroller. The project aims to transform this minimalist hardware into an open application platform where users can discover, download, and run third-party programs from a centralized store.

The core insight is that the X4's hardware profile — a 160MHz RISC-V CPU with ~327KB RAM, 16MB flash, microSD storage, Wi-Fi, and physical button input — closely mirrors the capabilities of early personal computers that successfully ran rich application ecosystems. PaperDOS applies the lessons of DOS-era computing: a small resident kernel providing hardware abstraction, a stable syscall-like API, and dynamically loaded native binaries that get full access to system resources.

The kernel is implemented in **Rust (no_std)** using the **Embassy** async runtime on **esp-hal**, with a **C ABI boundary** for dynamically loaded applications. This combines the memory safety and zero-cost abstractions of Rust with a stable, language-agnostic interface for third-party apps. The display subsystem uses **strip-based rendering** (4 KB per strip, no full framebuffer) adapted from the PulpOS project, reclaiming ~44 KB of RAM for applications. Books use a **pre-rendered page format** inspired by TernReader's .trbk, offloading all EPUB parsing and layout to a desktop companion tool.

The two flagship applications are an EPUB e-book reader (the device's primary use case, enhanced beyond the stock firmware) and the PaperDOS Store itself — a built-in app for browsing, downloading, and managing applications over Wi-Fi.

---

## 2. Hardware Reference

### 2.1 Xteink X4 Specifications

| Component | Specification | Notes |
|-----------|---------------|-------|
| CPU | ESP32-C3 (RISC-V RV32IMC), 160 MHz | Single core, no FPU, no MMU |
| RAM | 400 KB SRAM (~327 KB usable, ~172 KB heap) | No PSRAM available |
| Flash | 16 MB SPI (6.5 MB per OTA slot) | Dual app partitions |
| Display | 4.26" e-ink, 800×480, 220 PPI | SSD1677 controller, no touch |
| Input | 6 buttons via ADC + power button | GPIO1 (4 btn), GPIO2 (2 btn), GPIO3 (pwr) |
| Storage | microSD (up to 512 GB) | FAT filesystem, shared SPI bus with display |
| Wireless | Wi-Fi 2.4 GHz + Bluetooth LE | esp-wifi crate for networking |
| Battery | 650 mAh Li-Po | GPIO0 voltage readout via divider |
| USB | USB-C | Charging + firmware flashing (CDC-ACM serial) |
| Dimensions | 114 × 69 × 5.9 mm, 74 g | Credit-card form factor |

### 2.2 Pin Mapping

| Function | GPIO | Protocol |
|----------|------|----------|
| Display SCLK | GPIO8 | SPI |
| Display MOSI | GPIO10 | SPI |
| Display CS | GPIO21 | SPI |
| Display DC | GPIO4 | SPI |
| Display RST | GPIO5 | Direct |
| Display BUSY | GPIO6 | Direct |
| SD Card CS | GPIO12 | SPI (shared bus) |
| Buttons (4) | GPIO1 | ADC |
| Buttons (2) | GPIO2 | ADC |
| Power Button | GPIO3 | Direct |
| Battery Voltage | GPIO0 | ADC (voltage divider) |

### 2.3 SPI Bus Sharing

The display and SD card share SPI2. The kernel uses `CriticalSectionDevice` (from the `embedded-hal-bus` crate) to arbitrate access, ensuring all SD I/O completes before any display render pass begins. DMA is used for display SPI transfers, freeing the CPU during the relatively slow writes to the SSD1677.

---

## 3. Architecture Overview

### 3.1 Design Philosophy

PaperDOS draws directly from the MS-DOS/CP/M model: a small, permanently resident kernel that abstracts the hardware behind a stable API, combined with transient user programs loaded from storage into a dedicated memory region. Key design principles:

- **Simplicity over safety.** Like DOS, the system is single-task with no memory protection. A misbehaving app can crash the system, but a watchdog timer will recover. This eliminates the overhead of an MMU (which the ESP32-C3 lacks anyway).
- **Native performance.** Apps are compiled RISC-V binaries, not interpreted scripts. Every CPU cycle is available to the application.
- **Stable ABI.** The kernel exposes a versioned function pointer table (the "syscall table") across a C ABI boundary. Apps compiled against ABI v1 will run on any kernel that supports ABI v1, regardless of kernel version or language.
- **SD-card-centric.** Programs, data, and configuration all live on the microSD card. The kernel and bootloader live in flash. This separation means the app ecosystem survives firmware updates.
- **Memory-first design.** With only ~172 KB of usable heap, every byte matters. The kernel uses strip-based rendering (no full framebuffer), static allocation for large structures, and on-demand Wi-Fi to maximize the memory available to applications.
- **Async by default.** The Embassy async runtime handles concurrent concerns (input polling, display refresh, networking, background work) without threads or RTOS overhead. The CPU enters WFI (wait-for-interrupt) whenever all tasks await, minimizing power consumption.

### 3.2 System Layers

The system is organized into four layers, from bottom to top:

| Layer | Resides In | Description |
|-------|------------|-------------|
| ESP32-C3 Hardware | — | CPU, RAM, peripherals, SPI bus |
| PaperDOS Kernel (Rust) | Flash (app partition) | esp-hal drivers, Embassy tasks, display driver, filesystem, Wi-Fi, loader, syscall table |
| App Binary | RAM (app region) | Relocatable RISC-V binary loaded from SD, communicates via C ABI syscall table |
| App Data / Assets | SD Card | Config files, fonts, images, downloaded content, pre-rendered books |

### 3.3 Memory Map

The ESP32-C3 has approximately 400 KB of SRAM. PulpOS reports ~172 KB usable heap after system initialization (108 KB main + 64 KB reclaimed after Wi-Fi teardown). PaperDOS targets a similar layout, with the strip-based rendering approach eliminating the need for a full framebuffer.

| Region | Size (approx.) | Purpose |
|--------|----------------|---------|
| Kernel (.text + .rodata + .data + .bss) | ~60 KB | Rust kernel code, Embassy executor, drivers, syscall table |
| Kernel stack | ~8 KB | Main stack + interrupt handlers |
| Embassy task futures | ~2 KB | Five async tasks (~200 bytes each + overhead) |
| Static allocations | ~40 KB | Large structs via `StaticCell` (display strip buffer 4 KB, SD sector buffer, font cache, etc.) |
| Wi-Fi / Networking | ~64 KB | esp-wifi buffers (released when Wi-Fi not active) |
| **APP REGION** | **~108–172 KB** | **Loaded binary .text + .data + .bss + heap** |
| Safety margin / stack guard | ~4 KB | Canary zone to detect stack overflow |

**Note:** Wi-Fi buffers are allocated dynamically by esp-wifi. When Wi-Fi is not active (e.g., during reading), that ~64 KB is reclaimed for the app region, giving apps up to ~172 KB. The kernel exposes `sys_wifi_release()` / `sys_wifi_acquire()` syscalls so apps can choose to trade Wi-Fi for memory.

### 3.4 Embassy Task Model

The kernel runs five concurrent Embassy tasks, adapted from PulpOS's proven architecture:

| Task | Interval | Purpose |
|------|----------|---------|
| Main event loop | Event-driven | App loading, launcher UI, syscall dispatch |
| Input poller | 10 ms | ADC button polling with 15 ms debounce, 1-second long-press detection |
| Housekeeping | 1 s | Status bar updates, battery level, bookmark auto-save |
| Idle monitor | Configurable | Deep sleep timeout when no input detected |
| Background worker | On demand | CPU-heavy operations (image dithering, file operations) isolated from UI |

The CPU enters WFI (wait-for-interrupt) whenever all tasks are awaiting, minimizing power draw to the microamps range between events.

---

## 4. The PaperDOS Kernel

### 4.1 Implementation Language

The kernel is written in **Rust (no_std)** targeting `riscv32imc-unknown-none-elf`. This choice is validated by two community firmware projects (PulpOS and TernReader) that demonstrate Rust is production-ready on the X4's ESP32-C3. Key advantages over C/C++:

- **Memory safety at compile time.** With no MMU for hardware protection, Rust's ownership model catches buffer overflows, use-after-free, and data races before they reach the device.
- **Zero-cost abstractions.** Generics, iterators, and pattern matching compile to the same efficient RISC-V instructions as hand-written C, but with stronger guarantees.
- **No runtime overhead.** `no_std` Rust has no garbage collector, no standard library heap allocator (we bring our own), and no hidden costs.
- **Ecosystem.** The `esp-rs` project provides first-class ESP32-C3 support via `esp-hal`, `esp-wifi`, and `esp-alloc`.

### 4.2 Core Dependencies

| Crate | Purpose |
|-------|---------|
| `esp-hal` | Hardware Abstraction Layer — GPIO, SPI, ADC, timers, DMA |
| `embassy-executor` | Async task executor, cooperative multitasking |
| `embassy-time` | Timer primitives for delays, timeouts, periodic tasks |
| `esp-wifi` | Wi-Fi driver (allocated on demand, teardown supported) |
| `embedded-sdmmc` | FAT filesystem on SD card via SPI |
| `embedded-hal-bus` | `CriticalSectionDevice` for SPI bus sharing |
| `esp-alloc` | Heap allocator for dynamic allocations |
| `heapless` | Fixed-capacity collections (Vec, String) for stack-allocated buffers |

### 4.3 Responsibilities

The kernel boots from flash and remains resident for the lifetime of the device. It is responsible for:

- Hardware initialization (display, SPI bus, buttons, SD card, power management) via esp-hal
- Strip-based display rendering (4 KB strips, DMA-backed SPI transfers)
- 3-phase partial refresh management for the SSD1677
- Providing the syscall table — a fixed struct of `extern "C"` function pointers exposed to loaded apps
- The binary loader — reading .pdb app binaries from SD, relocating them, and branching to their entry point
- The home screen / app launcher UI
- Embassy async task orchestration (input, housekeeping, idle, background work)
- Wi-Fi connection management via esp-wifi (on-demand allocation/teardown)
- Watchdog timer to recover from app crashes
- OTA self-update mechanism for kernel upgrades
- Wi-Fi upload server (HTTP on port 80 + mDNS) for developer workflow

### 4.4 Display Driver

The display driver is a custom SSD1677 implementation using strip-based rendering, adapted from PulpOS's proven approach.

#### 4.4.1 Strip-Based Rendering

Rather than maintaining a full 800×480 framebuffer (~48 KB), the display is rendered in 12 horizontal strips of 40 rows each. Each strip occupies only 4 KB (800 × 40 ÷ 8 bytes). The rendering pipeline:

1. For each strip (top to bottom), the kernel calls a draw callback that fills the 4 KB buffer.
2. While DMA transfers the current strip to the SSD1677 via SPI, the CPU renders the next strip.
3. This double-buffered approach overlaps computation and I/O, keeping both the CPU and SPI bus busy.

This reclaims ~44 KB of RAM compared to a full framebuffer approach. The syscall table's `display_draw_*` functions manage strip rendering internally — apps draw using pixel coordinates as if they had a full framebuffer, and the kernel handles the strip decomposition transparently.

#### 4.4.2 3-Phase Partial Refresh

The SSD1677's partial refresh is split into three phases for optimal speed and image quality:

1. **Write BW RAM.** Push the new pixel data to the display's black/white SRAM.
2. **Apply DU waveform.** Trigger a direct-update (DU) partial refresh (~400 ms). During this window, input is collected.
3. **Sync RED RAM.** Update the display's secondary (red/gray) SRAM to match. This can be deferred during rapid navigation.

A full GC (ghosting-clear) refresh is promoted after a configurable number of partial refreshes to prevent ghost images from accumulating.

The `display_refresh(mode)` syscall exposes three modes: `0` = full GC refresh, `1` = partial (3-phase), `2` = fast (phase 1+2 only, skip RED sync).

#### 4.4.3 Grayscale Support

The SSD1677 supports 4-level grayscale via three bitplanes (base BW + LSB + MSB). The kernel supports both 1-bit and 4-gray rendering, with grayscale primarily used for images and the .tri format. Text rendering defaults to 1-bit for maximum sharpness.

### 4.5 The Syscall Table

The syscall table is the heart of PaperDOS. It is a C struct of function pointers, defined with `#[repr(C)]` on the Rust side, located at a fixed, known memory address. When an app is loaded, the kernel passes a pointer to this struct as the single argument to the app's entry function. The app then calls kernel services through this table.

This is directly analogous to the DOS INT 21h vector, but implemented as a C vtable for simplicity and performance. Because Rust does not have a stable ABI, the syscall boundary uses `extern "C"` functions exclusively — this allows apps to be written in C, Rust (with `extern "C"`), or any language that can produce RISC-V binaries with C calling convention.

#### 4.5.1 Syscall Table Structure (v1)

The Rust kernel defines the table as:

```rust
#[repr(C)]
pub struct PdSyscalls {
    pub abi_version: u32,
    pub kernel_version: u32,
    pub app_heap_start: u32,
    pub app_heap_size: u32,

    // ── Display ──
    pub display_clear: extern "C" fn(color: u8),
    pub display_set_pixel: extern "C" fn(x: i32, y: i32, color: u8),
    pub display_draw_rect: extern "C" fn(x: i32, y: i32, w: i32, h: i32, color: u8),
    pub display_fill_rect: extern "C" fn(x: i32, y: i32, w: i32, h: i32, color: u8),
    pub display_draw_bitmap: extern "C" fn(x: i32, y: i32, w: i32, h: i32, data: *const u8),
    pub display_draw_text: extern "C" fn(x: i32, y: i32, s: *const u8, font: *const PdFont),
    pub display_refresh: extern "C" fn(mode: i32),
    pub display_set_rotation: extern "C" fn(r: i32),
    pub display_width: extern "C" fn() -> i32,
    pub display_height: extern "C" fn() -> i32,

    // ── Input ──
    pub input_get_buttons: extern "C" fn() -> u32,
    pub input_wait_button: extern "C" fn(timeout_ms: i32) -> u32,
    pub input_get_battery_pct: extern "C" fn() -> i32,

    // ── Filesystem (SD Card) ──
    pub fs_open: extern "C" fn(path: *const u8, mode: *const u8) -> *mut PdFile,
    pub fs_close: extern "C" fn(f: *mut PdFile) -> i32,
    pub fs_read: extern "C" fn(f: *mut PdFile, buf: *mut u8, size: i32) -> i32,
    pub fs_write: extern "C" fn(f: *mut PdFile, buf: *const u8, size: i32) -> i32,
    pub fs_seek: extern "C" fn(f: *mut PdFile, offset: i32, whence: i32) -> i32,
    pub fs_tell: extern "C" fn(f: *mut PdFile) -> i32,
    pub fs_eof: extern "C" fn(f: *mut PdFile) -> i32,
    pub fs_mkdir: extern "C" fn(path: *const u8) -> i32,
    pub fs_remove: extern "C" fn(path: *const u8) -> i32,
    pub fs_opendir: extern "C" fn(path: *const u8) -> *mut PdDir,
    pub fs_readdir: extern "C" fn(d: *mut PdDir, entry: *mut PdDirent) -> i32,
    pub fs_closedir: extern "C" fn(d: *mut PdDir) -> i32,
    pub fs_stat: extern "C" fn(path: *const u8, st: *mut PdStat) -> i32,

    // ── Network ──
    pub net_wifi_connect: extern "C" fn(ssid: *const u8, pass: *const u8) -> i32,
    pub net_wifi_disconnect: extern "C" fn() -> i32,
    pub net_wifi_status: extern "C" fn() -> i32,
    pub net_http_get: extern "C" fn(url: *const u8, buf: *mut u8, buf_size: i32) -> i32,
    pub net_http_post: extern "C" fn(url: *const u8, body: *const u8, body_len: i32,
                                      resp_buf: *mut u8, resp_size: i32) -> i32,
    pub net_http_begin: extern "C" fn(url: *const u8, method: *const u8) -> *mut PdHttp,
    pub net_http_set_header: extern "C" fn(h: *mut PdHttp, key: *const u8, val: *const u8) -> i32,
    pub net_http_send: extern "C" fn(h: *mut PdHttp, body: *const u8, len: i32) -> i32,
    pub net_http_read: extern "C" fn(h: *mut PdHttp, buf: *mut u8, size: i32) -> i32,
    pub net_http_status: extern "C" fn(h: *mut PdHttp) -> i32,
    pub net_http_end: extern "C" fn(h: *mut PdHttp) -> i32,

    // ── System ──
    pub sys_sleep_ms: extern "C" fn(ms: i32),
    pub sys_millis: extern "C" fn() -> u32,
    pub sys_exit: extern "C" fn(code: i32),
    pub sys_reboot: extern "C" fn(),
    pub sys_log: extern "C" fn(level: i32, fmt: *const u8, ...),
    pub sys_get_free_heap: extern "C" fn() -> i32,
    pub sys_wifi_release: extern "C" fn(),
    pub sys_wifi_acquire: extern "C" fn() -> i32,

    // ── Memory ──
    pub mem_alloc: extern "C" fn(size: i32) -> *mut u8,
    pub mem_free: extern "C" fn(ptr: *mut u8),
    pub mem_realloc: extern "C" fn(ptr: *mut u8, size: i32) -> *mut u8,

    // ── Font / Assets ──
    pub font_load: extern "C" fn(path: *const u8) -> *const PdFont,
    pub font_free: extern "C" fn(font: *const PdFont),
    pub font_text_width: extern "C" fn(font: *const PdFont, s: *const u8) -> i32,
    pub font_line_height: extern "C" fn(font: *const PdFont) -> i32,
}
```

The C SDK header (`paperdos.h`) mirrors this struct identically using C function pointer types, so app developers can work in either language.

*Total: ~70 function pointers × 4 bytes = ~280 bytes for the table itself. Trivial footprint.*

#### 4.5.2 Kernel-Side Implementation

Each syscall function is implemented as an `extern "C" fn` in Rust. For example:

```rust
extern "C" fn display_clear_impl(color: u8) {
    // Access the global display driver via a StaticCell
    critical_section::with(|cs| {
        let display = DISPLAY.borrow(cs);
        display.clear(color);
    });
}

extern "C" fn sys_sleep_ms_impl(ms: i32) {
    // Block the app for the given duration
    // (apps run synchronously; Embassy tasks handle async)
    embassy_time::block_for(Duration::from_millis(ms as u64));
}
```

The syscall table is constructed at kernel startup, with each field pointing to the corresponding `_impl` function. The table is then placed at a well-known address and passed to apps via register `a0`.

#### 4.5.3 ABI Versioning

The syscall table is append-only. New functions are added at the end; existing function slots never move or change semantics. Each app binary declares the minimum ABI version it requires. The kernel checks this at load time:

- If `app.abi_version <= kernel.abi_version`: load and run. All syscalls the app knows about are present.
- If `app.abi_version > kernel.abi_version`: refuse to load. Display "Please update PaperDOS" message.

### 4.6 Font Handling

Fonts are rasterized at build time using the `fontdue` crate in `build.rs`. TTF/OTF fonts are converted to 1-bit bitmap glyph atlases baked into the kernel binary. This eliminates runtime font parsing overhead and ensures consistent rendering. The `font_load` syscall loads pre-rasterized font data from the kernel's flash storage or from .bdf files on the SD card.

### 4.7 Static Allocation Strategy

Following PulpOS's approach, the kernel uses `ConstStaticCell` and `StaticCell` for large structures instead of heap allocation. This keeps Embassy task futures small (~200 bytes per task) and avoids heap fragmentation:

- **Strip buffer:** 4 KB via `StaticCell`
- **SD sector buffer:** 512 bytes via `StaticCell`
- **Font cache:** configurable, via `StaticCell`
- **HTTP response buffer:** 4 KB via `StaticCell` (allocated only when Wi-Fi is active)

### 4.8 Stack Watermark Monitoring

In debug builds, the kernel paints the stack with `0xDEADBEEF` at boot and logs the high-water mark every 5 seconds. This is essential for tuning stack sizes in a no-MMU environment where stack overflows silently corrupt memory.

---

## 5. The PaperDOS Binary Format (.PDB)

### 5.1 Design Goals

The binary format must be simple enough to load with minimal code (the loader runs in the kernel's limited memory), yet flexible enough to support relocation and metadata. Inspired by the DOS .COM format's simplicity, with a small header for modern needs.

### 5.2 File Structure

A .pdb (PaperDOS Binary) file consists of a fixed header followed by the loadable image:

| Offset | Size | Field | Description |
|--------|------|-------|-------------|
| 0x00 | 4 bytes | magic | "PDOS" (0x504F4450) |
| 0x04 | 2 bytes | format_version | Binary format version (1) |
| 0x06 | 2 bytes | abi_version | Minimum kernel ABI required |
| 0x08 | 4 bytes | entry_offset | Offset from image base to entry() |
| 0x0C | 4 bytes | text_size | Size of .text (code) section |
| 0x10 | 4 bytes | data_size | Size of .data (initialized data) |
| 0x14 | 4 bytes | bss_size | Size of .bss (zero-initialized data) |
| 0x18 | 4 bytes | reloc_count | Number of relocation entries |
| 0x1C | 4 bytes | flags | Bit 0: needs WiFi, Bit 1: needs BT |
| 0x20 | 32 bytes | app_name | Null-terminated UTF-8 app name |
| 0x40 | 32 bytes | app_version | Null-terminated version string |
| 0x60 | 4 bytes | min_heap | Minimum heap bytes required |
| 0x64 | 4 bytes | checksum | CRC32 of everything after header |
| 0x68 | ... | reloc_table | Array of uint32 offsets to patch |
| ... | ... | image | .text + .data (loadable image) |

### 5.3 Loading Process

When a user selects an app from the launcher, the kernel's loader (implemented in Rust) performs these steps:

1. **Read header.** Validate magic, check ABI version, verify CRC32.
2. **Check resources.** Ensure enough free RAM for text_size + data_size + bss_size + min_heap. If the app requests Wi-Fi (flags bit 0), ensure esp-wifi is initialized (call `sys_wifi_acquire` internally).
3. **Load image.** Read .text + .data into the app region. Zero-fill .bss after .data.
4. **Relocate.** For each entry in reloc_table, add the actual load address to the 32-bit value at that offset. This converts position-independent addresses into absolute addresses.
5. **Arm watchdog.** Configure the hardware watchdog with a 10-second timeout.
6. **Prepare syscall pointer.** Place the address of the syscall table in register `a0` (the first argument per RISC-V calling convention).
7. **Jump.** Use an `unsafe` inline assembly block to branch to `image_base + entry_offset`. The app is now running.
8. **On return.** When the app calls `sys_exit()` or returns from its entry function, the kernel reclaims the app region, resets the display, disarms the watchdog, and returns to the launcher.

```rust
/// Jump to a loaded .pdb app's entry point.
/// # Safety
/// The caller must ensure the app region contains valid, relocated RISC-V code.
unsafe fn jump_to_app(entry: *const u8, syscalls: *const PdSyscalls) -> i32 {
    let result: i32;
    core::arch::asm!(
        "jalr {entry}",
        entry = in(reg) entry,
        in("a0") syscalls,
        lateout("a0") result,
        // Clobber caller-saved registers
        clobber_abi("C"),
    );
    result
}
```

### 5.4 Comparison with DOS

| Aspect | MS-DOS .COM | MS-DOS .EXE | PaperDOS .PDB |
|--------|-------------|-------------|---------------|
| Max size | 64 KB | ~640 KB | ~172 KB (RAM-limited) |
| Header | None | MZ header + reloc table | 104-byte header + reloc table |
| Relocation | None (org 100h) | Segment-based | Flat 32-bit offset patching |
| Entry point | 0x0100 always | Header-specified | Header-specified |
| API access | INT 21h | INT 21h | Syscall table pointer (a0) |
| Memory model | Tiny (64K total) | Small/Large | Flat (single region) |

---

## 6. SDK and Toolchain

### 6.1 Overview

App developers can write PaperDOS apps in **C** (primary path) or **Rust** (advanced path). Both produce the same .pdb binary format and call the same syscall table through the C ABI boundary. The goal is to make the development experience as simple as possible — ideally, a developer should be able to write a single source file, run one build command, and get a .pdb file ready to test.

### 6.2 C Toolchain (Primary)

- **Compiler:** `riscv32-esp-elf-gcc` (ships with ESP-IDF, or the standalone RISC-V GCC). The ESP32-C3 uses the RV32IMC instruction set.
- **Linker script:** A custom linker script (`paperdos.ld`) that produces a flat binary with a known layout: .text, then .data, then .bss, with all symbols relative to address 0.
- **Entry trampoline:** An assembly file (`pd_entry.S`) that receives the syscall table pointer in `a0`, saves it to a callee-saved register (`s0`), calls `pd_main(syscalls)`, and invokes `sys_exit` on return.
- **Post-processor:** A Python script (`pdpack.py`) that takes the ELF output, extracts sections, generates the relocation table, prepends the .pdb header, and computes the CRC32.
- **SDK header:** A single C header file (`paperdos.h`) that defines the `pd_syscalls_t` struct (matching the kernel's `#[repr(C)]` layout) and convenience macros.

### 6.3 Rust Toolchain (Advanced)

Rust developers can write apps using a thin Rust SDK crate (`paperdos-app`) that wraps the syscall table in safe Rust types:

```rust
// paperdos-app/src/lib.rs
#![no_std]

#[repr(C)]
pub struct Syscalls { /* mirrors pd_syscalls_t */ }

impl Syscalls {
    pub fn display_clear(&self, color: u8) {
        (self.display_clear)(color);
    }
    pub fn display_draw_text(&self, x: i32, y: i32, text: &str, font: &PdFont) {
        (self.display_draw_text)(x, y, text.as_ptr(), font as *const _);
    }
    // ... safe wrappers for all syscalls
}
```

Apps compile with `cargo build --target riscv32imc-unknown-none-elf` using the same linker script, then run through `pdpack.py` to produce a .pdb. The entry point must be `#[no_mangle] extern "C" fn pd_main(sys: *const Syscalls)`.

### 6.4 Minimal App Example (C)

A complete PaperDOS application in a single file:

```c
#include "paperdos.h"

void pd_main(pd_syscalls_t *sys) {
    const pd_font_t *font = sys->font_load("/fonts/default.bdf");
    sys->display_clear(0xFF);  // white
    sys->display_draw_text(10, 10, "Hello from PaperDOS!", font);
    sys->display_refresh(0);   // full refresh

    // Wait for any button press, then exit
    sys->input_wait_button(0);  // 0 = no timeout
    sys->font_free(font);
    sys->sys_exit(0);
}
```

### 6.5 Build Process (C)

```bash
$ riscv32-esp-elf-gcc -march=rv32imc -Os -nostdlib \
    -T paperdos.ld -o hello.elf pd_entry.S hello.c
$ python3 pdpack.py hello.elf hello.pdb \
    --name "Hello World" --version "1.0" --abi 1
```

### 6.6 Build Process (Rust)

```bash
$ cargo build --release --target riscv32imc-unknown-none-elf
$ python3 pdpack.py target/riscv32imc-unknown-none-elf/release/hello hello.pdb \
    --name "Hello World" --version "1.0" --abi 1
```

### 6.7 Testing

Developers can test their apps through several methods:

- **On-device via Wi-Fi push.** The kernel's built-in HTTP upload server (port 80, mDNS-discoverable) accepts .pdb uploads over the local network. No SD card removal needed — the fastest development loop.
- **On-device via SD card.** Copy the .pdb file to `/paperdos/apps/` on the SD card, insert into the X4, and select from the launcher.
- **Desktop emulator.** A desktop emulator (using SDL2 for display and input) that implements the same syscall table in Rust, allowing rapid iteration without hardware. The emulator renders at the exact 800×480 resolution with simulated e-ink latency. Built from the shared `paperdos-core` crate for pixel-perfect parity.
- **CI validation.** The `pdpack.py` tool includes a `--validate` flag that checks header integrity, relocation sanity, and estimated memory usage against the target device.

---

## 7. The PaperDOS Store

### 7.1 Overview

The Store is a built-in application (bundled in the kernel flash) that provides an app-store experience adapted to the constraints of a button-navigated e-ink device. It connects to a central server over Wi-Fi, presents a catalog of available apps, and handles downloading and installing .pdb files to the SD card.

### 7.2 Server Architecture

The backend is deliberately simple — a static file server with a JSON manifest is sufficient for v1. No accounts, no payment, no authentication. Pure open-source app distribution.

#### 7.2.1 Manifest Format

```json
{
  "store_version": 1,
  "updated": "2026-03-12T00:00:00Z",
  "apps": [
    {
      "id": "epub-reader",
      "name": "PaperRead",
      "version": "1.2.0",
      "abi_version": 1,
      "author": "PaperDOS Team",
      "description": "Pre-rendered book reader with bookmarks...",
      "size_bytes": 45000,
      "min_heap": 80000,
      "category": "reading",
      "download_url": "/apps/epub-reader-1.2.0.pdb",
      "checksum": "a1b2c3d4...",
      "icon_1bit": "/icons/epub-reader.pbm"
    }
  ]
}
```

#### 7.2.2 Discovery Flow

1. User opens Store app from launcher.
2. Kernel connects to configured Wi-Fi (or prompts for credentials) via esp-wifi.
3. Store fetches `manifest.json` from the server (HTTP GET, ~2–5 KB).
4. Parses JSON using a minimal no_std JSON parser (e.g., `serde-json-core` or `miniserde`).
5. Displays scrollable list of apps with name, author, size, and installed/update status.
6. User navigates with buttons, selects an app to view details or install.
7. Download: HTTP GET to fetch the .pdb file, streamed directly to SD card via embedded-sdmmc.
8. Verify CRC32, register in local app index, return to store or launcher.

### 7.3 SD Card Layout

```
/paperdos/
  config.json              # Wi-Fi creds, preferences
  apps/
    epub-reader.pdb        # Installed app binaries
    chess.pdb
    weather.pdb
  appdata/
    epub-reader/           # Per-app data directory
      bookmarks.json
    chess/
      saves/
  books/
    *.trbk                 # Pre-rendered book files
  images/
    *.tri                  # Device-optimized images
  fonts/
    default.bdf            # System font
    serif.bdf
  icons/
    epub-reader.pbm        # 1-bit app icons
  cache/
    manifest.json          # Cached store manifest
```

---

## 8. Flagship Applications

### 8.1 PaperRead (Pre-Rendered Book Reader)

The primary application and the device's raison d'être. PaperRead adopts TernReader's pre-rendered book format approach: all EPUB parsing, text layout, and page composition happen on the desktop via a companion tool. The device receives a compact .trbk file containing pre-rasterized page data, ready for direct rendering.

#### 8.1.1 Why Pre-Rendered?

On-device EPUB rendering requires an XHTML parser, CSS engine, text layout engine, font rasterizer, and ZIP decompressor — easily 30–50 KB of code and significant RAM for parsing state. Pre-rendering eliminates all of this. The device-side reader is a simple page viewer: seek to page N in the .trbk file, read draw operations, blit to display. This approach was proven by TernReader to produce a superior reading experience on the X4.

#### 8.1.2 The .trbk Book Format

The .trbk format (adapted from TernReader) contains:

- **Header:** Metadata (title, author, page count, screen dimensions)
- **Page index:** Offset table for O(1) page seeking
- **Page data:** Per-page draw operations:
  - `TextRun(x, y, glyph_ids[], font_id)` — pre-positioned text with pre-rasterized glyphs
  - `Image(x, y, w, h, pixel_data[])` — inline images, pre-dithered to 1-bit or 4-gray
  - `Rule(x1, y1, x2, y2)` — horizontal/vertical lines
- **Font table:** Embedded 1-bit glyph bitmaps for all characters used in the book
- **Table of contents:** Chapter titles with page numbers

The desktop companion tool (`pdbook.py` or a Rust CLI sharing the `paperdos-core` crate) converts EPUB to .trbk at the exact 800×480 resolution, with configurable font size and margins.

#### 8.1.3 Features

- Fast page turns via partial refresh (no parsing delay)
- Persistent bookmarks and reading position (saved to appdata/)
- Table of contents navigation
- Library view with book metadata (title, author)
- Font size selection (requires re-conversion on desktop)
- Estimated ~25 KB code footprint (page viewer + UI), well within app region

#### 8.1.4 Content Pipeline

```
User's Computer                          X4 Device
┌──────────────┐                    ┌──────────────┐
│  book.epub   │                    │              │
│      │       │                    │  PaperRead   │
│      ▼       │                    │  (25 KB app) │
│  pdbook.py   │   SD card or      │      │       │
│  (or WASM    │──Wi-Fi push──────▶│      ▼       │
│   in browser)│                    │  book.trbk   │
│      │       │                    │  (on SD)     │
│      ▼       │                    │      │       │
│  book.trbk   │                    │      ▼       │
└──────────────┘                    │  Display     │
                                    └──────────────┘
```

The web flasher (see PRD_Web_Flasher.md) also provides browser-based EPUB-to-.trbk conversion via a WASM module compiled from the shared `paperdos-core` crate.

### 8.2 PaperDOS Store

As described in Section 7. The Store is both a built-in kernel feature and a reference implementation of a PaperDOS application, serving as a tutorial for other developers.

### 8.3 Image Viewing (.tri Format)

PaperDOS supports a simple image format (`.tri`, adapted from TernReader) for app icons, UI assets, and standalone images:

- **16-byte header:** magic, version, format (1-bit or 4-gray), width, height
- **Pixel data:** Bitpacked, directly blittable to the SSD1677 — zero decompression overhead
- **Conversion:** A desktop tool (`pdimage.py` or the web flasher's WASM module) converts PNG/JPEG to .tri, applying Floyd-Steinberg dithering for optimal 1-bit or 4-gray output

### 8.4 Future App Ideas

| App | Category | Description |
|-----|----------|-------------|
| PaperWeather | Utility | Weather dashboard with daily/hourly forecast via API |
| PaperNews | Reading | RSS/Atom feed reader with offline caching |
| PaperNote | Productivity | Simple text note viewer/editor |
| PaperClock | Utility | Always-on clock with alarms (leverages e-ink power efficiency) |
| PaperChess | Games | Chess puzzles or play-vs-engine (micro chess engine fits in ~50 KB) |
| PaperCards | Games | Flashcard study app (Anki-compatible deck import) |
| PaperCalc | Utility | Scientific calculator |
| PaperPomodoro | Productivity | Pomodoro timer with session tracking |
| PaperMQTT | IoT | MQTT dashboard for home automation status display |

---

## 9. Security Considerations

PaperDOS intentionally operates without memory protection or privilege separation, like its DOS namesake. However, several practical safeguards are included:

### 9.1 App Verification

- **CRC32 integrity.** Every .pdb file includes a checksum verified at load time. Corrupt downloads are rejected.
- **Manifest checksums.** The store manifest includes per-app checksums. Downloaded files are verified against the manifest before installation.
- **HTTPS.** Store communication uses HTTPS (the ESP32-C3 supports TLS via the `esp-tls` crate).
- **Optional signing.** Future versions may support Ed25519 code signing. The kernel would carry the store's public key and reject unsigned binaries. This is deferred to v2 to reduce initial complexity.

### 9.2 Runtime Safeguards

- **Watchdog timer.** The kernel configures a hardware watchdog via esp-hal. If an app fails to yield or feed the watchdog within 10 seconds, the system hard-resets to the launcher.
- **Stack canary.** A known pattern (`0xDEADBEEF`) is written at the boundary between the app region and kernel memory. Checked on syscall entry — if corrupted, the app is terminated.
- **Filesystem sandboxing.** The `fs_*` syscalls restrict apps to `/paperdos/` on the SD card. Attempts to access paths outside this directory return an error. Apps cannot overwrite other apps' binaries (the `/paperdos/apps/` directory is read-only to apps; only the Store and kernel can write there).
- **Memory bounds checking.** The `mem_alloc` syscall only allocates from the app heap region. Requests exceeding the app's allocated heap are rejected.

### 9.3 Threat Model

The primary threats for this device class are malicious apps that could brick the device (mitigated by watchdog + dual OTA partitions allowing recovery) or exfiltrate Wi-Fi credentials (mitigated by filesystem sandboxing — config.json is readable only by the kernel). A full security audit is recommended before any production deployment.

---

## 10. Development Roadmap

### Phase 1: Foundation (Weeks 1–4)

- Set up Cargo project with `esp-hal` and Embassy for ESP32-C3 (`riscv32imc-unknown-none-elf`)
- Implement SSD1677 display driver with strip-based rendering (4 KB strips, DMA)
- Implement 3-phase partial refresh
- Implement button input via ADC with 10ms polling and debounce
- Implement SD card access via embedded-sdmmc with SPI bus sharing
- Define `#[repr(C)]` syscall table struct and implement core syscalls (display, input, filesystem)
- Build the .pdb binary loader in Rust (header parsing, relocation, `unsafe` jump-to-entry)
- Create minimal launcher UI (list .pdb files on SD, select and run)
- Add stack watermark monitoring for debug builds

### Phase 2: SDK + Hello World (Weeks 5–6)

- Write `paperdos.h` C SDK header (mirrors the `#[repr(C)]` struct)
- Write `paperdos.ld` linker script and `pd_entry.S` entry trampoline
- Write `pdpack.py` ELF-to-.pdb post-processor
- Create `paperdos-app` Rust SDK crate with safe wrappers
- Build and run a "Hello World" .pdb on real hardware (both C and Rust versions)
- Write developer documentation and build instructions

### Phase 3: Store + Networking (Weeks 7–10)

- Integrate esp-wifi and implement Wi-Fi syscalls (connect, disconnect, status)
- Implement HTTP client syscalls using esp-wifi's networking stack
- Implement Wi-Fi upload server (HTTP port 80 + mDNS) for developer push workflow
- Build the Store app (manifest fetch, list display, download flow)
- Set up a simple static-file store server with manifest.json
- Implement OTA kernel self-update via espflash-compatible image format

### Phase 4: PaperRead + Content Pipeline (Weeks 11–14)

- Implement the `paperdos-core` shared crate (compiles to both device and WASM/desktop)
- Build `pdbook.py` (or Rust CLI) for EPUB-to-.trbk conversion at 800×480
- Build `pdimage.py` (or Rust CLI) for PNG/JPEG-to-.tri conversion with Floyd-Steinberg dithering
- Implement PaperRead: .trbk page viewer with bookmarks, TOC navigation, library UI
- Build WASM module from `paperdos-core` for browser-based conversion (see Web Flasher PRD)

### Phase 5: Polish + Community (Weeks 15+)

- Desktop emulator (SDL2-based) built from `paperdos-core` for developer testing
- Web flasher deployment (WebSerial + esptool-js, see PRD_Web_Flasher.md)
- App submission guidelines and store curation process
- Community app development tutorials (C and Rust paths)
- Optional: Ed25519 code signing
- Optional: Berry scripting engine as a loadable app for rapid prototyping

---

## 11. Open Questions

The following design decisions are deferred for community discussion:

- **Grayscale rendering mode.** The SSD1677 supports 4-level grayscale. The kernel implements both 1-bit and 4-gray, but should the default mode be configurable per-app via the .pdb header flags? What's the performance tradeoff for strip-based grayscale rendering?
- **Bluetooth functionality.** The ESP32-C3 has BLE (supported via `esp-wifi`'s BLE feature). Potential uses include wireless file transfer from phone, keyboard input for a note-taking app, or device-to-device communication. Worth the RAM cost?
- **Power management API.** Should apps be able to control deep sleep, or should the kernel manage it via the idle monitor Embassy task? Deep sleep saves power but requires careful state preservation.
- **Localization.** UTF-8 support in the font/text rendering syscalls is essential, but full Unicode coverage requires large font files. What's the right baseline character set? The `fontdue` build-time rasterization can subset fonts, but which code pages to include by default?
- **Multi-app data sharing.** Should apps be able to read each other's appdata directories? Useful for e.g., a dictionary app that can be invoked from the EPUB reader, but complicates sandboxing.
- **Filesystem crate choice.** `embedded-sdmmc` (pure Rust, used by PulpOS) vs. FatFs C bindings (used by TernReader, more feature-complete). The pure Rust option is preferred for consistency, but `embedded-sdmmc` has known limitations with long filenames and some FAT32 edge cases.
- **Community governance.** How should the store be curated? Fully open? Reviewed submissions? Separate official vs. community channels?

---

## Appendix A: Prior Art Analysis

Several open-source firmware projects targeting the Xteink X4 have emerged. Two Rust-based projects are particularly relevant to PaperDOS and have directly informed its architecture: PulpOS and TernReader.

### A.1 PulpOS (github.com/hansmrtn/pulp-os)

PulpOS is a minimal, bare-metal e-ink OS written entirely in Rust (no_std). It targets the same ESP32-C3 + SSD1677 hardware and is the most technically sophisticated community firmware.

**Key contributions adopted by PaperDOS:**

- Strip-based rendering (4 KB per strip, no framebuffer) → PaperDOS display driver (Section 4.4.1)
- 3-phase partial refresh → PaperDOS refresh modes (Section 4.4.2)
- Embassy async with 5 concurrent tasks → PaperDOS task model (Section 3.4)
- SPI bus arbitration via CriticalSectionDevice → PaperDOS SPI sharing (Section 2.3)
- DMA-backed SPI transfers → PaperDOS display driver
- ADC button polling (10ms/15ms debounce) → PaperDOS input syscalls
- Stack watermark monitoring (0xDEADBEEF) → PaperDOS debug builds (Section 4.8)
- Static allocation via StaticCell → PaperDOS kernel design (Section 4.7)
- Wi-Fi upload server (HTTP + mDNS) → PaperDOS developer workflow (Section 4.3)

**Key difference:** PulpOS is monolithic — all apps are compiled into a single binary. PaperDOS's core differentiator is the .pdb dynamic loader and open app ecosystem.

### A.2 TernReader (github.com/azw413/TernReader)

TernReader is a Rust-based firmware focused on digital wallet and EPUB reader use cases. Its most innovative contribution is the pre-rendered book format.

**Key contributions adopted by PaperDOS:**

- .trbk pre-rendered book format → PaperRead content pipeline (Section 8.1)
- .tri image format (1-bit/4-gray, direct blit) → PaperDOS image format (Section 8.3)
- Shared core crate (device + desktop + WASM) → `paperdos-core` architecture (Section 8.1.4)
- Web-based flashing (WebSerial) → PaperDOS Web Flasher (see PRD_Web_Flasher.md)
- 4-level grayscale rendering → PaperDOS grayscale support (Section 4.4.3)

**Key difference:** TernReader is also monolithic with a narrow scope (wallet + reader). PaperDOS generalizes these techniques into a platform for arbitrary third-party apps.

### A.3 Other Community Firmwares

| Project | Language | Notable Features |
|---------|----------|------------------|
| CrossPoint Reader | C/C++ (ESP-IDF) | Drop-in replacement for stock firmware; most feature-complete |
| Papyrix Reader | C/C++ | Lightweight; EPUB/FB2/MD/TXT support; custom themes and fonts |
| Stock Sample Code | C++ (Arduino/PlatformIO) | Official GPIO/SPI pin mappings; GxEPD2 library integration |

### A.4 Community Validation

The community projects collectively validate several important points:

- **Rust is viable and preferable.** Both PulpOS and TernReader demonstrate that no_std Rust on ESP32-C3 is production-ready. PaperDOS builds on this foundation.
- **~172 KB usable heap is the real number.** PulpOS reports ~172 KB heap (108 KB main + 64 KB reclaimed after Wi-Fi teardown). PaperDOS's memory map reflects this.
- **Pre-rendered content formats win.** TernReader's .trbk format proves that offloading parsing and layout to the desktop produces a dramatically better reading experience. PaperDOS adopts this as the default strategy.
- **Embassy async is the right execution model.** PulpOS's five-task architecture handles concurrent concerns elegantly. PaperDOS uses the same model.
- **Community wants hackability.** Six+ independent firmware projects in under three months shows strong demand for an open, extensible platform.

---

*This document is a living draft intended to seed community discussion and development. Contributions, criticism, and pull requests are welcome.*

**References:** [PulpOS](https://github.com/hansmrtn/pulp-os) (MIT License), [TernReader](https://github.com/azw413/TernReader), [CrossPoint Reader](https://github.com/crosspoint-reader/crosspoint-reader), [Papyrix](https://github.com/bigbag/papyrix-reader), [Xteink X4 Sample Code](https://github.com/CidVonHighwind/xteink-x4-sample).
