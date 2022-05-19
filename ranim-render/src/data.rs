use bytemuck::{Pod, Zeroable};
use wgpu::util::DeviceExt;

pub struct RenderData {
    pub vertex_buffer: wgpu::Buffer,
    pub index_buffer: wgpu::Buffer,
    pub num_indices: u32,
}
impl RenderData {
    pub fn new(device: &wgpu::Device) -> Self {
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
        Self {
            vertex_buffer,
            index_buffer,
            num_indices: INDICES.len() as u32,
        }
    }
}

#[repr(C)]
#[derive(Copy, Clone, Debug, Pod, Zeroable)]
pub struct Vertex {
    position: [f32; 3],
    color: [f32; 3],
}

macro_rules! vertex_attr_array {
    ($($t:tt)*) => {{
        const ATTRIBS: &[wgpu::VertexAttribute] = &wgpu::vertex_attr_array![$($t)*];
        ATTRIBS
    }};
}

impl Vertex {
    pub fn desc<'a>() -> wgpu::VertexBufferLayout<'a> {
        wgpu::VertexBufferLayout {
            array_stride: std::mem::size_of::<Vertex>() as wgpu::BufferAddress,
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
        position: [0.25, -0.75, 0.0],
        color: [0.0, 0.0, 0.0],
    },
    Vertex {
        position: [-0.5, -0.5, 0.0],
        color: [0.0, 0.0, 1.0],
    },
    Vertex {
        position: [-0.25, 0.75, 0.0],
        color: [1.0, 1.0, 1.0],
    },
];

pub const INDICES: &[u16] = &[0, 2, 1, 0, 3, 2];
