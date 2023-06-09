#![feature(array_chunks)]
#![feature(format_args_nl)]
#![feature(let_chains)]
#![feature(never_type)]
#![feature(int_roundings)]
#![feature(allocator_api)]
#![no_std]
#![no_main]

extern crate alloc;

mod allocator;
mod backtrace;
mod console;
mod discovery;
mod display;
mod ext2;
mod formatting;
mod fs;
mod gop;
mod guid;
mod logger;
mod panic;
mod partitions;
mod pata;
mod sector_storage;
mod serial;
mod virtio;
mod virtio_blk;
mod virtio_gpu;

use alloc::{string::String, vec::Vec};
use core::fmt::Write;

use sector_storage::SectorStorage;

include!("../../bootloader/src/common.rs");

static mut SYSTEM_TABLE: Option<uefi::table::SystemTable<uefi::table::Boot>> = None;
static BOOTLOADER_PROTOCOL: spin::Once<BootloaderProtocol> = spin::Once::new();
static mut DISPLAY: Option<discovery::Display> = None;

fn print_tree(session: &impl fs::Session, level: usize, inode_index: u64) {
    const UNIMPORTANT_STYLE: formatting::Style = formatting::Style {
        reset: false,
        foreground_color: Some(formatting::Color::BrightBlack),
        background_color: None,
    };
    const REGULAR_FILE_STYLE: formatting::Style = formatting::Style {
        reset: false,
        foreground_color: Some(formatting::Color::BrightGreen),
        background_color: None,
    };
    const DIR_STYLE: formatting::Style = formatting::Style {
        reset: false,
        foreground_color: Some(formatting::Color::Blue),
        background_color: None,
    };
    for dir_entry in session.read_dir(inode_index) {
        if dir_entry.inode_index == 0 {
            continue;
        }
        let file_stat = session.file_stat(dir_entry.inode_index);
        for _ in 0..level {
            logger::print!("  ");
        }
        logger::println!(
            "{}{}{}: {:?} {:o} {}{}:{} {}{}",
            match dir_entry.file_type {
                Some(fs::FileType::RegularFile) => REGULAR_FILE_STYLE,
                Some(fs::FileType::Dir) => DIR_STYLE,
                _ => formatting::Style::RESET,
            },
            &dir_entry.name,
            formatting::Style::RESET,
            file_stat.mode.file_type(),
            file_stat.mode.permissions(),
            UNIMPORTANT_STYLE,
            file_stat.uid,
            file_stat.gid,
            file_stat.size,
            formatting::Style::RESET
        );
        if dir_entry.file_type == Some(fs::FileType::Dir) && dir_entry.name != "." && dir_entry.name != ".." {
            print_tree(session, level + 1, dir_entry.inode_index);
        }
        if dir_entry.file_type == Some(fs::FileType::RegularFile) {
            for _ in 0..level {
                logger::print!("  ");
            }
            let file_data = session.read_regular_file_range(dir_entry.inode_index, 0..file_stat.size);
            logger::print!("`_Contents: {}", String::from_utf8_lossy(&file_data));
        }
    }
}

fn init() {
    serial::init();
    logger::init();
    log::info!("Hello world!");
    let mut discovery_result = discovery::discover();
    let display = core::mem::take(&mut discovery_result.displays).into_iter().next().expect("no display found");
    unsafe { DISPLAY = Some(display) };
    let mut disk_sector_storages_partitions = Vec::new();
    for disk_device_storage in &discovery_result.disk_sector_storages {
        let partition_table = partitions::read_partition_table(&disk_device_storage);
        log::debug!("Partition table: {:?}", partition_table);
        if let Some(partition_table) = partition_table {
            for partition in partition_table.partitions {
                disk_sector_storages_partitions.push((disk_device_storage, partition));
            }
        }
    }
    let root_disk_sector_storage_partition = disk_sector_storages_partitions
        .into_iter()
        .find(|(_, partition)| partition.type_id == guid::TYPE_ID_LINUX && partition.name.as_deref() == Some("kernel_root"))
        .expect("no root partition found");
    log::debug!("Root disk sector storage and partition: {:?}", root_disk_sector_storage_partition);
    let session = ext2::Session::new(&root_disk_sector_storage_partition);
    logger::println!(
        "{}Root dir listing:{}",
        formatting::Style {
            reset: false,
            foreground_color: Some(formatting::Color::Magenta),
            background_color: None,
        },
        formatting::Style::RESET
    );
    print_tree(&session, 0, 2);
    logger::println!(
        "{}Root dir listing end{}",
        formatting::Style {
            reset: false,
            foreground_color: Some(formatting::Color::Magenta),
            background_color: None,
        },
        formatting::Style::RESET
    );
    loop {
        logger::update();
    }
}

#[uefi::entry]
fn main(image_handle: uefi::Handle, mut system_table: uefi::table::SystemTable<uefi::table::Boot>) -> uefi::Status {
    unsafe {
        SYSTEM_TABLE = Some(system_table.unsafe_clone());
    }
    let gop_handle = system_table
        .boot_services()
        .get_handle_for_protocol::<uefi::proto::console::gop::GraphicsOutput>()
        .unwrap();
    let gop = system_table
        .boot_services()
        .open_protocol_exclusive::<uefi::proto::console::gop::GraphicsOutput>(gop_handle)
        .unwrap();
    let x = gop.current_mode_info();
    drop(gop);
    system_table.stdout().write_fmt(format_args!("{:?}\n", x)).unwrap();
    system_table.stdout().write_fmt(format_args!("Kernel start")).unwrap();
    BOOTLOADER_PROTOCOL.call_once(|| {
        *system_table
            .boot_services()
            .open_protocol_exclusive::<BootloaderProtocol>(image_handle)
            .unwrap()
    });
    backtrace::init();
    panic::catch_unwind_with_default_handler(init);
    system_table.boot_services().stall(100_000_000);
    uefi::Status::SUCCESS
}
