use bytemuck::{Pod, Zeroable};
use glam::{Vec3, Quat, Mat4};

macro_rules! vertex_attr_array {
    ($($t:tt)*) => {{
        const ATTRIBS: &[wgpu::VertexAttribute] = &wgpu::vertex_attr_array![$($t)*];
        ATTRIBS
    }};
}

pub type Index = u16;

#[repr(C)]
#[derive(Copy, Clone, Debug, Pod, Zeroable)]
pub struct Vertex {
    pub position: [f32; 3],
    pub color: [f32; 3],
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
        position: [0.5, -0.5, 0.0],
        color: [0.0, 0.0, 0.0],
    },
    Vertex {
        position: [-0.5, -0.5, 0.0],
        color: [0.0, 0.0, 1.0],
    },
    Vertex {
        position: [0.0, 0.5, 0.0],
        color: [1.0, 0.0, 0.0],
    },
];

pub struct Instance {
    pub position: Vec3,
    pub rotation: Quat,
}
#[repr(C)]
#[derive(Copy, Clone, Pod, Zeroable)]
pub struct InstanceRaw {
    pub model: [[f32; 4]; 4],
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
impl From<Instance> for InstanceRaw {
    fn from(ins: Instance) -> Self {
        Self {
            model: Mat4::from_rotation_translation(ins.rotation, ins.position).to_cols_array_2d(),
        }
    }
}
pub const INDICES: &[u16] = &[2, 1, 0];