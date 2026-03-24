// F4: pd_input_get_buttons and pd_input_wait_button delegate to registered fn pointers.

use std::sync::atomic::{AtomicU32, Ordering};

use kernel::syscall::input::{
    pd_input_get_buttons, pd_input_wait_button, set_input_get_buttons_fn, set_input_wait_button_fn,
};

static CALL_COUNT: AtomicU32 = AtomicU32::new(0);

fn mock_buttons() -> u32 {
    CALL_COUNT.fetch_add(1, Ordering::SeqCst);
    0b0000_0001 // PD_BTN_UP
}

fn mock_wait() -> u32 {
    0b0000_0010 // PD_BTN_DOWN
}

#[test]
fn pd_input_get_buttons_delegates_to_registered_fn_input_global() {
    CALL_COUNT.store(0, Ordering::SeqCst);
    // SAFETY: single-threaded test binary.
    unsafe { set_input_get_buttons_fn(mock_buttons) };
    let result = pd_input_get_buttons();
    assert_eq!(result, 0b0000_0001, "must return value from registered fn");
    assert_eq!(
        CALL_COUNT.load(Ordering::SeqCst),
        1,
        "registered fn must be called once"
    );
}

#[test]
fn pd_input_wait_button_delegates_to_registered_fn_input_global() {
    unsafe { set_input_wait_button_fn(mock_wait) };
    let result = pd_input_wait_button();
    assert_eq!(
        result, 0b0000_0010,
        "must return value from registered wait fn"
    );
}
