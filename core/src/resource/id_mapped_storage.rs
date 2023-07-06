use std::{mem::MaybeUninit, sync::Arc};

use erased_serde::Deserializer;

use crate::{ResourceId, VersionedIndexId, Resource, ResourceStorage, Gpu, BindingProvider, EntityComponentResourceStorage, EntityId};

use super::GpuResourceBuffer;

pub struct IdMappedResourceStorage<Id: VersionedIndexId, R: Resource> {
    // Stores all the resources. Note: not all slots contain valid resources for indices.
    // If a resource is removed, it just gets marked as "free", so the list may contain holes.
    // The locations of these holes are stored in a linked list. The first item in the list is
    // stored in m_free_list_head which points to the first free slot. The next slot can be then
    // looked up by indexing the m_forward_array with the first item index.
    //
    // Example:
    //
    //                  0 1 2 3 4 5 6 7 8 9
    //
    // resource:       [C C X C X X C]
    // forward_array:  [0 5 5 3 X 4 8]
    // reverse_array:  [0 X X 3 X 1 X X 5 X]
    // free_list_head: 2
    //
    // reverse array stores the resource index for each id. The vector is indexed by index(id)
    // and returns a resource index. If is_emplaced of the resource index is 1, the index
    // provides information at which position in the resource array we have to look. if is_emplaced
    // is 0 there is no associated resource for that id (indicated as an X). The resources not
    // associated with any id are also indicated by an X in the resource array. This can be caused
    // by removing a resource. The free spots in the resource array can be found by traversing
    // a linked list. The first free index of the list is stored in free_list_head. The next free slot
    // index can be found by looking up the value at the position of the previous free index in the
    // forward_array. If the array contains 0xffffffff we are at the end of the list. For each slot
    // that is not free, the forward_array stores the associated id of the resource.
    resources: Vec<MaybeUninit<R>>,
    gpu_buffers: Vec<GpuResourceBuffer>,
    forward_array: Vec<Id>,
    reverse_array: Vec<Id>, // Here id gets a little abused. Index refers to the actual index and version stores a "boolean" if the id has this resource.
    free_list_head: usize,
    resource_id: ResourceId,
}

impl<Id: VersionedIndexId + 'static, R: Resource + 'static> BindingProvider
    for IdMappedResourceStorage<Id, R>
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
        let base_binding: u32 = (4 * self.resource_id.index()).try_into().unwrap();
        return vec![
            wgpu::BindGroupEntry {
                binding: base_binding + 0,
                resource: wgpu::BindingResource::Buffer(wgpu::BufferBinding {
                    buffer: &self.gpu_buffers[gpu_index].resource_buffer,
                    offset: 0,
                    size: None,
                }),
            },
            wgpu::BindGroupEntry {
                binding: base_binding + 1,
                resource: wgpu::BindingResource::Buffer(wgpu::BufferBinding {
                    buffer: &self.gpu_buffers[gpu_index].reverse_array,
                    offset: 0,
                    size: None,
                }),
            },
        ];
    }
}

impl<R: Resource> EntityComponentResourceStorage for IdMappedResourceStorage<EntityId, R> {
    fn insert_default(&mut self, entity_id: EntityId) {
        self.insert(entity_id, R::default());
    }

    fn insert_serialized(&mut self, entity_id: EntityId, d: &mut dyn Deserializer) -> erased_serde::Result<()> {
        self.insert(entity_id, erased_serde::deserialize(d)?);
        return Ok(());
    }
}

impl<Id: VersionedIndexId + 'static, R: Resource + 'static> IdMappedResourceStorage<Id, R> {
    const FREE_LIST_END: usize = Id::MAX_VERSION;
    const INITIAL_BUFFER_SIZE: u64 = 1024;

    pub fn new(gpus: &[Arc<Gpu>], resource_id: ResourceId) -> Self {
        let gpu_buffers = gpus.iter().map(|gpu| {
            let resource_buffer = gpu.device().create_buffer(&wgpu::BufferDescriptor {
                label: Some(&format!("{} array", R::label())),
                size: Self::INITIAL_BUFFER_SIZE,
                usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,
                mapped_at_creation: false,
            });
            let reverse_array = gpu.device().create_buffer(&wgpu::BufferDescriptor {
                label: Some(&format!("{} reverse array", R::label())),
                size: Self::INITIAL_BUFFER_SIZE,
                usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,
                mapped_at_creation: false,
            });

            return GpuResourceBuffer {
                reverse_array,
                resource_buffer,
                gpu: gpu.clone(),
            };
        });

        return Self {
            resources: vec![],
            forward_array: vec![],
            reverse_array: vec![],
            free_list_head: Self::FREE_LIST_END,
            gpu_buffers: gpu_buffers.collect(),
            resource_id,
        };
    }

    pub fn insert(&mut self, id: Id, resource: R) -> Option<R> {
        if id.index() >= self.reverse_array.len() {
            self.reverse_array
                .resize_with(id.index() + 1, || Id::from_index_and_version(0, 0));
        }

        let reverse_ref = &mut self.reverse_array[id.index()];

        return if reverse_ref.version() == 0 {
            if self.free_list_head == Self::FREE_LIST_END {
                debug_assert!(self.forward_array.len() == self.resources.len());
                *reverse_ref = Id::from_index_and_version(self.forward_array.len(), 1);
                self.forward_array.push(id);
                self.resources.push(MaybeUninit::new(resource));
            } else {
                let insert_index = self.free_list_head;
                self.free_list_head = self.forward_array[self.free_list_head].index();
                self.resources[insert_index].write(resource);
                self.forward_array[insert_index] = id;
                *reverse_ref = Id::from_index_and_version(insert_index, 1);
            }

            None
        } else {
            let forward_index = reverse_ref.index();
            let result = unsafe { Some(self.resources[forward_index].assume_init_read()) };
            self.resources[forward_index].write(resource);
            result
        };
    }

    pub fn remove(&mut self, id: Id) -> Option<R> {
        if id.index() >= self.reverse_array.len() {
            return None;
        }
        let reverse_ref = &mut self.reverse_array[id.index()];

        if reverse_ref.version() == 0 {
            return None;
        }

        let index = reverse_ref.index();
        self.forward_array[index] = Id::from_index_and_version(self.free_list_head, 0);
        self.free_list_head = index;
        *reverse_ref = Id::from_index_and_version(reverse_ref.index(), 0);
        return Some(unsafe { self.resources[index].assume_init_read() });
    }

    pub fn get(&self, id: Id) -> Option<&R> {
        return if id.index() < self.reverse_array.len() {
            let reverse = self.reverse_array[id.index()];
            if reverse.version() == 1 {
                Some(unsafe { self.resources[reverse.index()].assume_init_ref() })
            } else {
                None
            }
        } else {
            None
        };
    }

    pub fn update_gpu_buffers(&self) {
        for buffer in &self.gpu_buffers {
            let resource_buffer_slice = unsafe {
                std::slice::from_raw_parts(
                    self.resources.as_ptr() as *const u8,
                    self.resources.len() * std::mem::size_of::<R>(),
                )
            };
            buffer
                .gpu
                .queue()
                .write_buffer(&buffer.resource_buffer, 0, resource_buffer_slice);

            let reverse_array_slice = unsafe {
                std::slice::from_raw_parts(
                    self.reverse_array.as_ptr() as *const u8,
                    self.reverse_array.len() * std::mem::size_of::<Id>(),
                )
            };
            buffer
                .gpu
                .queue()
                .write_buffer(&buffer.reverse_array, 0, reverse_array_slice);
        }
    }

    pub fn iter(&self) -> IdMappedResourceStorageIterator<'_, Id, R> {
        return IdMappedResourceStorageIterator::new(self);
    }

    // pub fn factory(gpus: &[Arc<Gpu>], resource_id: ResourceId) -> Box<dyn ResourceStorage> {
    //     return Box::new(Self::new(gpus, resource_id));
    // }
}

pub struct IdMappedResourceStorageIterator<
    'a,
    Id: VersionedIndexId + 'static,
    R: Resource + 'static,
> {
    storage: &'a IdMappedResourceStorage<Id, R>,
    index: Option<usize>,
}

impl<'a, Id: VersionedIndexId + 'static, R: Resource + 'static>
    IdMappedResourceStorageIterator<'a, Id, R>
{
    fn new(storage: &'a IdMappedResourceStorage<Id, R>) -> Self {
        return Self {
            storage,
            index: Self::increment_to_valid_index(0, storage),
        };
    }

    fn increment_to_valid_index(
        start: usize,
        storage: &'a IdMappedResourceStorage<Id, R>,
    ) -> Option<usize> {
        // TODO: remove recursion
        if start >= storage.forward_array.len() {
            return None;
        } else if storage.forward_array[start].index() == start {
            return Some(start);
        } else {
            return Self::increment_to_valid_index(start + 1, storage);
        }
    }
}

impl<'a, Id: VersionedIndexId + 'static, R: Resource + 'static> Iterator
    for IdMappedResourceStorageIterator<'a, Id, R>
{
    type Item = (Id, &'a R);

    fn next(&mut self) -> Option<Self::Item> {
        match self.index {
            Some(index) => {
                self.index = Self::increment_to_valid_index(index + 1, self.storage);
                return Some((self.storage.forward_array[index], unsafe {
                    self.storage.resources[index].assume_init_ref()
                }));
            }
            None => None,
        }
    }
}
