use bincode::deserialize;
use gfx_hal;
use gfx_hal::device::Device;
use serde::Deserialize;
use std::{error::Error, mem::ManuallyDrop, sync::Arc};

#[derive(serde::Deserialize)]
#[repr(C)]
pub struct Vertex {
    position: [f32; 3],
    normal: [f32; 3],
}

#[derive(Debug)]
pub struct Mesh<B: gfx_hal::Backend> {
    device: Arc<B::Device>,
    pub vertex_buffer: ManuallyDrop<B::Buffer>,
    pub vertex_memory: ManuallyDrop<B::Memory>,
    pub vertex_length: u32,
}

impl<B: gfx_hal::Backend> Drop for Mesh<B> {
    fn drop(&mut self) {
        unsafe {
            self.device
                .destroy_buffer(ManuallyDrop::take(&mut self.vertex_buffer));
            self.device
                .free_memory(ManuallyDrop::take(&mut self.vertex_memory));
        }
    }
}

pub type MeshError = Box<dyn Error>;

pub fn load_teapot() -> Result<Vec<Vertex>, MeshError> {
    let binary_mesh_data = include_bytes!("../../assets/meshes/teapot_mesh.bin");

    Ok(deserialize(binary_mesh_data).expect("failed to load teapot"))
}

unsafe fn make_buffer<B: gfx_hal::Backend>(
    device: &B::Device,
    physical_device: &B::PhysicalDevice,
    buffer_len: usize,
    usage: gfx_hal::buffer::Usage,
    properties: gfx_hal::memory::Properties,
) -> Result<(B::Memory, B::Buffer), MeshError> {
    use gfx_hal::{adapter::PhysicalDevice, MemoryTypeId};

    let mut buffer = device.create_buffer(buffer_len as u64, usage)?;

    let req = device.get_buffer_requirements(&buffer);

    let memory_types = physical_device.memory_properties().memory_types;

    let memory_type = memory_types
        .iter()
        .enumerate()
        .find(|(id, mem_type)| {
            let type_supported = req.type_mask & (1_u64 << id) != 0;
            type_supported && mem_type.properties.contains(properties)
        })
        .map(|(id, _ty)| MemoryTypeId(id))
        .expect("did not find memory type");

    let buffer_memory = device.allocate_memory(memory_type, req.size)?;

    device.bind_buffer_memory(&buffer_memory, 0, &mut buffer)?;

    Ok((buffer_memory, buffer))
}

pub fn create_mesh<B: gfx_hal::Backend>(
    device: Arc<B::Device>,
    adapter: &gfx_hal::adapter::Adapter<B>,
) -> Result<Mesh<B>, MeshError> {
    let teapot = load_teapot()?;

    let vertex_buffer_len = teapot.len() * std::mem::size_of::<Vertex>();

    let (vertex_buffer_memory, vertex_buffer) = unsafe {
        use gfx_hal::buffer::Usage;
        use gfx_hal::memory::Properties;

        let result = make_buffer::<B>(
            &device,
            &adapter.physical_device,
            vertex_buffer_len,
            Usage::VERTEX,
            Properties::CPU_VISIBLE,
        )?;

        Ok::<(B::Memory, B::Buffer), MeshError>(result)
    }?;

    unsafe {
        use gfx_hal::memory::Segment;

        let mapped_memory = device
            .map_memory(&vertex_buffer_memory, Segment::ALL)
            .expect("TODO");

        std::ptr::copy_nonoverlapping(teapot.as_ptr() as *const u8, mapped_memory, vertex_buffer_len);

        device
            .flush_mapped_memory_ranges(vec![(&vertex_buffer_memory, Segment::ALL)])
            .expect("TODO");

        device.unmap_memory(&vertex_buffer_memory);
    }

    Ok(Mesh {
        device,
        vertex_buffer: ManuallyDrop::new(vertex_buffer),
        vertex_memory: ManuallyDrop::new(vertex_buffer_memory),
        vertex_length: teapot.len() as u32,
    })
}
