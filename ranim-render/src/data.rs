use winit::dpi::PhysicalSize;

use crate::camera::CameraGroup;

use self::{
    buffer::DynamicBuffer,
    types::{Index, InstanceRaw, Vertex},
};

pub mod buffer;
pub mod types;

pub struct RenderData {
    pub vertices: DynamicBuffer<Vertex>,
    pub indices: DynamicBuffer<Index>,
    pub instances: DynamicBuffer<InstanceRaw>,
    pub camera: CameraGroup,
}
impl RenderData {
    pub fn new(device: &wgpu::Device, size: PhysicalSize<u32>) -> Self {
        let vertices =
            DynamicBuffer::new(device, Some("Vertex Buffer"), wgpu::BufferUsages::VERTEX);
        let indices = DynamicBuffer::new(device, Some("Index Buffer"), wgpu::BufferUsages::INDEX);
        let instances =
            DynamicBuffer::new(device, Some("Instance Buffer"), wgpu::BufferUsages::VERTEX);

        let camera = CameraGroup::new(device, size);

        Self {
            vertices,
            indices,
            instances,
            camera,
        }
    }
    pub fn update(&mut self, device: &wgpu::Device, queue: &wgpu::Queue) {
        self.camera.update(queue);
        self.vertices.update(device, queue);
        self.indices.update(device, queue);
        self.instances.update(device, queue);
    }
}
