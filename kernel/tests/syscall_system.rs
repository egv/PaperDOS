use kernel::abi::{PD_ABI_VERSION, PdSyscalls};
use kernel::syscall::build_syscall_table;

#[test]
fn syscall_table_metadata_fields_syscall_system() {
    let t: PdSyscalls = build_syscall_table(0x3C00_0000, 0x8000);
    assert_eq!(t.abi_version, PD_ABI_VERSION);
    assert_eq!(t.app_heap_start, 0x3C00_0000);
    assert_eq!(t.app_heap_size, 0x8000);
    // Wired fields must be non-zero.
    assert_ne!(t.net_wifi_connect, 0, "net_wifi_connect must be wired");
    assert_ne!(t.mem_alloc, 0, "mem_alloc must be wired");
    assert_ne!(t.font_load, 0, "font_load must be wired");
}

#[test]
fn syscall_table_system_fields_populated_syscall_system() {
    let t: PdSyscalls = build_syscall_table(0, 0);
    // Each of the five system syscall slots must be populated.
    // The stored value is the function address truncated to 32 bits.
    assert_ne!(t.sys_sleep_ms, 0, "sys_sleep_ms must be wired");
    assert_ne!(t.sys_millis, 0, "sys_millis must be wired");
    assert_ne!(t.sys_exit, 0, "sys_exit must be wired");
    assert_ne!(t.sys_log, 0, "sys_log must be wired");
    assert_ne!(t.sys_get_free_heap, 0, "sys_get_free_heap must be wired");
}
