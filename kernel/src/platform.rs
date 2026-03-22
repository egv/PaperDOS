use core::cell::Cell;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum StorageError {
    NotFound,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum LogLevel {
    Error,
    Warn,
    Info,
    Debug,
}

pub trait StorageReader {
    fn read(&self, path: &str, buffer: &mut [u8]) -> Result<usize, StorageError>;
}

pub trait KernelSupport {
    fn millis(&self) -> u32;
    fn sleep_ms(&self, ms: u32);
    fn feed_watchdog(&self);
    fn log(&self, level: LogLevel, message: &str);
}

pub struct HostStorage<'a> {
    path: &'a str,
    data: &'a [u8],
}

impl<'a> HostStorage<'a> {
    pub fn new(path: &'a str, data: &'a [u8]) -> Self {
        Self { path, data }
    }
}

impl StorageReader for HostStorage<'_> {
    fn read(&self, path: &str, buffer: &mut [u8]) -> Result<usize, StorageError> {
        if path != self.path || buffer.len() < self.data.len() {
            return Err(StorageError::NotFound);
        }

        buffer[..self.data.len()].copy_from_slice(self.data);
        Ok(self.data.len())
    }
}

pub struct HostSupport {
    now_ms: u32,
    last_sleep_ms: Cell<u32>,
    watchdog_fed: Cell<bool>,
    logged: Cell<bool>,
}

impl HostSupport {
    pub fn new(now_ms: u32) -> Self {
        Self {
            now_ms,
            last_sleep_ms: Cell::new(0),
            watchdog_fed: Cell::new(false),
            logged: Cell::new(false),
        }
    }

    pub fn last_sleep_ms(&self) -> u32 {
        self.last_sleep_ms.get()
    }

    pub fn watchdog_fed(&self) -> bool {
        self.watchdog_fed.get()
    }

    pub fn logged(&self) -> bool {
        self.logged.get()
    }
}

impl KernelSupport for HostSupport {
    fn millis(&self) -> u32 {
        self.now_ms
    }

    fn sleep_ms(&self, ms: u32) {
        self.last_sleep_ms.set(ms);
    }

    fn feed_watchdog(&self) {
        self.watchdog_fed.set(true);
    }

    fn log(&self, _level: LogLevel, _message: &str) {
        self.logged.set(true);
    }
}

pub struct HostPlatform<'a> {
    pub storage: HostStorage<'a>,
    pub support: HostSupport,
}

impl<'a> HostPlatform<'a> {
    pub fn new(path: &'a str, data: &'a [u8], now_ms: u32) -> Self {
        Self {
            storage: HostStorage::new(path, data),
            support: HostSupport::new(now_ms),
        }
    }
}
