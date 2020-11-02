use gfx_hal::{device::Device, Backend, MemoryTypeId};
use std::cmp::Ordering;
use std::mem::ManuallyDrop;
use std::ops::Range;
use std::sync::Arc;
use thiserror::Error;

#[derive(Debug)]
pub(crate) struct MemoryPage<B: Backend> {
    pub(crate) memory_handle: ManuallyDrop<B::Memory>,
    pub(crate) allocations: Allocations,
}

impl<B: Backend> MemoryPage<B> {
    pub(crate) fn new(device: &Arc<B::Device>, memory_id: MemoryTypeId, size: u64) -> Self {
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
            memory_handle: ManuallyDrop::new(memory_handle),
            allocations: Allocations::new(size),
        }
    }

    pub(crate) fn free(&mut self, device: &Arc<B::Device>) {
        unsafe {
            let mem = ManuallyDrop::take(&mut self.memory_handle);
            device.free_memory(mem);
        }
    }
}

#[derive(Debug, Clone)]
struct Allocation {
    offset: u64,
    size: u64,
}

// Trait implementations for Allocation
impl From<&Allocation> for Range<u64> {
    fn from(a: &Allocation) -> Self {
        a.offset..a.offset + a.size
    }
}
impl PartialEq for Allocation {
    fn eq(&self, other: &Self) -> bool {
        self.offset.eq(&other.offset)
    }
}
impl Eq for Allocation {}
impl PartialOrd for Allocation {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        self.offset.partial_cmp(&other.offset)
    }
}
impl Ord for Allocation {
    fn cmp(&self, other: &Self) -> Ordering {
        self.offset.cmp(&other.offset)
    }
}

#[derive(Debug, Error)]
pub(crate) enum AllocationError {
    #[error("This Memory Page has not enough available coherent storage space")]
    OutOfMemory,
}

/// struct that keeps track of a size and corresponding allocations
#[derive(Debug)]
pub(crate) struct Allocations {
    size: u64,
    allocations: Vec<Allocation>,
}

impl Allocations {
    pub(crate) fn new(size: u64) -> Self {
        Self {
            size,
            allocations: Vec::new(),
        }
    }

    fn is_compatible(first: Range<u64>, second: Range<u64>) -> bool {
        first.end - 1 < second.start || second.end - 1 < first.start
    }

    /// Safety: allocation_size needs to be aligned to meet gfx_hal expectations
    /// it is not the job of this abstraction to keep track of that
    pub(crate) unsafe fn try_alloc(
        &mut self,
        allocation_size: u64,
    ) -> Result<u64, AllocationError> {
        // This is essentially the resulting offset
        let mut head = 0u64;
        // Expect that allocations is sorted
        debug_assert!(self.allocations.is_sorted());
        // And every allocation is compatible
        debug_assert!(self.allocations.iter().all(|a| {
            self.allocations
                .iter()
                .all(|b| b.eq(a) || Self::is_compatible(a.into(), b.into()))
        }));
        for a in self.allocations.iter() {
            if !Self::is_compatible(head..head + allocation_size, a.into()) {
                // meaning a and the possible new allocation are incompatible
                // since self.allocations is sorted by offsets, this means we need to inc. to the end of a
                head = a.offset + a.size
            }
        }
        if head + allocation_size <= self.size {
            let allocation = Allocation {
                offset: head,
                size: allocation_size,
            };
            let pos = self.allocations.binary_search(&allocation).err().unwrap();
            self.allocations.insert(pos, allocation);
            // Expect that allocations is sorted
            debug_assert!(self.allocations.is_sorted());
            // And every allocation is compatible
            debug_assert!(self.allocations.iter().all(|a| {
                self.allocations
                    .iter()
                    .all(|b| b.eq(a) || Self::is_compatible(a.into(), b.into()))
            }));
            Ok(head)
        } else {
            Err(AllocationError::OutOfMemory)
        }
    }

    pub(crate) fn dealloc(&mut self, at_offset: u64) {
        if let Some(position) = self.allocations.iter().position(|a| a.offset == at_offset) {
            // this cannot destroy the sorting of the allocations array
            self.allocations.remove(position);
        } else {
            panic!(
                "[Allocations] (dealloc) invalid offset was passed {}",
                at_offset
            );
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn al(size: u64, offset: u64) -> Allocation {
        Allocation { size, offset }
    }

    #[test]
    fn alloc() {
        let mut a = Allocations::new(24);
        let _first = unsafe { a.try_alloc(4) }.expect("failed first");
        let second = unsafe { a.try_alloc(8) }.expect("failed second");
        let _third = unsafe { a.try_alloc(4) }.expect("failed third");
        let _fourth = unsafe { a.try_alloc(8) }.expect("failed fourth");
        assert_eq!(
            a.allocations,
            vec![al(4, 0), al(8, 4), al(4, 12), al(8, 16)]
        );
        a.dealloc(second);
        assert_eq!(a.allocations, vec![al(4, 0), al(4, 12), al(8, 16)]);
    }
}

/*
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
 */
