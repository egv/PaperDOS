#![cfg_attr(all(target_arch = "riscv32", target_os = "none"), no_std)]
#![cfg_attr(all(target_arch = "riscv32", target_os = "none"), no_main)]

#[cfg(all(target_arch = "riscv32", target_os = "none"))]
esp_bootloader_esp_idf::esp_app_desc!();

#[cfg(all(target_arch = "riscv32", target_os = "none"))]
mod device {
    use core::cell::RefCell;
    use core::hint::spin_loop;
    use core::{slice, str};
    use core::fmt::Write as _;

    use critical_section::Mutex;
    use embassy_executor::Spawner;
    use embedded_sdmmc::sdcard::DummyCsPin;
    use embedded_hal_bus::spi::CriticalSectionDevice;
    use esp_hal::{
        analog::adc::{Adc, AdcCalCurve, AdcConfig, AdcPin, Attenuation},
        clock::CpuClock, interrupt::software::SoftwareInterruptControl, main,
        peripherals::{ADC1, GPIO1, GPIO2},
        spi::{self, master::Config as SpiConfig, Mode},
        timer::timg::TimerGroup, Config,
        usb_serial_jtag::UsbSerialJtag,
    };
    use esp_hal::delay::Delay;
    use esp_hal::dma::{DmaDescriptor, DmaRxBuf, DmaTxBuf};
    use esp_hal::gpio::{Level, Output, OutputConfig};
    use esp_hal::time::Rate;
    use esp_hal::Blocking;
    use kernel::abi::{PdDirent, PD_FTYPE_DIR, PD_FTYPE_FILE};
    use kernel::boot_app::{load_and_run, JumpMode};
    use kernel::device::display::X4DisplayTransport;
    use kernel::device::raw_gpio::RawOutputPin;
    use kernel::device::storage::{RuntimeSdFs, SdSpiDevice};
    use kernel::display::ssd1677::{
        emit_addressing_init_block, emit_power_init_block, emit_reset_preamble,
    };
    use kernel::input::adc::AdcSource;
    use kernel::input::poller::InputPoller;
    use kernel::input::{ButtonEvent, ButtonId};
    use kernel::launcher::draw_text;
    use kernel::storage::fs::{DirHandle, PdDirEntry};
    use kernel::storage::fs::EntryType;
    use kernel::storage::StorageError;
    use kernel::syscall::build_syscall_table;
    use kernel::syscall::display::{
        display_clear_to, display_refresh_flush, display_scene_flush_current, draw_rect_in,
        fill_rect_in, set_display_flush_fn, FrameBuffer, FRAME_BYTES,
    };
    use kernel::syscall::fs::{set_fs_closedir_fn, set_fs_opendir_fn, set_fs_readdir_fn};
    use kernel::syscall::input::{
        button_event_to_mask, button_id_to_mask, set_input_get_buttons_fn, set_input_wait_button_fn,
    };
    use kernel::device::serial::{serial_write_bytes, set_serial_write_fn};
    use kernel::jump::jump_to_app;
    use kernel::launcher::{format_app_name, run_launcher_with_refresh};
    use static_cell::StaticCell;

    type SharedSpiBus = spi::master::SpiDmaBus<'static, Blocking>;
    type SharedSpiMutex = Mutex<RefCell<SharedSpiBus>>;
    type DisplaySpi =
        CriticalSectionDevice<'static, SharedSpiBus, Output<'static>, Delay>;
    type DeviceDisplay = X4DisplayTransport<DisplaySpi>;
    type DeviceFs = RuntimeSdFs<Delay>;

    const APP_REGION_BYTES: usize = 96 * 1024;
    const PDB_BUF_BYTES: usize = 64 * 1024;
    const ADC_OVERSAMPLE: u32 = 4;
    const BUTTON_DIAGNOSTIC_MODE: bool = false;
    /// When `true`, the kernel loads and prepares the selected app but does not
    /// jump to its entry point.  Set to `true` to isolate crashes that occur
    /// inside `jump_to_app` vs. those that occur in the load/prepare path.
    const LAUNCH_DRY_RUN: bool = false;

    #[derive(Clone, Copy, Debug, Eq, PartialEq)]
    struct ButtonDiagSnapshot {
        gpio1_mv: u16,
        gpio2_mv: u16,
        gpio1_decoded: Option<ButtonId>,
        gpio2_decoded: Option<ButtonId>,
        stable: Option<ButtonId>,
        last_event: Option<ButtonEvent>,
    }

    static SERIAL_CELL: StaticCell<UsbSerialJtag<'static>> = StaticCell::new();

    /// Raw pointer to the live USB Serial/JTAG instance; set once in `main()`.
    static mut SERIAL_PTR: *mut UsbSerialJtag<'static> = core::ptr::null_mut();

    fn device_serial_write(bytes: &[u8]) {
        // SAFETY: SERIAL_PTR written once before any task runs; single-core.
        unsafe {
            if SERIAL_PTR.is_null() {
                return;
            }
            let _ = (*SERIAL_PTR).write_bytes(bytes);
        }
    }

    static EXECUTOR: StaticCell<esp_rtos::embassy::Executor> = StaticCell::new();
    static TRANSPORT_CELL: StaticCell<DeviceDisplay> = StaticCell::new();
    static FS_CELL: StaticCell<DeviceFs> = StaticCell::new();
    static INPUT_CELL: StaticCell<InputPoller<X4AdcSource>> = StaticCell::new();
    static LAUNCHER_BUF_CELL: StaticCell<FrameBuffer> = StaticCell::new();
    static APP_REGION_CELL: StaticCell<[u8; APP_REGION_BYTES]> = StaticCell::new();
    static PDB_BUF_CELL: StaticCell<[u8; PDB_BUF_BYTES]> = StaticCell::new();
    static SPI_BUS_CELL: StaticCell<SharedSpiMutex> = StaticCell::new();
    static TX_DESC: StaticCell<[DmaDescriptor; 2]> = StaticCell::new();
    static RX_DESC: StaticCell<[DmaDescriptor; 2]> = StaticCell::new();
    static TX_BUF: StaticCell<[u8; 4096]> = StaticCell::new();
    static RX_BUF: StaticCell<[u8; 4096]> = StaticCell::new();

    /// Raw pointer to the live transport; set once in `main()` before the executor starts.
    static mut TRANSPORT_PTR: *mut DeviceDisplay = core::ptr::null_mut();
    static mut FS_PTR: *mut DeviceFs = core::ptr::null_mut();
    static mut INPUT_PTR: *mut InputPoller<X4AdcSource> = core::ptr::null_mut();
    static mut LAUNCHER_BUF_PTR: *mut FrameBuffer = core::ptr::null_mut();
    static mut APP_REGION_PTR: *mut [u8; APP_REGION_BYTES] = core::ptr::null_mut();
    static mut PDB_BUF_PTR: *mut [u8; PDB_BUF_BYTES] = core::ptr::null_mut();
    static mut BOOT_FAILURE: BootFailureKind = BootFailureKind::None;

    #[derive(Clone, Copy, Debug, Eq, PartialEq)]
    enum BootFailureKind {
        None,
        SdInitNotReady,
        SdInitIo,
        SdCapacityNotReady,
        SdCapacityIo,
    }

    struct X4AdcSource {
        adc: Adc<'static, ADC1<'static>, Blocking>,
        row1: AdcPin<GPIO1<'static>, ADC1<'static>, AdcCalCurve<ADC1<'static>>>,
        row2: AdcPin<GPIO2<'static>, ADC1<'static>, AdcCalCurve<ADC1<'static>>>,
    }

    impl X4AdcSource {
        fn new(
            adc1: ADC1<'static>,
            gpio1: GPIO1<'static>,
            gpio2: GPIO2<'static>,
        ) -> Self {
            let mut cfg = AdcConfig::new();
            let row1 = cfg.enable_pin_with_cal::<_, AdcCalCurve<ADC1>>(
                gpio1,
                Attenuation::_11dB,
            );
            let row2 = cfg.enable_pin_with_cal::<_, AdcCalCurve<ADC1>>(
                gpio2,
                Attenuation::_11dB,
            );
            let adc = Adc::new(adc1, cfg);

            Self { adc, row1, row2 }
        }
    }

    impl AdcSource for X4AdcSource {
        type Error = ();

        fn read_gpio1(&mut self) -> Result<u16, Self::Error> {
            sample_averaged(&mut self.adc, &mut self.row1)
        }

        fn read_gpio2(&mut self) -> Result<u16, Self::Error> {
            sample_averaged(&mut self.adc, &mut self.row2)
        }
    }

    fn sample_averaged<PIN>(
        adc: &mut Adc<'static, ADC1<'static>, Blocking>,
        pin: &mut AdcPin<PIN, ADC1<'static>, AdcCalCurve<ADC1<'static>>>,
    ) -> Result<u16, ()>
    where
        PIN: esp_hal::analog::adc::AdcChannel,
    {
        let mut sum = 0u32;
        for _ in 0..ADC_OVERSAMPLE {
            sum += nb::block!(adc.read_oneshot(pin)).map_err(|_| ())? as u32;
        }
        Ok((sum / ADC_OVERSAMPLE) as u16)
    }

    // ── F8: Device-backed fn-pointer implementations ──────────────────────────

    /// Flush the retained scene to the SSD1677 and trigger a refresh.
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

    fn device_refresh_frame(buf: &FrameBuffer) {
        unsafe {
            if TRANSPORT_PTR.is_null() {
                return;
            }
            let _ = display_refresh_flush(&mut *TRANSPORT_PTR, buf);
        }
    }

    fn render_boot_placeholder(buf: &mut FrameBuffer, failure: BootFailureKind) {
        display_clear_to(buf, 0xFF);
        draw_rect_in(buf, 8, 8, 464, 784, 0x00);
        fill_rect_in(buf, 24, 24, 432, 64, 0x00);
        let bars = match failure {
            BootFailureKind::None => 1,
            BootFailureKind::SdInitNotReady => 2,
            BootFailureKind::SdInitIo => 3,
            BootFailureKind::SdCapacityNotReady => 4,
            BootFailureKind::SdCapacityIo => 5,
        };
        for idx in 0..bars {
            fill_rect_in(buf, 24, 120 + idx * 48, 432 - idx * 24, 24, 0x00);
        }
    }

    fn button_name(id: Option<ButtonId>) -> &'static [u8] {
        match id {
            Some(ButtonId::Up) => b"VOLUP",
            Some(ButtonId::Down) => b"VOLDOWN",
            Some(ButtonId::Left) => b"LEFT",
            Some(ButtonId::Right) => b"RIGHT",
            Some(ButtonId::Select) => b"CONFIRM",
            Some(ButtonId::Back) => b"BACK",
            None => b"-",
        }
    }

    fn event_name(event: Option<ButtonEvent>) -> &'static [u8] {
        match event {
            Some(ButtonEvent::Press(_)) => b"PRESS",
            Some(ButtonEvent::Release(_)) => b"RELEASE",
            Some(ButtonEvent::LongPress(_)) => b"LONG",
            Some(ButtonEvent::Repeat(_)) => b"REPEAT",
            None => b"-",
        }
    }

    fn event_button(event: Option<ButtonEvent>) -> Option<ButtonId> {
        match event {
            Some(ButtonEvent::Press(id))
            | Some(ButtonEvent::Release(id))
            | Some(ButtonEvent::LongPress(id))
            | Some(ButtonEvent::Repeat(id)) => Some(id),
            None => None,
        }
    }

    fn append_bytes(buf: &mut [u8], len: &mut usize, bytes: &[u8]) {
        let n = core::cmp::min(bytes.len(), buf.len().saturating_sub(*len));
        buf[*len..*len + n].copy_from_slice(&bytes[..n]);
        *len += n;
    }

    fn append_u16(buf: &mut [u8], len: &mut usize, mut value: u16) {
        let mut digits = [0u8; 5];
        let mut count = 0usize;
        loop {
            digits[count] = b'0' + (value % 10) as u8;
            count += 1;
            value /= 10;
            if value == 0 {
                break;
            }
        }
        while count > 0 {
            count -= 1;
            append_bytes(buf, len, &digits[count..count + 1]);
        }
    }

    fn draw_diag_line(buf: &mut FrameBuffer, y: i32, prefix: &[u8], value: u16, suffix: &[u8]) {
        let mut line = [b' '; 32];
        let mut len = 0usize;
        append_bytes(&mut line, &mut len, prefix);
        append_u16(&mut line, &mut len, value);
        append_bytes(&mut line, &mut len, suffix);
        draw_text(buf, 16, y, &line[..len], 0x00);
    }

    fn render_button_diagnostics(buf: &mut FrameBuffer, snap: &ButtonDiagSnapshot) {
        display_clear_to(buf, 0xFF);
        draw_rect_in(buf, 8, 8, 464, 784, 0x00);
        fill_rect_in(buf, 0, 0, 480, 64, 0x00);
        draw_text(buf, 16, 12, b"BUTTON DIAG", 0xFF);

        draw_diag_line(buf, 96, b"G1 ", snap.gpio1_mv, b" ");
        draw_text(buf, 16, 132, button_name(snap.gpio1_decoded), 0x00);

        draw_diag_line(buf, 220, b"G2 ", snap.gpio2_mv, b" ");
        draw_text(buf, 16, 256, button_name(snap.gpio2_decoded), 0x00);

        draw_text(buf, 16, 372, b"STABLE", 0x00);
        draw_text(buf, 16, 408, button_name(snap.stable), 0x00);

        draw_text(buf, 16, 524, b"EVENT", 0x00);
        draw_text(buf, 16, 560, event_name(snap.last_event), 0x00);
        draw_text(buf, 16, 596, button_name(event_button(snap.last_event)), 0x00);

        draw_text(buf, 16, 704, b"VOLUP VOLDOWN", 0x00);
        draw_text(buf, 16, 740, b"CONFIRM BACK", 0x00);
    }

    fn render_diag_boot_marker(buf: &mut FrameBuffer) {
        display_clear_to(buf, 0xFF);
        fill_rect_in(buf, 0, 0, 480, 96, 0x00);
        fill_rect_in(buf, 0, 704, 480, 96, 0x00);
        draw_rect_in(buf, 16, 120, 448, 560, 0x00);
        fill_rect_in(buf, 48, 176, 384, 48, 0x00);
        fill_rect_in(buf, 48, 288, 320, 48, 0x00);
        fill_rect_in(buf, 48, 400, 256, 48, 0x00);
        draw_text(buf, 32, 28, b"BUTTON DIAG", 0xFF);
    }

    fn render_launcher_boot_marker(buf: &mut FrameBuffer) {
        display_clear_to(buf, 0xFF);
        fill_rect_in(buf, 0, 0, 480, 96, 0x00);
        draw_rect_in(buf, 16, 136, 448, 528, 0x00);
        fill_rect_in(buf, 48, 208, 384, 56, 0x00);
        fill_rect_in(buf, 48, 336, 288, 40, 0x00);
        fill_rect_in(buf, 48, 440, 224, 40, 0x00);
        draw_text(buf, 32, 28, b"LAUNCHER BOOT", 0xFF);
    }

    fn map_boot_failure(stage: &'static str, err: StorageError) -> BootFailureKind {
        match (stage, err) {
            ("sd_init", StorageError::NotReady) => BootFailureKind::SdInitNotReady,
            ("sd_init", _) => BootFailureKind::SdInitIo,
            ("sd_capacity", StorageError::NotReady) => BootFailureKind::SdCapacityNotReady,
            ("sd_capacity", _) => BootFailureKind::SdCapacityIo,
            _ => BootFailureKind::None,
        }
    }

    fn device_get_buttons() -> u32 {
        unsafe {
            if INPUT_PTR.is_null() {
                return 0;
            }
            (*INPUT_PTR)
                .current_button()
                .map(button_id_to_mask)
                .unwrap_or(0)
        }
    }

    fn device_wait_button() -> u32 {
        loop {
            unsafe {
                if INPUT_PTR.is_null() {
                    return 0;
                }
                match (*INPUT_PTR).poll() {
                    Ok(Some(event @ (ButtonEvent::Press(_)
                    | ButtonEvent::LongPress(_)
                    | ButtonEvent::Repeat(_)))) => return button_event_to_mask(event),
                    Ok(Some(ButtonEvent::Release(_))) | Ok(None) | Err(_) => {}
                }
            }
            Delay::new().delay_millis(10);
        }
    }

    unsafe fn device_opendir(path: *const u8, len: usize) -> i32 {
        if FS_PTR.is_null() {
            return -1;
        }
        let Ok(path) = str::from_utf8(unsafe { slice::from_raw_parts(path, len) }) else {
            return -1;
        };
        unsafe { (&mut *FS_PTR).fs_opendir(path).map(|h| h.to_raw()).unwrap_or(-1) }
    }

    unsafe fn device_readdir(handle: i32, dirent_buf: *mut u8) -> i32 {
        if FS_PTR.is_null() {
            return -1;
        }
        let Some(handle) = DirHandle::from_raw(handle) else {
            return -1;
        };

        match unsafe { (&mut *FS_PTR).fs_readdir(handle) } {
            Ok(Some(entry)) => {
                unsafe { write_pd_dirent(dirent_buf, &entry) };
                0
            }
            Ok(None) => 1,
            Err(_) => -1,
        }
    }

    fn device_closedir(handle: i32) -> i32 {
        let Some(handle) = DirHandle::from_raw(handle) else {
            return -1;
        };
        unsafe {
            if FS_PTR.is_null() {
                return -1;
            }
            (&mut *FS_PTR).fs_closedir(handle).map(|_| 0).unwrap_or(-1)
        }
    }

    unsafe fn write_pd_dirent(buf: *mut u8, entry: &PdDirEntry) {
        let dirent = &mut *buf.cast::<PdDirent>();
        *dirent = PdDirent {
            name: [0u8; 256],
            entry_type: match entry.entry_type {
                EntryType::File => PD_FTYPE_FILE,
                EntryType::Directory => PD_FTYPE_DIR,
            },
            size: entry.size,
        };
        let mut name = [0u8; 13];
        let len = format_app_name(&entry.name, &mut name);
        dirent.name[..len].copy_from_slice(&name[..len]);
    }

    #[panic_handler]
    fn panic(info: &core::panic::PanicInfo<'_>) -> ! {
        // Emit a deterministic crash report via USB Serial/JTAG so the panic
        // location is visible on the host instead of the device freezing silently.
        serial_write_bytes(b"\r\n!!! PANIC !!!\r\n");
        if let Some(loc) = info.location() {
            // Format "file:line\r\n" into a small stack buffer; no heap needed.
            let mut buf = [0u8; 128];
            let mut cursor = &mut buf[..];
            let _ = write!(cursor, "{}:{}\r\n", loc.file(), loc.line());
            let written = 128 - cursor.len();
            serial_write_bytes(&buf[..written]);
        }
        loop {
            spin_loop();
        }
    }

    // ── F9: Launcher + app-load Embassy task ──────────────────────────────────
    #[embassy_executor::task]
    async fn launcher_task(_spawner: Spawner) {
        unsafe {
            if TRANSPORT_PTR.is_null() || LAUNCHER_BUF_PTR.is_null() {
                loop {
                    spin_loop();
                }
            }

            let launcher_buf = &mut *LAUNCHER_BUF_PTR;

            if BUTTON_DIAGNOSTIC_MODE {
                let mut last_event = None;
                let mut last_snapshot = ButtonDiagSnapshot {
                    gpio1_mv: u16::MAX,
                    gpio2_mv: u16::MAX,
                    gpio1_decoded: None,
                    gpio2_decoded: None,
                    stable: None,
                    last_event: None,
                };

                loop {
                    if INPUT_PTR.is_null() {
                        render_boot_placeholder(launcher_buf, BootFailureKind::None);
                        device_refresh_frame(launcher_buf);
                        loop {
                            spin_loop();
                        }
                    }

                    if let Ok(event) = (&mut *INPUT_PTR).poll() {
                        if let Some(event) = event {
                            last_event = Some(event);
                        }
                    }

                    let (gpio1_mv, gpio2_mv) = (&*INPUT_PTR).last_samples();
                    let (gpio1_decoded, gpio2_decoded) = (&*INPUT_PTR).last_decoded();
                    let snapshot = ButtonDiagSnapshot {
                        gpio1_mv,
                        gpio2_mv,
                        gpio1_decoded,
                        gpio2_decoded,
                        stable: (&*INPUT_PTR).current_button(),
                        last_event,
                    };

                    if snapshot != last_snapshot {
                        render_button_diagnostics(launcher_buf, &snapshot);
                        device_refresh_frame(launcher_buf);
                        last_snapshot = snapshot;
                    }

                    Delay::new().delay_millis(20);
                }
            }

            if FS_PTR.is_null() || APP_REGION_PTR.is_null() || PDB_BUF_PTR.is_null() {
                render_boot_placeholder(launcher_buf, BOOT_FAILURE);
                device_refresh_frame(launcher_buf);
                loop {
                    spin_loop();
                }
            }

            let fs = &mut *FS_PTR;
            let app_region = &mut (&mut *APP_REGION_PTR)[..];
            let pdb_buf = &mut (&mut *PDB_BUF_PTR)[..];

            loop {
                let filename = run_launcher_with_refresh(fs, launcher_buf, device_refresh_frame);
                serial_write_bytes(b"LAUNCH:confirm\n");
                let syscalls = build_syscall_table(app_region.as_ptr() as u32, app_region.len() as u32);
                let mode = if LAUNCH_DRY_RUN {
                    JumpMode::DryRun
                } else {
                    JumpMode::Jump(jump_to_app)
                };
                let _ = load_and_run(fs, &filename, pdb_buf, app_region, &syscalls, mode);
            }
        }
    }

    #[main]
    fn main() -> ! {
        let config = Config::default().with_cpu_clock(CpuClock::max());
        let peripherals = esp_hal::init(config);

        // ── G1: USB Serial/JTAG console — must come first so panics can emit ──
        let serial = SERIAL_CELL.init(UsbSerialJtag::new(peripherals.USB_DEVICE));
        // SAFETY: written once before any task or panic handler runs; single-core.
        unsafe {
            SERIAL_PTR = serial as *mut _;
            set_serial_write_fn(device_serial_write);
        }
        serial_write_bytes(b"\r\nPaperDOS kernel boot\r\n");

        let adc1 = unsafe { peripherals.ADC1.clone_unchecked() };
        let gpio1 = unsafe { peripherals.GPIO1.clone_unchecked() };
        let gpio2 = unsafe { peripherals.GPIO2.clone_unchecked() };

        let tx_buf = DmaTxBuf::new(
            TX_DESC.init([DmaDescriptor::EMPTY; 2]),
            TX_BUF.init([0u8; 4096]),
        )
        .unwrap();
        let rx_buf = DmaRxBuf::new(
            RX_DESC.init([DmaDescriptor::EMPTY; 2]),
            RX_BUF.init([0u8; 4096]),
        )
        .unwrap();

        let mut spi_raw = spi::master::Spi::new(
            peripherals.SPI2,
            SpiConfig::default()
                .with_frequency(Rate::from_khz(400))
                .with_mode(Mode::_0),
        )
        .unwrap()
        .with_sck(peripherals.GPIO8)
        .with_mosi(peripherals.GPIO10)
        .with_miso(peripherals.GPIO7);
        let _ = spi_raw.write(&[0xFF; 10]);
        let spi_bus = spi_raw
            .with_dma(peripherals.DMA_CH0)
            .with_buffers(rx_buf, tx_buf);
        let spi_ref = SPI_BUS_CELL.init(Mutex::new(RefCell::new(spi_bus)));

        let fs = if BUTTON_DIAGNOSTIC_MODE {
            Err(BootFailureKind::None)
        } else {
            let sd_spi: SdSpiDevice<'static, Delay> = CriticalSectionDevice::new(
                spi_ref,
                DummyCsPin,
                Delay::new(),
            )
            .unwrap();
            DeviceFs::from_spi2(
                sd_spi,
                // SAFETY: GPIO12 is free for SD CS on the X4 in DIO flash mode.
                unsafe { RawOutputPin::new(12) },
                Delay::new(),
            )
            .map_err(|err| map_boot_failure("sd_init", err))
        };

        let display_spi = CriticalSectionDevice::new(
            spi_ref,
            Output::new(peripherals.GPIO21, Level::High, OutputConfig::default()),
            Delay::new(),
        )
        .unwrap();
        let mut display = X4DisplayTransport::new(
            display_spi,
            peripherals.GPIO4,
            peripherals.GPIO5,
            peripherals.GPIO6,
        );
        emit_reset_preamble(&mut display).unwrap();
        emit_power_init_block(&mut display).unwrap();
        emit_addressing_init_block(&mut display).unwrap();

        // F8: Store the transport and wire all device globals before spawning tasks.
        let transport_ref = TRANSPORT_CELL.init(display);
        let launcher_buf_ref = LAUNCHER_BUF_CELL.init([0xFF; FRAME_BYTES]);
        let app_region_ref = APP_REGION_CELL.init([0u8; APP_REGION_BYTES]);
        let pdb_buf_ref = PDB_BUF_CELL.init([0u8; PDB_BUF_BYTES]);
        let fs_ref = match fs {
            Ok(fs) => FS_CELL.init(fs) as *mut _,
            Err(failure) => {
                unsafe { BOOT_FAILURE = failure; }
                core::ptr::null_mut()
            }
        };
        // SAFETY: written once here before the executor starts; single-core.
        unsafe {
            TRANSPORT_PTR = transport_ref as *mut _;
            FS_PTR = fs_ref;
            LAUNCHER_BUF_PTR = launcher_buf_ref as *mut _;
            APP_REGION_PTR = app_region_ref as *mut _;
            PDB_BUF_PTR = pdb_buf_ref as *mut _;
        }

        if BUTTON_DIAGNOSTIC_MODE {
            render_diag_boot_marker(launcher_buf_ref);
            let _ = display_refresh_flush(transport_ref, launcher_buf_ref);
        } else {
            render_launcher_boot_marker(launcher_buf_ref);
            let _ = display_refresh_flush(transport_ref, launcher_buf_ref);
        }

        let input = X4AdcSource::new(adc1, gpio1, gpio2);
        let input_ref = INPUT_CELL.init(InputPoller::new(input, 2, 100));

        // SAFETY: each setter is called exactly once at init; no concurrent access.
        unsafe {
            set_display_flush_fn(device_display_flush);
            set_input_get_buttons_fn(device_get_buttons);
            set_input_wait_button_fn(device_wait_button);
            set_fs_opendir_fn(device_opendir);
            set_fs_readdir_fn(device_readdir);
            set_fs_closedir_fn(device_closedir);
            INPUT_PTR = input_ref as *mut _;
        }

        let timers = TimerGroup::new(peripherals.TIMG0);
        let software_interrupts = SoftwareInterruptControl::new(peripherals.SW_INTERRUPT);

        esp_rtos::start(timers.timer0, software_interrupts.software_interrupt0);

        let executor = EXECUTOR.init(esp_rtos::embassy::Executor::new());
        executor.run(|spawner| {
            spawner.must_spawn(launcher_task(spawner));
        })
    }
}

#[cfg(not(all(target_arch = "riscv32", target_os = "none")))]
fn main() {}
