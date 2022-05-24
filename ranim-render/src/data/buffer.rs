use std::{slice::SliceIndex, ops::RangeBounds, marker::PhantomData};

use bytemuck::Pod;
use wgpu::util::DeviceExt;

use crate::util;

pub struct DynamicBuffer<T> {
    pub data: Vec<T>,
    pub raw: Vec<u8>,
    pub buffer: wgpu::Buffer,
    label: wgpu::Label<'static>,
    usage: wgpu::BufferUsages,
}
impl<T: Pod> DynamicBuffer<T> {
    pub fn new(
        device: &wgpu::Device,
        label: wgpu::Label<'static>,
        usage: wgpu::BufferUsages,
    ) -> Self {
        let data = vec![];
        let raw = vec![0u8; wgpu::COPY_BYTES_PER_ROW_ALIGNMENT as usize];

        let buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label,
            contents: &raw,
            usage: usage | wgpu::BufferUsages::COPY_DST,
        });
        Self {
            data,
            raw,
            buffer,
            label,
            usage
        }
    }

    // Vector operations
    pub fn len(&self) -> usize {
        self.data.len()
    }
    pub fn push(&mut self, t: T) {
        self.data.push(t)
    }

    // Buffer operations
    pub fn size(&self) -> usize {
        self.raw.len()
    }
    pub fn update(&mut self, device: &wgpu::Device, queue: &wgpu::Queue) {
        let cast_data = bytemuck::cast_slice(&self.data);
        let cast_len = cast_data.len();
        if cast_len > self.raw.len() {
            // resize
            let size = (self.raw.len() * 2).min(cast_len);
            let size = util::pad_to_bytes_per_row_alignment(size);

            self.raw = vec![0; size];
            self.raw[..cast_len].copy_from_slice(cast_data);
            self.buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: self.label,
                contents: &self.raw,
                usage: self.usage | wgpu::BufferUsages::COPY_DST,
            });
        } else {
            self.raw[..cast_len].copy_from_slice(cast_data);
            self.raw[cast_len..].fill(0);
        }
        queue.write_buffer(&self.buffer, 0, &self.raw);
    }
    pub fn slice<S>(&self, bounds: S) -> wgpu::BufferSlice<'_>
    where
        S: RangeBounds<wgpu::BufferAddress>,
    {
        self.buffer.slice(bounds)
    }
}
impl<T, I> std::ops::Index<I> for DynamicBuffer<T>
where
    I: SliceIndex<[T]>,
{
    type Output = <I as SliceIndex<[T]>>::Output;

    fn index(&self, index: I) -> &Self::Output {
        &self.data[index]
    }
}
impl<T, I> std::ops::IndexMut<I> for DynamicBuffer<T>
where
    I: SliceIndex<[T]>,
{
    fn index_mut(&mut self, index: I) -> &mut Self::Output {
        &mut self.data[index]
    }
}
impl<T> Extend<T> for DynamicBuffer<T> {
    fn extend<I: IntoIterator<Item = T>>(&mut self, iter: I) {
        self.data.extend(iter)
    }
}