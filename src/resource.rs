use std::{any::Any, mem::MaybeUninit};

use crate::{EntityId, StandardVersionedIndexId, VersionedIndexId};

pub enum ResourceKind {
    Event,
    SceneComponent,
    EntityComponent,
    ViewportComponent,
}

pub type ResourceId = StandardVersionedIndexId<8>;

pub trait Resource: Send + Sync {}

// pub trait Event: Resource {
//     fn kind(&self) -> ResourceKind {
//         return ResourceKind::Event;
//     }
// }

// pub trait EntityComponent: Resource {
//     fn kind(&self) -> ResourceKind {
//         return ResourceKind::EntityComponent;
//     }
// }

pub trait ResourceStorage: Send + Sync + Any {}

// pub struct SimpleResourceStorage<R> {
//     value: Option<R>,
//     changed: bool,
// }

// // TODO: split this into two arrays for alignment purposes
// pub struct StoredEvent<E> {
//     event: E,
//     handled: bool,
// }

// pub struct EventStorage<E> {
//     events: Vec<StoredEvent<E>>,
// }

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
    // resource:      [C C X C X X C]
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
    forward_array: Vec<Id>,
    reverse_array: Vec<Id>, // Here id gets a little abused. Index refers to the actual index and
    // version stores a "boolean" the id has this resource.
    free_list_head: usize,
}

impl<Id: VersionedIndexId + 'static, R: Resource + 'static> ResourceStorage
    for IdMappedResourceStorage<Id, R>
{
}

impl<Id: VersionedIndexId + 'static, R: Resource + 'static> IdMappedResourceStorage<Id, R> {
    const FREE_LIST_END: usize = Id::MAX_VERSION;

    pub fn new() -> Self {
        return Self {
            resources: vec![],
            forward_array: vec![],
            reverse_array: vec![],
            free_list_head: Self::FREE_LIST_END,
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

    pub fn iter(&self) -> IdMappedResourceStorageIterator<'_, Id, R> {
        return IdMappedResourceStorageIterator::new(self);
    }

    pub fn factory() -> Box<dyn ResourceStorage> {
        return Box::new(Self::new());
    }
}

// impl<Id: VersionedIndexId + 'static, R: Resource + 'static> Drop for IdMappedResourceStorage<Id, R> {
//     fn drop(&mut self) {
//         for id in &self.ids {
//             unsafe {
//                 self.values[id.index()].assume_init_drop();
//             }
//         }
//     }
// }

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

// impl<'a, Id: VersionedIndexId + 'static, R: Resource + 'static> Iterator
//     for &'a IdMappedResourceStorage<Id, R>
// {
//     type Item = (Id, &'a R);
//     type IntoIter = IdMappedResourceStorageIterator<'a, Id, R>;

//     fn into_iter(self) -> Self::IntoIter {
//         return Self::IntoIter::new(self);
//     }
// }

mod test {
    use super::*;
    use std::sync::Arc;

    #[derive(Debug)]
    struct R(Arc<u32>);

    impl Resource for R {}

    #[test]
    fn test() {
        type Id = StandardVersionedIndexId;
        let mut resource_storage = IdMappedResourceStorage::<Id, R>::new();

        let id = Id::from_index(0);
        assert!(resource_storage.insert(id, R(Arc::new(100))).is_none());

        let recv = resource_storage.get(id);
        assert!(recv.is_some());
        assert_eq!(*recv.unwrap().0, 100);

        let recv = resource_storage.insert(id, R(Arc::new(200)));
        assert!(recv.is_some());
        assert_eq!(*recv.unwrap().0, 100);

        let recv = resource_storage.remove(id);
        assert!(recv.is_some());
        assert_eq!(*recv.unwrap().0, 200);

        let recv = resource_storage.get(id);
        assert!(recv.is_none());
    }
}
