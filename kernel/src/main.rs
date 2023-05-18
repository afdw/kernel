#![feature(array_chunks)]
#![feature(format_args_nl)]
#![feature(naked_functions)]
#![feature(never_type)]
#![no_std]
#![no_main]

extern crate alloc;

mod allocator;
mod backtrace;
mod disk;
mod ext2;
mod guid;
mod logger;
mod partitions;
mod sector_storage;

use alloc::{borrow::Cow, rc::Rc, string::String, vec, vec::Vec};
use serde::{Deserialize, Serialize};

use sector_storage::SectorStorage;

include!("../../inline_debug_info/src/structures.rs");

static mut SYSTEM_TABLE: Option<uefi::table::SystemTable<uefi::table::Boot>> = None;
static mut DEBUG_INFO: Option<DebugInfo> = None;

fn read_image_data() -> Vec<u8> {
    use uefi::{proto::media::file::File, Identify};

    let system_table = unsafe { SYSTEM_TABLE.as_mut().unwrap() };
    let loaded_image_protocol = system_table
        .boot_services()
        .open_protocol_exclusive::<uefi::proto::loaded_image::LoadedImage>(system_table.boot_services().image_handle())
        .unwrap();

    let device_path_to_text_handle = *system_table
        .boot_services()
        .locate_handle_buffer(uefi::table::boot::SearchType::ByProtocol(
            &uefi::proto::device_path::text::DevicePathToText::GUID,
        ))
        .unwrap()
        .first()
        .unwrap();

    let device_path_to_text = system_table
        .boot_services()
        .open_protocol_exclusive::<uefi::proto::device_path::text::DevicePathToText>(device_path_to_text_handle)
        .unwrap();

    let image_device_path = loaded_image_protocol.file_path().unwrap();
    let image_device_path_text = device_path_to_text
        .convert_device_path_to_text(
            system_table.boot_services(),
            image_device_path,
            uefi::proto::device_path::text::DisplayOnly(true),
            uefi::proto::device_path::text::AllowShortcuts(false),
        )
        .unwrap();

    let mut file_system_protocol = system_table
        .boot_services()
        .open_protocol_exclusive::<uefi::proto::media::fs::SimpleFileSystem>(loaded_image_protocol.device())
        .unwrap();

    let file_handle: uefi::proto::media::file::FileHandle = file_system_protocol
        .open_volume()
        .unwrap()
        .open(
            uefi::CStr16::from_u16_with_nul(image_device_path_text.to_u16_slice_with_nul()).unwrap(),
            uefi::proto::media::file::FileMode::Read,
            uefi::proto::media::file::FileAttribute::empty(),
        )
        .unwrap();

    let mut regular_file = file_handle.into_regular_file().unwrap();

    let mut buffer = vec![0; 4096];
    let file_info = regular_file.get_info::<uefi::proto::media::file::FileInfo>(&mut buffer).unwrap();
    let image_size = file_info.file_size() as usize;

    let mut image_data = vec![0; image_size];
    assert_eq!(regular_file.read(&mut image_data).unwrap(), image_size);

    image_data
}

pub fn new_context<'data: 'file, 'file, O: addr2line::object::Object<'data, 'file>>(
    file: &'file O,
) -> Result<addr2line::Context<gimli::EndianRcSlice<gimli::RunTimeEndian>>, gimli::Error> {
    addr2line::Context::from_dwarf(gimli::Dwarf::load(|id| {
        use addr2line::object::ObjectSection;
        Ok(gimli::EndianRcSlice::new(
            Rc::from(
                file.section_by_name(id.name())
                    .and_then(|section| section.uncompressed_data().ok())
                    .unwrap_or(Cow::Borrowed(&[])),
            ),
            if file.is_little_endian() {
                gimli::RunTimeEndian::Little
            } else {
                gimli::RunTimeEndian::Big
            },
        ))
    })?)
}

#[uefi::entry]
fn main(_image_handle: uefi::Handle, system_table: uefi::table::SystemTable<uefi::table::Boot>) -> uefi::Status {
    unsafe {
        SYSTEM_TABLE = Some(system_table.unsafe_clone());
    }
    logger::init();
    log::info!("Hello world!");
    use object::Object;

    let image_data = read_image_data();
    let object_file = &object::read::File::parse(&image_data[..]).unwrap();

    for section in object_file.sections() {
        logger::println!("{:?}", section);
    }

    let dwarf = gimli::Dwarf::load::<_, !>(|id| {
        use addr2line::object::ObjectSection;
        Ok(gimli::EndianRcSlice::new(
            Rc::from(
                object_file
                    .section_by_name(id.name())
                    .and_then(|section| section.uncompressed_data().ok())
                    .unwrap_or(Cow::Borrowed(&[])),
            ),
            if object_file.is_little_endian() {
                gimli::RunTimeEndian::Little
            } else {
                gimli::RunTimeEndian::Big
            },
        ))
    })
    .unwrap();

    let x = new_context(object_file).unwrap();
    logger::dbg!(x.find_location(x86::bits64::registers::rip()).unwrap().unwrap().file);
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
    #[inline(never)]
    fn xxxxx() {
        logger::dbg!(backtrace::capture_backtrace(None));
    }
    #[inline(never)]
    fn yyyyy() {
        xxxxx();
    }
    yyyyy();
    system_table.boot_services().stall(100_000_000);
    uefi::Status::SUCCESS
}
