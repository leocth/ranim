use glam::{Quat, Vec3};
use winit::dpi::PhysicalSize;

use crate::{camera::CameraGroup};

use self::{buffer::{DynamicBuffer, MappedDynamicBuffer}, types::{Vertex, Instance, InstanceRaw, Index, INDICES, VERTICES}};

pub mod buffer;
pub mod types;

pub struct RenderData {
    pub vertices: DynamicBuffer<Vertex>,
    pub indices: DynamicBuffer<Index>,
    pub instances: MappedDynamicBuffer<Instance, InstanceRaw>,

    pub num_indices: u32,
    pub camera: CameraGroup,
}
impl RenderData {
    pub fn new(device: &wgpu::Device, size: PhysicalSize<u32>) -> Self {
        let vertices = DynamicBuffer::from_data(
            device,
            Vec::from(VERTICES),
            Some("Vertex Buffer"),
            wgpu::BufferUsages::VERTEX,
        );
        let indices = DynamicBuffer::from_data(
            device,
            Vec::from(INDICES),
            Some("Index Buffer"),
            wgpu::BufferUsages::INDEX,
        );

        let camera = CameraGroup::new(device, size);

        let instances = MappedDynamicBuffer::from_data(
            device,
            vec![Instance {
                position: Vec3::ZERO,
                rotation: Quat::IDENTITY,
            }],
            Some("Instance Buffer"),
            wgpu::BufferUsages::VERTEX
        );
        Self {
            vertices,
            indices,
            instances,

            num_indices: INDICES.len() as u32,
            camera,
        }
    }
    pub fn update(&mut self, queue: &wgpu::Queue) {
        self.camera.update(queue);
        self.vertices.update(queue);
        self.indices.update(queue);
        self.instances.update(queue);
    }
}