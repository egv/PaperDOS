use kernel::abi::{PD_BTN_BACK, PD_BTN_DOWN, PD_BTN_LEFT, PD_BTN_OK, PD_BTN_RIGHT, PD_BTN_UP};
use kernel::input::{ButtonEvent, ButtonId};
use kernel::syscall::build_syscall_table;
use kernel::syscall::input::{
    button_event_to_mask, button_id_to_mask, pd_input_get_battery_pct, pd_input_get_buttons,
    pd_input_wait_button,
};

#[test]
fn button_id_to_mask_maps_each_id_syscall_input() {
    assert_eq!(button_id_to_mask(ButtonId::Up), PD_BTN_UP);
    assert_eq!(button_id_to_mask(ButtonId::Down), PD_BTN_DOWN);
    assert_eq!(button_id_to_mask(ButtonId::Left), PD_BTN_LEFT);
    assert_eq!(button_id_to_mask(ButtonId::Right), PD_BTN_RIGHT);
    assert_eq!(button_id_to_mask(ButtonId::Select), PD_BTN_OK);
    assert_eq!(button_id_to_mask(ButtonId::Back), PD_BTN_BACK);
}

#[test]
fn button_event_to_mask_returns_button_mask_syscall_input() {
    assert_eq!(button_event_to_mask(ButtonEvent::Tap(ButtonId::Up)), PD_BTN_UP);
    assert_eq!(button_event_to_mask(ButtonEvent::LongPress(ButtonId::Select)), PD_BTN_OK);
}

#[test]
fn input_get_battery_pct_returns_minus_one_syscall_input() {
    assert_eq!(pd_input_get_battery_pct(), -1, "stub must return -1 (unknown)");
}

#[test]
fn syscall_table_input_fields_populated_syscall_input() {
    let t = build_syscall_table(0, 0);
    assert_eq!(t.input_get_buttons, pd_input_get_buttons as usize as u32);
    assert_eq!(t.input_wait_button, pd_input_wait_button as usize as u32);
    assert_eq!(t.input_get_battery_pct, pd_input_get_battery_pct as usize as u32);
}
