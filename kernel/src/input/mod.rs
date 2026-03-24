pub mod adc;
pub mod debounce;
pub mod decoder;
pub mod longpress;
pub mod poller;

/// One of the six physical buttons on the X4 board.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ButtonId {
    Up,
    Down,
    Left,
    Right,
    Select,
    Back,
}

/// A resolved button action emitted by the input pipeline.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ButtonEvent {
    /// Button became stably pressed.
    Press(ButtonId),
    /// Button was released.
    Release(ButtonId),
    /// Button held for at least `threshold_ticks` polling ticks.
    LongPress(ButtonId),
    /// Button repeat after long-press.
    Repeat(ButtonId),
}
