use kernel::boot::{boot, AbiMetadata, BootState};
use kernel::platform::HostPlatform;

#[test]
fn boot_smoke() {
    let platform = HostPlatform::new("/apps/demo.pdb", b"demo", 99);

    assert_eq!(
        boot(&platform.storage, &platform.support, "/apps/demo.pdb"),
        Ok(BootState {
            boot_millis: 99,
            storage_probe_len: 4,
            abi: AbiMetadata {
                abi_version: 1,
                kernel_version: 1,
            },
        })
    );

    assert!(platform.support.watchdog_fed());
    assert!(platform.support.logged());
}
