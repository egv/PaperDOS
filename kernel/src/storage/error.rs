/// Errors returned by storage operations.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StorageError {
    /// Card not initialised or not present.
    NotReady,
    /// Low-level SPI transfer failure.
    IoError,
    /// File or directory not found.
    NotFound,
    /// Filesystem has no space for the requested operation.
    NoSpace,
    /// Filesystem format is unrecognised or corrupt.
    InvalidFormat,
}
