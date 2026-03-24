use kernel::input::{ButtonEvent, ButtonId};

#[test]
fn button_id_variants_input_types() {
    let _up = ButtonId::Up;
    let _down = ButtonId::Down;
    let _left = ButtonId::Left;
    let _right = ButtonId::Right;
    let _select = ButtonId::Select;
    let _back = ButtonId::Back;
}

#[test]
fn button_event_variants_input_types() {
    let tap = ButtonEvent::Press(ButtonId::Up);
    let lp = ButtonEvent::LongPress(ButtonId::Up);
    assert_ne!(tap, lp);
}

#[test]
fn button_event_copy_input_types() {
    let e = ButtonEvent::Press(ButtonId::Select);
    let e2 = e;
    assert_eq!(e, e2);
}
