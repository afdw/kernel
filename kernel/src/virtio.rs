use core::alloc::Allocator;

pub struct Hal;

unsafe impl virtio_drivers::Hal for Hal {
    fn dma_alloc(pages: usize, _direction: virtio_drivers::BufferDirection) -> (virtio_drivers::PhysAddr, core::ptr::NonNull<u8>) {
        let buffer = alloc::alloc::Global
            .allocate_zeroed(core::alloc::Layout::from_size_align(pages * virtio_drivers::PAGE_SIZE, virtio_drivers::PAGE_SIZE).unwrap())
            .unwrap()
            .as_ptr() as *mut u8;
        (buffer as usize, core::ptr::NonNull::new(buffer).unwrap())
    }

    unsafe fn dma_dealloc(_paddr: virtio_drivers::PhysAddr, vaddr: core::ptr::NonNull<u8>, pages: usize) -> i32 {
        alloc::alloc::Global.deallocate(
            vaddr,
            core::alloc::Layout::from_size_align(pages * virtio_drivers::PAGE_SIZE, virtio_drivers::PAGE_SIZE).unwrap(),
        );
        0
    }

    unsafe fn mmio_phys_to_virt(paddr: virtio_drivers::PhysAddr, _size: usize) -> core::ptr::NonNull<u8> {
        core::ptr::NonNull::new(paddr as *mut u8).unwrap()
    }

    unsafe fn share(buffer: core::ptr::NonNull<[u8]>, _direction: virtio_drivers::BufferDirection) -> virtio_drivers::PhysAddr {
        buffer.as_ptr() as *mut u8 as usize
    }

    unsafe fn unshare(_paddr: virtio_drivers::PhysAddr, _buffer: core::ptr::NonNull<[u8]>, _direction: virtio_drivers::BufferDirection) {}
}
