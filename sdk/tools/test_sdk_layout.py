"""
Tests for SDK non-Python artefacts.

Covers:
  A4 — paperdos.h compiles with a native C compiler; key macros and struct
       fields are present; PD_ABI_VERSION == 1.
  A5 — pd_entry.S contains the expected sys_exit offset (0xAC), a jalr
       instruction, and the s0 save of the syscall pointer.
"""

import re
import shutil
import subprocess
import sys
import tempfile
import unittest
from pathlib import Path

REPO_ROOT = Path(__file__).resolve().parents[2]
HEADER_PATH = REPO_ROOT / "sdk" / "include" / "paperdos.h"
ENTRY_PATH = REPO_ROOT / "sdk" / "linker" / "pd_entry.S"


# ── A4: paperdos.h compile check ─────────────────────────────────────────────


def _find_c_compiler():
    """Return the first available C compiler, or None."""
    for cc in ("cc", "clang", "gcc"):
        if shutil.which(cc):
            return cc
    return None


class TestPaperdosHeaderCompiles(unittest.TestCase):
    def setUp(self):
        self.cc = _find_c_compiler()
        if self.cc is None:
            self.skipTest("No C compiler found")

    def _compile(self, src: str) -> subprocess.CompletedProcess:
        """Compile a C source snippet that includes paperdos.h; return result."""
        with tempfile.TemporaryDirectory() as tmpdir:
            src_file = Path(tmpdir) / "check.c"
            obj_file = Path(tmpdir) / "check.o"
            src_file.write_text(src)
            return subprocess.run(
                [
                    self.cc,
                    "-c",
                    "-w",                          # suppress warnings — we only care about errors
                    f"-I{HEADER_PATH.parent}",
                    str(src_file),
                    "-o",
                    str(obj_file),
                ],
                capture_output=True,
                text=True,
            )

    def test_header_compiles_without_errors(self):
        result = self._compile('#include "paperdos.h"\n')
        self.assertEqual(
            result.returncode, 0,
            f"paperdos.h failed to compile:\n{result.stderr}",
        )

    def test_pd_abi_version_equals_1(self):
        src = (
            '#include "paperdos.h"\n'
            "_Static_assert(PD_ABI_VERSION == 1, \"ABI version must be 1\");\n"
        )
        result = self._compile(src)
        self.assertEqual(
            result.returncode, 0,
            f"PD_ABI_VERSION != 1 or header error:\n{result.stderr}",
        )

    def test_pd_screen_dimensions_defined(self):
        src = (
            '#include "paperdos.h"\n'
            "_Static_assert(PD_SCREEN_WIDTH == 800, \"width\");\n"
            "_Static_assert(PD_SCREEN_HEIGHT == 480, \"height\");\n"
        )
        result = self._compile(src)
        self.assertEqual(result.returncode, 0, result.stderr)

    def test_pd_syscalls_t_is_a_struct(self):
        """pd_syscalls_t must be usable as a struct type."""
        src = (
            '#include "paperdos.h"\n'
            "void check(const pd_syscalls_t *s) { (void)s; }\n"
        )
        result = self._compile(src)
        self.assertEqual(result.returncode, 0, result.stderr)

    def test_sys_exit_field_position(self):
        """
        sys_exit must be preceded by exactly 4 uint32_t metadata fields and
        39 function-pointer fields (10 display + 3 input + 13 fs + 11 net +
        2 sys before it).  On riscv32 (4-byte pointers) this resolves to
        4*4 + 39*4 = 172 = 0xAC, which pd_entry.S hardcodes.

        The algebraic form is used here so the assertion holds on both 32-bit
        and 64-bit hosts, catching field-order changes on either platform.
        """
        src = (
            '#include <stddef.h>\n'
            '#include "paperdos.h"\n'
            '/* 4 metadata uint32s + 39 fn-ptr fields before sys_exit */\n'
            '_Static_assert(\n'
            '    offsetof(pd_syscalls_t, sys_exit) ==\n'
            '        4 * sizeof(uint32_t) + 39 * sizeof(void *),\n'
            '    "sys_exit field order changed — update pd_entry.S offset 0xAC");\n'
        )
        result = self._compile(src)
        self.assertEqual(
            result.returncode, 0,
            f"sys_exit field position mismatch:\n{result.stderr}",
        )

    def test_pd_main_prototype_compiles(self):
        """pd_main() must be declared in the header."""
        src = (
            '#include "paperdos.h"\n'
            "void pd_main(pd_syscalls_t *sys) { (void)sys; }\n"
        )
        result = self._compile(src)
        self.assertEqual(result.returncode, 0, result.stderr)

    def test_convenience_macros_expand(self):
        """Key convenience macros must expand without error."""
        src = (
            '#include "paperdos.h"\n'
            "void use_macros(pd_syscalls_t *s) {\n"
            "    (void)pd_screen_w(s);\n"
            "    (void)pd_screen_h(s);\n"
            "    (void)pd_buttons(s);\n"
            "    (void)pd_millis(s);\n"
            "    (void)pd_free_heap(s);\n"
            "}\n"
        )
        result = self._compile(src)
        self.assertEqual(result.returncode, 0, result.stderr)


# ── A5: pd_entry.S content check ─────────────────────────────────────────────


class TestPdEntryAssembly(unittest.TestCase):
    def setUp(self):
        self.src = ENTRY_PATH.read_text()

    def test_sys_exit_offset_is_0xac(self):
        """sys_exit must be loaded via lw from 0xAC(s0) — not just mentioned in a comment."""
        self.assertTrue(
            re.search(r"lw\s+\w+,\s*0xAC\(s0\)", self.src),
            "pd_entry.S must load sys_exit with 'lw <reg>, 0xAC(s0)'",
        )

    def test_jalr_used_for_indirect_call(self):
        """The entry must use jalr for an indirect call through the function pointer."""
        self.assertIn("jalr", self.src)

    def test_syscall_pointer_saved_to_s0(self):
        """The syscall table pointer (a0) must be saved into s0 for later use."""
        # Accept both "mv s0, a0" and "mv      s0, a0" (tab-aligned).
        self.assertTrue(
            re.search(r"mv\s+s0,\s*a0", self.src),
            "pd_entry.S must save a0 into s0 (mv s0, a0)",
        )

    def test_pd_entry_symbol_exported(self):
        """_pd_entry must be declared .global."""
        self.assertIn(".global _pd_entry", self.src)

    def test_text_section_declared(self):
        """Code must live in a .text section."""
        self.assertIn(".section .text", self.src)


if __name__ == "__main__":
    unittest.main()
