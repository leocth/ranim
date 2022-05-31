use crate::{camera::CameraGroup, Renderer};

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
    pub fn new(renderer: &Renderer) -> Self {
        let vertices = DynamicBuffer::new(
            &renderer.device,
            Some("Vertex Buffer"),
            wgpu::BufferUsages::VERTEX,
        );
        let indices = DynamicBuffer::new(
            &renderer.device,
            Some("Index Buffer"),
            wgpu::BufferUsages::INDEX,
        );
        let instances = DynamicBuffer::new(
            &renderer.device,
            Some("Instance Buffer"),
            wgpu::BufferUsages::VERTEX,
        );

        let camera = CameraGroup::new(renderer);

        Self {
            vertices,
            indices,
            instances,
            camera,
        }
    }
    pub fn update(&mut self, renderer: &Renderer) {
        self.camera.update(renderer);
        self.vertices.update(renderer);
        self.indices.update(renderer);
        self.instances.update(renderer);
    }
}
