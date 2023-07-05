use crate::{EntityId, Gpu, IdMap, Instance, StandardVersionedIndexId, VersionedIndexId};
use lazy_static::lazy_static;
use std::{
    any::Any,
    sync::{Arc, RwLock},
};

pub enum ResourceKind {
    Event,
    SceneComponent,
    EntityComponent,
    ViewportComponent,
}

pub type ResourceId = StandardVersionedIndexId<8>;

pub trait Resource: Send + Sync + 'static {
    type Type;
    type Storage: ResourceStorage;

    fn id() -> ResourceId;
    fn kind() -> ResourceKind;
    fn label() -> &'static str;
    fn register();
}

pub trait ResourceStorage: Send + Sync + Any {
    fn bind_group_layout_entries(&self) -> Vec<wgpu::BindGroupLayoutEntry>;
    fn bind_group_entries(&self, gpu_index: usize) -> Vec<wgpu::BindGroupEntry>;
}

struct GpuResourceBuffer {
    gpu: Arc<Gpu>,
    resource_buffer: wgpu::Buffer,
    reverse_array: wgpu::Buffer,
}

mod id_mapped_storage;
pub use id_mapped_storage::*;

mod id_mapped_slice_storage;
pub use id_mapped_slice_storage::*;

struct ResourceRegistration {
    kind: ResourceKind,
    storage_factory: fn(gpus: &[Arc<Gpu>], resource_id: ResourceId) -> Box<dyn ResourceStorage>,
}

lazy_static! {
    static ref REGISTERED_RESOURCES: RwLock<IdMap<ResourceId, ResourceRegistration>> =
        RwLock::new(IdMap::new());
}

pub fn register_resource<C: Resource + 'static>() -> ResourceId {
    return REGISTERED_RESOURCES
        .write()
        .unwrap()
        .insert(ResourceRegistration {
            kind: ResourceKind::EntityComponent,
            storage_factory: IdMappedResourceStorage::<EntityId, C>::factory,
        })
        .0;
}

pub fn make_resource_storages(instance: &Instance) -> Vec<Option<Box<dyn ResourceStorage>>> {
    let mut vec = Vec::new();

    println!(
        "creating resource storages for {} resources",
        REGISTERED_RESOURCES.read().unwrap().len()
    );

    for (resource_id, resource) in &*REGISTERED_RESOURCES.read().unwrap() {
        if resource_id.index() >= vec.len() {
            vec.resize_with(resource_id.index() + 1, || None);
        }
        vec[resource_id.index()] = Some((resource.storage_factory)(&instance.gpus(), resource_id));
    }

    return vec;
}

mod test {
    use super::*;
    use std::sync::Arc;

    #[derive(Debug)]
    struct R(Arc<u32>);

    impl Resource for R {
        type Type = R;
        type Storage = IdMappedResourceStorage<EntityId, R>;

        fn id() -> ResourceId {
            todo!()
        }

        fn kind() -> ResourceKind {
            todo!()
        }

        fn label() -> &'static str {
            todo!()
        }

        fn register() {
            todo!()
        }
    }

    #[test]
    fn test() {
        type Id = StandardVersionedIndexId;
        let mut resource_storage =
            IdMappedResourceStorage::<Id, R>::new(&[], ResourceId::from_index(100));

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
