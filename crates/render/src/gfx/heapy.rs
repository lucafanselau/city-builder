use generational_arena::{Arena, Index};
use gfx_hal::{
    adapter::PhysicalDevice,
    device::Device,
    memory::{Properties, Requirements},
    Backend, MemoryTypeId,
};
use parking_lot::RwLock;
use std::ops::Range;
use std::sync::atomic::AtomicU64;
use std::{collections::HashMap, sync::Arc};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum MemoryType {
    DeviceLocal,
    HostVisible,
}

#[derive(Debug)]
struct PageInfo {
    id: MemoryTypeId,
    heap_size: u64,
    properties: Properties,
}

#[derive(Debug, Clone)]
struct Allocation {
    offset: u64,
    size: u64,
}

impl From<&Allocation> for Range<u64> {
    fn from(a: &Allocation) -> Self {
        a.offset..a.offset + a.size
    }
}

#[derive(Debug)]
struct MemoryPage<B: Backend> {
    memory_handle: B::Memory,
    allocations: Arena<Allocation>,
    size: u64,
}

impl<B: Backend> MemoryPage<B> {
    fn new(device: &Arc<B::Device>, memory_id: MemoryTypeId, size: u64) -> Self {
        let memory_handle = unsafe {
            match device.allocate_memory(memory_id, size) {
                Ok(m) => m,
                Err(e) => panic!(
                    "[Heapy] failed to allocate new memory page, for memory_type: {:?}: {:#?}",
                    memory_id, e
                ),
            }
        };
        Self {
            memory_handle,
            allocations: Arena::new(),
            size,
        }
    }

    fn is_compatible(first: Range<u64>, second: Range<u64>) -> bool {
        first.end < second.start || second.end < first.start
    }

    fn available_allocation(&self, size: u64) -> Option<u64> {
        // We will advance this head through all allocations
        let mut head = 0u64;
        let mut found_space = false;
        while !found_space {
            match self
                .allocations
                .iter()
                .find(|(_id, a)| Self::is_compatible((*a).into(), head..head + size))
            {
                Some((_, a)) => head = a.offset + a.size,
                None => found_space = true,
            }
            if head + size > self.size {
                break;
            };
        }
        if found_space {
            Some(head)
        } else {
            None
        }
    }

    fn has_space(&self, size: u64) -> bool {
        self.available_allocation(size).is_some()
    }

    fn allocate(&mut self, size: u64) -> Index {
        let offset = self
            .available_allocation(size)
            .expect("[Heapy] Page Memory mismatch");
        self.allocations.insert(Allocation { offset, size })
    }
}

#[derive(Copy, Clone)]
pub struct AllocationIndex(MemoryType, Index, Index);

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
        let (page_idx, allocation_idx) = match pages.iter_mut().find(|(_id, p)| p.has_space(size)) {
            Some((id, p)) => (id, p.allocate(size)),
            None => {
                let mut page = MemoryPage::<B>::new(
                    &self.device,
                    page_info.id,
                    BLOCK_SIZE
                        * self
                            .min_alignment
                            .load(std::sync::atomic::Ordering::Acquire),
                );
                let allocation_idx = page.allocate(size);
                let page_idx = pages.insert(page);
                (page_idx, allocation_idx)
            }
        };

        AllocationIndex(memory_type, page_idx, allocation_idx)
    }

    pub(crate) fn get_bind_data(&self, at: AllocationIndex) -> (&B::Memory, u64) {
        unimplemented!();
    }

    pub(crate) fn deallocate(&self, at: AllocationIndex) {
        unimplemented!();
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
