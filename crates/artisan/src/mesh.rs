use bytemuck::{Pod, Zeroable};
use render::{
    prelude::{BufferUsage, GpuContext, MemoryType},
    resource::{
        buffer::BufferDescriptor,
        pipeline::{
            AttributeDescriptor, VertexAttributeFormat, VertexBufferDescriptor, VertexInputRate,
        },
    },
};
use std::{borrow::Cow, collections::HashMap, sync::Arc};
use uuid::Uuid;

use crate::renderer::ActiveContext;

#[derive(Copy, Clone, Zeroable, Pod)]
#[repr(C)]
pub struct Vertex {
    pub pos: glam::Vec3,
    pub normal: glam::Vec3,
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

pub type MeshId = Uuid;

pub struct Mesh<Context: GpuContext> {
    handle: <Context as GpuContext>::BufferHandle,
    name: Cow<'static, str>,
    vertex_count: u32,
}

impl<Context: GpuContext> Mesh<Context> {
    /// Get a reference to the mesh's name.
    pub fn name(&self) -> &Cow<'static, str> {
        &self.name
    }
}

pub struct MeshMap {
    ctx: Arc<ActiveContext>,
    data: HashMap<MeshId, Mesh<ActiveContext>>,
}

impl MeshMap {
    pub fn new(ctx: Arc<ActiveContext>) -> Self {
        Self {
            ctx,
            data: HashMap::new(),
        }
    }

    pub fn load_mesh<N>(&mut self, name: N, data: Vec<Vertex>) -> MeshId
    where
        N: Into<Cow<'static, str>>,
    {
        let id = Uuid::new_v4();
        let name = name.into();

        let vertex_size = std::mem::size_of::<Vertex>();

        let handle = self.ctx.create_buffer(&BufferDescriptor {
            name: name.clone(),
            size: (vertex_size * data.len()) as u64,
            // NOTE: should be upgraded to device local memory (but i dont give a s*** right now)
            memory_type: MemoryType::HostVisible,
            usage: BufferUsage::Vertex,
        });

        unsafe {
            let buffer: &[u8] = bytemuck::cast_slice(data.as_slice());
            self.ctx.write_to_buffer_raw(&handle, buffer);
        }

        let mesh = Mesh {
            handle,
            name,
            vertex_count: data.len() as _,
        };
        self.data.insert(id, mesh);
        id
    }

    pub(crate) fn draw_info(
        &self,
        id: &MeshId,
    ) -> (u32, &<ActiveContext as GpuContext>::BufferHandle) {
        let mesh = self
            .data
            .get(id)
            .expect("[MeshMap] (draw_info) invalid mesh id");

        (mesh.vertex_count, &mesh.handle)
    }
}
