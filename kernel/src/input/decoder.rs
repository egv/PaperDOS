use crate::input::ButtonId;

/// GPIO1 4-button resistor ladder: Right=3mV±50, Left=1113mV±150, Select=1984mV±150, Back=2556mV±150.
///
/// Returns `None` when the voltage does not fall within any button's window (float / no press).
/// When windows overlap (they don't here), the lower center wins.
///
/// The Right window starts at 0 mV. This is correct: the pin is actively driven by the
/// ladder whenever the poller is running, so a 0 mV reading unambiguously means Right is
/// pressed. A floating/disconnected pin cannot reach this rail by accident.
pub fn decode_gpio1(mv: u16) -> Option<ButtonId> {
    const WINDOWS: &[(u16, u16, ButtonId)] = &[
        (0,    53,   ButtonId::Right),
        (963,  1263, ButtonId::Left),
        (1834, 2134, ButtonId::Select),
        (2406, 2706, ButtonId::Back),
    ];
    decode(mv, WINDOWS)
}

/// GPIO2 2-button resistor ladder: Down=3mV±50, Up=1659mV±150.
///
/// Returns `None` when the voltage does not fall within any button's window.
/// Same 0 mV assumption as `decode_gpio1`: pin is actively driven when polled.
pub fn decode_gpio2(mv: u16) -> Option<ButtonId> {
    const WINDOWS: &[(u16, u16, ButtonId)] = &[
        (0,    53,   ButtonId::Down),
        (1509, 1809, ButtonId::Up),
    ];
    decode(mv, WINDOWS)
}

fn decode(mv: u16, windows: &[(u16, u16, ButtonId)]) -> Option<ButtonId> {
    for &(lo, hi, id) in windows {
        if mv >= lo && mv <= hi {
            return Some(id);
        }
    }
    None
}
