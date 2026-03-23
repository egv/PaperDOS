// ── Network syscall stubs ─────────────────────────────────────────────────────
//
// All stubs return -1 / ENOSYS.  Real implementations are wired in Phase 3
// (EPIC-P3-A WiFi, EPIC-P3-B HTTP).
//
// Functions that accept raw-pointer parameters are marked `unsafe extern "C"`
// to match the convention in fs.rs and make the pointer contract explicit.

/// # Safety
/// `ssid` must point to `ssid_len` valid bytes; `pass` must point to `pass_len` valid bytes.
pub unsafe extern "C" fn pd_net_wifi_connect(
    _ssid: *const u8, _ssid_len: usize,
    _pass: *const u8, _pass_len: usize,
) -> i32 { -1 }

pub extern "C" fn pd_net_wifi_disconnect() -> i32 { -1 }
pub extern "C" fn pd_net_wifi_status() -> i32 { -1 }

/// # Safety
/// `url` must point to `url_len` valid bytes; `buf` must be valid for `buf_len` bytes of writes.
pub unsafe extern "C" fn pd_net_http_get(
    _url: *const u8, _url_len: usize,
    _buf: *mut u8, _buf_len: usize,
) -> i32 { -1 }

/// # Safety
/// `url` and `body` must be valid for their respective lengths; `buf` valid for `buf_len` writes.
pub unsafe extern "C" fn pd_net_http_post(
    _url: *const u8, _url_len: usize,
    _body: *const u8, _body_len: usize,
    _buf: *mut u8, _buf_len: usize,
) -> i32 { -1 }

/// # Safety
/// `url` must point to `url_len` valid bytes.
pub unsafe extern "C" fn pd_net_http_begin(_url: *const u8, _url_len: usize) -> i32 { -1 }

/// # Safety
/// `name` must point to `name_len` valid bytes; `value` must point to `value_len` valid bytes.
pub unsafe extern "C" fn pd_net_http_set_header(
    _session: i32,
    _name: *const u8, _name_len: usize,
    _value: *const u8, _value_len: usize,
) -> i32 { -1 }

pub extern "C" fn pd_net_http_send(_session: i32) -> i32 { -1 }

/// # Safety
/// `buf` must be valid for `len` bytes of writes.
pub unsafe extern "C" fn pd_net_http_read(_session: i32, _buf: *mut u8, _len: usize) -> i32 { -1 }

pub extern "C" fn pd_net_http_status_code(_session: i32) -> i32 { -1 }
pub extern "C" fn pd_net_http_end(_session: i32) -> i32 { -1 }
