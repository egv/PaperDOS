# PaperDOS

**A lightweight, DOS-inspired runtime for downloadable applications on the Xteink X4 e-ink reader.**

PaperDOS transforms the Xteink X4 (ESP32-C3, 4.26" e-ink, ~327KB RAM) into an open application platform. Users can browse, download, and run third-party apps from a built-in store — all on a credit-card-sized e-ink device.

The kernel is planned in **Rust (no_std)** using Embassy async on esp-hal, with a C ABI boundary for dynamically loaded apps. Inspired by the approaches of [PulpOS](https://github.com/hansmrtn/pulp-os) and [TernReader](https://github.com/azw413/TernReader).

## Project Structure

```
PaperDOS/
├── docs/
│   ├── PaperDOS_Technical_Design_v0.1.md   # Full architecture & design document
│   └── PRD_Web_Flasher.md                  # Web-based flasher PRD
│
├── kernel/                     # PaperDOS kernel (Rust, targeting ESP32-C3)
│   ├── include/
│   │   └── pd_loader.h        # Binary loader API (C ABI reference)
│   └── src/
│       └── pd_loader.c        # Loader reference implementation
│
└── sdk/                        # App development SDK
    ├── include/
    │   ├── paperdos.h          # Main SDK header — syscall table & macros
    │   └── pdb_format.h        # .pdb binary format definition
    ├── linker/
    │   ├── paperdos.ld         # Linker script for app binaries
    │   └── pd_entry.S          # RISC-V entry point trampoline
    ├── tools/
    │   ├── pdpack.py           # ELF → .pdb packager & validator
    │   ├── pdbook.py           # EPUB → .trbk pre-rendered book converter
    │   └── pdimage.py          # PNG/JPEG → .tri e-ink image converter
    └── examples/
        └── hello_world/        # Minimal example app
            ├── main.c
            └── Makefile
```

## How It Works

1. The **PaperDOS kernel** lives in ESP32-C3 flash and provides a stable API (syscall table) for display, input, filesystem, networking, and memory.
2. **Apps** are native RISC-V binaries (`.pdb` files) stored on the microSD card.
3. The **loader** reads a `.pdb` from SD, relocates it into RAM, and jumps to execution.
4. Apps call kernel services through a function pointer table — like DOS INT 21h, but as a C struct.
5. The **Store** app browses a remote catalog and downloads `.pdb` files over Wi-Fi.
6. A **web flasher** (browser-based, via WebSerial + esptool-js) handles initial installation and updates — no toolchain needed.

## Quick Start (when toolchain is ready)

```bash
# Build the hello world example
cd sdk/examples/hello_world
make

# Copy to SD card
cp hello.pdb /path/to/sdcard/paperdos/apps/

# Or validate without hardware
python3 ../../tools/pdpack.py hello.elf /dev/null --validate
```

## Documentation

- [Technical Design Document](docs/PaperDOS_Technical_Design_v0.1.md) — architecture, memory map, syscall table, binary format, store, roadmap, and prior art analysis
- [Web Flasher PRD](docs/PRD_Web_Flasher.md) — browser-based firmware flashing and content conversion tool

## Recommended Stack

| Component | Technology |
|-----------|------------|
| Kernel | Rust (no_std) + Embassy async + esp-hal |
| Display | Custom SSD1677 driver, strip-based rendering (4 KB strips) |
| App format | .pdb — relocatable RISC-V binaries with C ABI |
| Book format | .trbk-inspired pre-rendered pages (desktop conversion) |
| Flasher | WebSerial + esptool-js (browser-based) |
| Build | Cargo + espflash |

## Prerequisites

- **Rust** (stable >= 1.88) with `riscv32imc-unknown-none-elf` target
- **espflash** (for flashing the kernel)
- **riscv32-esp-elf-gcc** (for building C apps against the SDK)
- **Python 3** (for pdpack.py, no external deps)

## Status

Early design phase. The design document, SDK headers, and web flasher PRD define the architecture. Kernel implementation in Rust is next.

## License

TBD — intended to be open source.
