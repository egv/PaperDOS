mod common;

use common::ScriptedAdc;
use kernel::input::poller::InputPoller;
use kernel::input::{ButtonEvent, ButtonId};

fn make_poller<'a>(gpio1: &'a [u16], gpio2: &'a [u16]) -> InputPoller<ScriptedAdc<'a>> {
    // debounce=1 (immediate), longpress_threshold=3
    InputPoller::new(ScriptedAdc::new(gpio1, gpio2), 1, 3)
}

// GPIO1 Right ~= 3 mV, GPIO2 idle ~= 3000+ mV.

#[test]
fn stable_gpio1_right_emits_press_and_release_input_poller() {
    let mut p = make_poller(&[3, 3000], &[3000, 3000]);
    assert_eq!(p.poll().unwrap(), Some(ButtonEvent::Press(ButtonId::Right)));
    assert_eq!(
        p.poll().unwrap(),
        Some(ButtonEvent::Release(ButtonId::Right))
    );
}

#[test]
fn stable_gpio2_up_emits_press_and_release_input_poller() {
    let mut p = make_poller(&[3000, 3000], &[1659, 3000]);
    assert_eq!(p.poll().unwrap(), Some(ButtonEvent::Press(ButtonId::Up)));
    assert_eq!(p.poll().unwrap(), Some(ButtonEvent::Release(ButtonId::Up)));
}

#[test]
fn long_hold_gpio1_right_emits_longpress_input_poller() {
    let mut p = make_poller(&[3, 3, 3, 3, 3000], &[3000, 3000, 3000, 3000, 3000]);
    assert_eq!(p.poll().unwrap(), Some(ButtonEvent::Press(ButtonId::Right)));
    assert_eq!(p.poll().unwrap(), None);
    assert_eq!(p.poll().unwrap(), None);
    assert_eq!(
        p.poll().unwrap(),
        Some(ButtonEvent::LongPress(ButtonId::Right))
    );
    assert_eq!(
        p.poll().unwrap(),
        Some(ButtonEvent::Release(ButtonId::Right))
    );
}

#[test]
fn gpio1_wins_when_both_active_input_poller() {
    let mut p = make_poller(&[3], &[3]);
    assert_eq!(p.poll().unwrap(), Some(ButtonEvent::Press(ButtonId::Right)));
}
