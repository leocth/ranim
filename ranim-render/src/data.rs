use bytemuck::{Pod, Zeroable};
use glam::{Mat4, Quat, Vec3};
use wgpu::util::DeviceExt;
use winit::dpi::PhysicalSize;

use crate::camera::CameraGroup;

pub struct RenderData {
    pub vertex_buffer: wgpu::Buffer,
    pub index_buffer: wgpu::Buffer,
    pub instances: Vec<InstanceRaw>,
    pub instance_buffer: wgpu::Buffer,

    pub num_indices: u32,
    pub camera: CameraGroup,
}
impl RenderData {
    pub fn new(device: &wgpu::Device, size: PhysicalSize<u32>) -> Self {
        let vertex_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Vertex Buffer"),
            contents: bytemuck::cast_slice(VERTICES), // XXX
            usage: wgpu::BufferUsages::VERTEX,
        });
        let index_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Index Buffer"),
            contents: bytemuck::cast_slice(INDICES), // XXX
            usage: wgpu::BufferUsages::INDEX,
        });
        let camera = CameraGroup::new(device, size);

        let instances: Vec<_> = (0..NUM_INSTANCES_PER_ROW)
            .flat_map(|z| {
                (0..NUM_INSTANCES_PER_ROW).map(move |x| {
                    let position = Vec3::new(
                        x as f32 - INSTANCE_DISPLACEMENT,
                        z as f32 - INSTANCE_DISPLACEMENT,
                        0.0,
                    );
                    let rotation = Quat::IDENTITY;
                    Instance { position, rotation }.to_raw()
                })
            })
            .collect();
        let instance_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Instance Buffer"),
            contents: bytemuck::cast_slice(&instances),
            usage: wgpu::BufferUsages::VERTEX,
        });

        Self {
            vertex_buffer,
            index_buffer,
            num_indices: INDICES.len() as u32,
            camera,
            instances,
            instance_buffer,
        }
    }
    pub fn update(&mut self, queue: &wgpu::Queue) {
        self.camera.update(queue)
    }
}
const NUM_INSTANCES_PER_ROW: u32 = 5;
const INSTANCE_DISPLACEMENT: f32 = NUM_INSTANCES_PER_ROW as f32 * 0.5;

macro_rules! vertex_attr_array {
    ($($t:tt)*) => {{
        const ATTRIBS: &[wgpu::VertexAttribute] = &wgpu::vertex_attr_array![$($t)*];
        ATTRIBS
    }};
}

#[repr(C)]
#[derive(Copy, Clone, Debug, Pod, Zeroable)]
pub struct Vertex {
    position: [f32; 3],
    color: [f32; 3],
}
impl Vertex {
    pub fn desc<'a>() -> wgpu::VertexBufferLayout<'a> {
        wgpu::VertexBufferLayout {
            array_stride: std::mem::size_of::<Self>() as wgpu::BufferAddress,
            step_mode: wgpu::VertexStepMode::Vertex,
            attributes: vertex_attr_array![0 => Float32x3, 1 => Float32x3],
        }
    }
}
pub const VERTICES: &[Vertex] = &[
    Vertex {
        position: [0.5, 0.5, 0.0],
        color: [1.0, 0.0, 0.0],
    },
    Vertex {
        position: [0.5, -0.5, 0.0],
        color: [0.0, 0.0, 0.0],
    },
    Vertex {
        position: [-0.5, -0.5, 0.0],
        color: [0.0, 0.0, 1.0],
    },
    Vertex {
        position: [-0.5, 0.5, 0.0],
        color: [1.0, 1.0, 1.0],
    },
];

pub struct Instance {
    position: Vec3,
    rotation: Quat,
}
impl Instance {
    fn to_raw(&self) -> InstanceRaw {
        InstanceRaw {
            model: Mat4::from_rotation_translation(self.rotation, self.position).to_cols_array_2d(),
        }
    }
}
#[repr(C)]
#[derive(Copy, Clone, Pod, Zeroable)]
pub struct InstanceRaw {
    model: [[f32; 4]; 4],
}
impl InstanceRaw {
    pub fn desc<'a>() -> wgpu::VertexBufferLayout<'a> {
        wgpu::VertexBufferLayout {
            array_stride: std::mem::size_of::<Self>() as wgpu::BufferAddress,
            step_mode: wgpu::VertexStepMode::Instance,
            attributes: vertex_attr_array![
                5 => Float32x4,
                6 => Float32x4,
                7 => Float32x4,
                8 => Float32x4,
            ],
        }
    }
}
pub const INDICES: &[u16] = &[0, 2, 1, 0, 3, 2];
