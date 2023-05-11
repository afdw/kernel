use core::{
    alloc::{GlobalAlloc, Layout},
    cmp::max,
};

pub struct Allocator;

unsafe impl GlobalAlloc for Allocator {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        // `allocate_pool` is only guaranteed to provide 8-byte alignment, so allocate extra space to align and to put the original pointer.
        let align = max(layout.align(), core::mem::size_of::<*mut u8>());
        match super::SYSTEM_TABLE
            .as_ref()
            .unwrap()
            .boot_services()
            .allocate_pool(uefi::table::boot::MemoryType::LOADER_DATA, align * 2 + layout.size())
        {
            Ok(ptr) => {
                let aligned_ptr = ptr.add(ptr.align_offset(align)).add(align);
                (aligned_ptr as *mut *mut u8).sub(1).write(ptr);
                aligned_ptr
            }
            _ => core::ptr::null_mut(),
        }
    }

    unsafe fn dealloc(&self, aligned_ptr: *mut u8, _layout: Layout) {
        let ptr = (aligned_ptr as *mut *mut u8).sub(1).read();
        super::SYSTEM_TABLE.as_ref().unwrap().boot_services().free_pool(ptr).unwrap();
    }
}

#[global_allocator]
static ALLOCATOR: Allocator = Allocator;
