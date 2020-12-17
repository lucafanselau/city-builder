use crate::gfx::memory_page::MemoryPage;
use crate::resource::buffer::MemoryType;
use generational_arena::{Arena, Index};
use gfx_hal::{
    adapter::PhysicalDevice,
    device::Device,
    memory::{Properties, Requirements},
    Backend, MemoryTypeId,
};
use parking_lot::RwLock;
use std::ops::Deref;
use std::sync::atomic::AtomicU64;
use std::{collections::HashMap, sync::Arc};

#[derive(Debug)]
pub(crate) struct PageInfo {
    id: MemoryTypeId,
    heap_size: u64,
    properties: Properties,
}

#[derive(Debug)]
pub struct AllocationIndex {
    memory_type: MemoryType,
    page: Index,
    offset: u64,
}

const BLOCK_SIZE: u64 = 20;

/// Heapy will be our own little memory allocation utility
#[derive(Debug, Default)]
pub struct Heapy<B: Backend> {
    device: Arc<B::Device>,
    // And our memory pages
    // allocations: Arena<Allocation>,
    // TODO: This is horrible, we need a better multi-threading ready model here
    pages: RwLock<HashMap<MemoryType, (PageInfo, Arena<MemoryPage<B>>)>>,
    min_alignment: AtomicU64,
}

impl<B: Backend> Heapy<B> {
    pub(crate) fn new(device: Arc<B::Device>, physical_device: &B::PhysicalDevice) -> Self {
        let mut pages = HashMap::with_capacity(2);
        pages.insert(
            MemoryType::DeviceLocal,
            (
                Self::get_page_info(physical_device, Properties::DEVICE_LOCAL),
                Arena::<MemoryPage<B>>::new(),
            ),
        );
        pages.insert(
            MemoryType::HostVisible,
            (
                Self::get_page_info(
                    physical_device,
                    Properties::CPU_VISIBLE | Properties::COHERENT,
                ),
                Arena::<MemoryPage<B>>::new(),
            ),
        );

        let min_alignment = physical_device.limits().buffer_image_granularity;

        Self {
            device,
            // allocations: Arena::new(),
            pages: RwLock::new(pages),
            min_alignment: min_alignment.into(),
        }
    }

    pub(crate) fn alloc(
        &self,
        size: u64,
        memory_type: MemoryType,
        requirements: Option<Requirements>,
    ) -> AllocationIndex {
        // Size needs to be corrected in respect to the alignment requirements
        let min_alignment = self
            .min_alignment
            .load(std::sync::atomic::Ordering::Acquire);

        let size = round_up_to_nearest_multiple(size, min_alignment);

        // We will always need writing access to pages to allocate (maybe we can optimize here alot)
        let mut pages = self.pages.write();
        let (page_info, pages) = pages
            .get_mut(&memory_type)
            .expect("Memory Type uninitialized");

        if let Some(requirements) = requirements {
            assert_ne!(
                requirements.type_mask & (1_u32 << page_info.id.0),
                0,
                "[Heapy] Requirements for allocation could not be met"
            )
        }

        // otherwise we will try to find an available space in the pages
        let (page, offset) = unsafe {
            let mut result = None;
            for (id, p) in pages.iter_mut() {
                match p.allocations.try_alloc(size) {
                    Ok(offset) => {
                        result = Some((id, offset));
                        break;
                    }
                    _ => (),
                }
            }
            if result.is_none() {
                // We didn't find a suitable memory_page so create a new one
                let mut page = MemoryPage::<B>::new(
                    &self.device,
                    page_info.id,
                    std::cmp::max(BLOCK_SIZE * min_alignment, size),
                );
                // unwrap, because we just constructed a page with at least the size of size
                let offset = page.allocations.try_alloc(size).unwrap();
                let page_idx = pages.insert(page);
                (page_idx, offset)
            } else {
                result.unwrap()
            }
        };

        AllocationIndex {
            memory_type,
            page,
            offset,
        }
    }

    fn get_bind_data(&self, at: &AllocationIndex, mut cb: impl FnMut(&B::Memory, u64)) {
        let pages = self.pages.read();
        let (_page_info, pages) = pages
            .get(&at.memory_type)
            .expect("[Heapy] (get_bind_data) invalid index");
        let page = pages
            .get(at.page)
            .expect("[Heapy] (get_bind_data) invalid index");
        cb(page.memory_handle.deref(), at.offset);
    }

    pub(crate) fn bind_buffer(&self, at: &AllocationIndex, buffer: &mut B::Buffer) {
        self.get_bind_data(at, |memory, offset| unsafe {
            self.device
                .bind_buffer_memory(memory.deref(), offset, buffer)
                .expect("[Heapy] (bind_buffer) bind_memory failed with error");
        });
    }

    #[allow(dead_code)]
    pub(crate) fn bind_image(&self, at: &AllocationIndex, image: &mut B::Image) {
        self.get_bind_data(at, |memory, offset| unsafe {
            self.device
                .bind_image_memory(memory.deref(), offset, image)
                .expect("[Heapy] (bind_image) bind_memory failed with error");
        });
    }

    pub(crate) fn deallocate(&self, at: AllocationIndex) {
        let mut pages = self.pages.write();
        let result = (move || {
            let (_page_info, pages) = pages.get_mut(&at.memory_type)?;
            let page = pages.get_mut(at.page)?;
            page.allocations.dealloc(at.offset);

            Some(())
        })();
        if let None = result {
            panic!("[Heapy] (deallocate) failed, probably because \"at\" was invalid")
        }
    }

    /// Safety: Will panic if allocation is not HostVisible
    pub(crate) unsafe fn write(&self, at: &AllocationIndex, data: &[u8]) {
        if at.memory_type != MemoryType::HostVisible {
            panic!("[Heapy] (write) tried to map un-mappable memory");
        }
        let pages = self.pages.write();
        let (_, pages) = pages
            .get(&at.memory_type)
            .expect("[Heapy] (write) invalid index");
        let page = pages.get(at.page).expect("[Heapy] (write) invalid index");
        let allocation = page
            .allocations
            .allocations
            .iter()
            .find(|a| a.offset == at.offset)
            .expect("[Heapy] (write) invalid index");
        let data_length = data.len();
        if (allocation.size as usize) < data_length {
            panic!(
                "[Heapy] (write) data is larger than buffer {} vs. {}",
                allocation.size, data_length
            );
        }
        // Map that with a device
        use gfx_hal::memory::Segment;
        let dst = self
            .device
            .map_memory(
                page.memory_handle.deref(),
                Segment {
                    offset: allocation.offset,
                    size: Some(allocation.size),
                },
            )
            .expect("[Heapy] (write) map_memory failed");

        std::ptr::copy_nonoverlapping(data.as_ptr(), dst, data_length);

        // TODO: Maybe flush here, but in heapy we request coherent memory -> no flushing (al my guess)
        // https://stackoverflow.com/questions/36241009/what-is-coherent-memory-on-gpu

        // unmap memory again
        self.device.unmap_memory(page.memory_handle.deref());
    }

    fn get_page_info(device: &B::PhysicalDevice, props: Properties) -> PageInfo {
        let memory_properties = device.memory_properties();

        let mem_info = memory_properties
            .memory_types
            .iter()
            .enumerate()
            .find(|(_id, mem_type)| mem_type.properties.contains(props))
            .map(|(id, mem_type)| {
                (
                    MemoryTypeId(id),
                    mem_type.heap_index,
                    mem_type.properties.clone(),
                )
            });

        match mem_info {
            Some((id, heap_index, properties)) => PageInfo {
                id,
                heap_size: *memory_properties
                    .memory_heaps
                    .get(heap_index)
                    .expect("[Heapy] (internal) wrong heap index?"),
                properties,
            },
            None => panic!(
                "[Heapy] failed to find memory type for properties: {:#?}",
                props
            ),
        }
    }
}

impl<B: Backend> Drop for Heapy<B> {
    fn drop(&mut self) {
        // We need to drop all memory pages
        let mut pages = self.pages.write();
        for (_k, (_info, mut pages)) in pages.drain() {
            for (_id, mut memory_page) in pages.drain() {
                memory_page.free(&self.device)
            }
        }
    }
}

// util: helper function for mathy shit
fn round_up_to_nearest_multiple(value: u64, multiple_of: u64) -> u64 {
    (value + multiple_of - 1) & !(multiple_of - 1)
}

#[cfg(test)]
mod tests {
    use super::round_up_to_nearest_multiple;

    #[test]
    fn test_rounding() {
        assert_eq!(round_up_to_nearest_multiple(15, 8), 16);
        assert_eq!(round_up_to_nearest_multiple(0, 8), 0);
        assert_eq!(round_up_to_nearest_multiple(23, 8), 24);
        assert_eq!(round_up_to_nearest_multiple(9, 8), 16);
        assert_eq!(round_up_to_nearest_multiple(157, 8), 160);
    }
}
