use core::ffi::{c_char, c_void};

pub const PD_ABI_VERSION: u32 = 1;
pub const PD_SDK_VERSION: &str = "0.1.0";

pub const PD_SCREEN_WIDTH: i32 = 800;
pub const PD_SCREEN_HEIGHT: i32 = 480;

pub const PD_COLOR_BLACK: u8 = 0x00;
pub const PD_COLOR_WHITE: u8 = 0xFF;

pub const PD_REFRESH_FULL: i32 = 0;
pub const PD_REFRESH_PARTIAL: i32 = 1;
pub const PD_REFRESH_FAST: i32 = 2;

pub const PD_ROTATION_0: i32 = 0;
pub const PD_ROTATION_90: i32 = 1;
pub const PD_ROTATION_180: i32 = 2;
pub const PD_ROTATION_270: i32 = 3;

pub const PD_BTN_UP: u32 = 1 << 0;
pub const PD_BTN_DOWN: u32 = 1 << 1;
pub const PD_BTN_LEFT: u32 = 1 << 2;
pub const PD_BTN_RIGHT: u32 = 1 << 3;
pub const PD_BTN_OK: u32 = 1 << 4;
pub const PD_BTN_BACK: u32 = 1 << 5;
pub const PD_BTN_POWER: u32 = 1 << 6;

pub const PD_SEEK_SET: i32 = 0;
pub const PD_SEEK_CUR: i32 = 1;
pub const PD_SEEK_END: i32 = 2;

pub const PD_MAX_PATH: usize = 256;

pub const PD_FTYPE_FILE: u8 = 0;
pub const PD_FTYPE_DIR: u8 = 1;

pub const PD_WIFI_OFF: i32 = 0;
pub const PD_WIFI_CONNECTING: i32 = 1;
pub const PD_WIFI_CONNECTED: i32 = 2;
pub const PD_WIFI_ERROR: i32 = 3;

pub const PD_LOG_ERROR: i32 = 0;
pub const PD_LOG_WARN: i32 = 1;
pub const PD_LOG_INFO: i32 = 2;
pub const PD_LOG_DEBUG: i32 = 3;

#[repr(C)]
pub struct PdFile {
    _private: [u8; 0],
}

#[repr(C)]
pub struct PdDir {
    _private: [u8; 0],
}

#[repr(C)]
pub struct PdFont {
    _private: [u8; 0],
}

#[repr(C)]
pub struct PdHttp {
    _private: [u8; 0],
}

#[repr(C)]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct PdDirent {
    pub name: [u8; PD_MAX_PATH],
    pub entry_type: u8,
    pub size: u32,
}

#[repr(C)]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct PdStat {
    pub entry_type: u8,
    pub size: u32,
    pub mtime: u32,
}

#[repr(C)]
pub struct PdSyscalls {
    pub abi_version: u32,
    pub kernel_version: u32,
    pub app_heap_start: u32,
    pub app_heap_size: u32,

    pub display_clear: extern "C" fn(u8),
    pub display_set_pixel: extern "C" fn(i32, i32, u8),
    pub display_draw_rect: extern "C" fn(i32, i32, i32, i32, u8),
    pub display_fill_rect: extern "C" fn(i32, i32, i32, i32, u8),
    pub display_draw_bitmap: extern "C" fn(i32, i32, i32, i32, *const u8),
    pub display_draw_text: extern "C" fn(i32, i32, *const c_char, *const PdFont),
    pub display_refresh: extern "C" fn(i32),
    pub display_set_rotation: extern "C" fn(i32),
    pub display_width: extern "C" fn() -> i32,
    pub display_height: extern "C" fn() -> i32,

    pub input_get_buttons: extern "C" fn() -> u32,
    pub input_wait_button: extern "C" fn(i32) -> u32,
    pub input_get_battery_pct: extern "C" fn() -> i32,

    pub fs_open: extern "C" fn(*const c_char, *const c_char) -> *mut PdFile,
    pub fs_close: extern "C" fn(*mut PdFile) -> i32,
    pub fs_read: extern "C" fn(*mut PdFile, *mut c_void, i32) -> i32,
    pub fs_write: extern "C" fn(*mut PdFile, *const c_void, i32) -> i32,
    pub fs_seek: extern "C" fn(*mut PdFile, i32, i32) -> i32,
    pub fs_tell: extern "C" fn(*mut PdFile) -> i32,
    pub fs_eof: extern "C" fn(*mut PdFile) -> i32,
    pub fs_mkdir: extern "C" fn(*const c_char) -> i32,
    pub fs_remove: extern "C" fn(*const c_char) -> i32,
    pub fs_opendir: extern "C" fn(*const c_char) -> *mut PdDir,
    pub fs_readdir: extern "C" fn(*mut PdDir, *mut PdDirent) -> i32,
    pub fs_closedir: extern "C" fn(*mut PdDir) -> i32,
    pub fs_stat: extern "C" fn(*const c_char, *mut PdStat) -> i32,

    pub net_wifi_connect: extern "C" fn(*const c_char, *const c_char) -> i32,
    pub net_wifi_disconnect: extern "C" fn() -> i32,
    pub net_wifi_status: extern "C" fn() -> i32,
    pub net_http_get: extern "C" fn(*const c_char, *mut c_void, i32) -> i32,
    pub net_http_post: extern "C" fn(*const c_char, *const c_void, i32, *mut c_void, i32) -> i32,
    pub net_http_begin: extern "C" fn(*const c_char, *const c_char) -> *mut PdHttp,
    pub net_http_set_header: extern "C" fn(*mut PdHttp, *const c_char, *const c_char) -> i32,
    pub net_http_send: extern "C" fn(*mut PdHttp, *const c_void, i32) -> i32,
    pub net_http_read: extern "C" fn(*mut PdHttp, *mut c_void, i32) -> i32,
    pub net_http_status_code: extern "C" fn(*mut PdHttp) -> i32,
    pub net_http_end: extern "C" fn(*mut PdHttp) -> i32,

    pub sys_sleep_ms: extern "C" fn(i32),
    pub sys_millis: extern "C" fn() -> u32,
    pub sys_exit: extern "C" fn(i32),
    pub sys_reboot: extern "C" fn(),
    pub sys_log: unsafe extern "C" fn(i32, *const c_char, ...),
    pub sys_get_free_heap: extern "C" fn() -> i32,
    pub sys_wifi_release: extern "C" fn(),
    pub sys_wifi_acquire: extern "C" fn() -> i32,

    pub mem_alloc: extern "C" fn(i32) -> *mut c_void,
    pub mem_free: extern "C" fn(*mut c_void),
    pub mem_realloc: extern "C" fn(*mut c_void, i32) -> *mut c_void,

    pub font_load: extern "C" fn(*const c_char) -> *const PdFont,
    pub font_free: extern "C" fn(*const PdFont),
    pub font_text_width: extern "C" fn(*const PdFont, *const c_char) -> i32,
    pub font_line_height: extern "C" fn(*const PdFont) -> i32,
}
