use crate::gfx::memory_page::MemoryPage;
use generational_arena::{Arena, Index};
use gfx_hal::{
    adapter::PhysicalDevice,
    device::Device,
    memory::{Properties, Requirements},
    Backend, MemoryTypeId,
};
use owning_ref::RwLockReadGuardRef;
use raw_window_handle::RawWindowHandle;
use std::ops::{Deref, Range};
use std::sync::atomic::AtomicU64;
use std::sync::RwLock;
use std::{collections::HashMap, sync::Arc};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum MemoryType {
    DeviceLocal,
    HostVisible,
}

#[derive(Debug)]
pub(crate) struct PageInfo {
    id: MemoryTypeId,
    heap_size: u64,
    properties: Properties,
}

#[derive(Copy, Clone)]
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
    pages: RwLock<HashMap<MemoryType, (PageInfo, Arena<MemoryPage<B>>)>>,
    min_alignment: AtomicU64,
}

type MemoryBindingRef<'a, B: Backend> =
    RwLockReadGuardRef<'a, HashMap<MemoryType, (PageInfo, Arena<MemoryPage<B>>)>, B::Memory>;

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
        let mut pages = self
            .pages
            .write()
            .expect("[Heapy] (allocate) failed to acquire lock");
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

    fn get_bind_data(&self, at: AllocationIndex) -> Option<(MemoryBindingRef<B>, u64)> {
        let memory = RwLockReadGuardRef::new(self.pages.read().expect("")).map(|pages| {
            let (_page_info, pages) = pages.get(&at.memory_type).expect("");
            let page = pages.get(at.page).expect("");
            page.memory_handle.deref()
        });
        Some((memory, at.offset))
    }

    pub(crate) fn bind_buffer(&self, at: AllocationIndex, buffer: &mut B::Buffer) {
        if let Some((memory, offset)) = self.get_bind_data(at) {
            unsafe {
                if let Err(e) = self
                    .device
                    .bind_buffer_memory(memory.deref(), offset, buffer)
                {
                    panic!(
                        "[Heapy] (bind_buffer) bind_memory failed with error: {:#?}",
                        e
                    );
                }
            }
        } else {
            panic!("[Heapy] (bind_buffer) invalid allocation index");
        }
    }

    pub(crate) fn bind_image(&self, at: AllocationIndex, image: &mut B::Image) {
        if let Some((memory, offset)) = self.get_bind_data(at) {
            unsafe {
                if let Err(e) = self.device.bind_image_memory(memory.deref(), offset, image) {
                    panic!(
                        "[Heapy] (bind_image) bind_memory failed with error: {:#?}",
                        e
                    );
                }
            }
        } else {
            panic!("[Heapy] (bind_image) invalid allocation index");
        }
    }

    pub(crate) fn deallocate(&self, at: AllocationIndex) {
        let mut pages = self.pages.write().unwrap();
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

    fn get_page_info(device: &B::PhysicalDevice, props: Properties) -> PageInfo {
        let memory_properties = device.memory_properties();

        let mem_info = memory_properties
            .memory_types
            .iter()
            .enumerate()
            .find(|(id, mem_type)| mem_type.properties.contains(props))
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
        let mut pages = self.pages.write().unwrap();
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
