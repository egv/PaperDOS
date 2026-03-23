use crate::abi::{PD_BTN_BACK, PD_BTN_DOWN, PD_BTN_LEFT, PD_BTN_OK, PD_BTN_RIGHT, PD_BTN_UP};
use crate::input::{ButtonEvent, ButtonId};

// ── Pure conversion helpers ───────────────────────────────────────────────────

/// Map a [`ButtonId`] to the corresponding `PD_BTN_*` bitmask bit.
pub fn button_id_to_mask(id: ButtonId) -> u32 {
    match id {
        ButtonId::Up => PD_BTN_UP,
        ButtonId::Down => PD_BTN_DOWN,
        ButtonId::Left => PD_BTN_LEFT,
        ButtonId::Right => PD_BTN_RIGHT,
        ButtonId::Select => PD_BTN_OK,
        ButtonId::Back => PD_BTN_BACK,
    }
}

/// Map a [`ButtonEvent`] to the `PD_BTN_*` bitmask of the underlying button.
///
/// Both `Tap` and `LongPress` events carry the button identity; the caller can
/// distinguish them via the event type before calling this helper.
pub fn button_event_to_mask(event: ButtonEvent) -> u32 {
    match event {
        ButtonEvent::Tap(id) | ButtonEvent::LongPress(id) => button_id_to_mask(id),
    }
}

// ── Syscall stubs ─────────────────────────────────────────────────────────────

/// Return the bitmask of currently-pressed buttons.
///
/// Stub: returns 0 (no buttons pressed).
/// Device impl: polls the ADC pipeline and aggregates pressed button bits.
pub extern "C" fn pd_input_get_buttons() -> u32 {
    0
}

/// Block until a button is pressed and return its bitmask.
///
/// Stub: returns 0 immediately (host has no ADC hardware).
/// Device impl: waits for a `ButtonEvent` from the Embassy task queue.
pub extern "C" fn pd_input_wait_button() -> u32 {
    0
}

/// Return the battery charge level as a percentage (0–100), or −1 if unknown.
///
/// Stub: always returns −1.
/// Device impl: reads the battery ADC channel and applies the voltage curve.
pub extern "C" fn pd_input_get_battery_pct() -> i32 {
    -1
}
