use bytemuck::{Pod, Zeroable};

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

pub const INDICES: &[u16] = &[0, 2, 1, 0, 3, 2];
