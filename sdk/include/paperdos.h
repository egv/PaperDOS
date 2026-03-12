/**
 * PaperDOS SDK - Application Development Header
 *
 * This is the only header file needed to develop PaperDOS applications.
 * Include this file, implement pd_main(), compile with the PaperDOS
 * linker script, and run pdpack.py to produce a .pdb binary.
 *
 * ABI Version: 1
 * Target: ESP32-C3 (RV32IMC)
 */

#ifndef PAPERDOS_H
#define PAPERDOS_H

#include <stdint.h>
#include <stddef.h>

#ifdef __cplusplus
extern "C" {
#endif

/* ──────────────────────────────────────────────────────────────
 * Version Constants
 * ────────────────────────────────────────────────────────────── */

#define PD_ABI_VERSION      1
#define PD_SDK_VERSION      "0.1.0"

/* ──────────────────────────────────────────────────────────────
 * Display Constants
 * ────────────────────────────────────────────────────────────── */

#define PD_SCREEN_WIDTH     800
#define PD_SCREEN_HEIGHT    480

#define PD_COLOR_BLACK      0x00
#define PD_COLOR_WHITE      0xFF

/* Display refresh modes */
#define PD_REFRESH_FULL     0   /* Full refresh - no ghosting, slow (~2s) */
#define PD_REFRESH_PARTIAL  1   /* Partial refresh - some ghosting, fast (~0.5s) */
#define PD_REFRESH_FAST     2   /* Fast/DU mode - most ghosting, fastest (~0.2s) */

/* Display rotation */
#define PD_ROTATION_0       0
#define PD_ROTATION_90      1
#define PD_ROTATION_180     2
#define PD_ROTATION_270     3   /* Default for X4 hardware */

/* ──────────────────────────────────────────────────────────────
 * Input Constants
 * ────────────────────────────────────────────────────────────── */

/* Button bitmask values (from ADC readings) */
#define PD_BTN_UP           (1 << 0)
#define PD_BTN_DOWN         (1 << 1)
#define PD_BTN_LEFT         (1 << 2)
#define PD_BTN_RIGHT        (1 << 3)
#define PD_BTN_OK           (1 << 4)    /* Center/confirm */
#define PD_BTN_BACK         (1 << 5)    /* Back/cancel */
#define PD_BTN_POWER        (1 << 6)    /* Power button (short press) */

/* ──────────────────────────────────────────────────────────────
 * Filesystem Constants
 * ────────────────────────────────────────────────────────────── */

#define PD_SEEK_SET         0
#define PD_SEEK_CUR         1
#define PD_SEEK_END         2

#define PD_MAX_PATH         256

/* File types in pd_stat_t */
#define PD_FTYPE_FILE       0
#define PD_FTYPE_DIR        1

/* ──────────────────────────────────────────────────────────────
 * Network Constants
 * ────────────────────────────────────────────────────────────── */

#define PD_WIFI_OFF         0
#define PD_WIFI_CONNECTING  1
#define PD_WIFI_CONNECTED   2
#define PD_WIFI_ERROR       3

/* ──────────────────────────────────────────────────────────────
 * Log Levels
 * ────────────────────────────────────────────────────────────── */

#define PD_LOG_ERROR        0
#define PD_LOG_WARN         1
#define PD_LOG_INFO         2
#define PD_LOG_DEBUG        3

/* ──────────────────────────────────────────────────────────────
 * Opaque Types
 * ────────────────────────────────────────────────────────────── */

typedef struct pd_file    pd_file_t;
typedef struct pd_dir     pd_dir_t;
typedef struct pd_font    pd_font_t;
typedef struct pd_http    pd_http_t;

/* Directory entry */
typedef struct {
    char     name[PD_MAX_PATH];
    uint8_t  type;          /* PD_FTYPE_FILE or PD_FTYPE_DIR */
    uint32_t size;
} pd_dirent_t;

/* File stat */
typedef struct {
    uint8_t  type;
    uint32_t size;
    uint32_t mtime;         /* Unix timestamp */
} pd_stat_t;

/* ──────────────────────────────────────────────────────────────
 * Syscall Table
 *
 * This struct is the entire interface between apps and the kernel.
 * The kernel passes a pointer to this struct as the sole argument
 * to pd_main(). All kernel services are accessed through it.
 *
 * Rules:
 *   - Slots are APPEND-ONLY. Existing slots never move.
 *   - New slots are added at the end in future ABI versions.
 *   - Apps check abi_version to know which slots are available.
 * ────────────────────────────────────────────────────────────── */

typedef struct {
    /* ── Metadata (offsets 0x00-0x0F) ── */
    uint32_t    abi_version;        /* ABI version of this table */
    uint32_t    kernel_version;     /* Kernel build number */
    uint32_t    app_heap_start;     /* Start address of app heap */
    uint32_t    app_heap_size;      /* Bytes available for heap */

    /* ── Display (offsets 0x10-0x3F) ── */
    void        (*display_clear)(uint8_t color);
    void        (*display_set_pixel)(int x, int y, uint8_t color);
    void        (*display_draw_rect)(int x, int y, int w, int h, uint8_t color);
    void        (*display_fill_rect)(int x, int y, int w, int h, uint8_t color);
    void        (*display_draw_bitmap)(int x, int y, int w, int h, const uint8_t *data);
    void        (*display_draw_text)(int x, int y, const char *str, const pd_font_t *font);
    void        (*display_refresh)(int mode);
    void        (*display_set_rotation)(int rotation);
    int         (*display_width)(void);
    int         (*display_height)(void);

    /* ── Input (offsets 0x40-0x4F) ── */
    uint32_t    (*input_get_buttons)(void);
    uint32_t    (*input_wait_button)(int timeout_ms);
    int         (*input_get_battery_pct)(void);

    /* ── Filesystem (offsets 0x50-0x8F) ── */
    pd_file_t*  (*fs_open)(const char *path, const char *mode);
    int         (*fs_close)(pd_file_t *f);
    int         (*fs_read)(pd_file_t *f, void *buf, int size);
    int         (*fs_write)(pd_file_t *f, const void *buf, int size);
    int         (*fs_seek)(pd_file_t *f, int offset, int whence);
    int         (*fs_tell)(pd_file_t *f);
    int         (*fs_eof)(pd_file_t *f);
    int         (*fs_mkdir)(const char *path);
    int         (*fs_remove)(const char *path);
    pd_dir_t*   (*fs_opendir)(const char *path);
    int         (*fs_readdir)(pd_dir_t *d, pd_dirent_t *entry);
    int         (*fs_closedir)(pd_dir_t *d);
    int         (*fs_stat)(const char *path, pd_stat_t *st);

    /* ── Network (offsets 0x90-0xBF) ── */
    int         (*net_wifi_connect)(const char *ssid, const char *pass);
    int         (*net_wifi_disconnect)(void);
    int         (*net_wifi_status)(void);
    int         (*net_http_get)(const char *url, void *buf, int buf_size);
    int         (*net_http_post)(const char *url, const void *body, int body_len,
                                 void *resp_buf, int resp_size);
    pd_http_t*  (*net_http_begin)(const char *url, const char *method);
    int         (*net_http_set_header)(pd_http_t *h, const char *key, const char *val);
    int         (*net_http_send)(pd_http_t *h, const void *body, int len);
    int         (*net_http_read)(pd_http_t *h, void *buf, int size);
    int         (*net_http_status_code)(pd_http_t *h);
    int         (*net_http_end)(pd_http_t *h);

    /* ── System (offsets 0xC0-0xDF) ── */
    void        (*sys_sleep_ms)(int ms);
    uint32_t    (*sys_millis)(void);
    void        (*sys_exit)(int code);
    void        (*sys_reboot)(void);
    void        (*sys_log)(int level, const char *fmt, ...);
    int         (*sys_get_free_heap)(void);
    void        (*sys_wifi_release)(void);
    int         (*sys_wifi_acquire)(void);

    /* ── Memory (offsets 0xE0-0xEF) ── */
    void*       (*mem_alloc)(int size);
    void        (*mem_free)(void *ptr);
    void*       (*mem_realloc)(void *ptr, int size);

    /* ── Fonts / Assets (offsets 0xF0-0xFF) ── */
    const pd_font_t* (*font_load)(const char *path);
    void        (*font_free)(const pd_font_t *font);
    int         (*font_text_width)(const pd_font_t *font, const char *str);
    int         (*font_line_height)(const pd_font_t *font);

} pd_syscalls_t;


/* ──────────────────────────────────────────────────────────────
 * Convenience Macros
 *
 * These make calling syscalls more ergonomic. Usage:
 *   pd_display_clear(sys, PD_COLOR_WHITE);
 *   pd_display_refresh(sys, PD_REFRESH_FULL);
 * ────────────────────────────────────────────────────────────── */

/* Display */
#define pd_display_clear(s, c)              ((s)->display_clear(c))
#define pd_display_pixel(s, x, y, c)        ((s)->display_set_pixel(x, y, c))
#define pd_display_rect(s, x, y, w, h, c)   ((s)->display_draw_rect(x, y, w, h, c))
#define pd_display_fill(s, x, y, w, h, c)   ((s)->display_fill_rect(x, y, w, h, c))
#define pd_display_bitmap(s, x, y, w, h, d)  ((s)->display_draw_bitmap(x, y, w, h, d))
#define pd_display_text(s, x, y, t, f)       ((s)->display_draw_text(x, y, t, f))
#define pd_display_refresh(s, m)            ((s)->display_refresh(m))
#define pd_screen_w(s)                      ((s)->display_width())
#define pd_screen_h(s)                      ((s)->display_height())

/* Input */
#define pd_buttons(s)                       ((s)->input_get_buttons())
#define pd_wait_button(s, t)                ((s)->input_wait_button(t))
#define pd_battery(s)                       ((s)->input_get_battery_pct())

/* System */
#define pd_sleep(s, ms)                     ((s)->sys_sleep_ms(ms))
#define pd_millis(s)                        ((s)->sys_millis())
#define pd_exit(s, code)                    ((s)->sys_exit(code))
#define pd_log(s, lvl, fmt, ...)            ((s)->sys_log(lvl, fmt, ##__VA_ARGS__))
#define pd_free_heap(s)                     ((s)->sys_get_free_heap())

/* Memory */
#define pd_alloc(s, sz)                     ((s)->mem_alloc(sz))
#define pd_free(s, p)                       ((s)->mem_free(p))
#define pd_realloc(s, p, sz)                ((s)->mem_realloc(p, sz))

/* Fonts */
#define pd_font_load(s, path)               ((s)->font_load(path))
#define pd_font_free(s, f)                  ((s)->font_free(f))


/* ──────────────────────────────────────────────────────────────
 * App Entry Point
 *
 * Every PaperDOS app must implement this function.
 * The kernel calls it with a pointer to the syscall table.
 * Return or call sys_exit() to return to the launcher.
 * ────────────────────────────────────────────────────────────── */

void pd_main(pd_syscalls_t *sys);


#ifdef __cplusplus
}
#endif

#endif /* PAPERDOS_H */
