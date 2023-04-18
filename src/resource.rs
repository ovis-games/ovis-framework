use std::mem::MaybeUninit;

use crate::{StandardVersionedIndexId, VersionedIndexId};

pub enum ResourceKind {
    Event,
    SceneComponent,
    EntityComponent,
    ViewportComponent,
}

type ResourceId = StandardVersionedIndexId<8>;

pub trait Resource {
    fn kind(&self) -> ResourceKind;
}

pub trait Event : Resource {
    fn kind(&self) -> ResourceKind {
        return ResourceKind::Event;
    }
}

pub struct SimpleResourceStorage<R: Resource> {
    value: Option<R>,
    changed: bool,
}

// pub struct SimpleResourceListStorage<R: Resource> {
//     resources
// }

// TODO: split this into two arrays for alignment purposes
pub struct StoredEvent<E: Event> {
    event: E,
    handled: bool,
}

pub struct EventStorage<E: Event> {
    events: Vec<StoredEvent<E>>,
}

pub struct IdMappedResourceStorage<Id: VersionedIndexId, R: Resource> {
    resources: Vec<MaybeUninit<R>>,
    forward_array: Vec<Id>,
    reverse_array: Vec<u32>,
}

impl<Id: VersionedIndexId, R: Resource> IdMappedResourceStorage<Id, R> {
    // fn test(&self) {
    //     // self.resources.as_ptr_range
    // }
}
