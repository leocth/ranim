use std::{slice::SliceIndex, ops::RangeBounds, marker::PhantomData};

use bytemuck::Pod;
use wgpu::util::DeviceExt;

use crate::util;

pub struct DynamicBuffer<T> {
    pub data: Vec<T>,
    pub raw: Vec<u8>,
    pub buffer: wgpu::Buffer,
    pub size: usize,
}
impl<T: Pod> DynamicBuffer<T> {
    pub fn from_data(
        device: &wgpu::Device,
        data: Vec<T>,
        label: wgpu::Label<'_>,
        usage: wgpu::BufferUsages,
    ) -> Self {
        let cast_data = bytemuck::cast_slice(&data);
        let size = util::pad_to_bytes_per_row_alignment(cast_data.len());

        let mut raw = vec![0u8; size];
        raw[..cast_data.len()].copy_from_slice(cast_data);
        raw[cast_data.len()..].fill(0);

        let buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label,
            contents: &raw,
            usage: usage | wgpu::BufferUsages::COPY_DST,
        });
        Self {
            data,
            raw,
            buffer,
            size,
        }
    }
    pub fn update(&mut self, queue: &wgpu::Queue) {
        let cast_data = bytemuck::cast_slice(&self.data);
        self.raw[..cast_data.len()].copy_from_slice(cast_data);
        self.raw[cast_data.len()..].fill(0);
        queue.write_buffer(&self.buffer, 0, &self.raw);
    }
    pub fn len(&self) -> usize {
        self.data.len()
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

pub struct MappedDynamicBuffer<T, U> {
    buf: DynamicBuffer<U>,
    _phan: PhantomData<T>,
}
impl<T, U> MappedDynamicBuffer<T, U>
where
    U: Pod + From<T>,
{
    pub fn from_data(
        device: &wgpu::Device,
        data: Vec<T>,
        label: wgpu::Label<'_>,
        usage: wgpu::BufferUsages,
    ) -> Self {
        let data: Vec<_> = data.into_iter().map(U::from).collect();
        let buf = DynamicBuffer::from_data(device, data, label, usage);

        Self {
            buf,
            _phan: PhantomData,
        }
    }
    pub fn update(&mut self, queue: &wgpu::Queue) {
        self.buf.update(queue);
    }
    pub fn len(&self) -> usize {
        self.buf.len()
    }

    pub fn slice<S>(&self, bounds: S) -> wgpu::BufferSlice<'_>
    where
        S: RangeBounds<wgpu::BufferAddress>,
    {
        self.buf.slice(bounds)
    }
}
impl<T, U, I> std::ops::Index<I> for MappedDynamicBuffer<T, U>
where
    I: SliceIndex<[U]>,
{
    type Output = <I as SliceIndex<[U]>>::Output;

    fn index(&self, index: I) -> &Self::Output {
        &self.buf.data[index]
    }
}
impl<T, U, I> std::ops::IndexMut<I> for MappedDynamicBuffer<T, U>
where
    I: SliceIndex<[U]>,
{
    fn index_mut(&mut self, index: I) -> &mut Self::Output {
        &mut self.buf.data[index]
    }
}
