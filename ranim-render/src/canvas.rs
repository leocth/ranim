use std::num::NonZeroU32;

use wgpu::COPY_BYTES_PER_ROW_ALIGNMENT;
use winit::dpi::PhysicalSize;

use crate::{data::RenderData, output::PIXEL_STRIDE};

pub struct Canvas {
    pub view: wgpu::TextureView,
    pub encoder: wgpu::CommandEncoder,
}
impl Canvas {
    pub fn render_pass(&mut self, pipeline: &wgpu::RenderPipeline, data: &RenderData) {
        let mut render_pass = self.encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("Render Pass"),
            color_attachments: &[wgpu::RenderPassColorAttachment {
                view: &self.view,
                resolve_target: None,
                ops: wgpu::Operations {
                    load: wgpu::LoadOp::Clear(wgpu::Color {
                        r: 0.1,
                        g: 0.2,
                        b: 0.3,
                        a: 1.0,
                    }),
                    store: true,
                },
            }],
            depth_stencil_attachment: None,
        });

        render_pass.set_pipeline(pipeline);
        render_pass.set_bind_group(0, &data.camera.bind_group, &[]);
        render_pass.set_vertex_buffer(0, data.vertex_buffer.slice(..));
        render_pass.set_vertex_buffer(1, data.instance_buffer.slice(..));
        render_pass.set_index_buffer(data.index_buffer.slice(..), wgpu::IndexFormat::Uint16);

        render_pass.draw_indexed(0..data.num_indices, 0, 0..data.instances.len() as u32);
    }

    pub fn copy_to_output(&mut self, src: &wgpu::Texture, dst: &CanvasBuffer) {
        self.encoder.copy_texture_to_buffer(
            wgpu::ImageCopyTexture {
                aspect: wgpu::TextureAspect::All,
                texture: src,
                mip_level: 0,
                origin: wgpu::Origin3d::ZERO,
            },
            wgpu::ImageCopyBuffer {
                buffer: &dst.buffer,
                layout: wgpu::ImageDataLayout {
                    offset: 0,
                    bytes_per_row: NonZeroU32::new(dst.size.bytes_per_row),
                    rows_per_image: NonZeroU32::new(dst.size.size.height),
                },
            },
            dst.size.extent(),
        );
    }
    pub fn finish(self, queue: &wgpu::Queue) {
        queue.submit(std::iter::once(self.encoder.finish()));
    }
}

#[derive(Clone, Copy, Debug)]
pub struct CanvasSize {
    pub size: PhysicalSize<u32>,
    pub bytes_per_row: u32,
}
impl CanvasSize {
    pub fn new(size: PhysicalSize<u32>) -> Self {
        // bytes per row must be aligned to COPY_BYTES_PER_ROW_ALIGNMENT
        let bytes_per_row = (size.width * PIXEL_STRIDE / COPY_BYTES_PER_ROW_ALIGNMENT + 1)
            * COPY_BYTES_PER_ROW_ALIGNMENT;
        Self {
            size,
            bytes_per_row,
        }
    }
    pub fn buffer_size(self) -> usize {
        (self.bytes_per_row * self.size.height) as usize
    }
    pub fn extent(self) -> wgpu::Extent3d {
        wgpu::Extent3d {
            width: self.size.width,
            height: self.size.height,
            depth_or_array_layers: 1,
        }
    }
}

pub struct CanvasBuffer {
    pub buffer: wgpu::Buffer,
    pub size: CanvasSize,
}
impl CanvasBuffer {
    pub fn new(device: &wgpu::Device, size: CanvasSize) -> Self {
        let buffer = device.create_buffer(&wgpu::BufferDescriptor {
            size: size.buffer_size() as wgpu::BufferAddress,
            usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
            label: None,
            mapped_at_creation: false,
        });
        Self { buffer, size }
    }
    pub async fn view(&self, device: &wgpu::Device) -> CanvasBufferView<'_> {
        let slice = self.buffer.slice(..);

        // NOTE: We have to create the mapping THEN device.poll() before await
        // the future. Otherwise the application will freeze.
        let mapping = slice.map_async(wgpu::MapMode::Read);
        device.poll(wgpu::Maintain::Wait);
        mapping.await.unwrap();

        CanvasBufferView {
            view: slice.get_mapped_range(),
            size: self.size,
        }
    }
    pub fn unmap(&self, view: CanvasBufferView<'_>) {
        drop(view); // kill it
        self.buffer.unmap();
    }
}
pub struct CanvasBufferView<'a> {
    pub view: wgpu::BufferView<'a>,
    pub size: CanvasSize,
}
