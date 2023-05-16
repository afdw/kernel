#![feature(array_chunks)]
#![feature(format_args_nl)]
#![no_std]
#![no_main]

extern crate alloc;

mod allocator;
mod disk;
mod ext2;
mod guid;
mod logger;
mod partitions;
mod sector_storage;

use alloc::vec::Vec;
use core::slice;

use sector_storage::SectorStorage;

static mut SYSTEM_TABLE: Option<uefi::table::SystemTable<uefi::table::Boot>> = None;

#[uefi::entry]
fn main(_image_handle: uefi::Handle, system_table: uefi::table::SystemTable<uefi::table::Boot>) -> uefi::Status {
    unsafe {
        SYSTEM_TABLE = Some(system_table.unsafe_clone());
    }
    logger::init();
    log::info!("Hello world!");
    let (image_base, image_size) = system_table
        .boot_services()
        .open_protocol_exclusive::<uefi::proto::loaded_image::LoadedImage>(system_table.boot_services().image_handle())
        .unwrap()
        .info();
    let image = unsafe { slice::from_raw_parts(image_base as *mut u8, image_size as usize) };
    log::debug!("Image, at {:p}: {}", image, pretty_hex::pretty_hex(&&image[..128]));
    let object_file = &object::read::File::parse(image).unwrap();
    // logger::dbg!(object_file);
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
    let mounted = ext2::Mounted::new(&root_disk_sector_storage_partition);
    log::debug!("{:?}", mounted);
    mounted.write_superblock_copies();
    system_table.boot_services().stall(100_000_000);
    uefi::Status::SUCCESS
}
