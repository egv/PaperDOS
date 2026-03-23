use kernel::device::display::{
    DISPLAY_BUSY_PIN, DISPLAY_CS_PIN, DISPLAY_DC_PIN, DISPLAY_MOSI_PIN, DISPLAY_RST_PIN,
    DISPLAY_SCLK_PIN,
};

#[test]
fn x4_display_pins_display_device() {
    assert_eq!(DISPLAY_SCLK_PIN, 8);
    assert_eq!(DISPLAY_MOSI_PIN, 10);
    assert_eq!(DISPLAY_CS_PIN, 21);
    assert_eq!(DISPLAY_DC_PIN, 4);
    assert_eq!(DISPLAY_RST_PIN, 5);
    assert_eq!(DISPLAY_BUSY_PIN, 6);
}
