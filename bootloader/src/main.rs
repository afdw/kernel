#![no_main]
#![no_std]
#![feature(allocator_api)]

extern crate alloc;

use alloc::boxed::Box;
use core::alloc::Allocator;

include!("../../embed/src/common.rs");

static KERNEL_FILE_DATA: [u8; KERNEL_FILE_DATA_MAX_SIZE] = [KERNEL_FILE_DATA_FILLER; KERNEL_FILE_DATA_MAX_SIZE];

const PAGE_SIZE: usize = 4096;

include!("common.rs");

#[uefi::entry]
fn main(image_handle: uefi::Handle, mut system_table: uefi::table::SystemTable<uefi::table::Boot>) -> uefi::Status {
    uefi_services::init(&mut system_table).unwrap();
    uefi_services::println!("Bootloader start");
    let kernel_file = elf::ElfBytes::<elf::endian::AnyEndian>::minimal_parse(&KERNEL_FILE_DATA).unwrap();
    let segments = kernel_file.segments().unwrap();
    let memory_image_size = segments
        .iter()
        .filter(|segment| segment.p_type == elf::abi::PT_LOAD)
        .map(|segment| segment.p_vaddr + segment.p_memsz)
        .max()
        .unwrap_or(0) as usize;
    uefi_services::println!("Memory image size: {}", memory_image_size);
    let mut memory_image = alloc::alloc::Global
        .allocate_zeroed(core::alloc::Layout::from_size_align(memory_image_size, PAGE_SIZE).unwrap())
        .unwrap();
    let memory_image_ptr = memory_image.as_ptr() as *const u8 as u64;
    let memory_image = unsafe { memory_image.as_mut() };
    for segment in segments {
        if segment.p_type == elf::abi::PT_LOAD {
            assert!(segment.p_filesz <= segment.p_memsz);
            memory_image[segment.p_vaddr as usize..(segment.p_vaddr + segment.p_filesz) as usize]
                .copy_from_slice(&KERNEL_FILE_DATA[segment.p_offset as usize..(segment.p_offset + segment.p_filesz) as usize]);
        }
    }
    let mut rela_table_offset = None;
    let mut rela_table_size = None;
    for r#dyn in kernel_file.dynamic().unwrap().unwrap() {
        match r#dyn.d_tag {
            elf::abi::DT_RELA => rela_table_offset = Some(r#dyn.d_ptr() as usize),
            elf::abi::DT_RELASZ => rela_table_size = Some(r#dyn.d_val() as usize),
            _ => (),
        }
    }
    let rela_table_data = &memory_image[rela_table_offset.unwrap()..rela_table_offset.unwrap() + rela_table_size.unwrap()].to_vec();
    for rela in elf::relocation::RelaIterator::new(kernel_file.ehdr.endianness, kernel_file.ehdr.class, rela_table_data) {
        match rela.r_type {
            elf::abi::R_X86_64_RELATIVE => memory_image[rela.r_offset as usize..rela.r_offset as usize + 8]
                .copy_from_slice(&(memory_image_ptr.wrapping_add_signed(rela.r_addend)).to_le_bytes()),
            relocation_type => panic!("unknown relocation type: {}", relocation_type),
        }
    }
    unsafe {
        system_table
            .boot_services()
            .install_protocol_interface(
                Some(image_handle),
                &BOOTLOADER_PROTOCOL_ID,
                Box::leak(Box::new(BootloaderProtocol {
                    kernel_file_data: &KERNEL_FILE_DATA,
                    memory_image_start: memory_image.as_mut_ptr(),
                })) as *mut _ as *mut _,
            )
            .unwrap();
    }
    let entry: extern "efiapi" fn(uefi::Handle, uefi::table::SystemTable<uefi::table::Boot>) -> uefi::Status =
        unsafe { core::mem::transmute(&memory_image[kernel_file.ehdr.e_entry as usize] as *const u8) };
    uefi_services::println!("Running entry: {:?}", entry);
    let status = entry(image_handle, unsafe { system_table.unsafe_clone() });
    uefi_services::println!("Kernel exited with status {:?}", status);
    system_table.boot_services().stall(10_000_000);
    status
}
