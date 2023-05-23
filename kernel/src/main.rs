#![feature(array_chunks)]
#![feature(format_args_nl)]
#![feature(let_chains)]
#![feature(never_type)]
#![feature(int_roundings)]
#![no_std]
#![no_main]

extern crate alloc;

mod allocator;
mod backtrace;
mod disk;
mod ext2;
mod fs;
mod guid;
mod logger;
mod panic;
mod partitions;
mod sector_storage;

use alloc::{string::String, vec::Vec};

use fs::Session;
use sector_storage::SectorStorage;

include!("../../bootloader/src/common.rs");

static mut SYSTEM_TABLE: Option<uefi::table::SystemTable<uefi::table::Boot>> = None;
static BOOTLOADER_PROTOCOL: spin::Once<BootloaderProtocol> = spin::Once::new();

#[uefi::entry]
fn main(image_handle: uefi::Handle, system_table: uefi::table::SystemTable<uefi::table::Boot>) -> uefi::Status {
    unsafe {
        SYSTEM_TABLE = Some(system_table.unsafe_clone());
    }
    logger::init();
    log::info!("Hello world!");
    BOOTLOADER_PROTOCOL.call_once(|| {
        *system_table
            .boot_services()
            .open_protocol_exclusive::<BootloaderProtocol>(image_handle)
            .unwrap()
    });
    backtrace::init();
    let mut disk_sector_storages_partitions = Vec::new();
    for device in [
        disk::Device::PrimaryMaster,
        disk::Device::PrimarySlave,
        disk::Device::SecondaryMaster,
        disk::Device::SecondarySlave,
    ] {
        match disk::DiskSectorStorage::new(device) {
            None => log::debug!("Disk {:?}: none", device),
            Some(disk_device_storage) => {
                log::debug!("Disk {:?}: {} sectors", device, disk_device_storage.sector_count());
                log::debug!("Second sector: {}", pretty_hex::pretty_hex(&&disk_device_storage.read_sector(1)[..128]));
                let partition_table = partitions::read_partition_table(&disk_device_storage);
                log::debug!("Partition table: {:?}", partition_table);
                if let Some(partition_table) = partition_table {
                    for partition in partition_table.partitions {
                        disk_sector_storages_partitions.push((disk_device_storage, partition));
                    }
                }
            }
        }
    }
    let root_disk_sector_storage_partition = disk_sector_storages_partitions
        .into_iter()
        .find(|(_, partition)| partition.type_id == guid::TYPE_ID_LINUX && partition.name.as_deref() == Some("kernel_root"))
        .expect("no root partition found");
    log::debug!("Root disk sector storage and partition: {:?}", root_disk_sector_storage_partition);
    let session = ext2::Session::new(&root_disk_sector_storage_partition);
    logger::dbg!(session.read_dir(2));
    logger::dbg!(String::from_utf8_lossy(&session.read_regular_file_range(12, 0..5)));
    system_table.boot_services().stall(100_000_000);
    uefi::Status::SUCCESS
}
