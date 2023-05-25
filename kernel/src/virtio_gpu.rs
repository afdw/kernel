use core::{
    cell::{Cell, RefCell},
    fmt::Debug,
};
use virtio_drivers::transport::Transport;

pub struct Display {
    pci_root: virtio_drivers::transport::pci::bus::PciRoot,
    device_function: virtio_drivers::transport::pci::bus::DeviceFunction,
    device: RefCell<Option<virtio_drivers::device::gpu::VirtIOGpu<'static, super::virtio::Hal, virtio_drivers::transport::pci::PciTransport>>>,
    last_resolution: Cell<(u32, u32)>,
    framebuffer: Cell<core::ptr::NonNull<[u8]>>,
}

impl Debug for Display {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("Display").field("device_function", &self.device_function).finish()
    }
}

impl Display {
    pub fn new(
        pci_root: &mut virtio_drivers::transport::pci::bus::PciRoot,
        device_function: virtio_drivers::transport::pci::bus::DeviceFunction,
        device_function_info: virtio_drivers::transport::pci::bus::DeviceFunctionInfo,
    ) -> Option<Self> {
        let virtio_type = virtio_drivers::transport::pci::virtio_device_type(&device_function_info)?;
        if virtio_type == virtio_drivers::transport::DeviceType::GPU {
            pci_root.set_command(
                device_function,
                virtio_drivers::transport::pci::bus::Command::IO_SPACE
                    | virtio_drivers::transport::pci::bus::Command::MEMORY_SPACE
                    | virtio_drivers::transport::pci::bus::Command::BUS_MASTER,
            );
            let mut transport = virtio_drivers::transport::pci::PciTransport::new::<super::virtio::Hal>(pci_root, device_function).ok()?;
            transport.set_status(virtio_drivers::transport::DeviceStatus::empty());
            let mut device = virtio_drivers::device::gpu::VirtIOGpu::<super::virtio::Hal, _>::new(transport).ok()?;
            let last_resolution = device.resolution().ok()?;
            let framebuffer = device.setup_framebuffer().ok()?.into();
            Some(Display {
                pci_root: unsafe { core::ptr::read(pci_root) },
                device_function,
                device: RefCell::new(Some(device)),
                last_resolution: Cell::new(last_resolution),
                framebuffer: Cell::new(framebuffer),
            })
        } else {
            None
        }
    }
}

impl super::display::Display for Display {
    fn reinitialize_if_needed(&self) {
        let mut device = self.device.take().unwrap();
        let resolution = device.resolution().unwrap();
        if resolution == self.last_resolution.get() {
            self.device.replace(Some(device));
        } else {
            log::debug!("Resolution change: {:?} -> {:?}", self.last_resolution.get(), resolution);
            drop(device);
            let mut transport =
                virtio_drivers::transport::pci::PciTransport::new::<super::virtio::Hal>(&mut unsafe { core::ptr::read(&self.pci_root) }, self.device_function)
                    .unwrap();
            transport.set_status(virtio_drivers::transport::DeviceStatus::empty());
            let mut device = virtio_drivers::device::gpu::VirtIOGpu::<super::virtio::Hal, _>::new(transport).unwrap();
            let last_resolution = device.resolution().unwrap();
            let framebuffer = device.setup_framebuffer().unwrap().into();
            self.device.replace(Some(device));
            self.last_resolution.set(last_resolution);
            self.framebuffer.set(framebuffer);
        }
    }

    fn resolution(&self) -> (usize, usize) {
        (self.last_resolution.get().0 as usize, self.last_resolution.get().1 as usize)
    }

    fn update(&self, pixel_data: &[u32]) {
        let framebuffer: &mut [u32] = bytemuck::cast_slice_mut(unsafe { &mut *self.framebuffer.get().as_ptr() });
        let min_len = core::cmp::min(framebuffer.len(), pixel_data.len());
        framebuffer[..min_len].copy_from_slice(&pixel_data[..min_len]);
        self.device.borrow_mut().as_mut().unwrap().flush().unwrap();
    }
}
