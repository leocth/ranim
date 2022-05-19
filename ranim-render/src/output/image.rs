use std::path::PathBuf;

use color_eyre::Result;

use crate::output::{CanvasBufferView, CanvasSize, PIXEL_STRIDE};

use super::OutputBehavior;

pub struct ImageOutput {
    size: CanvasSize,
    image_buffer: Vec<u8>,
    output_path: PathBuf,
}
impl ImageOutput {
    pub fn new(size: CanvasSize, mut output_path: PathBuf) -> Self {
        let image_buffer = vec![0; size.buffer_size()];
        if output_path.extension().is_none() {
            output_path.set_extension("png");
        }
        Self {
            size,
            image_buffer,
            output_path,
        }
    }
}
impl OutputBehavior for ImageOutput {
    fn encode_frame<'bv>(&mut self, view: &CanvasBufferView<'bv>) -> Result<()> {
        let actual_width = (view.size.size.width * PIXEL_STRIDE) as usize;

        for (dst, src) in self
            .image_buffer
            .chunks_mut(actual_width)
            .zip(view.view.chunks(view.size.bytes_per_row as usize))
        {
            dst.copy_from_slice(&src[..actual_width])
        }
        Ok(())
    }

    fn conclude(&mut self) -> Result<()> {
        use image::{ImageBuffer, Rgba};
        let buffer = ImageBuffer::<Rgba<u8>, _>::from_raw(
            self.size.size.width,
            self.size.size.height,
            self.image_buffer.as_slice(),
        )
        .expect("Image buffer is somehow not big enough");
        buffer.save(&self.output_path)?;
        Ok(())
    }
}
