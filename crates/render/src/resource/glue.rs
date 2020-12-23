use std::{mem::ManuallyDrop, ops::Range, sync::Arc};

use super::pipeline::ShaderType;
use crate::context::GpuContext;
use crate::resource::buffer::BufferRange;

#[derive(Debug, Clone)]
pub enum PartType {
    Uniform(usize),
    Sampler,
}

#[derive(Debug, Clone)]
pub enum PartIndex {
    Name(String),
    Binding(u32),
}

#[derive(Debug, Clone)]
pub struct MixturePart {
    pub binding: u32,
    pub name: String,
    pub shader_type: ShaderType,
    pub is_dynamic: bool,
    pub array_size: usize,
    /// Which atm is just a size (but thats probably enough)
    pub type_info: PartType,
}

type RawDescriptorLayout<Context> = <Context as GpuContext>::DescriptorLayout;

#[derive(Debug)]
pub struct DescriptorLayout<Context: GpuContext + ?Sized> {
    pub ctx: Arc<Context>,
    pub handle: ManuallyDrop<RawDescriptorLayout<Context>>,
}

impl<Context: GpuContext + ?Sized> Drop for DescriptorLayout<Context> {
    fn drop(&mut self) {
        unsafe {
            let handle = ManuallyDrop::take(&mut self.handle);
            self.ctx.drop_descriptor_layout(handle);
        }
    }
}

#[derive(Debug)]
pub struct Mixture<Context: GpuContext + ?Sized> {
    pub parts: Vec<MixturePart>,
    pub gpu_layout: DescriptorLayout<Context>,
}

impl<Context: GpuContext> Mixture<Context> {
    pub fn new(
        parts: Vec<MixturePart>,
        ctx: Arc<Context>,
        gpu_layout: RawDescriptorLayout<Context>,
    ) -> Self {
        Self {
            parts,
            gpu_layout: DescriptorLayout {
                ctx,
                handle: ManuallyDrop::new(gpu_layout),
            },
        }
    }
}

#[derive(Debug)]
pub enum Descriptor<'a, Context: GpuContext + ?Sized> {
    Buffer(&'a <Context as GpuContext>::BufferHandle, BufferRange),
}

#[derive(Debug)]
pub struct DescriptorWrite<'a, Context: GpuContext + ?Sized> {
    pub binding: u32,
    pub array_offset: usize,
    pub descriptor: Descriptor<'a, Context>,
}

#[derive(Debug)]
pub struct DescriptorSet<Context: GpuContext + ?Sized> {
    pub ctx: Arc<Context>,
    pub handle: ManuallyDrop<Context::DescriptorSet>,
}

impl<Context: GpuContext + ?Sized> Drop for DescriptorSet<Context> {
    fn drop(&mut self) {
        let set = unsafe { ManuallyDrop::take(&mut self.handle) };
        self.ctx.drop_descriptor_set(set);
    }
}

#[derive(Debug)]
pub struct GlueBottle<'a, Context: GpuContext + ?Sized> {
    pub handle: DescriptorSet<Context>,
    parts: Vec<MixturePart>,

    // Maps from binding to write
    writes: Vec<DescriptorWrite<'a, Context>>,
}

#[derive(Debug)]
pub struct Glue<Context: GpuContext + ?Sized> {
    pub handle: DescriptorSet<Context>,
    pub parts: Vec<MixturePart>,
}

impl<'a, Context: GpuContext> GlueBottle<'a, Context> {
    pub fn new(handle: DescriptorSet<Context>, parts: Vec<MixturePart>) -> Self {
        Self {
            handle,
            parts,
            writes: vec![],
        }
    }

    pub fn write_buffer(
        &mut self,
        index: PartIndex,
        buffer: &'a Context::BufferHandle,
        buffer_offset: Option<u64>,
    ) {
        let part = self
            .parts
            .iter()
            .find(|p| match &index {
                PartIndex::Name(name) => &p.name == name,
                PartIndex::Binding(id) => &p.binding == id,
            })
            .expect("[GlueBottle] (write_buffer) failed to find matching part index");

        match part.type_info {
            PartType::Uniform(size) => {
                let start = buffer_offset.unwrap_or(0);
                let end = start + size as u64;
                let write = DescriptorWrite {
                    binding: part.binding,
                    array_offset: 0,
                    descriptor: Descriptor::Buffer(buffer, (start..end).into()),
                };
                self.writes.push(write);
            }
            PartType::Sampler => {
                panic!("[GlueBottle] (write_buffer) mismatch descriptor is sampler")
            }
        }
    }

    pub fn write_array(
        &mut self,
        index: PartIndex,
        range: Range<u64>,
        buffer: &'a Context::BufferHandle,
        buffer_offset: Option<u64>,
    ) {
        let part = self
            .parts
            .iter()
            .find(|p| match &index {
                PartIndex::Name(name) => &p.name == name,
                PartIndex::Binding(id) => &p.binding == id,
            })
            .expect("[GlueBottle] (write_array) failed to find matching part index");

        match part.type_info {
            PartType::Uniform(size) => {
                let start = buffer_offset.unwrap_or(0);
                let size = size as u64 * (range.end - range.start);
                let end = start + size;
                let write = DescriptorWrite {
                    binding: part.binding,
                    array_offset: range.start as usize,
                    descriptor: Descriptor::Buffer(buffer, (start..end).into()),
                };
                self.writes.push(write);
            }
            PartType::Sampler => {
                panic!("[GlueBottle] (write_array) mismatch descriptor is sampler")
            }
        }
    }

    pub fn apply(self) -> Glue<Context> {
        self.handle
            .ctx
            .update_descriptor_set(&self.handle.handle, self.writes);

        Glue {
            handle: self.handle,
            parts: self.parts,
        }
    }
}
