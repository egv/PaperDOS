use core::mem::{align_of, offset_of, size_of};

use kernel::abi::{
    PdDir, PdDirent, PdFile, PdFont, PdHttp, PdStat, PdSyscalls, PD_ABI_VERSION, PD_BTN_BACK,
    PD_BTN_DOWN, PD_BTN_LEFT, PD_BTN_OK, PD_BTN_POWER, PD_BTN_RIGHT, PD_BTN_UP, PD_COLOR_BLACK,
    PD_COLOR_WHITE, PD_FTYPE_DIR, PD_FTYPE_FILE, PD_LOG_DEBUG, PD_LOG_ERROR, PD_LOG_INFO,
    PD_LOG_WARN, PD_MAX_PATH, PD_REFRESH_FAST, PD_REFRESH_FULL, PD_REFRESH_PARTIAL, PD_ROTATION_0,
    PD_ROTATION_180, PD_ROTATION_270, PD_ROTATION_90, PD_SCREEN_HEIGHT, PD_SCREEN_WIDTH,
    PD_SDK_VERSION, PD_SEEK_CUR, PD_SEEK_END, PD_SEEK_SET, PD_WIFI_CONNECTED, PD_WIFI_CONNECTING,
    PD_WIFI_ERROR, PD_WIFI_OFF,
};

#[test]
fn abi_layout_and_abi_support_types() {
    assert_eq!(PD_ABI_VERSION, 1);
    assert_eq!(PD_SDK_VERSION, "0.1.0");

    assert_eq!(PD_SCREEN_WIDTH, 800);
    assert_eq!(PD_SCREEN_HEIGHT, 480);
    assert_eq!(PD_COLOR_BLACK, 0x00);
    assert_eq!(PD_COLOR_WHITE, 0xFF);

    assert_eq!(PD_REFRESH_FULL, 0);
    assert_eq!(PD_REFRESH_PARTIAL, 1);
    assert_eq!(PD_REFRESH_FAST, 2);

    assert_eq!(PD_ROTATION_0, 0);
    assert_eq!(PD_ROTATION_90, 1);
    assert_eq!(PD_ROTATION_180, 2);
    assert_eq!(PD_ROTATION_270, 3);

    assert_eq!(PD_BTN_UP, 1 << 0);
    assert_eq!(PD_BTN_DOWN, 1 << 1);
    assert_eq!(PD_BTN_LEFT, 1 << 2);
    assert_eq!(PD_BTN_RIGHT, 1 << 3);
    assert_eq!(PD_BTN_OK, 1 << 4);
    assert_eq!(PD_BTN_BACK, 1 << 5);
    assert_eq!(PD_BTN_POWER, 1 << 6);

    assert_eq!(PD_SEEK_SET, 0);
    assert_eq!(PD_SEEK_CUR, 1);
    assert_eq!(PD_SEEK_END, 2);
    assert_eq!(PD_MAX_PATH, 256);
    assert_eq!(PD_FTYPE_FILE, 0);
    assert_eq!(PD_FTYPE_DIR, 1);

    assert_eq!(PD_WIFI_OFF, 0);
    assert_eq!(PD_WIFI_CONNECTING, 1);
    assert_eq!(PD_WIFI_CONNECTED, 2);
    assert_eq!(PD_WIFI_ERROR, 3);

    assert_eq!(PD_LOG_ERROR, 0);
    assert_eq!(PD_LOG_WARN, 1);
    assert_eq!(PD_LOG_INFO, 2);
    assert_eq!(PD_LOG_DEBUG, 3);

    assert_eq!(size_of::<*mut PdFile>(), size_of::<usize>());
    assert_eq!(size_of::<*mut PdDir>(), size_of::<usize>());
    assert_eq!(size_of::<*const PdFont>(), size_of::<usize>());
    assert_eq!(size_of::<*mut PdHttp>(), size_of::<usize>());

    assert_eq!(size_of::<PdDirent>(), 264);
    assert_eq!(align_of::<PdDirent>(), 4);
    assert_eq!(offset_of!(PdDirent, name), 0);
    assert_eq!(offset_of!(PdDirent, entry_type), 256);
    assert_eq!(offset_of!(PdDirent, size), 260);

    assert_eq!(size_of::<PdStat>(), 12);
    assert_eq!(align_of::<PdStat>(), 4);
    assert_eq!(offset_of!(PdStat, entry_type), 0);
    assert_eq!(offset_of!(PdStat, size), 4);
    assert_eq!(offset_of!(PdStat, mtime), 8);
}

#[test]
fn syscall_metadata_prefix_abi_layout() {
    assert_eq!(offset_of!(PdSyscalls, abi_version), 0);
    assert_eq!(offset_of!(PdSyscalls, kernel_version), 4);
    assert_eq!(offset_of!(PdSyscalls, app_heap_start), 8);
    assert_eq!(offset_of!(PdSyscalls, app_heap_size), 12);
}

#[test]
fn syscall_display_block_abi_layout() {
    let slot = size_of::<usize>();
    let display_base = 16;

    assert_eq!(offset_of!(PdSyscalls, display_clear), display_base);
    assert_eq!(
        offset_of!(PdSyscalls, display_set_pixel),
        display_base + (slot * 1)
    );
    assert_eq!(
        offset_of!(PdSyscalls, display_draw_rect),
        display_base + (slot * 2)
    );
    assert_eq!(
        offset_of!(PdSyscalls, display_fill_rect),
        display_base + (slot * 3)
    );
    assert_eq!(
        offset_of!(PdSyscalls, display_draw_bitmap),
        display_base + (slot * 4)
    );
    assert_eq!(
        offset_of!(PdSyscalls, display_draw_text),
        display_base + (slot * 5)
    );
    assert_eq!(
        offset_of!(PdSyscalls, display_refresh),
        display_base + (slot * 6)
    );
    assert_eq!(
        offset_of!(PdSyscalls, display_set_rotation),
        display_base + (slot * 7)
    );
    assert_eq!(
        offset_of!(PdSyscalls, display_width),
        display_base + (slot * 8)
    );
    assert_eq!(
        offset_of!(PdSyscalls, display_height),
        display_base + (slot * 9)
    );
}

#[test]
fn syscall_input_block_abi_layout() {
    let slot = size_of::<usize>();
    let input_base = 16 + (slot * 10);

    assert_eq!(offset_of!(PdSyscalls, input_get_buttons), input_base);
    assert_eq!(
        offset_of!(PdSyscalls, input_wait_button),
        input_base + (slot * 1)
    );
    assert_eq!(
        offset_of!(PdSyscalls, input_get_battery_pct),
        input_base + (slot * 2)
    );
}

#[test]
fn syscall_filesystem_block_abi_layout() {
    let slot = size_of::<usize>();
    let filesystem_base = 16 + (slot * 13);

    assert_eq!(offset_of!(PdSyscalls, fs_open), filesystem_base);
    assert_eq!(
        offset_of!(PdSyscalls, fs_close),
        filesystem_base + (slot * 1)
    );
    assert_eq!(
        offset_of!(PdSyscalls, fs_read),
        filesystem_base + (slot * 2)
    );
    assert_eq!(
        offset_of!(PdSyscalls, fs_write),
        filesystem_base + (slot * 3)
    );
    assert_eq!(
        offset_of!(PdSyscalls, fs_seek),
        filesystem_base + (slot * 4)
    );
    assert_eq!(
        offset_of!(PdSyscalls, fs_tell),
        filesystem_base + (slot * 5)
    );
    assert_eq!(offset_of!(PdSyscalls, fs_eof), filesystem_base + (slot * 6));
    assert_eq!(
        offset_of!(PdSyscalls, fs_mkdir),
        filesystem_base + (slot * 7)
    );
    assert_eq!(
        offset_of!(PdSyscalls, fs_remove),
        filesystem_base + (slot * 8)
    );
    assert_eq!(
        offset_of!(PdSyscalls, fs_opendir),
        filesystem_base + (slot * 9)
    );
    assert_eq!(
        offset_of!(PdSyscalls, fs_readdir),
        filesystem_base + (slot * 10)
    );
    assert_eq!(
        offset_of!(PdSyscalls, fs_closedir),
        filesystem_base + (slot * 11)
    );
    assert_eq!(
        offset_of!(PdSyscalls, fs_stat),
        filesystem_base + (slot * 12)
    );
}

#[test]
fn syscall_network_block_abi_layout() {
    let slot = size_of::<usize>();
    let network_base = 16 + (slot * 26);

    assert_eq!(offset_of!(PdSyscalls, net_wifi_connect), network_base);
    assert_eq!(
        offset_of!(PdSyscalls, net_wifi_disconnect),
        network_base + (slot * 1)
    );
    assert_eq!(
        offset_of!(PdSyscalls, net_wifi_status),
        network_base + (slot * 2)
    );
    assert_eq!(
        offset_of!(PdSyscalls, net_http_get),
        network_base + (slot * 3)
    );
    assert_eq!(
        offset_of!(PdSyscalls, net_http_post),
        network_base + (slot * 4)
    );
    assert_eq!(
        offset_of!(PdSyscalls, net_http_begin),
        network_base + (slot * 5)
    );
    assert_eq!(
        offset_of!(PdSyscalls, net_http_set_header),
        network_base + (slot * 6)
    );
    assert_eq!(
        offset_of!(PdSyscalls, net_http_send),
        network_base + (slot * 7)
    );
    assert_eq!(
        offset_of!(PdSyscalls, net_http_read),
        network_base + (slot * 8)
    );
    assert_eq!(
        offset_of!(PdSyscalls, net_http_status_code),
        network_base + (slot * 9)
    );
    assert_eq!(
        offset_of!(PdSyscalls, net_http_end),
        network_base + (slot * 10)
    );
}

#[test]
fn syscall_system_block_abi_layout() {
    let slot = size_of::<usize>();
    let system_base = 16 + (slot * 37);

    assert_eq!(offset_of!(PdSyscalls, sys_sleep_ms), system_base);
    assert_eq!(offset_of!(PdSyscalls, sys_millis), system_base + (slot * 1));
    assert_eq!(offset_of!(PdSyscalls, sys_exit), system_base + (slot * 2));
    assert_eq!(offset_of!(PdSyscalls, sys_reboot), system_base + (slot * 3));
    assert_eq!(offset_of!(PdSyscalls, sys_log), system_base + (slot * 4));
    assert_eq!(
        offset_of!(PdSyscalls, sys_get_free_heap),
        system_base + (slot * 5)
    );
    assert_eq!(
        offset_of!(PdSyscalls, sys_wifi_release),
        system_base + (slot * 6)
    );
    assert_eq!(
        offset_of!(PdSyscalls, sys_wifi_acquire),
        system_base + (slot * 7)
    );
}

#[test]
fn syscall_tail_layout_abi_layout() {
    let slot = size_of::<usize>();
    let memory_base = 16 + (slot * 45);
    let font_base = 16 + (slot * 48);

    assert_eq!(offset_of!(PdSyscalls, mem_alloc), memory_base);
    assert_eq!(offset_of!(PdSyscalls, mem_free), memory_base + (slot * 1));
    assert_eq!(
        offset_of!(PdSyscalls, mem_realloc),
        memory_base + (slot * 2)
    );

    assert_eq!(offset_of!(PdSyscalls, font_load), font_base);
    assert_eq!(offset_of!(PdSyscalls, font_free), font_base + (slot * 1));
    assert_eq!(
        offset_of!(PdSyscalls, font_text_width),
        font_base + (slot * 2)
    );
    assert_eq!(
        offset_of!(PdSyscalls, font_line_height),
        font_base + (slot * 3)
    );

    assert_eq!(align_of::<PdSyscalls>(), align_of::<usize>());
    assert_eq!(size_of::<PdSyscalls>(), 16 + (slot * 52));
}
