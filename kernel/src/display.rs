pub trait Display {
    fn reinitialize_if_needed(&self);

    fn resolution(&self) -> (usize, usize);

    fn update(&self, pixel_data: &[u32]);
}

impl<D: Display> Display for &D {
    fn reinitialize_if_needed(&self) {
        (*self).reinitialize_if_needed();
    }

    fn resolution(&self) -> (usize, usize) {
        (*self).resolution()
    }

    fn update(&self, pixel_data: &[u32]) {
        (*self).update(pixel_data)
    }
}
