use std::{collections::HashMap, mem::MaybeUninit, sync::Arc};

use crate::{Gpu, Resource, ResourceId, ResourceStorage, VersionedIndexId};

#[derive(Clone, Copy)]
struct UsedBlock<Id: VersionedIndexId> {
    id: Id,
    offset: usize,
    size: usize,
    capacity: usize,
}

pub struct IdMappedResourceSliceStorage<Id: VersionedIndexId, R: Resource> {
    resources: Vec<MaybeUninit<R>>,
    used_blocks: HashMap<usize, UsedBlock<Id>>, // id.index -> UsedBlock
    free_blocks: HashMap<usize, usize>,         // offset -> size

    resource_id: ResourceId,
}

impl<Id: VersionedIndexId + 'static, R: Resource + 'static> IdMappedResourceSliceStorage<Id, R> {
    // const GROW_FACTOR: f32 = 1.5;

    pub fn new(gpus: &[Arc<Gpu>], resource_id: ResourceId) -> Self {
        // todo!();
        // let gpu_buffers = gpus.iter().map(|gpu| {
        //     let resource_buffer = gpu.device().create_buffer(&wgpu::BufferDescriptor {
        //         label: Some(&format!("{} array", R::label())),
        //         size: Self::INITIAL_BUFFER_SIZE,
        //         usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,
        //         mapped_at_creation: false,
        //     });
        //     let reverse_array = gpu.device().create_buffer(&wgpu::BufferDescriptor {
        //         label: Some(&format!("{} reverse array", R::label())),
        //         size: Self::INITIAL_BUFFER_SIZE,
        //         usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,
        //         mapped_at_creation: false,
        //     });

        //     return GpuResourceBuffer {
        //         reverse_array,
        //         resource_buffer,
        //         gpu: gpu.clone(),
        //     };
        // });

        return Self {
            free_blocks: HashMap::new(),
            used_blocks: HashMap::new(),
            resources: Vec::new(),
            resource_id,
        };
    }

    // fn get_block_slice(&self, block: UsedBlock<Id>) -> &[R] {
    //     unsafe { MaybeUninit::slice_assume_init_ref(&self.resources[block.offset..block.offset + block.size]) }
    // }

    // fn get_block_slice_mut(&mut self, block: UsedBlock<Id>) -> &mut [R] {
    //     unsafe { MaybeUninit::slice_assume_init_mut(&self.resources[block.offset..block.offset + block.size]) }
    // }

    fn allocate_block(&mut self, size: usize) -> UsedBlock<Id> {
        if let Some((offset, block_size)) = self.free_blocks.iter().find(|(_, block_size)| **block_size >= size) {
            let block = UsedBlock {
                id: Id::from_index_and_version(0, 0),
                offset: *offset,
                size,
                capacity: size,
            };
            let offset = *offset;
            let block_size = *block_size;
            self.free_blocks.remove(&offset);
            self.free_blocks.insert(offset + size, block_size - size);
            return block;
        } else {
            let block = UsedBlock {
                id: Id::from_index_and_version(0, 0),
                offset: self.resources.len(),
                size,
                capacity: size,
            };
            self.resources.resize_with(self.resources.len() + size, || MaybeUninit::uninit());
            return block;
        }
    }

    fn free_block(&mut self, block: &UsedBlock<Id>) {
        self.free_blocks.insert(block.offset, block.capacity);
        // TODO: merge with previous and next block if they are adjacent
    }

    // Return copy instead of reference because the borrow checker is not smart enough
    // https://github.com/rust-lang/rfcs/blob/master/text/2094-nll.md#problem-case-3-conditional-control-flow-across-functions
    fn reserve_for_index(&mut self, index: usize, additional_size: usize) -> UsedBlock<Id> {
        let block = self.used_blocks.get_mut(&index).unwrap();
        if block.capacity >= block.size + additional_size {
            return *block;
        }

        let adjacent_block_offset = block.offset + block.capacity;
        let additional_capacity_needed = block.size + additional_size - block.capacity;
        if let Some(adjacent_free_block_size) = self.free_blocks.get(&adjacent_block_offset).map(|v| *v) && adjacent_free_block_size >= additional_capacity_needed {
            self.free_blocks.remove(&adjacent_block_offset);
            block.capacity = block.capacity + additional_capacity_needed;
            if adjacent_free_block_size > additional_capacity_needed {
                self.free_blocks.insert(block.offset + block.capacity, adjacent_free_block_size - additional_capacity_needed);
            }
            return *block;
        } else {
            let block = *block;
            let new_block = self.allocate_block(block.size + additional_size);
            for i in 0..block.size {
                unsafe {
                    let value = self.resources[block.offset + i].assume_init_read();
                    self.resources[new_block.offset + i].write(value);
                }
            }
            self.free_block(&block);
            return new_block;
        }
    }

    // pub fn insert<I: Iterator<Item = R>>(&mut self, id: Id, resources: I, capacity: Option<usize>) {
    //     if let Some(block) = self.used_blocks.get(&id.index()) {
    //         let block = *block;
    //         self.free_block(&block);
    //     }

    //     let block = self.allocate_block(capacity.unwrap_or(1));
    //     self.used_blocks.insert(id.index(), block);
    //     for r in resources { self. push(id, r); }
    // }

    pub fn push(&mut self, id: Id, resource: R) {
        // self.used_blocks
    }

    // pub fn extend(&mut self, id: Id, resources: &[R]) {
    //     todo!();
    // }

    pub fn remove(&mut self, id: Id) {
        todo!();
    }

    pub fn get(&self, id: Id) -> Option<&[R]> {
        let block = self.used_blocks.get(&id.index())?;
        Some(unsafe { MaybeUninit::slice_assume_init_ref(&self.resources[block.offset..block.offset + block.size]) })
    }

    pub fn update_gpu_buffers(&self) {
        todo!();
        // for buffer in &self.gpu_buffers {
        //     let resource_buffer_slice = unsafe {
        //         std::slice::from_raw_parts(
        //             self.resources.as_ptr() as *const u8,
        //             self.resources.len() * std::mem::size_of::<R>(),
        //         )
        //     };
        //     buffer
        //         .gpu
        //         .queue()
        //         .write_buffer(&buffer.resource_buffer, 0, resource_buffer_slice);

        //     let reverse_array_slice = unsafe {
        //         std::slice::from_raw_parts(
        //             self.reverse_array.as_ptr() as *const u8,
        //             self.reverse_array.len() * std::mem::size_of::<Id>(),
        //         )
        //     };
        //     buffer
        //         .gpu
        //         .queue()
        //         .write_buffer(&buffer.reverse_array, 0, reverse_array_slice);
        // }
    }

    // pub fn iter(&self) -> IdMappedResourceStorageIterator<'_, Id, R> {
    //     todo!();
    // }

    pub fn factory(gpus: &[Arc<Gpu>], resource_id: ResourceId) -> Box<dyn ResourceStorage> {
        return Box::new(Self::new(gpus, resource_id));
    }
}


impl<Id: VersionedIndexId + 'static, R: Clone + Resource + 'static> IdMappedResourceSliceStorage<Id, R> {
    pub fn insert_slice(&mut self, id: Id, resources: &[R]) {
        if let Some(block) = self.used_blocks.get(&id.index()) {
            let block = *block;
            self.free_block(&block);
        }

        let mut block = self.allocate_block(resources.len());

        for (i, r) in resources.iter().enumerate() {
            self.resources[block.offset + i].write(r.clone());
        }

        block.id = id;
        block.size = resources.len();
        self.used_blocks.insert(id.index(), block);
    }

}

impl<Id: VersionedIndexId + 'static, R: Resource + 'static> ResourceStorage
    for IdMappedResourceSliceStorage<Id, R>
{
    fn bind_group_layout_entries(&self) -> Vec<wgpu::BindGroupLayoutEntry> {
        let base_binding: u32 = (4 * self.resource_id.index()).try_into().unwrap();
        return vec![
            wgpu::BindGroupLayoutEntry {
                binding: base_binding + 0,
                visibility: wgpu::ShaderStages::VERTEX_FRAGMENT,
                ty: wgpu::BindingType::Buffer {
                    ty: wgpu::BufferBindingType::Storage { read_only: true },
                    has_dynamic_offset: false,
                    min_binding_size: None,
                },
                count: None,
            },
            wgpu::BindGroupLayoutEntry {
                binding: base_binding + 1,
                visibility: wgpu::ShaderStages::VERTEX_FRAGMENT,
                ty: wgpu::BindingType::Buffer {
                    ty: wgpu::BufferBindingType::Storage { read_only: true },
                    has_dynamic_offset: false,
                    min_binding_size: None,
                },
                count: None,
            },
        ];
    }

    fn bind_group_entries(&self, gpu_index: usize) -> Vec<wgpu::BindGroupEntry> {
        todo!();
        // let base_binding: u32 = (4 * self.resource_id.index()).try_into().unwrap();
        // return vec![
        //     wgpu::BindGroupEntry {
        //         binding: base_binding + 0,
        //         resource: wgpu::BindingResource::Buffer(wgpu::BufferBinding {
        //             buffer: &self.gpu_buffers[gpu_index].resource_buffer,
        //             offset: 0,
        //             size: None,
        //         }),
        //     },
        //     wgpu::BindGroupEntry {
        //         binding: base_binding + 1,
        //         resource: wgpu::BindingResource::Buffer(wgpu::BufferBinding {
        //             buffer: &self.gpu_buffers[gpu_index].reverse_array,
        //             offset: 0,
        //             size: None,
        //         }),
        //     },
        // ];
    }
}

#[cfg(test)]
mod test {
    use crate::{Resource, EntityId, IdMappedResourceSliceStorage, ResourceId, ResourceKind};

    #[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
    struct TestResource {
        number: u32,
    }

    impl Resource for TestResource {
        type Type = Self;
        type Storage = IdMappedResourceSliceStorage<EntityId, Self>;

        fn id() -> ResourceId { ResourceId::from_index_and_version(0, 0) }
        fn kind() -> ResourceKind { ResourceKind::EntityComponent }
        fn label() -> &'static str { "TestResource" }
        fn register() { todo!() }
    }

    #[test]
    fn inserting_works() {
        let mut storage = <TestResource as Resource>::Storage::new(&[], <TestResource as Resource>::id());

        let mut resources = Vec::<TestResource>::new();
        for i in 0..10000000 {
            resources.push(TestResource {number: i});
        }
        assert_eq!(resources.len(), 10000000);

        let entity = EntityId::from_index_and_version(0, 0);
        storage.insert_slice(entity, &resources);

        let slice = storage.get(entity);
        assert!(slice.is_some());
        let slice = slice.unwrap();
        assert_eq!(slice, resources);
    }
}
