use wgpu::COPY_BYTES_PER_ROW_ALIGNMENT;

pub const PIXEL_STRIDE: usize = 4;

pub fn pad_to_bytes_per_row_alignment(a: usize) -> usize {
    (a * PIXEL_STRIDE / COPY_BYTES_PER_ROW_ALIGNMENT as usize + 1)
        * COPY_BYTES_PER_ROW_ALIGNMENT as usize
}
