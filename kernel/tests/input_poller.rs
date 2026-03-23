use kernel::input::adc::ScriptedAdc;
use kernel::input::poller::InputPoller;
use kernel::input::{ButtonEvent, ButtonId};

fn make_poller<'a>(gpio1: &'a [u16], gpio2: &'a [u16]) -> InputPoller<ScriptedAdc<'a>> {
    // debounce=1 (immediate), longpress_threshold=3
    InputPoller::new(ScriptedAdc::new(gpio1, gpio2), 1, 3)
}

// GPIO1 Right = 3 mV, GPIO2 float = 3300 mV

#[test]
fn stable_gpio1_right_emits_tap_input_poller() {
    // Single press: one reading Right, then float (release)
    let mut p = make_poller(&[3, 3300], &[3300, 3300]);
    assert_eq!(p.poll().unwrap(), None);       // Right debounced in, no event yet
    assert_eq!(p.poll().unwrap(), Some(ButtonEvent::Tap(ButtonId::Right))); // release -> Tap
}

#[test]
fn stable_gpio2_up_emits_tap_input_poller() {
    // GPIO1 float, GPIO2 Up then release
    let mut p = make_poller(&[3300, 3300], &[1659, 3300]);
    assert_eq!(p.poll().unwrap(), None);
    assert_eq!(p.poll().unwrap(), Some(ButtonEvent::Tap(ButtonId::Up)));
}

#[test]
fn long_hold_gpio1_right_emits_longpress_input_poller() {
    // debounce=1, longpress=3: 3 consecutive Right readings
    let mut p = make_poller(&[3, 3, 3, 3300], &[3300, 3300, 3300, 3300]);
    assert_eq!(p.poll().unwrap(), None);    // tick 1 — Right stable
    assert_eq!(p.poll().unwrap(), None);    // tick 2
    assert_eq!(p.poll().unwrap(), Some(ButtonEvent::LongPress(ButtonId::Right))); // tick 3
    assert_eq!(p.poll().unwrap(), None);    // release — no Tap after LongPress
}

#[test]
fn gpio1_wins_when_both_active_input_poller() {
    // GPIO1 = Right (3 mV), GPIO2 = Down (3 mV) — both active; GPIO1 has priority
    let mut p = make_poller(&[3, 3300], &[3, 3300]);
    assert_eq!(p.poll().unwrap(), None);
    assert_eq!(p.poll().unwrap(), Some(ButtonEvent::Tap(ButtonId::Right)));
}
