use kernel::input::longpress::LongPressDetector;
use kernel::input::{ButtonEvent, ButtonId};

#[test]
fn short_hold_emits_tap_on_release_input_longpress() {
    let mut lp = LongPressDetector::new(5);
    // Hold Up for 3 ticks (< threshold)
    assert_eq!(lp.update(Some(ButtonId::Up)), None);
    assert_eq!(lp.update(Some(ButtonId::Up)), None);
    assert_eq!(lp.update(Some(ButtonId::Up)), None);
    // Release
    assert_eq!(lp.update(None), Some(ButtonEvent::Tap(ButtonId::Up)));
}

#[test]
fn long_hold_emits_longpress_at_threshold_input_longpress() {
    let mut lp = LongPressDetector::new(3);
    assert_eq!(lp.update(Some(ButtonId::Select)), None);
    assert_eq!(lp.update(Some(ButtonId::Select)), None);
    // Tick 3 — threshold reached
    assert_eq!(lp.update(Some(ButtonId::Select)), Some(ButtonEvent::LongPress(ButtonId::Select)));
}

#[test]
fn longpress_does_not_re_fire_input_longpress() {
    let mut lp = LongPressDetector::new(3);
    lp.update(Some(ButtonId::Back));
    lp.update(Some(ButtonId::Back));
    lp.update(Some(ButtonId::Back)); // fires
    // Continue holding
    assert_eq!(lp.update(Some(ButtonId::Back)), None);
    assert_eq!(lp.update(Some(ButtonId::Back)), None);
}

#[test]
fn no_tap_after_longpress_input_longpress() {
    let mut lp = LongPressDetector::new(3);
    lp.update(Some(ButtonId::Down));
    lp.update(Some(ButtonId::Down));
    lp.update(Some(ButtonId::Down)); // LongPress fired
    // Release should produce no Tap
    assert_eq!(lp.update(None), None);
}

#[test]
fn state_resets_after_release_input_longpress() {
    let mut lp = LongPressDetector::new(3);
    // Trigger a long press
    lp.update(Some(ButtonId::Up));
    lp.update(Some(ButtonId::Up));
    lp.update(Some(ButtonId::Up));
    lp.update(None); // release
    // Next short hold should Tap again
    lp.update(Some(ButtonId::Up));
    assert_eq!(lp.update(None), Some(ButtonEvent::Tap(ButtonId::Up)));
}
