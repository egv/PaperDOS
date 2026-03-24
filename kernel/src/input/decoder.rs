use crate::input::ButtonId;

const GPIO1_THRESHOLDS: &[(u16, u16, ButtonId)] = &[
    (3, 50, ButtonId::Right),
    (1113, 150, ButtonId::Left),
    (1984, 150, ButtonId::Select),
    (2556, 150, ButtonId::Back),
];

const GPIO2_THRESHOLDS: &[(u16, u16, ButtonId)] =
    &[(3, 50, ButtonId::Down), (1659, 150, ButtonId::Up)];

/// GPIO1 4-button resistor ladder in calibrated millivolts.
///
/// These thresholds are ported directly from `pulp-os`.
pub fn decode_gpio1(mv: u16) -> Option<ButtonId> {
    decode_ladder(mv, GPIO1_THRESHOLDS)
}

/// GPIO2 2-button resistor ladder in calibrated millivolts.
///
/// These thresholds are ported directly from `pulp-os`.
pub fn decode_gpio2(mv: u16) -> Option<ButtonId> {
    decode_ladder(mv, GPIO2_THRESHOLDS)
}

fn decode_ladder(mv: u16, thresholds: &[(u16, u16, ButtonId)]) -> Option<ButtonId> {
    for &(center, tolerance, button) in thresholds {
        let low = center.saturating_sub(tolerance);
        let high = center.saturating_add(tolerance);
        if mv >= low && mv <= high {
            return Some(button);
        }
    }
    None
}
