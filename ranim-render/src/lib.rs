#![feature(portable_simd)]
#![feature(array_chunks)]
#![deny(rust_2018_idioms)]

use args::Args;
use color_eyre::Result;
use data::{types::{Vertex, InstanceRaw}, RenderData};
use util::Size;
use winit::window::Window;

pub mod args;
pub mod camera;
pub mod data;
pub mod util;
pub mod video;

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("No adapter found.")]
    NoAdapterFound,
    #[error(transparent)]
    RequestDeviceError(#[from] wgpu::RequestDeviceError),
}

pub struct Renderer {
    pub(crate) device: wgpu::Device,
    pub(crate) queue: wgpu::Queue,
    pub(crate) size: Size,
}
impl Renderer {
    pub async fn new(args: &Args) -> Result<Self, Error> {
        let instance = wgpu::Instance::new(wgpu::Backends::all());
        Self::new_inner(instance, None, args.quality.size()).await
    }
    pub async fn from_window(window: &Window) -> Result<Self, Error> {
        let instance = wgpu::Instance::new(wgpu::Backends::all());
        let size = window.inner_size();
        let surface = unsafe { instance.create_surface(window) };
        Self::new_inner(instance, Some(surface), size.into()).await
    }
    async fn new_inner(
        instance: wgpu::Instance,
        surface: Option<wgpu::Surface>,
        size: Size,
    ) -> Result<Self, Error> {
        let adapter = instance
            .request_adapter(&wgpu::RequestAdapterOptions {
                power_preference: wgpu::PowerPreference::default(),
                compatible_surface: surface.as_ref(),
                force_fallback_adapter: false,
            })
            .await
            .ok_or(Error::NoAdapterFound)?;

        let (device, queue) = adapter
            .request_device(&wgpu::DeviceDescriptor::default(), None)
            .await?;
        Ok(Self { device, queue, size })
    }
}


pub struct RenderPass {
    pipeline: wgpu::RenderPipeline,
}
impl RenderPass {
    pub fn new(renderer: &Renderer, data: &RenderData) -> Self {
        let layout = renderer.device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("Render Pipeline Layout"),
            bind_group_layouts: &[&data.camera.bind_group_layout],
            push_constant_ranges: &[],
        });
        let shader = renderer.device.create_shader_module(&wgpu::include_wgsl!("shaders/shader.wgsl"));
        let pipeline = renderer.device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("Render Pipeline"),
            layout: Some(&layout),
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: "vertex",
                buffers: &[Vertex::desc(), InstanceRaw::desc()],
            },
            fragment: Some(wgpu::FragmentState {
                module: &shader,
                entry_point: "fragment",
                targets: &[wgpu::ColorTargetState {
                    format: wgpu::TextureFormat::Rgba8Unorm,
                    blend: Some(wgpu::BlendState::REPLACE),
                    write_mask: wgpu::ColorWrites::ALL,
                }],
            }),
            primitive: wgpu::PrimitiveState {
                topology: wgpu::PrimitiveTopology::TriangleStrip,
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
        Self { pipeline }
    }
    pub fn execute(
        &mut self,
        encoder: &mut wgpu::CommandEncoder,
        rgb: &RgbTexture,
        data: &RenderData,
    ) {
        let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("Render Pass"),
            color_attachments: &[wgpu::RenderPassColorAttachment {
                view: &rgb.tv.view,
                resolve_target: None,
                ops: wgpu::Operations {
                    load: wgpu::LoadOp::Clear(wgpu::Color {
                        r: 0.3,
                        g: 0.6,
                        b: 0.9,
                        a: 1.0,
                    }),
                    store: true,
                },
            }],
            depth_stencil_attachment: None,
        });

        pass.set_pipeline(&self.pipeline);
        pass.set_bind_group(0, &data.camera.bind_group, &[]);
        pass.set_vertex_buffer(0, data.vertices.slice(..));
        pass.set_vertex_buffer(1, data.instances.slice(..));
        pass.set_index_buffer(data.indices.slice(..), wgpu::IndexFormat::Uint16);

        pass.draw_indexed(
            0..data.indices.len() as u32,
            0,
            0..data.instances.len() as u32,
        );
    }
}

struct TextureAndView {
    texture: wgpu::Texture,
    view: wgpu::TextureView,
}
impl TextureAndView {
    fn new(device: &wgpu::Device, desc: &wgpu::TextureDescriptor<'_>) -> Self {
        let texture = device.create_texture(desc);
        let view = texture.create_view(&wgpu::TextureViewDescriptor::default());
        Self { texture, view }
    }
}

pub struct RgbTexture {
    tv: TextureAndView,
}
impl RgbTexture {
    pub fn new(renderer: &Renderer) -> Self {
        let desc = wgpu::TextureDescriptor {
            size: renderer.size.extent(),
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Rgba8Unorm,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT
                | wgpu::TextureUsages::TEXTURE_BINDING
                | wgpu::TextureUsages::COPY_SRC,
            label: Some("RGB texture"),
        };
        let tv = TextureAndView::new(&renderer.device, &desc);
        Self { tv }
    }
}