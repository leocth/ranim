use color_eyre::{eyre::eyre, Result};
use winit::{dpi::PhysicalSize, window::Window};

use crate::{
    args::Args,
    data::{RenderData, Vertex},
    output::{image::ImageOutput, Canvas, CanvasBuffer, CanvasSize, Output, OutputBehavior, video::VideoOutput},
};

pub enum RenderMode<'a> {
    Preview(&'a Window),
    Output { args: Args },
}
enum RenderTarget {
    Window {
        surface: wgpu::Surface,
        config: wgpu::SurfaceConfiguration,
    },
    Output {
        texture: wgpu::Texture,
        canvas_buf: CanvasBuffer,
        output: Output,
    },
}

pub struct Renderer {
    device: wgpu::Device,
    queue: wgpu::Queue,
    render_pipeline: wgpu::RenderPipeline,
    render_target: RenderTarget,
    data: RenderData,
}

impl Renderer {
    pub async fn new(render_mode: RenderMode<'_>) -> Result<Self> {
        // The instance is a handle to our GPU
        // Backends::all => Vulkan + Metal + DX12 + Browser WebGPU
        let instance = wgpu::Instance::new(wgpu::Backends::all());

        let (size, surface) = match &render_mode {
            RenderMode::Preview(window) => {
                let size = window.inner_size();
                let surface = unsafe { instance.create_surface(window) };

                (size, Some(surface))
            }
            RenderMode::Output { args } => (args.quality.size(), None),
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
            RenderMode::Output { args } => {
                let size = CanvasSize::new(size);

                let texture_desc = wgpu::TextureDescriptor {
                    size: size.extent(),
                    mip_level_count: 1,
                    sample_count: 1,
                    dimension: wgpu::TextureDimension::D2,
                    format: wgpu::TextureFormat::Rgba8UnormSrgb,
                    usage: wgpu::TextureUsages::COPY_SRC | wgpu::TextureUsages::RENDER_ATTACHMENT,
                    label: None,
                };
                let texture = device.create_texture(&texture_desc);

                // TODO: also use image output when there's no animation
                let output = if args.single_frame {
                    ImageOutput::new(size, args.output_file).into()
                } else {
                    VideoOutput::new(size, args.output_file)?.into()
                };
                let canvas_buf = CanvasBuffer::new(&device, size);

                (
                    RenderTarget::Output {
                        texture,
                        canvas_buf,
                        output,
                    },
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

        let data = RenderData::new(&device);

        Ok(Self {
            device,
            queue,
            render_pipeline,
            render_target,
            data,
        })
    }

    pub fn resize(&mut self, new_size: PhysicalSize<u32>) {
        if new_size.width > 0 && new_size.height > 0 {
            // TODO: adjust canvas size
            match &mut self.render_target {
                RenderTarget::Window { surface, config } => {
                    config.width = new_size.width;
                    config.height = new_size.height;
                    surface.configure(&self.device, config);
                }
                _ => todo!(),
            }
        }
    }

    pub fn update(&mut self) {}

    pub async fn render(&mut self) -> Result<()> {
        let encoder = self
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("Render Encoder"),
            });

        match &mut self.render_target {
            RenderTarget::Window { surface, .. } => {
                let texture = surface.get_current_texture()?;
                let view = texture
                    .texture
                    .create_view(&wgpu::TextureViewDescriptor::default());
                let mut canvas = Canvas { view, encoder };

                canvas.render_pass(&self.render_pipeline, &self.data);
                canvas.finish(&self.queue);
                texture.present();
            }
            RenderTarget::Output {
                texture,
                canvas_buf,
                output,
            } => {
                let view = texture.create_view(&Default::default());
                let mut canvas = Canvas { view, encoder };

                canvas.render_pass(&self.render_pipeline, &self.data);
                canvas.copy_to_output(texture, canvas_buf);
                canvas.finish(&self.queue);

                let mut view = canvas_buf.view(&self.device).await;
                output.encode_frame(&mut view)?;
                canvas_buf.unmap(view);
            }
        };

        // match &mut self.render_target {
        //     RenderTarget::Window { surface, .. } => {
        //         let output = surface.get_current_texture()?;
        //         let view = output
        //             .texture
        //             .create_view(&wgpu::TextureViewDescriptor::default());
        //         render_pass(&view);
        //         self.queue.submit(std::iter::once(encoder.finish()));
        //         output.present();
        //     }
        //     RenderTarget::Image { canvas: target } => {
        //         render_pass(&target.view);
        //         texture_target_common(encoder, target, &self.queue, &self.device).await;

        //         // write_frame
        //         use image::{ImageBuffer, Rgba};
        //         let buffer = ImageBuffer::<Rgba<u8>, _>::from_raw(
        //             target.extent.width,
        //             target.extent.height,
        //             target.image_buffer.as_slice(),
        //         )
        //         .unwrap();
        //         // encode
        //         buffer.save("image.png").unwrap();

        //         target.buffer.unmap();
        //     }
        //     RenderTarget::Video {
        //         canvas: target,
        //         encoder,
        //     } => {
        //         render_pass(&target.view);
        //         texture_target_common(encoder, target, &self.queue, &self.device).await;
        //         encoder.write_frame(&target.image_buffer);
        //         encoder.encode()?;
        //         target.buffer.unmap();
        //     }
        // }

        Ok(())
    }

    pub fn finish(&mut self) -> Result<()> {
        // TODO: images, write to file; videos, end encoding and write to file

        if let RenderTarget::Output { output, .. } = &mut self.render_target {
            output.conclude()?;
        }
        Ok(())
    }
}
