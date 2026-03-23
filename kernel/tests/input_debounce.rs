use kernel::input::debounce::DebounceFilter;
use kernel::input::ButtonId;

#[test]
fn debounce_bouncy_sequence_holds_none_until_threshold_input_debounce() {
    // threshold=3: needs 3 consecutive identical readings to change stable
    let mut f = DebounceFilter::new(3);
    // bounce: A None A A A
    assert_eq!(f.update(Some(ButtonId::Up)), None);   // 1x Up — not stable yet
    assert_eq!(f.update(None), None);                  // broken streak
    assert_eq!(f.update(Some(ButtonId::Up)), None);   // 1x Up again
    assert_eq!(f.update(Some(ButtonId::Up)), None);   // 2x Up
    assert_eq!(f.update(Some(ButtonId::Up)), Some(ButtonId::Up)); // 3x — stable
}

#[test]
fn debounce_flip_resets_counter_input_debounce() {
    let mut f = DebounceFilter::new(3);
    f.update(Some(ButtonId::Up));
    f.update(Some(ButtonId::Up));
    // flip to B before reaching threshold
    f.update(Some(ButtonId::Down));
    // Two more B's — still not 3 consecutive
    assert_eq!(f.update(Some(ButtonId::Down)), None);
    assert_eq!(f.update(Some(ButtonId::Down)), Some(ButtonId::Down));
}

#[test]
fn debounce_release_path_returns_to_none_input_debounce() {
    let mut f = DebounceFilter::new(2);
    // stabilise on Up
    f.update(Some(ButtonId::Up));
    f.update(Some(ButtonId::Up));
    // release: 2 consecutive Nones
    assert_eq!(f.update(None), Some(ButtonId::Up)); // still Up
    assert_eq!(f.update(None), None);                // now stable None
}
