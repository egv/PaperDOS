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
pub fn button_event_to_mask(event: ButtonEvent) -> u32 {
    match event {
        ButtonEvent::Press(id)
        | ButtonEvent::Release(id)
        | ButtonEvent::LongPress(id)
        | ButtonEvent::Repeat(id) => button_id_to_mask(id),
    }
}

// ── Global input callbacks ────────────────────────────────────────────────────
//
// Function pointer slots so main.rs can wire the ADC poller without generics.

static mut GET_BUTTONS_FN: fn() -> u32 = || 0;
static mut WAIT_BUTTON_FN: fn() -> u32 = || 0;

/// Register the get-buttons callback.
///
/// # Safety
/// Must be called once at init, before the first `pd_input_get_buttons` call.
pub unsafe fn set_input_get_buttons_fn(f: fn() -> u32) {
    (&raw mut GET_BUTTONS_FN).write(f);
}

/// Register the wait-button callback.
///
/// # Safety
/// Must be called once at init, before the first `pd_input_wait_button` call.
pub unsafe fn set_input_wait_button_fn(f: fn() -> u32) {
    (&raw mut WAIT_BUTTON_FN).write(f);
}

// ── Syscall stubs ─────────────────────────────────────────────────────────────

/// Return the bitmask of currently-pressed buttons.
pub extern "C" fn pd_input_get_buttons() -> u32 {
    // SAFETY: GET_BUTTONS_FN written once at init; no concurrent modification.
    unsafe { (*(&raw const GET_BUTTONS_FN))() }
}

/// Block until a button is pressed and return its bitmask.
pub extern "C" fn pd_input_wait_button() -> u32 {
    // SAFETY: WAIT_BUTTON_FN written once at init; no concurrent modification.
    unsafe { (*(&raw const WAIT_BUTTON_FN))() }
}

/// Return the battery charge level as a percentage (0–100), or −1 if unknown.
///
/// Stub: always returns −1.
/// Device impl: reads the battery ADC channel and applies the voltage curve.
pub extern "C" fn pd_input_get_battery_pct() -> i32 {
    -1
}
