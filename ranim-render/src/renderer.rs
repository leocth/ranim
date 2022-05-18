use std::num::NonZeroU32;

use color_eyre::{eyre::eyre, Result};
use cstr::cstr;
use wgpu::{util::DeviceExt, COPY_BYTES_PER_ROW_ALIGNMENT};
use winit::{dpi::PhysicalSize, window::Window};

use crate::{buf::Vertex, video::Encoder};

pub enum RenderMode<'a> {
    Preview(&'a Window),
    Output { size: PhysicalSize<u32> },
}
enum RenderTarget {
    Window {
        surface: wgpu::Surface,
        config: wgpu::SurfaceConfiguration,
    },
    Image {
        target: TextureTarget,
    },
    Video {
        target: TextureTarget,
        encoder: Encoder,
    },
}

struct TextureTarget {
    texture: wgpu::Texture,
    extent: wgpu::Extent3d,
    view: wgpu::TextureView,
    output_buffer: wgpu::Buffer,
    image_buffer: Vec<u8>,

    bytes_per_row: u32,
    rows_per_image: u32,
}
impl TextureTarget {
    fn new(device: &wgpu::Device, size: PhysicalSize<u32>) -> Self {
        let extent = wgpu::Extent3d {
            width: size.width,
            height: size.height,
            depth_or_array_layers: 1,
        };
        let texture_desc = wgpu::TextureDescriptor {
            size: extent,
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Rgba8UnormSrgb,
            usage: wgpu::TextureUsages::COPY_SRC | wgpu::TextureUsages::RENDER_ATTACHMENT,
            label: None,
        };
        let texture = device.create_texture(&texture_desc);
        let view = texture.create_view(&Default::default());

        let out_width =
            (size.width * 4 / COPY_BYTES_PER_ROW_ALIGNMENT + 1) * COPY_BYTES_PER_ROW_ALIGNMENT;

        let output_buffer_size = (4 * out_width * size.height) as wgpu::BufferAddress;
        let output_buffer_desc = wgpu::BufferDescriptor {
            size: output_buffer_size,
            usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
            label: None,
            mapped_at_creation: false,
        };
        let output_buffer = device.create_buffer(&output_buffer_desc);

        let image_buffer = vec![0; (4 * size.width * size.height) as usize];

        Self {
            texture,
            extent,
            view,
            output_buffer,

            bytes_per_row: out_width,
            rows_per_image: size.height,
            image_buffer,
        }
    }
    async fn update_image_buffer(&mut self, device: &wgpu::Device) {
        let buffer_slice = self.output_buffer.slice(..);

        // NOTE: We have to create the mapping THEN device.poll() before await
        // the future. Otherwise the application will freeze.
        let mapping = buffer_slice.map_async(wgpu::MapMode::Read);
        device.poll(wgpu::Maintain::Wait);
        mapping.await.unwrap();

        let data = buffer_slice.get_mapped_range();

        let width = (self.extent.width * 4) as usize;

        for (dst, src) in self
            .image_buffer
            .chunks_mut(width)
            .zip(data.chunks(self.bytes_per_row as usize))
        {
            dst.copy_from_slice(&src[..width])
        }
    }
}

pub struct Renderer {
    device: wgpu::Device,
    queue: wgpu::Queue,
    pub size: PhysicalSize<u32>,
    render_pipeline: wgpu::RenderPipeline,

    render_target: RenderTarget,

    vertex_buffer: wgpu::Buffer,
    index_buffer: wgpu::Buffer,
    num_indices: u32,
}

impl Renderer {
    pub async fn new(render_mode: RenderMode<'_>) -> Result<Self> {
        // The instance is a handle to our GPU
        // Backends::all => Vulkan + Metal + DX12 + Browser WebGPU
        let instance = wgpu::Instance::new(wgpu::Backends::all());

        let (size, surface) = match render_mode {
            RenderMode::Preview(window) => {
                let size = window.inner_size();
                let surface = unsafe { instance.create_surface(window) };

                (size, Some(surface))
            }
            RenderMode::Output { size } => (size, None),
        };
        let adapter = instance
            .request_adapter(&wgpu::RequestAdapterOptions {
                power_preference: wgpu::PowerPreference::default(),
                compatible_surface: surface.as_ref(),
                force_fallback_adapter: false,
            })
            .await
            .ok_or_else(|| eyre!("No suitable adapter found"))?;

        let (device, queue) = adapter
            .request_device(&wgpu::DeviceDescriptor::default(), None)
            .await?;

        let (render_target, format) = match render_mode {
            RenderMode::Preview(_) => {
                let surface = surface.unwrap();
                let format = surface.get_preferred_format(&adapter).unwrap();
                let config = wgpu::SurfaceConfiguration {
                    usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
                    format,
                    width: size.width,
                    height: size.height,
                    present_mode: wgpu::PresentMode::Fifo,
                };
                surface.configure(&device, &config);
                (RenderTarget::Window { surface, config }, format)
            }
            // TODO: implement Image mode
            RenderMode::Output { size } => {
                let output_video_path = cstr!("out.mp4"); // XXX
                (
                    RenderTarget::Video {
                        target: TextureTarget::new(&device, size),
                        encoder: Encoder::new(size, output_video_path)?,
                    },
                    // RenderTarget::Image {
                    //     target: TextureTarget::new(&device, size),
                    // },
                    wgpu::TextureFormat::Rgba8UnormSrgb,
                )
            }
        };

        let shader = device.create_shader_module(&wgpu::include_wgsl!("shader.wgsl"));
        let render_pipeline_layout =
            device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: Some("Render Pipeline Layout"),
                bind_group_layouts: &[],
                push_constant_ranges: &[],
            });
        let render_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("Render Pipeline"),
            layout: Some(&render_pipeline_layout),
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: "vertex",
                buffers: &[Vertex::desc()],
            },
            fragment: Some(wgpu::FragmentState {
                module: &shader,
                entry_point: "fragment",
                targets: &[wgpu::ColorTargetState {
                    format,
                    blend: Some(wgpu::BlendState::REPLACE),
                    write_mask: wgpu::ColorWrites::ALL,
                }],
            }),
            primitive: wgpu::PrimitiveState {
                topology: wgpu::PrimitiveTopology::TriangleList,
                strip_index_format: None,
                front_face: wgpu::FrontFace::Ccw,
                cull_mode: Some(wgpu::Face::Back),
                polygon_mode: wgpu::PolygonMode::Fill,
                unclipped_depth: false,
                conservative: false,
            },
            depth_stencil: None,
            multisample: wgpu::MultisampleState {
                count: 1,
                mask: !0,
                alpha_to_coverage_enabled: false,
            },
            multiview: None,
        });

        let vertex_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Vertex Buffer"),
            contents: bytemuck::cast_slice(crate::buf::VERTICES), // XXX
            usage: wgpu::BufferUsages::VERTEX,
        });

        let index_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Index Buffer"),
            contents: bytemuck::cast_slice(crate::buf::INDICES), // XXX
            usage: wgpu::BufferUsages::INDEX,
        });

        Ok(Self {
            device,
            queue,
            size,
            render_pipeline,
            render_target,
            vertex_buffer,
            index_buffer,
            num_indices: crate::buf::INDICES.len() as u32,
        })
    }

    pub fn resize(&mut self, new_size: PhysicalSize<u32>) {
        if new_size.width > 0 && new_size.height > 0 {
            self.size = new_size;
            if let RenderTarget::Window { surface, config } = &mut self.render_target {
                config.width = new_size.width;
                config.height = new_size.height;
                surface.configure(&self.device, config);
            }
        }
    }

    pub fn update(&mut self) {}

    pub async fn render(&mut self) -> Result<()> {
        let mut cmdenc = self
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("Render Encoder"),
            });

        let mut render_pass = |view| {
            let mut render_pass = cmdenc.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("Render Pass"),
                color_attachments: &[wgpu::RenderPassColorAttachment {
                    view,
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

            render_pass.set_pipeline(&self.render_pipeline);
            render_pass.set_vertex_buffer(0, self.vertex_buffer.slice(..));
            render_pass.set_index_buffer(self.index_buffer.slice(..), wgpu::IndexFormat::Uint16);
            render_pass.draw_indexed(0..self.num_indices, 0, 0..1);
        };

        async fn texture_target_common(
            mut encoder: wgpu::CommandEncoder,
            target: &mut TextureTarget,
            queue: &wgpu::Queue,
            device: &wgpu::Device,
        ) {
            encoder.copy_texture_to_buffer(
                wgpu::ImageCopyTexture {
                    aspect: wgpu::TextureAspect::All,
                    texture: &target.texture,
                    mip_level: 0,
                    origin: wgpu::Origin3d::ZERO,
                },
                wgpu::ImageCopyBuffer {
                    buffer: &target.output_buffer,
                    layout: wgpu::ImageDataLayout {
                        offset: 0,
                        bytes_per_row: NonZeroU32::new(target.bytes_per_row),
                        rows_per_image: NonZeroU32::new(target.rows_per_image),
                    },
                },
                target.extent,
            );
            queue.submit(std::iter::once(encoder.finish()));
        
            target.update_image_buffer(device).await;
        }


        match &mut self.render_target {
            RenderTarget::Window { surface, .. } => {
                let output = surface.get_current_texture()?;
                let view = output
                    .texture
                    .create_view(&wgpu::TextureViewDescriptor::default());
                render_pass(&view);
                self.queue.submit(std::iter::once(cmdenc.finish()));
                output.present();
            }
            RenderTarget::Image { target } => {
                render_pass(&target.view);
                texture_target_common(cmdenc, target, &self.queue, &self.device).await;

                use image::{ImageBuffer, Rgba};
                let buffer = ImageBuffer::<Rgba<u8>, _>::from_raw(
                    target.extent.width,
                    target.extent.height,
                    target.image_buffer.as_slice(),
                )
                .unwrap();
                buffer.save("image.png").unwrap();

                target.output_buffer.unmap();
            }
            RenderTarget::Video { target, encoder } => {
                render_pass(&target.view);
                texture_target_common(cmdenc, target, &self.queue, &self.device).await;
                encoder.write_frame(&target.image_buffer);
                encoder.encode()?;
                target.output_buffer.unmap();
            }
        }

        Ok(())
    }

    pub fn finish(&mut self) -> Result<()> {
        if let RenderTarget::Video { encoder, .. } = &mut self.render_target {
            encoder.end()?;
        }
        Ok(())
    }
}


