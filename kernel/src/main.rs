#![cfg_attr(all(target_arch = "riscv32", target_os = "none"), no_std)]
#![cfg_attr(all(target_arch = "riscv32", target_os = "none"), no_main)]

#[cfg(all(target_arch = "riscv32", target_os = "none"))]
mod device {
    use core::future::pending;
    use core::hint::spin_loop;

    use embassy_executor::Spawner;
    use esp_hal::{
        clock::CpuClock, interrupt::software::SoftwareInterruptControl, main,
        timer::timg::TimerGroup, Config,
    };
    use kernel::device::display::X4DisplayTransport;
    use kernel::display::ssd1677::{
        emit_addressing_init_block, emit_power_init_block, emit_reset_preamble,
    };
    use static_cell::StaticCell;

    static EXECUTOR: StaticCell<esp_rtos::embassy::Executor> = StaticCell::new();

    #[panic_handler]
    fn panic(_info: &core::panic::PanicInfo<'_>) -> ! {
        loop {
            spin_loop();
        }
    }

    #[embassy_executor::task]
    async fn runtime_smoke(_spawner: Spawner) {
        pending::<()>().await;
    }

    #[main]
    fn main() -> ! {
        let config = Config::default().with_cpu_clock(CpuClock::max());
        let peripherals = esp_hal::init(config);

        let mut display = X4DisplayTransport::new(
            peripherals.SPI2,
            peripherals.DMA_CH0,
            peripherals.GPIO8,
            peripherals.GPIO10,
            peripherals.GPIO21,
            peripherals.GPIO4,
            peripherals.GPIO5,
            peripherals.GPIO6,
        );
        emit_reset_preamble(&mut display).unwrap();
        emit_power_init_block(&mut display).unwrap();
        emit_addressing_init_block(&mut display).unwrap();

        let timers = TimerGroup::new(peripherals.TIMG0);
        let software_interrupts = SoftwareInterruptControl::new(peripherals.SW_INTERRUPT);

        esp_rtos::start(timers.timer0, software_interrupts.software_interrupt0);

        let executor = EXECUTOR.init(esp_rtos::embassy::Executor::new());
        executor.run(|spawner| {
            spawner.must_spawn(runtime_smoke(spawner));
        })
    }
}

#[cfg(not(all(target_arch = "riscv32", target_os = "none")))]
fn main() {}
