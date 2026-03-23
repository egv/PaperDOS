pub mod display;
pub mod font;
pub mod fs;
pub mod input;
pub mod mem;
pub mod net;
pub mod sys;

use crate::abi::{PD_ABI_VERSION, PdSyscalls};
use display::{
    pd_display_clear, pd_display_draw_bitmap, pd_display_draw_rect, pd_display_draw_text,
    pd_display_fill_rect, pd_display_height, pd_display_refresh, pd_display_set_pixel,
    pd_display_set_rotation, pd_display_width,
};
use font::{pd_font_free, pd_font_line_height, pd_font_load, pd_font_text_width};
use fs::{
    pd_fs_close, pd_fs_closedir, pd_fs_eof, pd_fs_mkdir, pd_fs_open, pd_fs_opendir, pd_fs_read,
    pd_fs_readdir, pd_fs_remove, pd_fs_seek, pd_fs_stat, pd_fs_tell, pd_fs_write,
};
use input::{pd_input_get_battery_pct, pd_input_get_buttons, pd_input_wait_button};
use mem::{pd_mem_alloc, pd_mem_free, pd_mem_realloc};
use net::{
    pd_net_http_begin, pd_net_http_end, pd_net_http_get, pd_net_http_post, pd_net_http_read,
    pd_net_http_send, pd_net_http_set_header, pd_net_http_status_code, pd_net_wifi_connect,
    pd_net_wifi_disconnect, pd_net_wifi_status,
};
use sys::{
    pd_sys_exit, pd_sys_get_free_heap, pd_sys_log, pd_sys_millis, pd_sys_reboot,
    pd_sys_sleep_ms, pd_sys_wifi_acquire, pd_sys_wifi_release,
};

/// Build a fully-populated [`PdSyscalls`] table.
///
/// `heap_start` and `heap_size` describe the memory region reserved for the
/// application heap; the loader writes these values before jumping to the app.
///
/// All 52 syscall slots are wired to their `extern "C"` stubs.  Stub bodies
/// are host-compatible no-ops or error-returns; device-specific behaviour is
/// layered on in subsequent tasks (display global state, Embassy timer, etc.).
pub fn build_syscall_table(heap_start: u32, heap_size: u32) -> PdSyscalls {
    PdSyscalls {
        abi_version: PD_ABI_VERSION,
        kernel_version: 0,
        app_heap_start: heap_start,
        app_heap_size: heap_size,

        // Display
        display_clear: pd_display_clear as usize as u32,
        display_set_pixel: pd_display_set_pixel as usize as u32,
        display_draw_rect: pd_display_draw_rect as usize as u32,
        display_fill_rect: pd_display_fill_rect as usize as u32,
        display_draw_bitmap: pd_display_draw_bitmap as usize as u32,
        display_draw_text: pd_display_draw_text as usize as u32,
        display_refresh: pd_display_refresh as usize as u32,
        display_set_rotation: pd_display_set_rotation as usize as u32,
        display_width: pd_display_width as usize as u32,
        display_height: pd_display_height as usize as u32,

        // Input
        input_get_buttons: pd_input_get_buttons as usize as u32,
        input_wait_button: pd_input_wait_button as usize as u32,
        input_get_battery_pct: pd_input_get_battery_pct as usize as u32,

        // Filesystem
        fs_open: pd_fs_open as usize as u32,
        fs_close: pd_fs_close as usize as u32,
        fs_read: pd_fs_read as usize as u32,
        fs_write: pd_fs_write as usize as u32,
        fs_seek: pd_fs_seek as usize as u32,
        fs_tell: pd_fs_tell as usize as u32,
        fs_eof: pd_fs_eof as usize as u32,
        fs_mkdir: pd_fs_mkdir as usize as u32,
        fs_remove: pd_fs_remove as usize as u32,
        fs_opendir: pd_fs_opendir as usize as u32,
        fs_readdir: pd_fs_readdir as usize as u32,
        fs_closedir: pd_fs_closedir as usize as u32,
        fs_stat: pd_fs_stat as usize as u32,

        // Network
        net_wifi_connect: pd_net_wifi_connect as usize as u32,
        net_wifi_disconnect: pd_net_wifi_disconnect as usize as u32,
        net_wifi_status: pd_net_wifi_status as usize as u32,
        net_http_get: pd_net_http_get as usize as u32,
        net_http_post: pd_net_http_post as usize as u32,
        net_http_begin: pd_net_http_begin as usize as u32,
        net_http_set_header: pd_net_http_set_header as usize as u32,
        net_http_send: pd_net_http_send as usize as u32,
        net_http_read: pd_net_http_read as usize as u32,
        net_http_status_code: pd_net_http_status_code as usize as u32,
        net_http_end: pd_net_http_end as usize as u32,

        // System
        sys_sleep_ms: pd_sys_sleep_ms as usize as u32,
        sys_millis: pd_sys_millis as usize as u32,
        sys_exit: pd_sys_exit as usize as u32,
        sys_reboot: pd_sys_reboot as usize as u32,
        sys_log: pd_sys_log as usize as u32,
        sys_get_free_heap: pd_sys_get_free_heap as usize as u32,
        sys_wifi_release: pd_sys_wifi_release as usize as u32,
        sys_wifi_acquire: pd_sys_wifi_acquire as usize as u32,

        // Memory
        mem_alloc: pd_mem_alloc as usize as u32,
        mem_free: pd_mem_free as usize as u32,
        mem_realloc: pd_mem_realloc as usize as u32,

        // Font
        font_load: pd_font_load as usize as u32,
        font_free: pd_font_free as usize as u32,
        font_text_width: pd_font_text_width as usize as u32,
        font_line_height: pd_font_line_height as usize as u32,
    }
}
