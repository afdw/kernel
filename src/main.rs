#![feature(format_args_nl)]
#![no_std]
#![no_main]

extern crate alloc;

mod allocator;
mod logger;

static mut SYSTEM_TABLE: Option<uefi::table::SystemTable<uefi::table::Boot>> = None;

#[uefi::entry]
fn main(_image_handle: uefi::Handle, system_table: uefi::table::SystemTable<uefi::table::Boot>) -> uefi::Status {
    unsafe {
        SYSTEM_TABLE = Some(system_table.unsafe_clone());
    }
    logger::init();
    log::info!("Hello world!");
    let v = alloc::vec![1, 2, 3];
    log::debug!("{:p}", &v);
    logger::dbg!(v.iter().sum::<i32>());
    system_table.boot_services().stall(10_000_000);
    uefi::Status::SUCCESS
}
