/**
 * PaperDOS Application Linker Script
 *
 * Produces a flat binary suitable for relocation by the PaperDOS loader.
 * All addresses are relative to 0x0 - the loader will patch them at runtime.
 *
 * Usage:
 *   riscv32-esp-elf-gcc -march=rv32imc -Os -nostdlib \
 *       -T paperdos.ld -o app.elf app.c
 *   riscv32-esp-elf-objcopy -O binary app.elf app.bin
 *   python3 pdpack.py app.elf app.pdb --name "My App" --abi 1
 *
 * Memory layout after loading:
 *   [.text]  - executable code
 *   [.data]  - initialized globals
 *   [.bss]   - zero-initialized globals (zeroed by loader)
 *   [heap]   - remaining space managed by kernel allocator
 */

OUTPUT_FORMAT("elf32-littleriscv")
OUTPUT_ARCH(riscv)
ENTRY(_pd_entry)

/* Base address 0 - everything is relocated at load time */
SECTIONS
{
    . = 0x00000000;

    /* ── Code ── */
    .text : {
        _text_start = .;
        /* Entry trampoline comes first */
        *(.text.pd_entry)
        *(.text .text.*)
        *(.rodata .rodata.*)
        . = ALIGN(4);
        _text_end = .;
    }

    _text_size = _text_end - _text_start;

    /* ── Initialized data ── */
    .data : {
        _data_start = .;
        *(.data .data.*)
        *(.sdata .sdata.*)
        . = ALIGN(4);
        _data_end = .;
    }

    _data_size = _data_end - _data_start;

    /* ── Zero-initialized data ── */
    .bss : {
        _bss_start = .;
        *(.bss .bss.*)
        *(.sbss .sbss.*)
        *(COMMON)
        . = ALIGN(4);
        _bss_end = .;
    }

    _bss_size = _bss_end - _bss_start;

    /* Image end - heap starts here */
    _heap_start = .;

    /* Discard unneeded sections */
    /DISCARD/ : {
        *(.comment)
        *(.note.*)
        *(.eh_frame*)
        *(.debug*)
        *(.riscv.attributes)
    }
}
