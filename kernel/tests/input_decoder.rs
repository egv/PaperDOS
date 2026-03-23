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

#[test]
fn decode_gpio1_inter_window_gaps_return_none_input_decoder() {
    // One above each window's upper bound — must not match anything.
    assert_eq!(decode_gpio1(54), None);    // just above Right (0–53)
    assert_eq!(decode_gpio1(1264), None);  // just above Left (963–1263)
    assert_eq!(decode_gpio1(2135), None);  // just above Select (1834–2134)
    assert_eq!(decode_gpio1(2707), None);  // just above Back (2406–2706)
    // One below each window's lower bound (except Right which starts at 0).
    assert_eq!(decode_gpio1(962), None);   // just below Left
    assert_eq!(decode_gpio1(1833), None);  // just below Select
    assert_eq!(decode_gpio1(2405), None);  // just below Back
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

#[test]
fn decode_gpio2_inter_window_gaps_return_none_input_decoder() {
    assert_eq!(decode_gpio2(54), None);    // just above Down (0–53)
    assert_eq!(decode_gpio2(1508), None);  // just below Up (1509–1809)
    assert_eq!(decode_gpio2(1810), None);  // just above Up
}
