use glam::{
    f32::{Mat4, Vec3},
    vec3,
};
use wgpu::util::DeviceExt;
use winit::dpi::PhysicalSize;

pub struct Camera2D {
    pub position: Vec3,
    pub rotation: f32,
    pub scale: f32,

    pub aspect: f32,
}
impl Camera2D {
    pub fn new(size: PhysicalSize<u32>) -> Self {
        let mut cam = Self {
            position: Vec3::ZERO,
            rotation: 0.0,
            scale: 1.0 / 7.0,
            aspect: 1.0,
        };
        cam.resize(size);
        cam
    }
    pub fn build_view_projection_matrix(&self) -> Mat4 {
        let position = Mat4::from_translation(self.position);
        let rotation = Mat4::from_rotation_z(self.rotation);
        let scale = Mat4::from_scale(vec3(1.0, self.aspect, 1.0) * self.scale);
        position * scale * rotation
    }
    pub fn resize(&mut self, new_size: PhysicalSize<u32>) {
        self.aspect = new_size.width as f32 / new_size.height as f32;
    }
}

pub struct CameraGroup {
    pub camera: Camera2D,
    pub uniform: CameraUniform,
    pub buffer: wgpu::Buffer,
    pub bind_group: wgpu::BindGroup,
    pub bind_group_layout: wgpu::BindGroupLayout,
}
impl CameraGroup {
    pub fn new(device: &wgpu::Device, size: PhysicalSize<u32>) -> Self {
        let camera = Camera2D::new(size);

        let mut uniform = CameraUniform::new();
        uniform.update_view_proj(&camera);

        let buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Camera Buffer"),
            contents: bytemuck::cast_slice(&[uniform]),
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        });
        let bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            entries: &[wgpu::BindGroupLayoutEntry {
                binding: 0,
                visibility: wgpu::ShaderStages::VERTEX,
                ty: wgpu::BindingType::Buffer {
                    ty: wgpu::BufferBindingType::Uniform,
                    has_dynamic_offset: false,
                    min_binding_size: None,
                },
                count: None,
            }],
            label: Some("camera_bind_group_layout"),
        });
        let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            layout: &bind_group_layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: buffer.as_entire_binding(),
            }],
            label: Some("camera_bind_group"),
        });
        Self {
            camera,
            uniform,
            buffer,
            bind_group,
            bind_group_layout,
        }
    }
    pub fn update(&mut self, queue: &wgpu::Queue) {
        self.uniform.update_view_proj(&self.camera);
        queue.write_buffer(&self.buffer, 0, bytemuck::cast_slice(&[self.uniform]));
    }
}

#[repr(C)]
#[derive(Debug, Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
pub struct CameraUniform {
    view_proj: [[f32; 4]; 4],
}
impl CameraUniform {
    pub fn new() -> Self {
        Self {
            view_proj: Mat4::IDENTITY.to_cols_array_2d(),
        }
    }
    pub fn update_view_proj(&mut self, camera: &Camera2D) {
        self.view_proj = camera.build_view_projection_matrix().to_cols_array_2d();
    }
}

impl Default for CameraUniform {
    fn default() -> Self {
        Self::new()
    }
}
