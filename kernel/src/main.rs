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
