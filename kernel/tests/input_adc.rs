mod common;

use common::ScriptedAdc;
use kernel::input::adc::{trimmed_mean4, AdcSource};

#[test]
fn scripted_adc_returns_sequence_in_order_input_adc() {
    let mut adc = ScriptedAdc::new(&[100, 200, 300], &[10, 20]);
    assert_eq!(adc.read_gpio1().unwrap(), 100);
    assert_eq!(adc.read_gpio1().unwrap(), 200);
    assert_eq!(adc.read_gpio1().unwrap(), 300);
}

#[test]
fn scripted_adc_repeats_last_value_input_adc() {
    let mut adc = ScriptedAdc::new(&[42], &[0]);
    assert_eq!(adc.read_gpio1().unwrap(), 42);
    assert_eq!(adc.read_gpio1().unwrap(), 42);
    assert_eq!(adc.read_gpio1().unwrap(), 42);
}

#[test]
fn scripted_adc_channels_are_independent_input_adc() {
    let mut adc = ScriptedAdc::new(&[111, 222], &[333, 444]);
    assert_eq!(adc.read_gpio1().unwrap(), 111);
    assert_eq!(adc.read_gpio2().unwrap(), 333);
    assert_eq!(adc.read_gpio1().unwrap(), 222);
    assert_eq!(adc.read_gpio2().unwrap(), 444);
}

#[test]
fn trimmed_mean4_rejects_stale_first_sample_input_adc() {
    assert_eq!(trimmed_mean4([3, 4095, 4095, 4095]), 4095);
}

#[test]
fn trimmed_mean4_preserves_stable_button_reading_input_adc() {
    assert_eq!(trimmed_mean4([2655, 2650, 2660, 2655]), 2655);
}
