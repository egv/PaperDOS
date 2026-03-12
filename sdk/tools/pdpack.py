#!/usr/bin/env python3
"""
pdpack.py - PaperDOS Binary Packager

Takes an ELF file compiled with paperdos.ld and produces a .pdb
(PaperDOS Binary) file ready to load on the Xteink X4.

Usage:
    python3 pdpack.py app.elf output.pdb --name "My App" --version "1.0" --abi 1

The tool:
  1. Reads the ELF file to extract .text, .data, .bss sections
  2. Finds the entry point (_pd_entry)
  3. Scans for relocations that need runtime patching
  4. Generates the .pdb header with metadata
  5. Computes CRC32 over the payload
  6. Writes the final .pdb file

Requires: python3 (no external dependencies - uses only stdlib)
"""

import argparse
import struct
import sys
import zlib
from pathlib import Path

# ─── PDB Constants ───
PDB_MAGIC = 0x534F4450  # "PDOS" little-endian
PDB_FORMAT_VERSION = 1
HEADER_SIZE = 104  # bytes

# Flag bits
FLAG_NEEDS_WIFI = 1 << 0
FLAG_NEEDS_BT = 1 << 1


def read_elf_sections(elf_path):
    """
    Minimal ELF parser to extract section data.
    Only handles 32-bit little-endian RISC-V ELF files.
    """
    with open(elf_path, "rb") as f:
        data = f.read()

    # Verify ELF magic
    if data[:4] != b"\x7fELF":
        raise ValueError(f"{elf_path} is not a valid ELF file")

    # ELF32 little-endian check
    ei_class = data[4]
    ei_data = data[5]
    if ei_class != 1:
        raise ValueError("Expected 32-bit ELF (EI_CLASS=1)")
    if ei_data != 1:
        raise ValueError("Expected little-endian ELF (EI_DATA=1)")

    # Parse ELF header
    (e_type, e_machine, e_version, e_entry, e_phoff, e_shoff,
     e_flags, e_ehsize, e_phentsize, e_phnum, e_shentsize,
     e_shnum, e_shstrndx) = struct.unpack_from("<HHIIIIIHHHHHH", data, 16)

    # Parse section headers
    sections = {}
    # Read section name string table first
    shstrtab_off = e_shoff + e_shstrndx * e_shentsize
    (_, _, _, _, sh_offset, sh_size, _, _, _, _) = struct.unpack_from(
        "<IIIIIIIIII", data, shstrtab_off
    )
    strtab = data[sh_offset : sh_offset + sh_size]

    for i in range(e_shnum):
        off = e_shoff + i * e_shentsize
        (sh_name, sh_type, sh_flags, sh_addr, sh_offset, sh_size,
         sh_link, sh_info, sh_addralign, sh_entsize) = struct.unpack_from(
            "<IIIIIIIIII", data, off
        )

        # Get section name from string table
        name_end = strtab.index(b"\x00", sh_name)
        name = strtab[sh_name:name_end].decode("ascii")

        sections[name] = {
            "name": name,
            "type": sh_type,
            "flags": sh_flags,
            "addr": sh_addr,
            "offset": sh_offset,
            "size": sh_size,
            "data": data[sh_offset : sh_offset + sh_size] if sh_type != 8 else b"",
            # sh_type 8 = SHT_NOBITS (.bss)
        }

    return sections, e_entry, data


def extract_relocations(elf_data, sections):
    """
    Extract relocation entries from the ELF file.
    Returns a list of byte offsets into the image that need patching.
    """
    relocs = []

    for name, sec in sections.items():
        # Look for RELA and REL sections
        if sec["type"] not in (4, 9):  # SHT_RELA=4, SHT_REL=9
            continue

        entry_size = 12 if sec["type"] == 4 else 8  # RELA has addend
        num_entries = sec["size"] // entry_size

        for i in range(num_entries):
            off = sec["offset"] + i * entry_size
            if sec["type"] == 4:  # RELA
                r_offset, r_info, r_addend = struct.unpack_from("<III", elf_data, off)
            else:  # REL
                r_offset, r_info = struct.unpack_from("<II", elf_data, off)

            r_type = r_info & 0xFF

            # R_RISCV_32 = 1 (absolute 32-bit address - needs relocation)
            if r_type == 1:
                relocs.append(r_offset)

    return sorted(set(relocs))


def build_pdb(image_data, text_size, data_size, bss_size, entry_offset,
              relocs, app_name, app_version, abi_version, min_heap, flags):
    """
    Assemble a complete .pdb file from components.
    """
    # Build relocation table
    reloc_table = b""
    for r in relocs:
        reloc_table += struct.pack("<I", r)

    # Compute CRC32 over reloc_table + image
    payload = reloc_table + image_data
    checksum = zlib.crc32(payload) & 0xFFFFFFFF

    # Encode strings (pad with nulls to 32 bytes)
    name_bytes = app_name.encode("utf-8")[:31].ljust(32, b"\x00")
    version_bytes = app_version.encode("utf-8")[:31].ljust(32, b"\x00")

    # Pack header
    header = struct.pack(
        "<IHHIIIIIi",
        PDB_MAGIC,
        PDB_FORMAT_VERSION,
        abi_version,
        entry_offset,
        text_size,
        data_size,
        bss_size,
        len(relocs),
        flags,
    )
    header += name_bytes
    header += version_bytes
    header += struct.pack("<II", min_heap, checksum)

    assert len(header) == HEADER_SIZE, f"Header is {len(header)} bytes, expected {HEADER_SIZE}"

    return header + payload


def validate_pdb(pdb_data):
    """
    Validate a .pdb file's integrity.
    Returns (is_valid, messages).
    """
    messages = []

    if len(pdb_data) < HEADER_SIZE:
        return False, ["File too small for PDB header"]

    magic = struct.unpack_from("<I", pdb_data, 0)[0]
    if magic != PDB_MAGIC:
        return False, [f"Bad magic: 0x{magic:08X} (expected 0x{PDB_MAGIC:08X})"]

    fmt_ver = struct.unpack_from("<H", pdb_data, 4)[0]
    abi_ver = struct.unpack_from("<H", pdb_data, 6)[0]
    entry = struct.unpack_from("<I", pdb_data, 8)[0]
    text_sz = struct.unpack_from("<I", pdb_data, 12)[0]
    data_sz = struct.unpack_from("<I", pdb_data, 16)[0]
    bss_sz = struct.unpack_from("<I", pdb_data, 20)[0]
    reloc_n = struct.unpack_from("<I", pdb_data, 24)[0]
    min_heap = struct.unpack_from("<I", pdb_data, 96)[0]
    checksum = struct.unpack_from("<I", pdb_data, 100)[0]

    messages.append(f"Format version: {fmt_ver}")
    messages.append(f"ABI version: {abi_ver}")
    messages.append(f"Entry offset: 0x{entry:04X}")
    messages.append(f"Text: {text_sz} bytes, Data: {data_sz} bytes, BSS: {bss_sz} bytes")
    messages.append(f"Relocations: {reloc_n}")
    messages.append(f"Min heap: {min_heap} bytes")
    messages.append(f"Total RAM needed: {text_sz + data_sz + bss_sz + min_heap} bytes")

    # Verify payload CRC
    payload_start = HEADER_SIZE
    payload = pdb_data[payload_start:]
    computed_crc = zlib.crc32(payload) & 0xFFFFFFFF

    if computed_crc != checksum:
        messages.append(f"CRC32 MISMATCH: computed 0x{computed_crc:08X}, expected 0x{checksum:08X}")
        return False, messages

    messages.append(f"CRC32: OK (0x{checksum:08X})")

    # Check image size consistency
    expected_payload = reloc_n * 4 + text_sz + data_sz
    actual_payload = len(payload)
    if actual_payload != expected_payload:
        messages.append(
            f"Payload size mismatch: {actual_payload} bytes "
            f"(expected {expected_payload} = {reloc_n}*4 + {text_sz} + {data_sz})"
        )
        return False, messages

    messages.append("Payload size: OK")

    # RAM budget check
    APP_REGION_MIN = 140 * 1024
    APP_REGION_MAX = 220 * 1024
    total_ram = text_sz + data_sz + bss_sz + min_heap
    if total_ram > APP_REGION_MAX:
        messages.append(
            f"WARNING: Total RAM {total_ram} exceeds max app region "
            f"({APP_REGION_MAX} bytes). App may not load."
        )
    elif total_ram > APP_REGION_MIN:
        messages.append(
            f"NOTE: Total RAM {total_ram} bytes. App needs Wi-Fi disabled "
            f"to fit (>{APP_REGION_MIN} bytes)."
        )

    return True, messages


def main():
    parser = argparse.ArgumentParser(
        description="PaperDOS Binary Packager - builds .pdb files from ELF binaries"
    )
    parser.add_argument("input", help="Input ELF file (compiled with paperdos.ld)")
    parser.add_argument("output", help="Output .pdb file")
    parser.add_argument("--name", required=True, help="App display name (max 31 chars)")
    parser.add_argument("--version", default="1.0.0", help="App version string")
    parser.add_argument("--abi", type=int, default=1, help="Minimum ABI version required")
    parser.add_argument("--min-heap", type=int, default=0,
                        help="Minimum heap bytes required (0 = no requirement)")
    parser.add_argument("--wifi", action="store_true", help="App requires Wi-Fi")
    parser.add_argument("--bt", action="store_true", help="App requires Bluetooth")
    parser.add_argument("--validate", action="store_true",
                        help="Validate an existing .pdb file instead of building")

    args = parser.parse_args()

    # Validate mode
    if args.validate:
        with open(args.input, "rb") as f:
            pdb_data = f.read()
        valid, messages = validate_pdb(pdb_data)
        for msg in messages:
            print(f"  {msg}")
        if valid:
            print("\nValidation: PASS")
        else:
            print("\nValidation: FAIL")
            sys.exit(1)
        return

    # Build mode
    print(f"Reading ELF: {args.input}")
    sections, entry_addr, elf_data = read_elf_sections(args.input)

    # Extract section data
    text_data = sections.get(".text", {}).get("data", b"")
    rodata = sections.get(".rodata", {}).get("data", b"")
    data_data = sections.get(".data", {}).get("data", b"")
    sdata = sections.get(".sdata", {}).get("data", b"")
    bss_size = sections.get(".bss", {}).get("size", 0)
    sbss_size = sections.get(".sbss", {}).get("size", 0)

    # In our linker script, .rodata is part of .text section
    # and .sdata is part of .data section
    # So we work with what the linker produced
    text_size = len(text_data)
    data_size = len(data_data)
    total_bss = bss_size + sbss_size

    # Combine into loadable image
    image_data = text_data + data_data

    # Pad to 4-byte alignment
    while len(image_data) % 4 != 0:
        image_data += b"\x00"
        if len(data_data) > 0:
            data_size += 1
        else:
            text_size += 1

    # Extract relocations
    relocs = extract_relocations(elf_data, sections)

    # Compute flags
    flags = 0
    if args.wifi:
        flags |= FLAG_NEEDS_WIFI
    if args.bt:
        flags |= FLAG_NEEDS_BT

    print(f"  Text:   {text_size:>8} bytes")
    print(f"  Data:   {data_size:>8} bytes")
    print(f"  BSS:    {total_bss:>8} bytes")
    print(f"  Relocs: {len(relocs):>8} entries")
    print(f"  Entry:  0x{entry_addr:04X}")
    print(f"  Total RAM: {text_size + data_size + total_bss + args.min_heap} bytes")

    # Build .pdb
    pdb_data = build_pdb(
        image_data=image_data,
        text_size=text_size,
        data_size=data_size,
        bss_size=total_bss,
        entry_offset=entry_addr,
        relocs=relocs,
        app_name=args.name,
        app_version=args.version,
        abi_version=args.abi,
        min_heap=args.min_heap,
        flags=flags,
    )

    # Write output
    with open(args.output, "wb") as f:
        f.write(pdb_data)

    print(f"\nWrote {len(pdb_data)} bytes to {args.output}")

    # Auto-validate
    valid, messages = validate_pdb(pdb_data)
    print("\nValidation:")
    for msg in messages:
        print(f"  {msg}")
    if not valid:
        print("\nWARNING: Generated file failed validation!")
        sys.exit(1)


if __name__ == "__main__":
    main()
