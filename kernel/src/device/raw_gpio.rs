/// Direct-register GPIO output for pins that esp-hal does not expose on ESP32-C3.
///
/// Xteink X4 uses GPIO12 as SD chip-select in DIO flash mode.

const GPIO_OUT_W1TS: u32 = 0x6000_4008;
const GPIO_OUT_W1TC: u32 = 0x6000_400C;
const GPIO_ENABLE_W1TS: u32 = 0x6000_4024;
const IO_MUX_BASE: u32 = 0x6000_9000;
const IO_MUX_PIN_STRIDE: u32 = 0x04;

pub struct RawOutputPin {
    mask: u32,
}

impl RawOutputPin {
    /// # Safety
    /// The caller must ensure `pin` is free for GPIO output and not owned by
    /// flash or another peripheral.
    pub unsafe fn new(pin: u8) -> Self {
        let mask = 1u32 << pin;
        let mux_reg = (IO_MUX_BASE + pin as u32 * IO_MUX_PIN_STRIDE) as *mut u32;

        unsafe {
            let val = mux_reg.read_volatile();
            mux_reg.write_volatile((val & !(0b111 << 12)) | (1 << 12));

            let out_sel = (0x6000_4554 + pin as u32 * 4) as *mut u32;
            out_sel.write_volatile(0x80);

            (GPIO_ENABLE_W1TS as *mut u32).write_volatile(mask);
            (GPIO_OUT_W1TS as *mut u32).write_volatile(mask);
        }

        Self { mask }
    }
}

impl embedded_hal::digital::ErrorType for RawOutputPin {
    type Error = core::convert::Infallible;
}

impl embedded_hal::digital::OutputPin for RawOutputPin {
    fn set_high(&mut self) -> Result<(), Self::Error> {
        unsafe {
            (GPIO_OUT_W1TS as *mut u32).write_volatile(self.mask);
        }
        Ok(())
    }

    fn set_low(&mut self) -> Result<(), Self::Error> {
        unsafe {
            (GPIO_OUT_W1TC as *mut u32).write_volatile(self.mask);
        }
        Ok(())
    }
}
