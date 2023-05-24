use core::{alloc::Allocator, cell::RefCell, fmt::Debug};
use virtio_drivers::transport::Transport;

use super::{sector_storage::SECTOR_SIZE, SectorStorage};

struct Hal;

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

pub struct DiskSectorStorage {
    device_function: virtio_drivers::transport::pci::bus::DeviceFunction,
    device: RefCell<virtio_drivers::device::blk::VirtIOBlk<Hal, virtio_drivers::transport::pci::PciTransport>>,
}

impl Debug for DiskSectorStorage {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("DiskSectorStorage").field("device_function", &self.device_function).finish()
    }
}

impl DiskSectorStorage {
    pub fn new(
        pci_root: &mut virtio_drivers::transport::pci::bus::PciRoot,
        device_function: virtio_drivers::transport::pci::bus::DeviceFunction,
        device_function_info: virtio_drivers::transport::pci::bus::DeviceFunctionInfo,
    ) -> Option<Self> {
        let virtio_type = virtio_drivers::transport::pci::virtio_device_type(&device_function_info)?;
        if virtio_type == virtio_drivers::transport::DeviceType::Block {
            pci_root.set_command(
                device_function,
                virtio_drivers::transport::pci::bus::Command::IO_SPACE
                    | virtio_drivers::transport::pci::bus::Command::MEMORY_SPACE
                    | virtio_drivers::transport::pci::bus::Command::BUS_MASTER,
            );
            let mut transport = virtio_drivers::transport::pci::PciTransport::new::<Hal>(pci_root, device_function).ok()?;
            transport.set_status(virtio_drivers::transport::DeviceStatus::empty());
            let device = virtio_drivers::device::blk::VirtIOBlk::<Hal, _>::new(transport).ok()?;
            Some(DiskSectorStorage {
                device_function,
                device: RefCell::new(device),
            })
        } else {
            None
        }
    }
}

impl SectorStorage for DiskSectorStorage {
    fn sector_count(&self) -> u64 {
        self.device.borrow().capacity()
    }

    fn read_sector(&self, sector_index: u64) -> [u8; SECTOR_SIZE as usize] {
        assert!(sector_index < self.device.borrow().capacity());
        let mut sector_data = [0; SECTOR_SIZE as usize];
        self.device.borrow_mut().read_block(sector_index as usize, &mut sector_data).unwrap();
        sector_data
    }

    fn write_sector(&self, sector_index: u64, sector_data: [u8; SECTOR_SIZE as usize]) {
        assert!(sector_index < self.device.borrow().capacity());
        self.device.borrow_mut().write_block(sector_index as usize, &sector_data).unwrap();
    }
}
