use kernel::abi::{PD_ABI_VERSION, PdSyscalls};
use kernel::syscall::build_syscall_table;
use kernel::syscall::font::pd_font_load;
use kernel::syscall::mem::pd_mem_alloc;
use kernel::syscall::net::pd_net_wifi_connect;

#[test]
fn syscall_table_metadata_fields_syscall_system() {
    let t: PdSyscalls = build_syscall_table(0x3C00_0000, 0x8000);
    assert_eq!(t.abi_version, PD_ABI_VERSION);
    assert_eq!(t.app_heap_start, 0x3C00_0000);
    assert_eq!(t.app_heap_size, 0x8000);
    assert_eq!(t.net_wifi_connect, pd_net_wifi_connect as usize as u32);
    assert_eq!(t.mem_alloc, pd_mem_alloc as usize as u32);
    assert_eq!(t.font_load, pd_font_load as usize as u32);
}

#[test]
fn syscall_table_system_fields_populated_syscall_system() {
    use kernel::syscall::sys::{
        pd_sys_exit, pd_sys_get_free_heap, pd_sys_log, pd_sys_millis, pd_sys_sleep_ms,
    };
    let t: PdSyscalls = build_syscall_table(0, 0);
    assert_eq!(t.sys_sleep_ms, pd_sys_sleep_ms as usize as u32);
    assert_eq!(t.sys_millis, pd_sys_millis as usize as u32);
    assert_eq!(t.sys_exit, pd_sys_exit as usize as u32);
    assert_eq!(t.sys_log, pd_sys_log as usize as u32);
    assert_eq!(t.sys_get_free_heap, pd_sys_get_free_heap as usize as u32);
}
