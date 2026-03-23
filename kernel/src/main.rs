#![cfg_attr(all(target_arch = "riscv32", target_os = "none"), no_std)]
#![cfg_attr(all(target_arch = "riscv32", target_os = "none"), no_main)]

#[cfg(all(target_arch = "riscv32", target_os = "none"))]
esp_bootloader_esp_idf::esp_app_desc!();

#[cfg(all(target_arch = "riscv32", target_os = "none"))]
mod device {
    use core::hint::spin_loop;

    use embassy_executor::Spawner;
    use embedded_sdmmc::{Block, BlockCount, BlockDevice, BlockIdx};
    use esp_hal::{
        clock::CpuClock, interrupt::software::SoftwareInterruptControl, main,
        timer::timg::TimerGroup, Config,
    };
    use kernel::device::display::X4DisplayTransport;
    use kernel::display::ssd1677::{
        emit_addressing_init_block, emit_power_init_block, emit_reset_preamble,
    };
    use kernel::storage::StorageError;
    use kernel::syscall::display::{display_refresh_flush, display_scene_flush_current, set_display_flush_fn};
    use kernel::syscall::fs::{set_fs_closedir_fn, set_fs_opendir_fn, set_fs_readdir_fn};
    use kernel::syscall::input::{set_input_get_buttons_fn, set_input_wait_button_fn};
    use static_cell::StaticCell;

    static EXECUTOR: StaticCell<esp_rtos::embassy::Executor> = StaticCell::new();
    static TRANSPORT_CELL: StaticCell<X4DisplayTransport> = StaticCell::new();

    /// Raw pointer to the live transport; set once in `main()` before the executor starts.
    static mut TRANSPORT_PTR: *mut X4DisplayTransport = core::ptr::null_mut();

    // ── F8: Device-backed fn-pointer implementations ──────────────────────────

    /// Flush the packed framebuffer to the SSD1677 and trigger a full refresh.
    fn device_display_flush(_mode: i32) {
        // SAFETY: TRANSPORT_PTR is written once in main() before any task runs;
        // single-core kernel context, no concurrent modification.
        unsafe {
            if TRANSPORT_PTR.is_null() {
                return;
            }
            let _ = display_scene_flush_current(&mut *TRANSPORT_PTR);
        }
    }

    /// Return current button state bitmask (stub — real ADC wired in EPIC-P1-B).
    fn device_get_buttons() -> u32 {
        0
    }

    /// Block until a button event (stub — polls at 100 ms intervals until EPIC-P1-B wires ADC).
    fn device_wait_button() -> u32 {
        esp_hal::delay::Delay::new().delay_millis(100);
        0
    }

    /// Open a directory by path (stub — real SD-backed impl wired in EPIC-P1-C).
    unsafe fn device_opendir(_path: *const u8, _len: usize) -> i32 {
        -1
    }

    /// Read one directory entry (stub — real SD-backed impl wired in EPIC-P1-C).
    unsafe fn device_readdir(_handle: i32, _dirent_buf: *mut u8) -> i32 {
        -1
    }

    /// Close an open directory (stub — real SD-backed impl wired in EPIC-P1-C).
    fn device_closedir(_handle: i32) -> i32 {
        -1
    }

    /// Minimal no-op block device used as the `FsState` backing until the real SD
    /// card SPI bus wiring is completed in EPIC-P1-C.
    struct NoopBlockDevice;

    impl BlockDevice for NoopBlockDevice {
        type Error = StorageError;

        fn num_blocks(&self) -> Result<BlockCount, Self::Error> {
            Ok(BlockCount(0))
        }

        fn read(
            &self,
            _blocks: &mut [Block],
            _start_block_idx: BlockIdx,
            _reason: &str,
        ) -> Result<(), Self::Error> {
            Err(StorageError::NotReady)
        }

        fn write(
            &self,
            _blocks: &[Block],
            _start_block_idx: BlockIdx,
        ) -> Result<(), Self::Error> {
            Err(StorageError::NotReady)
        }
    }

    #[panic_handler]
    fn panic(_info: &core::panic::PanicInfo<'_>) -> ! {
        loop {
            spin_loop();
        }
    }

    // ── F9: Launcher + app-load Embassy task ──────────────────────────────────

    /// 1-bit packed framebuffer image of boris.jpg (800×480, MSB-first, white=1).
    const BORIS_IMAGE: &[u8; kernel::syscall::display::FRAME_BYTES] =
        include_bytes!("../assets/boris.bin");

    /// Main Embassy task: show boot splash.
    ///
    /// Displays boris.jpg as a hello-world proof that the display pipeline works.
    /// Full launcher loop is re-enabled once SD card and ADC are wired (EPIC-P1-B/C).
    #[embassy_executor::task]
    async fn launcher_task(_spawner: Spawner) {
        unsafe {
            if !TRANSPORT_PTR.is_null() {
                let _ = display_refresh_flush(&mut *TRANSPORT_PTR, BORIS_IMAGE);
            }
        }

        // Hold here so the e-ink refresh completes and the image stays on screen.
        loop {
            spin_loop();
        }
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

        // F8: Store the transport and wire all device globals before spawning tasks.
        let transport_ref = TRANSPORT_CELL.init(display);
        // SAFETY: written once here before the executor starts; single-core.
        unsafe { TRANSPORT_PTR = transport_ref as *mut _; }

        // SAFETY: each setter is called exactly once at init; no concurrent access.
        unsafe {
            set_display_flush_fn(device_display_flush);
            set_input_get_buttons_fn(device_get_buttons);
            set_input_wait_button_fn(device_wait_button);
            set_fs_opendir_fn(device_opendir);
            set_fs_readdir_fn(device_readdir);
            set_fs_closedir_fn(device_closedir);
        }

        let timers = TimerGroup::new(peripherals.TIMG0);
        let software_interrupts = SoftwareInterruptControl::new(peripherals.SW_INTERRUPT);

        esp_rtos::start(timers.timer0, software_interrupts.software_interrupt0);

        let executor = EXECUTOR.init(esp_rtos::embassy::Executor::new());
        executor.run(|spawner| {
            spawner.must_spawn(launcher_task(spawner)); // F9
        })
    }
}

#[cfg(not(all(target_arch = "riscv32", target_os = "none")))]
fn main() {}
