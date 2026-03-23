"""
Tests for pdpack.py — PaperDOS binary packager.

Covers:
  A1 — build_pdb() header layout (magic, sizes, name, CRC field position)
  A2 — extract_relocations() filters to R_RISCV_32 (type 1) only
  A3 — build_pdb() + validate_pdb() round-trip; CRC corruption is detected
"""

import struct
import sys
import unittest
import zlib
from pathlib import Path

# Make sure the tools directory is on the path.
sys.path.insert(0, str(Path(__file__).parent))

from pdpack import (
    HEADER_SIZE,
    PDB_MAGIC,
    PDB_FORMAT_VERSION,
    build_pdb,
    extract_relocations,
    validate_pdb,
)


def _minimal_pdb(**overrides):
    """Return a minimal build_pdb() result with sensible defaults."""
    defaults = dict(
        image_data=b"",
        text_size=0,
        data_size=0,
        bss_size=0,
        entry_offset=0,
        relocs=[],
        app_name="test",
        app_version="1.0",
        abi_version=1,
        min_heap=0,
        flags=0,
    )
    defaults.update(overrides)
    return build_pdb(**defaults)


# ── A1: build_pdb() header layout ────────────────────────────────────────────


class TestBuildPdbHeaderSize(unittest.TestCase):
    def test_empty_payload_produces_exactly_104_bytes(self):
        pdb = _minimal_pdb()
        self.assertEqual(len(pdb), HEADER_SIZE)

    def test_nonempty_image_appended_after_header(self):
        payload = b"\xAA" * 16
        pdb = _minimal_pdb(image_data=payload, text_size=16)
        self.assertEqual(len(pdb), HEADER_SIZE + len(payload))


class TestBuildPdbMagic(unittest.TestCase):
    def test_magic_at_offset_0(self):
        pdb = _minimal_pdb()
        magic = struct.unpack_from("<I", pdb, 0)[0]
        self.assertEqual(magic, PDB_MAGIC)

    def test_magic_value_is_pdos_le(self):
        self.assertEqual(PDB_MAGIC, 0x534F4450)  # "PDOS" in little-endian


class TestBuildPdbVersionFields(unittest.TestCase):
    def test_format_version_at_offset_4(self):
        pdb = _minimal_pdb()
        ver = struct.unpack_from("<H", pdb, 4)[0]
        self.assertEqual(ver, PDB_FORMAT_VERSION)

    def test_abi_version_at_offset_6(self):
        pdb = _minimal_pdb(abi_version=3)
        abi = struct.unpack_from("<H", pdb, 6)[0]
        self.assertEqual(abi, 3)


class TestBuildPdbSizeFields(unittest.TestCase):
    def test_entry_offset_at_offset_8(self):
        pdb = _minimal_pdb(entry_offset=0x1234)
        entry = struct.unpack_from("<I", pdb, 8)[0]
        self.assertEqual(entry, 0x1234)

    def test_text_size_at_offset_12(self):
        pdb = _minimal_pdb(text_size=512)
        self.assertEqual(struct.unpack_from("<I", pdb, 12)[0], 512)

    def test_data_size_at_offset_16(self):
        pdb = _minimal_pdb(data_size=256)
        self.assertEqual(struct.unpack_from("<I", pdb, 16)[0], 256)

    def test_bss_size_at_offset_20(self):
        pdb = _minimal_pdb(bss_size=1024)
        self.assertEqual(struct.unpack_from("<I", pdb, 20)[0], 1024)

    def test_reloc_count_at_offset_24(self):
        pdb = _minimal_pdb(relocs=[0x10, 0x20, 0x30])
        self.assertEqual(struct.unpack_from("<I", pdb, 24)[0], 3)

    def test_flags_at_offset_28(self):
        pdb = _minimal_pdb(flags=3)  # FLAG_NEEDS_WIFI | FLAG_NEEDS_BT
        self.assertEqual(struct.unpack_from("<i", pdb, 28)[0], 3)


class TestBuildPdbStringFields(unittest.TestCase):
    def test_name_at_offset_32_null_padded_to_32_bytes(self):
        pdb = _minimal_pdb(app_name="Hello")
        name_field = pdb[32:64]
        self.assertEqual(name_field[:5], b"Hello")
        self.assertEqual(name_field[5:], b"\x00" * 27)

    def test_version_at_offset_64_null_padded_to_32_bytes(self):
        pdb = _minimal_pdb(app_version="2.3.4")
        ver_field = pdb[64:96]
        self.assertEqual(ver_field[:5], b"2.3.4")
        self.assertEqual(ver_field[5:], b"\x00" * 27)

    def test_name_truncated_to_31_chars(self):
        long_name = "A" * 40
        pdb = _minimal_pdb(app_name=long_name)
        name_field = pdb[32:64]
        # First 31 bytes are the truncated name, last byte is null terminator.
        self.assertEqual(name_field[:31], b"A" * 31)
        self.assertEqual(name_field[31], 0)


class TestBuildPdbChecksumField(unittest.TestCase):
    def test_min_heap_at_offset_96(self):
        pdb = _minimal_pdb(min_heap=4096)
        self.assertEqual(struct.unpack_from("<I", pdb, 96)[0], 4096)

    def test_checksum_at_offset_100(self):
        image = b"\x01\x02\x03\x04"
        pdb = _minimal_pdb(image_data=image, text_size=4)
        stored_crc = struct.unpack_from("<I", pdb, 100)[0]
        # Recompute: payload = reloc_table + image
        payload = pdb[HEADER_SIZE:]
        expected_crc = zlib.crc32(payload) & 0xFFFFFFFF
        self.assertEqual(stored_crc, expected_crc)

    def test_reloc_table_is_prepended_before_image_in_payload(self):
        relocs = [0x0100, 0x0200]
        image = b"\xAA" * 8
        pdb = _minimal_pdb(image_data=image, text_size=8, relocs=relocs)
        payload = pdb[HEADER_SIZE:]
        # First 8 bytes are the reloc table (2 × 4-byte little-endian offsets).
        r0 = struct.unpack_from("<I", payload, 0)[0]
        r1 = struct.unpack_from("<I", payload, 4)[0]
        self.assertEqual(r0, 0x0100)
        self.assertEqual(r1, 0x0200)
        # Remaining bytes are the image.
        self.assertEqual(payload[8:], image)


# ── A2: extract_relocations() ────────────────────────────────────────────────


def _make_rela_section(entries):
    """
    Build a fake RELA section and the corresponding sections dict entry.

    Each entry is (r_offset, r_type).  Returns (sections_dict, elf_data).
    """
    rela_data = b""
    for r_offset, r_type in entries:
        rela_data += struct.pack("<III", r_offset, r_type, 0)

    sections = {
        ".rela.text": {
            "type": 4,      # SHT_RELA
            "offset": 0,
            "size": len(rela_data),
        }
    }
    return sections, rela_data


def _make_rel_section(entries):
    """Build a fake REL (no-addend) section. Each entry is (r_offset, r_type)."""
    rel_data = b""
    for r_offset, r_type in entries:
        rel_data += struct.pack("<II", r_offset, r_type)

    sections = {
        ".rel.text": {
            "type": 9,      # SHT_REL
            "offset": 0,
            "size": len(rel_data),
        }
    }
    return sections, rel_data


class TestExtractRelocationsRela(unittest.TestCase):
    def test_r_riscv_32_entries_are_returned(self):
        sections, elf_data = _make_rela_section([(0x100, 1)])
        relocs = extract_relocations(elf_data, sections)
        self.assertEqual(relocs, [0x100])

    def test_non_r_riscv_32_entries_are_filtered(self):
        # R_RISCV_HI20 = 23, R_RISCV_LO12_I = 24 — neither is type 1.
        sections, elf_data = _make_rela_section([(0x200, 23), (0x300, 24)])
        relocs = extract_relocations(elf_data, sections)
        self.assertEqual(relocs, [])

    def test_mixed_entries_keep_only_r_riscv_32(self):
        entries = [(0x10, 1), (0x20, 23), (0x30, 1), (0x40, 24)]
        sections, elf_data = _make_rela_section(entries)
        relocs = extract_relocations(elf_data, sections)
        self.assertEqual(relocs, [0x10, 0x30])

    def test_duplicates_are_deduplicated(self):
        entries = [(0x50, 1), (0x50, 1)]
        sections, elf_data = _make_rela_section(entries)
        relocs = extract_relocations(elf_data, sections)
        self.assertEqual(relocs, [0x50])

    def test_result_is_sorted(self):
        entries = [(0x300, 1), (0x100, 1), (0x200, 1)]
        sections, elf_data = _make_rela_section(entries)
        relocs = extract_relocations(elf_data, sections)
        self.assertEqual(relocs, [0x100, 0x200, 0x300])


class TestExtractRelocationsRel(unittest.TestCase):
    """Also test SHT_REL (no-addend) sections."""

    def test_rel_section_r_riscv_32_returned(self):
        sections, elf_data = _make_rel_section([(0x080, 1)])
        relocs = extract_relocations(elf_data, sections)
        self.assertEqual(relocs, [0x080])

    def test_rel_section_non_r_riscv_32_filtered(self):
        sections, elf_data = _make_rel_section([(0x090, 5)])
        relocs = extract_relocations(elf_data, sections)
        self.assertEqual(relocs, [])


class TestExtractRelocationsNoRelSections(unittest.TestCase):
    def test_empty_sections_returns_empty_list(self):
        sections = {}
        relocs = extract_relocations(b"", sections)
        self.assertEqual(relocs, [])

    def test_non_reloc_section_ignored(self):
        # SHT_PROGBITS = 1
        sections = {".text": {"type": 1, "offset": 0, "size": 8}}
        relocs = extract_relocations(b"\x00" * 8, sections)
        self.assertEqual(relocs, [])


# ── A3: build_pdb() + validate_pdb() round-trip ──────────────────────────────


class TestValidatePdbRoundTrip(unittest.TestCase):
    def test_valid_pdb_passes_validation(self):
        pdb = _minimal_pdb(
            image_data=b"\xDE\xAD\xBE\xEF" * 4,
            text_size=16,
            app_name="RoundTrip",
            app_version="1.0.0",
            abi_version=1,
        )
        is_valid, messages = validate_pdb(pdb)
        self.assertTrue(is_valid, f"Expected valid; messages: {messages}")

    def test_empty_image_passes_validation(self):
        pdb = _minimal_pdb()
        is_valid, _ = validate_pdb(pdb)
        self.assertTrue(is_valid)

    def test_pdb_with_relocs_passes_validation(self):
        relocs = [0x00, 0x04, 0x08]
        # Image must be at least as large as highest reloc offset + 4.
        image = b"\x00" * 16
        pdb = _minimal_pdb(image_data=image, text_size=16, relocs=relocs)
        is_valid, _ = validate_pdb(pdb)
        self.assertTrue(is_valid)


class TestValidatePdbCrcCorruption(unittest.TestCase):
    def test_bit_flip_in_payload_fails_crc(self):
        pdb = bytearray(
            _minimal_pdb(image_data=b"\x01\x02\x03\x04", text_size=4)
        )
        # Flip one byte in the payload (after the header).
        pdb[HEADER_SIZE] ^= 0xFF
        is_valid, messages = validate_pdb(bytes(pdb))
        self.assertFalse(is_valid)
        self.assertTrue(any("CRC" in m.upper() for m in messages))

    def test_bit_flip_in_header_not_covered_by_crc(self):
        # The CRC covers only the payload, not the header itself.  A flip in
        # the header fields (e.g. text_size) will not trigger a CRC error, but
        # may trigger a payload-size mismatch error.  Either way, the result
        # must be (False, ...) because validate_pdb() cross-checks sizes.
        pdb = bytearray(
            _minimal_pdb(image_data=b"\xAA" * 8, text_size=8)
        )
        # Corrupt text_size field at offset 12.
        struct.pack_into("<I", pdb, 12, 9999)
        is_valid, _ = validate_pdb(bytes(pdb))
        self.assertFalse(is_valid)


class TestValidatePdbBadInput(unittest.TestCase):
    def test_too_short_fails(self):
        is_valid, messages = validate_pdb(b"\x00" * 10)
        self.assertFalse(is_valid)

    def test_wrong_magic_fails(self):
        pdb = bytearray(_minimal_pdb())
        struct.pack_into("<I", pdb, 0, 0xDEADBEEF)
        is_valid, messages = validate_pdb(bytes(pdb))
        self.assertFalse(is_valid)
        self.assertTrue(any("magic" in m.lower() for m in messages))


if __name__ == "__main__":
    unittest.main()
