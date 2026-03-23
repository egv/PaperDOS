"""
Tests for EPIC-P2-B: Hello World App.

Covers:
  B1 — hello_world/main.c source: exists, declares pd_main, uses paperdos.h
       API, compiles with a native C compiler.
  B2 — pdpack.py end-to-end pipeline: read_elf_sections + build_pdb +
       validate_pdb round-trip on a synthetic ELF32-LE.  Verifies the
       produced .pdb has the right magic, ABI version, and a passing CRC.
       (B3 — run on hardware — is out of scope for host-only tests.)
"""

import shutil
import struct
import subprocess
import sys
import tempfile
import unittest
from pathlib import Path

sys.path.insert(0, str(Path(__file__).parent))

from pdpack import (
    HEADER_SIZE,
    PDB_MAGIC,
    PDB_FORMAT_VERSION,
    build_pdb,
    extract_relocations,
    read_elf_sections,
    validate_pdb,
)

REPO_ROOT = Path(__file__).resolve().parents[2]
HELLO_DIR = REPO_ROOT / "sdk" / "examples" / "hello_world"
HELLO_SRC = HELLO_DIR / "main.c"
HEADER_PATH = REPO_ROOT / "sdk" / "include" / "paperdos.h"


# ── Synthetic ELF32-LE builder ────────────────────────────────────────────────


def _build_elf32_le(text_data=b"\x00\x00\x00\x00", rela_entries=()):
    """
    Build a minimal 32-bit little-endian ELF in memory.

    Sections produced:
      0: NULL
      1: .shstrtab (string table for section names)
      2: .text      (provided text_data)
      3: .rela.text (one RELA entry per rela_entries tuple (r_offset, r_type))

    The ELF header, section data, and section headers are packed together
    in a single bytes object that pdpack.py's read_elf_sections() can parse.
    """
    ELF_HDR = 52
    SH_SIZE = 40

    # Build string table: null + section names
    has_rela = bool(rela_entries)
    strtab_parts = [b"\x00", b".shstrtab\x00", b".text\x00"]
    if has_rela:
        strtab_parts.append(b".rela.text\x00")
    strtab = b"".join(strtab_parts)

    # String-table offsets for each section name
    shstrtab_name = 1                       # b".shstrtab" at offset 1
    text_name = 1 + len(b".shstrtab\x00")   # = 11
    rela_name = text_name + len(b".text\x00")  # = 17

    # Build RELA data
    rela_data = b""
    for r_offset, r_type in rela_entries:
        rela_data += struct.pack("<III", r_offset, r_type, 0)

    # Layout:
    #   0         .. ELF_HDR            : ELF header
    #   ELF_HDR   .. ELF_HDR+strtab     : .shstrtab data
    #   +strtab   .. +text              : .text data
    #   +text     .. +rela              : .rela.text data (may be empty)
    #   aligned   ..                    : section headers
    strtab_off = ELF_HDR
    text_off = strtab_off + len(strtab)
    rela_off = text_off + len(text_data)
    data_end = rela_off + len(rela_data)
    # Align section-header table to 4 bytes.
    shoff = (data_end + 3) & ~3
    pad = shoff - data_end

    num_sections = 4 if has_rela else 3

    # ── ELF header ────────────────────────────────────────────────────────────
    elf_ident = struct.pack(
        "<4sBBBBBxxxxxxx",
        b"\x7fELF",
        1,      # EI_CLASS = ELFCLASS32
        1,      # EI_DATA  = ELFDATA2LSB
        1,      # EI_VERSION
        0,      # OSABI = ELFOSABI_NONE
        0,      # ABI version
    )
    elf_header = elf_ident + struct.pack(
        "<HHIIIIIHHHHHH",
        2,              # e_type     = ET_EXEC
        0xF3,           # e_machine  = EM_RISCV
        1,              # e_version
        0,              # e_entry
        0,              # e_phoff    (no program headers)
        shoff,          # e_shoff
        0,              # e_flags
        ELF_HDR,        # e_ehsize
        0,              # e_phentsize
        0,              # e_phnum
        SH_SIZE,        # e_shentsize
        num_sections,   # e_shnum
        1,              # e_shstrndx (section 1 = .shstrtab)
    )
    assert len(elf_header) == ELF_HDR

    # ── Section headers ───────────────────────────────────────────────────────
    def sh(name, typ, flags, addr, offset, size, link=0, info=0,
           align=1, entsize=0):
        return struct.pack(
            "<IIIIIIIIII",
            name, typ, flags, addr, offset, size, link, info, align, entsize,
        )

    sh0 = b"\x00" * SH_SIZE                          # NULL
    sh1 = sh(shstrtab_name, 3, 0, 0,                 # .shstrtab (SHT_STRTAB=3)
             strtab_off, len(strtab))
    sh2 = sh(text_name, 1, 6, 0,                     # .text (SHT_PROGBITS=1, AX)
             text_off, len(text_data), align=4)
    sections = sh0 + sh1 + sh2
    if has_rela:
        sh3 = sh(rela_name, 4, 0, 0,                 # .rela.text (SHT_RELA=4)
                 rela_off, len(rela_data),
                 entsize=12)
        sections += sh3

    return elf_header + strtab + text_data + rela_data + (b"\x00" * pad) + sections


# ── B1: hello_world/main.c source checks ─────────────────────────────────────


class TestHelloWorldSource(unittest.TestCase):
    def setUp(self):
        self.src = HELLO_SRC.read_text()

    def test_source_file_exists(self):
        self.assertTrue(HELLO_SRC.exists(), f"{HELLO_SRC} not found")

    def test_source_includes_paperdos_header(self):
        self.assertIn('paperdos.h"', self.src)

    def test_source_defines_pd_main(self):
        self.assertIn("void pd_main(", self.src)

    def test_source_calls_display_clear(self):
        self.assertIn("pd_display_clear", self.src)

    def test_source_calls_display_refresh_full(self):
        self.assertIn("PD_REFRESH_FULL", self.src)

    def test_source_waits_for_button(self):
        self.assertIn("pd_wait_button", self.src)

    def test_source_calls_display_text(self):
        self.assertIn("pd_display_text", self.src)

    def test_source_exits_cleanly(self):
        self.assertIn("pd_exit", self.src)


class TestHelloWorldCompiles(unittest.TestCase):
    """Verify main.c is syntactically valid C (native compiler, compile-only)."""

    def setUp(self):
        self.cc = next(
            (c for c in ("cc", "clang", "gcc") if shutil.which(c)), None
        )
        if self.cc is None:
            self.skipTest("No C compiler found")

    def test_source_compiles_without_errors(self):
        with tempfile.TemporaryDirectory() as tmpdir:
            obj = Path(tmpdir) / "hello.o"
            result = subprocess.run(
                [
                    self.cc,
                    "-c",
                    "-w",
                    f"-I{HEADER_PATH.parent}",
                    str(HELLO_SRC),
                    "-o",
                    str(obj),
                ],
                capture_output=True,
                text=True,
            )
        self.assertEqual(
            result.returncode, 0,
            f"hello_world/main.c failed to compile:\n{result.stderr}",
        )


# ── B2: pdpack.py end-to-end pipeline on synthetic ELF ───────────────────────


class TestReadElfSections(unittest.TestCase):
    """read_elf_sections() correctly parses the synthetic ELF32-LE."""

    def _parse(self, **kwargs):
        elf = _build_elf32_le(**kwargs)
        with tempfile.NamedTemporaryFile(suffix=".elf", delete=False) as f:
            f.write(elf)
            path = f.name
        try:
            sections, entry, elf_data = read_elf_sections(path)
        finally:
            Path(path).unlink(missing_ok=True)
        return sections, entry, elf_data

    def test_text_section_present(self):
        sections, _, _ = self._parse(text_data=b"\x01\x02\x03\x04")
        self.assertIn(".text", sections)

    def test_text_section_data_matches(self):
        text = b"\xAA\xBB\xCC\xDD"
        sections, _, _ = self._parse(text_data=text)
        self.assertEqual(sections[".text"]["data"], text)

    def test_rela_section_present_when_requested(self):
        sections, _, _ = self._parse(rela_entries=[(0x00, 1)])
        self.assertIn(".rela.text", sections)

    def test_rela_section_absent_when_not_requested(self):
        sections, _, _ = self._parse()
        rela_sections = [k for k in sections if "rela" in k.lower()]
        self.assertEqual(rela_sections, [])

    def test_entry_is_zero_for_minimal_elf(self):
        _, entry, _ = self._parse()
        self.assertEqual(entry, 0)


class TestPdpackEndToEnd(unittest.TestCase):
    """
    Full pipeline: build synthetic ELF → pdpack → validate_pdb.
    Mirrors what `make` + `pdpack.py` does for a real riscv32 build.
    """

    def _run_pipeline(self, text_data=b"\x00" * 16, rela_entries=(),
                      abi_version=1, app_name="Hello World",
                      app_version="1.0.0"):
        elf = _build_elf32_le(text_data=text_data, rela_entries=rela_entries)
        with tempfile.NamedTemporaryFile(suffix=".elf", delete=False) as f:
            f.write(elf)
            elf_path = f.name
        try:
            sections, entry_addr, elf_data = read_elf_sections(elf_path)
        finally:
            Path(elf_path).unlink(missing_ok=True)

        text = sections.get(".text", {}).get("data", b"")
        data = sections.get(".data", {}).get("data", b"")
        bss_size = sections.get(".bss", {}).get("size", 0)
        image = text + data
        relocs = extract_relocations(elf_data, sections)

        return build_pdb(
            image_data=image,
            text_size=len(text),
            data_size=len(data),
            bss_size=bss_size,
            entry_offset=entry_addr,
            relocs=relocs,
            app_name=app_name,
            app_version=app_version,
            abi_version=abi_version,
            min_heap=4096,
            flags=0,
        )

    def test_pipeline_produces_bytes(self):
        pdb = self._run_pipeline()
        self.assertIsInstance(pdb, bytes)
        self.assertGreater(len(pdb), HEADER_SIZE)

    def test_pdb_magic_is_pdos(self):
        pdb = self._run_pipeline()
        magic = struct.unpack_from("<I", pdb, 0)[0]
        self.assertEqual(magic, PDB_MAGIC)

    def test_pdb_format_version_is_1(self):
        pdb = self._run_pipeline()
        self.assertEqual(struct.unpack_from("<H", pdb, 4)[0], PDB_FORMAT_VERSION)

    def test_pdb_abi_version_stored(self):
        pdb = self._run_pipeline(abi_version=1)
        self.assertEqual(struct.unpack_from("<H", pdb, 6)[0], 1)

    def test_pdb_passes_validation(self):
        pdb = self._run_pipeline()
        is_valid, messages = validate_pdb(pdb)
        self.assertTrue(is_valid, f"validate_pdb failed:\n" + "\n".join(messages))

    def test_pdb_name_stored(self):
        pdb = self._run_pipeline(app_name="Hello World")
        name_field = pdb[32:64]
        self.assertTrue(name_field.startswith(b"Hello World"))

    def test_pdb_with_r_riscv_32_reloc_passes_validation(self):
        # Include one R_RISCV_32 relocation — loader would patch this at runtime.
        pdb = self._run_pipeline(
            text_data=b"\x00" * 16,
            rela_entries=[(0x00, 1)],  # R_RISCV_32 at offset 0
        )
        is_valid, messages = validate_pdb(pdb)
        self.assertTrue(is_valid, "\n".join(messages))

    def test_pdb_reloc_count_matches_r_riscv_32_entries(self):
        # Two R_RISCV_32 entries + one filtered (type 23).
        pdb = self._run_pipeline(
            text_data=b"\x00" * 16,
            rela_entries=[(0x00, 1), (0x04, 23), (0x08, 1)],
        )
        reloc_count = struct.unpack_from("<I", pdb, 24)[0]
        self.assertEqual(reloc_count, 2)


if __name__ == "__main__":
    unittest.main()
