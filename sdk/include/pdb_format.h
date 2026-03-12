/**
 * PaperDOS Binary Format (.PDB) Header Definition
 *
 * This header defines the on-disk format for PaperDOS application binaries.
 * Used by both the kernel loader and the pdpack.py build tool.
 *
 * File layout:
 *   [pdb_header_t]        - 104 bytes fixed header
 *   [reloc_table]         - reloc_count * 4 bytes (uint32_t offsets)
 *   [image]               - text_size + data_size bytes (loadable code + data)
 *
 * After loading into RAM, bss_size bytes are zeroed after the image.
 */

#ifndef PDB_FORMAT_H
#define PDB_FORMAT_H

#include <stdint.h>

#ifdef __cplusplus
extern "C" {
#endif

#define PDB_MAGIC           0x534F4450  /* "PDOS" in little-endian */
#define PDB_FORMAT_VERSION  1

/* App flags (bitfield) */
#define PDB_FLAG_NEEDS_WIFI (1 << 0)
#define PDB_FLAG_NEEDS_BT   (1 << 1)
#define PDB_FLAG_STORE_APP  (1 << 2)    /* Built-in store app (kernel-managed) */

/**
 * PaperDOS Binary Header
 *
 * Total size: 104 bytes (0x68)
 * All multi-byte fields are little-endian (native RISC-V byte order).
 */
typedef struct __attribute__((packed)) {
    /* 0x00 */ uint32_t magic;           /* Must be PDB_MAGIC */
    /* 0x04 */ uint16_t format_version;  /* PDB format version (1) */
    /* 0x06 */ uint16_t abi_version;     /* Minimum kernel ABI required */
    /* 0x08 */ uint32_t entry_offset;    /* Byte offset from image base to pd_main() */
    /* 0x0C */ uint32_t text_size;       /* Size of .text section in bytes */
    /* 0x10 */ uint32_t data_size;       /* Size of .data section in bytes */
    /* 0x14 */ uint32_t bss_size;        /* Size of .bss (zeroed at load time) */
    /* 0x18 */ uint32_t reloc_count;     /* Number of relocation entries */
    /* 0x1C */ uint32_t flags;           /* PDB_FLAG_* bitfield */
    /* 0x20 */ char     app_name[32];    /* Null-terminated UTF-8 display name */
    /* 0x40 */ char     app_version[32]; /* Null-terminated version string */
    /* 0x60 */ uint32_t min_heap;        /* Minimum heap bytes required */
    /* 0x64 */ uint32_t checksum;        /* CRC32 of reloc_table + image */
} pdb_header_t;

_Static_assert(sizeof(pdb_header_t) == 104, "pdb_header_t must be 104 bytes");

/**
 * Relocation entry
 *
 * Each entry is a uint32_t byte offset into the loaded image.
 * The loader adds the actual load address to the 32-bit value
 * at that offset, converting position-independent references
 * into absolute addresses.
 *
 * Example:
 *   If the image is loaded at 0x3FCA0000 and a reloc entry
 *   says offset 0x100, the loader reads the uint32 at
 *   image[0x100], adds 0x3FCA0000 to it, and writes it back.
 */

/* After the header, the file contains:
 *   uint32_t reloc_table[header.reloc_count];
 *   uint8_t  image[header.text_size + header.data_size];
 */

#ifdef __cplusplus
}
#endif

#endif /* PDB_FORMAT_H */
