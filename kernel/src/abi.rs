pub type PdSyscallFn = u32;

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

    pub display_clear: PdSyscallFn,
    pub display_set_pixel: PdSyscallFn,
    pub display_draw_rect: PdSyscallFn,
    pub display_fill_rect: PdSyscallFn,
    pub display_draw_bitmap: PdSyscallFn,
    pub display_draw_text: PdSyscallFn,
    pub display_refresh: PdSyscallFn,
    pub display_set_rotation: PdSyscallFn,
    pub display_width: PdSyscallFn,
    pub display_height: PdSyscallFn,

    pub input_get_buttons: PdSyscallFn,
    pub input_wait_button: PdSyscallFn,
    pub input_get_battery_pct: PdSyscallFn,

    pub fs_open: PdSyscallFn,
    pub fs_close: PdSyscallFn,
    pub fs_read: PdSyscallFn,
    pub fs_write: PdSyscallFn,
    pub fs_seek: PdSyscallFn,
    pub fs_tell: PdSyscallFn,
    pub fs_eof: PdSyscallFn,
    pub fs_mkdir: PdSyscallFn,
    pub fs_remove: PdSyscallFn,
    pub fs_opendir: PdSyscallFn,
    pub fs_readdir: PdSyscallFn,
    pub fs_closedir: PdSyscallFn,
    pub fs_stat: PdSyscallFn,

    pub net_wifi_connect: PdSyscallFn,
    pub net_wifi_disconnect: PdSyscallFn,
    pub net_wifi_status: PdSyscallFn,
    pub net_http_get: PdSyscallFn,
    pub net_http_post: PdSyscallFn,
    pub net_http_begin: PdSyscallFn,
    pub net_http_set_header: PdSyscallFn,
    pub net_http_send: PdSyscallFn,
    pub net_http_read: PdSyscallFn,
    pub net_http_status_code: PdSyscallFn,
    pub net_http_end: PdSyscallFn,

    pub sys_sleep_ms: PdSyscallFn,
    pub sys_millis: PdSyscallFn,
    pub sys_exit: PdSyscallFn,
    pub sys_reboot: PdSyscallFn,
    pub sys_log: PdSyscallFn,
    pub sys_get_free_heap: PdSyscallFn,
    pub sys_wifi_release: PdSyscallFn,
    pub sys_wifi_acquire: PdSyscallFn,

    pub mem_alloc: PdSyscallFn,
    pub mem_free: PdSyscallFn,
    pub mem_realloc: PdSyscallFn,

    pub font_load: PdSyscallFn,
    pub font_free: PdSyscallFn,
    pub font_text_width: PdSyscallFn,
    pub font_line_height: PdSyscallFn,
}
