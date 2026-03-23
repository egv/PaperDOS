use kernel::syscall::build_syscall_table;
use kernel::syscall::font::{pd_font_free, pd_font_line_height, pd_font_load, pd_font_text_width};
use kernel::syscall::mem::{pd_mem_alloc, pd_mem_free, pd_mem_realloc};
use kernel::syscall::net::{
    pd_net_http_begin, pd_net_http_end, pd_net_http_get, pd_net_http_post, pd_net_http_read,
    pd_net_http_send, pd_net_http_set_header, pd_net_http_status_code, pd_net_wifi_connect,
    pd_net_wifi_disconnect, pd_net_wifi_status,
};

#[test]
fn syscall_table_mem_fields_populated_syscall_mem_net_font() {
    let t = build_syscall_table(0, 0);
    assert_eq!(t.mem_alloc, pd_mem_alloc as usize as u32);
    assert_eq!(t.mem_free, pd_mem_free as usize as u32);
    assert_eq!(t.mem_realloc, pd_mem_realloc as usize as u32);
}

#[test]
fn syscall_table_net_fields_populated_syscall_mem_net_font() {
    let t = build_syscall_table(0, 0);
    assert_eq!(t.net_wifi_connect, pd_net_wifi_connect as usize as u32);
    assert_eq!(t.net_wifi_disconnect, pd_net_wifi_disconnect as usize as u32);
    assert_eq!(t.net_wifi_status, pd_net_wifi_status as usize as u32);
    assert_eq!(t.net_http_get, pd_net_http_get as usize as u32);
    assert_eq!(t.net_http_post, pd_net_http_post as usize as u32);
    assert_eq!(t.net_http_begin, pd_net_http_begin as usize as u32);
    assert_eq!(t.net_http_set_header, pd_net_http_set_header as usize as u32);
    assert_eq!(t.net_http_send, pd_net_http_send as usize as u32);
    assert_eq!(t.net_http_read, pd_net_http_read as usize as u32);
    assert_eq!(t.net_http_status_code, pd_net_http_status_code as usize as u32);
    assert_eq!(t.net_http_end, pd_net_http_end as usize as u32);
}

#[test]
fn syscall_table_font_fields_populated_syscall_mem_net_font() {
    let t = build_syscall_table(0, 0);
    assert_eq!(t.font_load, pd_font_load as usize as u32);
    assert_eq!(t.font_free, pd_font_free as usize as u32);
    assert_eq!(t.font_text_width, pd_font_text_width as usize as u32);
    assert_eq!(t.font_line_height, pd_font_line_height as usize as u32);
}
