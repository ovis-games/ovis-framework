use crate::{EntityId, Gpu, IdMap, Instance, StandardVersionedIndexId, VersionedIndexId};
use erased_serde::Deserializer;
use lazy_static::lazy_static;
use serde::{Deserialize, Serialize};
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

pub trait Resource: Serialize + for<'a> Deserialize<'a> + Default + Send + Sync + 'static {
    // type Type;
    // type Storage: ResourceStorage;

    fn id() -> ResourceId;
    fn kind() -> ResourceKind;
    fn label() -> &'static str;
    fn register();
    fn make_storage(gpus: &[Arc<Gpu>]) -> ResourceStorage;
}

// pub trait EntityComponent

pub trait BindingProvider {
    fn bind_group_layout_entries(&self) -> Vec<wgpu::BindGroupLayoutEntry>;
    fn bind_group_entries(&self, gpu_index: usize) -> Vec<wgpu::BindGroupEntry>;
}

pub enum ResourceStorage {
    EntityComponent(Box<dyn EntityComponentResourceStorage>),
    // IdMapped(IdMappedResourceStorage<EntityId, Box<dyn Resource>>),
    // IdMappedSlice(IdMappedSliceResourceStorage<EntityId, Box<dyn Resource>>),
}

impl ResourceStorage {
    pub fn bind_group_layout_entries(&self) -> Vec<wgpu::BindGroupLayoutEntry> {
        match self {
            ResourceStorage::EntityComponent(s) => s.bind_group_layout_entries(),
        }
    }

    pub fn bind_group_entries(&self, gpu_index: usize) -> Vec<wgpu::BindGroupEntry> {
        match self {
            ResourceStorage::EntityComponent(s) => s.bind_group_entries(gpu_index),
        }
    }
}

pub trait EntityComponentResourceStorage: BindingProvider + Send + Sync + 'static {
    fn insert_default(&mut self, entity_id: EntityId);
    fn insert_serialized(&mut self, entity_id: EntityId, d: &mut dyn Deserializer) -> erased_serde::Result<()>;
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
    label: &'static str,
    storage_factory: fn(gpus: &[Arc<Gpu>]) -> ResourceStorage,
}

lazy_static! {
    static ref REGISTERED_RESOURCES: RwLock<IdMap<ResourceId, ResourceRegistration>> =
        RwLock::new(IdMap::new());
}

pub fn resource_id_from_label(label: &str) -> Option<ResourceId> {
    for (resource_id, resource) in &*REGISTERED_RESOURCES.read().unwrap() {
        println!("checking {} against {}", label, resource.label);
        if resource.label == label {
            println!("found resource {} with id {}", label, resource_id);
            return Some(resource_id);
        }
    }
    return None;
}

pub fn register_resource<R: Resource + 'static>() -> ResourceId {
    println!("registering resource {}", R::label());
    return REGISTERED_RESOURCES
        .write()
        .unwrap()
        .insert(ResourceRegistration {
            label: R::label(),
            kind: ResourceKind::EntityComponent,
            storage_factory: R::make_storage,
        })
        .0;
}

pub fn make_resource_storages(instance: &Instance) -> Vec<Option<ResourceStorage>> {
    let mut vec = Vec::new();

    println!(
        "creating resource storages for {} resources",
        REGISTERED_RESOURCES.read().unwrap().len()
    );

    for (resource_id, resource) in &*REGISTERED_RESOURCES.read().unwrap() {
        if resource_id.index() >= vec.len() {
            vec.resize_with(resource_id.index() + 1, || None);
        }
        vec[resource_id.index()] = Some((resource.storage_factory)(&instance.gpus()));
    }

    return vec;
}

mod test {
    use super::*;

    #[derive(Default, Debug, Serialize, Deserialize)]
    struct R(u32);

    impl Resource for R {
        // type Type = R;
        // type Storage = IdMappedResourceStorage<EntityId, R>;

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

        fn make_storage(_gpus: &[Arc<Gpu>]) -> ResourceStorage {
            todo!()
        }
    }

    #[test]
    fn test() {
        type Id = StandardVersionedIndexId;
        let mut resource_storage =
            IdMappedResourceStorage::<Id, R>::new(&[], ResourceId::from_index(100));

        let id = Id::from_index(0);
        assert!(resource_storage.insert(id, R(100)).is_none());

        let recv = resource_storage.get(id);
        assert!(recv.is_some());
        assert_eq!(recv.unwrap().0, 100);

        let recv = resource_storage.insert(id, R(200));
        assert!(recv.is_some());
        assert_eq!(recv.unwrap().0, 100);

        let recv = resource_storage.remove(id);
        assert!(recv.is_some());
        assert_eq!(recv.unwrap().0, 200);

        let recv = resource_storage.get(id);
        assert!(recv.is_none());
    }
}
