use app::AssetHandle;
use bytemuck::{Pod, Zeroable};
use render::{
    command_encoder::{IndexType, Renderable},
    prelude::{BufferRange, BufferUsage, CommandEncoder, GpuContext, MemoryType},
    resource::{
        buffer::BufferDescriptor,
        pipeline::{
            AttributeDescriptor, VertexAttributeFormat, VertexBufferDescriptor, VertexInputRate,
        },
    },
};
use std::borrow::Cow;

use crate::{
    material::{Material, SolidMaterial},
    renderer::ActiveContext,
};

#[derive(Copy, Clone, Zeroable, Pod)]
#[repr(C)]
pub struct Vertex {
    pub pos: glam::Vec3,
    pub normal: glam::Vec3,
}

pub const VERTEX_SIZE: usize = std::mem::size_of::<Vertex>();

#[derive(Debug, Clone)]
pub enum Indices {
    U16(Vec<u16>),
    U32(Vec<u32>),
}

impl Indices {
    pub fn len(&self) -> usize {
        match self {
            Indices::U16(slice) => slice.len(),
            Indices::U32(slice) => slice.len(),
        }
    }
}

impl From<&Indices> for IndexType {
    fn from(indices: &Indices) -> Self {
        match indices {
            Indices::U16(_) => IndexType::U16,
            Indices::U32(_) => IndexType::U32,
        }
    }
}

impl Vertex {
    pub(crate) fn get_layout() -> (VertexBufferDescriptor, Vec<AttributeDescriptor>) {
        let size = std::mem::size_of::<Self>();
        let buffer_descriptor = VertexBufferDescriptor::new(0, size as _, VertexInputRate::Vertex);
        let attributes = vec![
            AttributeDescriptor::new(0, 0, 0, VertexAttributeFormat::Vec3),
            AttributeDescriptor::new(
                1,
                0,
                std::mem::size_of::<glam::Vec3>() as _,
                VertexAttributeFormat::Vec3,
            ),
        ];
        (buffer_descriptor, attributes)
    }
}

#[derive(Debug)]
pub struct MeshPart {
    pub(crate) vertex_buffer: <ActiveContext as GpuContext>::BufferHandle,
    pub(crate) index_buffer: <ActiveContext as GpuContext>::BufferHandle,
    pub(crate) index_type: IndexType,
    pub(crate) indices_count: u32,
    pub(crate) material: Material,
}

impl MeshPart {
    pub fn new(
        vertex_buffer: <ActiveContext as GpuContext>::BufferHandle,
        index_buffer: <ActiveContext as GpuContext>::BufferHandle,
        index_type: IndexType,
        indices_count: u32,
        material: Material,
    ) -> Self {
        Self {
            vertex_buffer,
            index_buffer,
            index_type,
            indices_count,
            material,
        }
    }

    pub fn from_data(
        name: &str,
        vertices: &[Vertex],
        indices: &Indices,
        material: Material,
        ctx: &ActiveContext,
    ) -> Self {
        // BIG TODO: Abstract that away (see story CPU <-> GPU Dataflow)
        let vertex_buffer = ctx.create_buffer(&BufferDescriptor {
            // TODO: Naming
            name: format!("{}-vertex-buffer", name).into(),
            size: (VERTEX_SIZE * vertices.len()) as u64,
            // NOTE: should be upgraded to device local memory (but i dont give a s*** right now)
            memory_type: MemoryType::HostVisible,
            usage: BufferUsage::Vertex,
        });
        // Upload vertex data
        unsafe {
            let vertex_data: &[u8] = bytemuck::cast_slice(vertices);
            ctx.write_to_buffer_raw(&vertex_buffer, vertex_data);
        }

        let index_type: IndexType = indices.into();
        let indices_count = indices.len();
        let index_buffer = ctx.create_buffer(&BufferDescriptor {
            name: format!("{}-index-buffer", name).into(),
            size: (index_type.get_size() * indices.len()) as u64,
            memory_type: MemoryType::HostVisible,
            usage: BufferUsage::Index,
        });
        // Upload index data
        unsafe {
            let index_data: &[u8] = match indices {
                Indices::U16(data) => bytemuck::cast_slice(&data),
                Indices::U32(data) => bytemuck::cast_slice(&data),
            };
            ctx.write_to_buffer_raw(&index_buffer, index_data);
        }

        Self {
            vertex_buffer,
            index_buffer,
            index_type,
            indices_count: indices_count as _,
            material,
        }
    }
}

impl Renderable<ActiveContext> for MeshPart {
    fn render<Encoder: CommandEncoder<ActiveContext>>(&self, encoder: &mut Encoder) {
        encoder.bind_vertex_buffer(0, &self.vertex_buffer, BufferRange::WHOLE);
        encoder.bind_index_buffer(
            &self.index_buffer,
            BufferRange::WHOLE,
            self.index_type.clone(),
        );
        encoder.draw_indexed(0..self.indices_count, 0, 0..1);
    }
}

#[derive(Debug)]
pub struct Mesh {
    name: String,
    pub parts: Vec<MeshPart>,
}

impl Mesh {
    pub fn new(name: impl Into<String>, parts: Vec<MeshPart>) -> Self {
        Self {
            name: name.into(),
            parts,
        }
    }
}

// TODO: Drop mesh

#[derive(Debug)]
pub struct Model {
    pub meshes: Vec<(glam::Mat4, AssetHandle<Mesh>)>,
}

impl Model {
    pub fn new() -> Self {
        Self {
            meshes: Default::default(),
        }
    }

    pub fn add_mesh(&mut self, transform: glam::Mat4, mesh: AssetHandle<Mesh>) {
        self.meshes.push((transform, mesh));
    }
}

impl Default for Model {
    fn default() -> Self {
        Self::new()
    }
}
