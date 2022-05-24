use bytemuck::{Pod, Zeroable};
use glam::{Mat4, Quat, Vec3, Vec4};

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

#[derive(Copy, Clone)]
pub struct Instance {
    pub position: Vec3,
    pub rotation: Quat,
    pub scale: Vec3,
    pub color: Vec4,
}
impl Default for Instance {
    fn default() -> Self {
        Self {
            position: Vec3::ZERO,
            rotation: Quat::IDENTITY,
            scale: Vec3::ONE,
            color: Vec4::ONE,
        }
    }
}
#[repr(C)]
#[derive(Copy, Clone, Pod, Zeroable)]
pub struct InstanceRaw {
    pub model: [[f32; 4]; 4],
    pub color: [f32; 4],
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
                9 => Float32x4,
            ],
        }
    }
}
impl From<Instance> for InstanceRaw {
    fn from(ins: Instance) -> Self {
        Self {
            model: Mat4::from_scale_rotation_translation(ins.scale, ins.rotation, ins.position)
                .to_cols_array_2d(),
            color: ins.color.into()
        }
    }
}
