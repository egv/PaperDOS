/**
 * PaperDOS Binary Loader
 *
 * Responsible for reading .pdb files from the SD card, validating them,
 * relocating them into the app memory region, and jumping to execution.
 */

#ifndef PD_LOADER_H
#define PD_LOADER_H

#include <stdint.h>
#include "pdb_format.h"

#ifdef __cplusplus
extern "C" {
#endif

/* Error codes returned by pd_loader_* functions */
typedef enum {
    PD_LOAD_OK = 0,
    PD_LOAD_ERR_FILE_NOT_FOUND,
    PD_LOAD_ERR_READ_FAILED,
    PD_LOAD_ERR_BAD_MAGIC,
    PD_LOAD_ERR_BAD_FORMAT,
    PD_LOAD_ERR_ABI_TOO_NEW,
    PD_LOAD_ERR_CHECKSUM,
    PD_LOAD_ERR_OUT_OF_MEMORY,
    PD_LOAD_ERR_WIFI_REQUIRED,
} pd_load_error_t;

/**
 * Information about a .pdb file, extracted from its header.
 * Used to display app info in the launcher before loading.
 */
typedef struct {
    char        name[32];
    char        version[32];
    uint16_t    abi_version;
    uint32_t    text_size;
    uint32_t    data_size;
    uint32_t    bss_size;
    uint32_t    min_heap;
    uint32_t    flags;
    uint32_t    total_ram_needed;   /* Computed: text + data + bss + min_heap */
} pd_app_info_t;

/**
 * Read just the header of a .pdb file to get app info.
 * Does not load the app into memory.
 *
 * @param path      Path to .pdb file on SD card
 * @param info      Output: populated app info struct
 * @return          PD_LOAD_OK on success, error code on failure
 */
pd_load_error_t pd_loader_get_info(const char *path, pd_app_info_t *info);

/**
 * Load and execute a .pdb application.
 *
 * This function:
 *   1. Reads and validates the .pdb header
 *   2. Checks ABI compatibility and memory requirements
 *   3. Loads the image into the app memory region
 *   4. Applies relocations
 *   5. Zeros the .bss section
 *   6. Arms the watchdog timer
 *   7. Jumps to the app entry point with syscall table in a0
 *
 * When the app exits (via sys_exit or return), this function returns.
 *
 * @param path      Path to .pdb file on SD card
 * @return          PD_LOAD_OK if app ran and exited cleanly,
 *                  or an error code if loading failed.
 *                  Note: if the app crashes, the watchdog resets
 *                  the system before this function can return.
 */
pd_load_error_t pd_loader_run(const char *path);

/**
 * Get a human-readable error string for a load error code.
 */
const char *pd_loader_strerror(pd_load_error_t err);

#ifdef __cplusplus
}
#endif

#endif /* PD_LOADER_H */
