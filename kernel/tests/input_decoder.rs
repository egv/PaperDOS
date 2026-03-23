use kernel::input::decoder::{decode_gpio1, decode_gpio2};
use kernel::input::ButtonId;

// GPIO1: Right=3mV±50, Left=1113mV±150, Select=1984mV±150, Back=2556mV±150

#[test]
fn decode_gpio1_right_input_decoder() {
    assert_eq!(decode_gpio1(3), Some(ButtonId::Right));
    assert_eq!(decode_gpio1(0), Some(ButtonId::Right));
    assert_eq!(decode_gpio1(53), Some(ButtonId::Right));
}

#[test]
fn decode_gpio1_left_input_decoder() {
    assert_eq!(decode_gpio1(1113), Some(ButtonId::Left));
    assert_eq!(decode_gpio1(963), Some(ButtonId::Left));
    assert_eq!(decode_gpio1(1263), Some(ButtonId::Left));
}

#[test]
fn decode_gpio1_select_input_decoder() {
    assert_eq!(decode_gpio1(1984), Some(ButtonId::Select));
    assert_eq!(decode_gpio1(1834), Some(ButtonId::Select));
    assert_eq!(decode_gpio1(2134), Some(ButtonId::Select));
}

#[test]
fn decode_gpio1_back_input_decoder() {
    assert_eq!(decode_gpio1(2556), Some(ButtonId::Back));
    assert_eq!(decode_gpio1(2406), Some(ButtonId::Back));
    assert_eq!(decode_gpio1(2706), Some(ButtonId::Back));
}

#[test]
fn decode_gpio1_float_returns_none_input_decoder() {
    assert_eq!(decode_gpio1(3300), None);
    assert_eq!(decode_gpio1(3000), None);
    assert_eq!(decode_gpio1(800), None);
}

// GPIO2: Down=3mV±50, Up=1659mV±150

#[test]
fn decode_gpio2_down_input_decoder() {
    assert_eq!(decode_gpio2(3), Some(ButtonId::Down));
    assert_eq!(decode_gpio2(0), Some(ButtonId::Down));
    assert_eq!(decode_gpio2(53), Some(ButtonId::Down));
}

#[test]
fn decode_gpio2_up_input_decoder() {
    assert_eq!(decode_gpio2(1659), Some(ButtonId::Up));
    assert_eq!(decode_gpio2(1509), Some(ButtonId::Up));
    assert_eq!(decode_gpio2(1809), Some(ButtonId::Up));
}

#[test]
fn decode_gpio2_float_returns_none_input_decoder() {
    assert_eq!(decode_gpio2(3300), None);
    assert_eq!(decode_gpio2(400), None);
}
