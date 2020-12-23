use std::{
    collections::HashMap,
    mem::ManuallyDrop,
    sync::{
        atomic::{AtomicUsize, Ordering},
        Arc,
    },
};

use gfx_hal::{
    device::Device,
    pso::{
        Descriptor, DescriptorPool, DescriptorPoolCreateFlags, DescriptorRangeDesc,
        DescriptorSetWrite,
    },
    Backend,
};
use parking_lot::RwLock;

use render::resource::glue::{DescriptorWrite, Mixture};

use super::{
    compat::{get_descriptor_type, ToHalType},
    gfx_context::GfxContext,
};

#[derive(Debug)]
pub struct Lane<B: Backend> {
    handle: ManuallyDrop<B::DescriptorPool>,
    allocation_count: AtomicUsize,
}

#[derive(Debug)]
pub struct Pool<B: Backend> {
    device: Arc<B::Device>,
    counter: AtomicUsize,
    lanes: RwLock<HashMap<usize, Lane<B>>>,
}

pub type LayoutHandle<B> = (usize, <B as Backend>::DescriptorSetLayout);
pub type SetHandle<B> = (usize, <B as Backend>::DescriptorSet);

impl<B: Backend> Pool<B> {
    const DEFAULT_POOL_SIZE: usize = 32;

    pub fn new(device: Arc<B::Device>) -> Self {
        Self {
            device,
            counter: AtomicUsize::new(0),
            lanes: Default::default(),
        }
    }

    pub fn create_layout<I>(&self, parts: I) -> LayoutHandle<B>
    where
        I: IntoIterator<Item = render::resource::glue::MixturePart>,
    {
        let bindings: Vec<gfx_hal::pso::DescriptorSetLayoutBinding> =
            parts.into_iter().map(|p| p.convert()).collect();

        let immutable_samplers = Vec::new();

        let handle = unsafe {
            self.device
                .create_descriptor_set_layout(bindings.as_slice(), immutable_samplers.as_slice())
                .expect("[GfxContext] failed to create descriptor layout (from mixture parts)")
        };

        let index = self.counter.fetch_add(1, Ordering::SeqCst);

        (index, handle)
    }

    pub fn drop_layout(&self, handle: LayoutHandle<B>) {
        unsafe {
            self.device.destroy_descriptor_set_layout(handle.1);
        }
    }

    pub fn allocate_set(&self, layout: &Mixture<GfxContext<B>>) -> SetHandle<B> {
        let mut lanes = self.lanes.write();
        let layout_handle = &layout.gpu_layout.handle;
        match lanes.get_mut(&layout_handle.0) {
            Some(lane) => {
                if lane.allocation_count.load(Ordering::Relaxed) >= Self::DEFAULT_POOL_SIZE {
                    panic!("[Pool] pool for layout: {:#?} is full", layout_handle.0);
                }

                let set = unsafe {
                    lane.handle
                        .allocate_set(&layout_handle.1)
                        .expect("[Pool] failed to allocate set")
                };

                lane.allocation_count.fetch_add(1, Ordering::SeqCst);

                (layout_handle.0, set)
            }
            None => {
                let mut ranges: Vec<DescriptorRangeDesc> = Vec::new();
                layout.parts.iter().for_each(|part| {
                    let ty = get_descriptor_type(part);
                    match ranges.iter_mut().find(|r| r.ty == ty) {
                        Some(r) => {
                            r.count += Self::DEFAULT_POOL_SIZE;
                        }
                        None => {
                            ranges.push(DescriptorRangeDesc {
                                ty,
                                count: Self::DEFAULT_POOL_SIZE,
                            });
                        }
                    };
                });

                let mut pool = unsafe {
                    self.device
                        .create_descriptor_pool(
                            Self::DEFAULT_POOL_SIZE,
                            ranges.as_slice(),
                            DescriptorPoolCreateFlags::FREE_DESCRIPTOR_SET,
                        )
                        .expect("[Pool] failed to create new descriptor pool")
                };

                let set = unsafe {
                    pool.allocate_set(&layout_handle.1)
                        .expect("[Pool] failed to allocate initial set")
                };

                let lane: Lane<B> = Lane {
                    handle: ManuallyDrop::new(pool),
                    allocation_count: AtomicUsize::new(1),
                };

                lanes.insert(layout_handle.0, lane);

                (layout_handle.0, set)
            }
        }
    }

    pub fn free_set(&self, handle: SetHandle<B>) {
        let mut lanes = self.lanes.write();
        let lane = lanes
            .get_mut(&handle.0)
            .expect("[Pool] failed to find matching lane to allocation");

        unsafe {
            lane.handle.free(vec![handle.1]);
        };

        lane.allocation_count.fetch_sub(1, Ordering::SeqCst);
    }

    pub fn write_set(&self, handle: &SetHandle<B>, writes: Vec<DescriptorWrite<GfxContext<B>>>) {
        let writes: Vec<DescriptorSetWrite<'_, B, Vec<Descriptor<B>>>> = writes
            .into_iter()
            .map(|w| {
                let descriptor = match w.descriptor {
                    render::resource::glue::Descriptor::Buffer(buffer, range) => {
                        Descriptor::Buffer(&buffer.0, range.convert())
                    }
                };
                DescriptorSetWrite {
                    set: &handle.1,
                    binding: w.binding,
                    array_offset: w.array_offset,
                    descriptors: vec![descriptor],
                }
            })
            .collect();

        unsafe {
            self.device.write_descriptor_sets(writes);
        }
    }
}

impl<B: Backend> Drop for Pool<B> {
    fn drop(&mut self) {
        for (_idx, mut lane) in self.lanes.get_mut().drain() {
            unsafe {
                let hal_pool = ManuallyDrop::take(&mut lane.handle);
                self.device.destroy_descriptor_pool(hal_pool);
            };
        }
    }
}
