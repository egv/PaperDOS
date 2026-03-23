// ── Font syscall stubs ────────────────────────────────────────────────────────
//
// Real implementations are wired in D8 (EPIC-P1-D task D8).

/// Load a font from `path`.
///
/// Returns an opaque font ID (≥ 0), or −1 on error.
///
/// Stub: returns −1.
pub extern "C" fn pd_font_load(_path: *const u8, _len: usize) -> i32 {
    -1
}

/// Free a font previously loaded by [`pd_font_load`].
///
/// Stub: no-op.
pub extern "C" fn pd_font_free(_font_id: i32) {}

/// Return the pixel width of `text` rendered in `font_id`.
///
/// Stub: returns 0.
pub extern "C" fn pd_font_text_width(_font_id: i32, _text: *const u8, _len: usize) -> i32 {
    0
}

/// Return the line height in pixels for `font_id`.
///
/// Stub: returns 0.
pub extern "C" fn pd_font_line_height(_font_id: i32) -> i32 {
    0
}
