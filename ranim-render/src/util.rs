use wgpu::COPY_BYTES_PER_ROW_ALIGNMENT;
use winit::dpi::PhysicalSize;

pub const PIXEL_STRIDE: usize = 4;

#[derive(Clone, Copy, Debug)]
pub struct Size {
    pub width: u32,
    pub height: u32,
    pub bytes_per_row: u32,
}
impl Size {
    pub fn new(width: u32, height: u32) -> Self {
        // bytes per row must be aligned to COPY_BYTES_PER_ROW_ALIGNMENT
        let bytes_per_row = crate::util::pad_to_bytes_per_row_alignment(width as usize) as u32;
        Self {
            width,
            height,
            bytes_per_row,
        }
    }
    pub fn buffer_size(self) -> usize {
        (self.bytes_per_row * self.height) as usize
    }
    pub fn extent(self) -> wgpu::Extent3d {
        wgpu::Extent3d {
            width: self.width,
            height: self.height,
            depth_or_array_layers: 1,
        }
    }
}
impl From<PhysicalSize<u32>> for Size {
    fn from(size: PhysicalSize<u32>) -> Self {
        Self::new(size.width, size.height)
    }
}

pub fn pad_to_bytes_per_row_alignment(a: usize) -> usize {
    (a * PIXEL_STRIDE / COPY_BYTES_PER_ROW_ALIGNMENT as usize + 1)
        * COPY_BYTES_PER_ROW_ALIGNMENT as usize
}
