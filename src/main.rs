#![feature(format_args_nl)]
#![no_std]
#![no_main]

extern crate alloc;

mod allocator;
mod disk;
mod logger;

static mut SYSTEM_TABLE: Option<uefi::table::SystemTable<uefi::table::Boot>> = None;

#[uefi::entry]
fn main(_image_handle: uefi::Handle, system_table: uefi::table::SystemTable<uefi::table::Boot>) -> uefi::Status {
    unsafe {
        SYSTEM_TABLE = Some(system_table.unsafe_clone());
    }
    logger::init();
    log::info!("Hello world!");
    for device in [
        disk::Device::PrimaryMaster,
        disk::Device::PrimarySlave,
        disk::Device::SecondaryMaster,
        disk::Device::SecondarySlave,
    ] {
        match disk::identify(device) {
            None => log::debug!("Disk {:?}: none", device),
            Some(sector_count) => {
                log::debug!("Disk {:?}: {} sectors", device, sector_count);
                log::debug!("{}", pretty_hex::pretty_hex(&&disk::read_sector(device, 1)[..128]));
                let mut sector_data = disk::read_sector(device, 0);
                sector_data[0] = 0x12;
                sector_data[1] = 0x34;
                sector_data[2] = 0x56;
                disk::write_sector(device, 0, sector_data);
            }
        }
    }
    system_table.boot_services().stall(100_000_000);
    uefi::Status::SUCCESS
}
