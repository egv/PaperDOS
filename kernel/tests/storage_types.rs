use kernel::storage::StorageError;

#[test]
fn storage_error_variants_storage_types() {
    let _ = StorageError::NotReady;
    let _ = StorageError::IoError;
    let _ = StorageError::NotFound;
    let _ = StorageError::NoSpace;
    let _ = StorageError::InvalidFormat;
}

#[test]
fn storage_error_debug_storage_types() {
    let e = StorageError::NotReady;
    let s = format!("{:?}", e);
    assert!(!s.is_empty());
}
