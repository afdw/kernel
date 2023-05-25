#[derive(Debug)]
pub struct Display {
    resolution: (usize, usize),
    framebuffer_ptr: *mut u32,
}

impl Display {
    pub fn new() -> Option<Self> {
        let boot_services = unsafe { super::SYSTEM_TABLE.as_mut().unwrap() }.boot_services();
        let gop_handle = boot_services.get_handle_for_protocol::<uefi::proto::console::gop::GraphicsOutput>().ok()?;
        let mut gop = boot_services
            .open_protocol_exclusive::<uefi::proto::console::gop::GraphicsOutput>(gop_handle)
            .ok()?;
        let mode_info = gop.current_mode_info();
        if mode_info.pixel_format() != uefi::proto::console::gop::PixelFormat::Bgr {
            return None;
        }
        Some(Display {
            resolution: mode_info.resolution(),
            framebuffer_ptr: gop.frame_buffer().as_mut_ptr() as _,
        })
    }
}

impl super::display::Display for Display {
    fn reinitialize_if_needed(&self) {}

    fn resolution(&self) -> (usize, usize) {
        self.resolution
    }

    fn update(&self, pixel_data: &[u32]) {
        let framebuffer: &mut [u32] = unsafe { core::slice::from_raw_parts_mut(self.framebuffer_ptr, self.resolution.0 * self.resolution.1) };
        let min_len = core::cmp::min(framebuffer.len(), pixel_data.len());
        framebuffer[..min_len].copy_from_slice(&pixel_data[..min_len]);
    }
}
