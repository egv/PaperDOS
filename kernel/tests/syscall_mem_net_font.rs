use kernel::syscall::build_syscall_table;

#[test]
fn syscall_table_mem_fields_populated_syscall_mem_net_font() {
    let t = build_syscall_table(0, 0);
    assert_ne!(t.mem_alloc, 0, "mem_alloc must be wired");
    assert_ne!(t.mem_free, 0, "mem_free must be wired");
    assert_ne!(t.mem_realloc, 0, "mem_realloc must be wired");
}

#[test]
fn syscall_table_net_fields_populated_syscall_mem_net_font() {
    let t = build_syscall_table(0, 0);
    assert_ne!(t.net_wifi_connect, 0, "net_wifi_connect must be wired");
    assert_ne!(t.net_wifi_disconnect, 0, "net_wifi_disconnect must be wired");
    assert_ne!(t.net_wifi_status, 0, "net_wifi_status must be wired");
    assert_ne!(t.net_http_get, 0, "net_http_get must be wired");
    assert_ne!(t.net_http_post, 0, "net_http_post must be wired");
    assert_ne!(t.net_http_begin, 0, "net_http_begin must be wired");
    assert_ne!(t.net_http_set_header, 0, "net_http_set_header must be wired");
    assert_ne!(t.net_http_send, 0, "net_http_send must be wired");
    assert_ne!(t.net_http_read, 0, "net_http_read must be wired");
    assert_ne!(t.net_http_status_code, 0, "net_http_status_code must be wired");
    assert_ne!(t.net_http_end, 0, "net_http_end must be wired");
}

#[test]
fn syscall_table_font_fields_populated_syscall_mem_net_font() {
    let t = build_syscall_table(0, 0);
    assert_ne!(t.font_load, 0, "font_load must be wired");
    assert_ne!(t.font_free, 0, "font_free must be wired");
    assert_ne!(t.font_text_width, 0, "font_text_width must be wired");
    assert_ne!(t.font_line_height, 0, "font_line_height must be wired");
}
