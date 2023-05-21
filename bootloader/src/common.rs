const BOOTLOADER_PROTOCOL_ID: uefi::Guid = uefi::data_types::Guid::parse_or_panic("402b2f47-1a22-455c-a81b-3a847f4ace23");

#[uefi::proto::unsafe_protocol(BOOTLOADER_PROTOCOL_ID)]
#[derive(Clone, Copy)]
struct BootloaderProtocol {
    #[allow(dead_code)]
    kernel_file_data: &'static [u8],
    #[allow(dead_code)]
    memory_image_start: *mut u8,
}

unsafe impl Send for BootloaderProtocol {}
unsafe impl Sync for BootloaderProtocol {}
