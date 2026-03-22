use core::cell::Cell;

use kernel::platform::{HostPlatform, KernelSupport, LogLevel, StorageError, StorageReader};

struct DummyStorage;

impl StorageReader for DummyStorage {
    fn read(&self, path: &str, buffer: &mut [u8]) -> Result<usize, StorageError> {
        if path != "/apps/test.pdb" {
            return Err(StorageError::NotFound);
        }

        let payload = b"pd";
        buffer[..payload.len()].copy_from_slice(payload);
        Ok(payload.len())
    }
}

fn read_from_storage(storage: &dyn StorageReader) -> Result<[u8; 2], StorageError> {
    let mut buffer = [0u8; 2];
    storage.read("/apps/test.pdb", &mut buffer)?;
    Ok(buffer)
}

struct DummySupport {
    slept_ms: Cell<u32>,
    watchdog_fed: Cell<bool>,
    logged: Cell<bool>,
}

impl DummySupport {
    fn new() -> Self {
        Self {
            slept_ms: Cell::new(0),
            watchdog_fed: Cell::new(false),
            logged: Cell::new(false),
        }
    }
}

impl KernelSupport for DummySupport {
    fn millis(&self) -> u32 {
        42
    }

    fn sleep_ms(&self, ms: u32) {
        self.slept_ms.set(ms);
    }

    fn feed_watchdog(&self) {
        self.watchdog_fed.set(true);
    }

    fn log(&self, level: LogLevel, message: &str) {
        if level == LogLevel::Info && message == "boot" {
            self.logged.set(true);
        }
    }
}

#[test]
fn platform_storage_reader_platform() {
    assert_eq!(read_from_storage(&DummyStorage), Ok(*b"pd"));

    let mut buffer = [0u8; 2];
    assert_eq!(
        DummyStorage.read("/missing.pdb", &mut buffer),
        Err(StorageError::NotFound)
    );
}

#[test]
fn platform_support_trait_platform() {
    let support = DummySupport::new();

    assert_eq!(support.millis(), 42);
    support.sleep_ms(25);
    support.feed_watchdog();
    support.log(LogLevel::Info, "boot");

    assert_eq!(support.slept_ms.get(), 25);
    assert!(support.watchdog_fed.get());
    assert!(support.logged.get());
}

#[test]
fn platform_host_fakes_platform() {
    let platform = HostPlatform::new("/apps/demo.pdb", b"demo", 99);

    let mut buffer = [0u8; 4];
    assert_eq!(platform.storage.read("/apps/demo.pdb", &mut buffer), Ok(4));
    assert_eq!(buffer, *b"demo");

    assert_eq!(platform.support.millis(), 99);
    platform.support.sleep_ms(12);
    platform.support.feed_watchdog();
    platform.support.log(LogLevel::Info, "hello");

    assert_eq!(platform.support.last_sleep_ms(), 12);
    assert!(platform.support.watchdog_fed());
    assert!(platform.support.logged());
}
