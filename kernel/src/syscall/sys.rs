// ── System syscall stubs ──────────────────────────────────────────────────────
//
// Device-specific implementations (Embassy timer, esp-hal, etc.) will be
// layered on via cfg-gated overrides in later tasks.

/// Suspend execution for `ms` milliseconds.
///
/// Stub: no-op.  Device impl: Embassy blocking delay.
pub extern "C" fn pd_sys_sleep_ms(_ms: u32) {}

/// Return the number of milliseconds since kernel start.
///
/// Stub: returns 0.  Device impl: reads the Embassy system timer.
pub extern "C" fn pd_sys_millis() -> u32 {
    0
}

/// Terminate the current application with exit code `code`.
///
/// Stub: returns silently (test harness can inspect return).
/// Device impl: triggers a soft reboot / returns to the launcher.
pub extern "C" fn pd_sys_exit(_code: i32) {}

/// Reboot the device.
///
/// Stub: no-op.  Device impl: calls `esp_hal::reset::software_reset()`.
pub extern "C" fn pd_sys_reboot() {}

/// Emit a log message.
///
/// `level` is one of the `PD_LOG_*` constants.
/// `msg` points to `len` bytes of UTF-8 text (no NUL terminator required).
///
/// Stub: discards all arguments.  Device impl: forwards to esp-println.
pub extern "C" fn pd_sys_log(_level: i32, _msg: *const u8, _len: usize) {}

/// Return the number of free heap bytes available to the kernel allocator.
///
/// Stub: returns 0.  Device impl: queries esp-alloc.
pub extern "C" fn pd_sys_get_free_heap() -> u32 {
    0
}

/// Release the WiFi subsystem to reclaim ~64 KB of heap.
///
/// Stub: no-op.  Device impl: tears down esp-wifi and returns memory to the allocator.
pub extern "C" fn pd_sys_wifi_release() {}

/// Re-initialise the WiFi subsystem after a prior [`pd_sys_wifi_release`].
///
/// Stub: no-op.  Device impl: re-initialises esp-wifi.
pub extern "C" fn pd_sys_wifi_acquire() {}
