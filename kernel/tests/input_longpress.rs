use kernel::input::longpress::LongPressDetector;
use kernel::input::{ButtonEvent, ButtonId};

#[test]
fn short_hold_emits_press_then_release_input_longpress() {
    let mut lp = LongPressDetector::new(5);
    assert_eq!(
        lp.update(Some(ButtonId::Up)),
        Some(ButtonEvent::Press(ButtonId::Up))
    );
    assert_eq!(lp.update(Some(ButtonId::Up)), None);
    assert_eq!(lp.update(Some(ButtonId::Up)), None);
    assert_eq!(lp.update(None), Some(ButtonEvent::Release(ButtonId::Up)));
}

#[test]
fn long_hold_emits_longpress_at_threshold_input_longpress() {
    let mut lp = LongPressDetector::new(3);
    assert_eq!(
        lp.update(Some(ButtonId::Select)),
        Some(ButtonEvent::Press(ButtonId::Select))
    );
    assert_eq!(lp.update(Some(ButtonId::Select)), None);
    // Tick 3 — threshold reached
    assert_eq!(
        lp.update(Some(ButtonId::Select)),
        Some(ButtonEvent::LongPress(ButtonId::Select))
    );
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
fn release_still_emits_after_longpress_input_longpress() {
    let mut lp = LongPressDetector::new(3);
    lp.update(Some(ButtonId::Down));
    lp.update(Some(ButtonId::Down));
    lp.update(Some(ButtonId::Down)); // LongPress fired
    assert_eq!(lp.update(None), Some(ButtonEvent::Release(ButtonId::Down)));
}

#[test]
fn state_resets_after_release_input_longpress() {
    let mut lp = LongPressDetector::new(3);
    // Trigger a long press
    lp.update(Some(ButtonId::Up));
    lp.update(Some(ButtonId::Up));
    lp.update(Some(ButtonId::Up));
    lp.update(None); // release
    assert_eq!(
        lp.update(Some(ButtonId::Up)),
        Some(ButtonEvent::Press(ButtonId::Up))
    );
    assert_eq!(lp.update(None), Some(ButtonEvent::Release(ButtonId::Up)));
}
