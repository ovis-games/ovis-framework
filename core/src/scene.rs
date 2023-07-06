use std::{
    any::Any,
    sync::{Arc, RwLock, RwLockWriteGuard},
    thread, marker::PhantomData,
};

use erased_serde::Deserializer;
use winit::dpi::PhysicalSize;

use crate::{
    make_resource_storages, Gpu, IdMap, IdStorage,
    Instance, JobKind, Resource, ResourceId, ResourceStorage, Result, Scheduler,
    StandardVersionedIndexId, VersionedIndexId, Error, result, resource_id_from_label,
};

pub type EntityId = StandardVersionedIndexId<8>;
pub type ViewportId = StandardVersionedIndexId<8>;

pub struct EntityDescriptor {}

impl EntityDescriptor {
    pub fn new() -> Self {
        EntityDescriptor {}
    }
}

pub struct Viewport {
    gpu: Arc<Gpu>,
    surface: wgpu::Surface,
    surface_config: wgpu::SurfaceConfiguration,
    texture: Option<wgpu::SurfaceTexture>,
    texture_view: Option<wgpu::TextureView>,
}

impl Viewport {
    pub fn gpu(&self) -> &Arc<Gpu> {
        &self.gpu
    }

    pub fn surface(&self) -> &wgpu::Surface {
        &self.surface
    }

    pub fn texture(&self) -> Option<&wgpu::SurfaceTexture> {
        self.texture.as_ref()
    }

    pub fn texture_view(&self) -> Option<&wgpu::TextureView> {
        self.texture_view.as_ref()
    }

    pub fn surface_config(&self) -> &wgpu::SurfaceConfiguration {
        &self.surface_config
    }
}

struct ResourceBindings {
    group_layout: wgpu::BindGroupLayout,
    group: wgpu::BindGroup,
}

pub struct SceneState {
    entities: Arc<RwLock<IdStorage<EntityId>>>,
    viewports: Arc<RwLock<IdMap<ViewportId, Viewport>>>,
    resources: Arc<Vec<Option<RwLock<ResourceStorage>>>>,
    resource_bindings: Arc<Vec<ResourceBindings>>,
}

impl SceneState {
    pub fn new(instance: &Instance) -> Self {
        let mut bind_group_entries = Vec::new();
        let resources = make_resource_storages(instance);

        for r in &resources {
            if let Some(r) = r {
                bind_group_entries.append(&mut r.bind_group_layout_entries());
            }
        }

        let bindings = instance
            .gpus()
            .iter()
            .map(|gpu| {
                let group_layout =
                    gpu.device()
                        .create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                            label: Some("Resources"),
                            entries: &bind_group_entries,
                        });

                let mut entries = Vec::new();

                for r in &resources {
                    if let Some(r) = r {
                        entries.append(&mut r.bind_group_entries(gpu.index()));
                    }
                }

                let group = gpu.device().create_bind_group(&wgpu::BindGroupDescriptor {
                    label: Some("Resources"),
                    layout: &group_layout,
                    entries: &entries,
                });

                return ResourceBindings {
                    group_layout,
                    group,
                };
            })
            .collect();

        return Self {
            entities: Arc::new(RwLock::new(IdStorage::new())),
            viewports: Arc::new(RwLock::new(IdMap::new())),
            resources: Arc::new(
                resources
                    .into_iter()
                    .map(|r| r.map(|r| RwLock::new(r)))
                    .collect(),
            ),
            resource_bindings: Arc::new(bindings),
        };
    }

    pub fn entities(&self) -> &RwLock<IdStorage<StandardVersionedIndexId<8>>> {
        self.entities.as_ref()
    }

    pub fn viewports(&self) -> &RwLock<IdMap<ViewportId, Viewport>> {
        self.viewports.as_ref()
    }

    // pub fn resource_storage(&self, id: ResourceId) -> Option<&RwLock<ResourceStorage>> {
    //     return self.resources[id.index()].as_ref();
    // }

    // pub fn resource_storage_mut<R: Resource>(&self) -> Option<MutableResourceStorageAccess<'_, R>> {
    //     if let Some(storage) = self.resources[R::id().index()].as_ref() {
    //         return Some(MutableResourceStorageAccess::new(storage.write().unwrap()));
    //     }
    //     todo!();
    // }

    pub fn resource_bind_group_layout(&self, gpu_index: usize) -> &wgpu::BindGroupLayout {
        &self.resource_bindings[gpu_index].group_layout
    }

    pub fn resource_bind_group(&self, gpu_index: usize) -> &wgpu::BindGroup {
        &self.resource_bindings[gpu_index].group
    }
}

// pub struct MutableResourceStorageAccess<'scene, R: Resource> {
//     guard: RwLockWriteGuard<'scene, ResourceStorage>,
//     phantom: PhantomData<R>,
// }

// impl<'scene, R: Resource> MutableResourceStorageAccess<'scene, R> {
//     fn new(guard: RwLockWriteGuard<'scene, ResourceStorage>) -> Self {
//         return Self { guard, phantom: PhantomData };
//     }
// }

// impl<R: Resource> std::ops::Deref for MutableResourceStorageAccess<'_, R> {
//     type Target = ResourceStorage;

//     fn deref(&self) -> &Self::Target {
//         return (&**self.guard as &dyn Any)
//             .downcast_ref::<R::Storage>()
//             .unwrap();
//     }
// }

// impl<R: Resource> std::ops::DerefMut for MutableResourceStorageAccess<'_, R> {
//     fn deref_mut(&mut self) -> &mut Self::Target {
//         return (&mut **self.guard as &mut dyn Any)
//             .downcast_mut::<R::Storage>()
//             .unwrap();
//     }
// }

pub struct Scene {
    game_time: f32,
    state: Arc<SceneState>,
    scheduler: Scheduler,
    viewports_changed: bool,
}

impl Scene {
    pub async fn new(instance: &Instance) -> Self {
        let state = Arc::new(SceneState::new(instance));

        return Self {
            viewports_changed: false,
            game_time: 0.0,
            scheduler: Scheduler::new(
                instance,
                JobKind::Update,
                state.clone(),
                thread::available_parallelism()
                    .map(|c| -> usize { c.into() })
                    .unwrap_or(4),
            ),
            state,
        };
    }

    pub fn state(&self) -> &Arc<SceneState> {
        &self.state
    }

    pub fn add_viewport(
        &mut self,
        gpu: Arc<Gpu>,
        surface: wgpu::Surface,
        size: PhysicalSize<u32>,
    ) -> ViewportId {
        let surface_caps = surface.get_capabilities(&gpu.adapter());
        let surface_format = surface_caps
            .formats
            .iter()
            .copied()
            .filter(|f| f.describe().srgb)
            .next()
            .unwrap_or(surface_caps.formats[0]);
        let config = wgpu::SurfaceConfiguration {
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            format: surface_format,
            width: size.width,
            height: size.height,
            present_mode: surface_caps.present_modes[0],
            alpha_mode: surface_caps.alpha_modes[0],
            view_formats: vec![],
        };
        surface.configure(&gpu.device(), &config);
        self.viewports_changed = true;
        self.viewports()
            .write()
            .unwrap()
            .insert(Viewport {
                gpu,
                surface,
                texture: None,
                texture_view: None,
                surface_config: config,
            })
            .0
    }

    pub fn entities(&self) -> &Arc<RwLock<IdStorage>> {
        return &self.state.entities;
    }

    pub fn viewports(&self) -> &Arc<RwLock<IdMap<ViewportId, Viewport>>> {
        return &self.state.viewports;
    }

    pub fn resource_storage(
        &self,
        resource_id: ResourceId,
    ) -> Option<&RwLock<ResourceStorage>> {
        return self.state.resources[resource_id.index()].as_ref();
    }

    pub fn resource_storage_from_label(
        &self,
        label: &str,
    ) -> Option<&RwLock<ResourceStorage>> {
        return self.resource_storage(resource_id_from_label(label)?);
    }


    pub fn tick(&mut self, delta_time: f32) -> Result<()> {
        if self.viewports_changed {
            self.scheduler.configure_pipelines();
        }

        for (_id, viewport) in &mut *self.viewports().write().unwrap() {
            let texture = viewport.surface().get_current_texture().unwrap();
            viewport.texture_view = Some(
                texture
                    .texture
                    .create_view(&wgpu::TextureViewDescriptor::default()),
            );
            viewport.texture = Some(texture);
        }
        self.game_time += delta_time;
        let result = self.scheduler.run_jobs(self.game_time, delta_time);

        for (_id, viewport) in &mut *self.viewports().write().unwrap() {
            viewport.texture_view = None;
            viewport.texture.take().unwrap().present();
        }

        return result;
    }

    pub async fn from_json(instance: &Instance, json: serde_json::Value) -> Result<Self> {
        let scene = Self::new(&instance).await;

        {
            let mut scene_entities = scene.entities().write().unwrap();

            if let Some(entities) = json.get("entities").and_then(|e| e.as_array()) {
                for entity in entities {
                    let entity_id = scene_entities.reserve();
                    if let Some(components) = entity.get("components").and_then(|c| c.as_object()) {
                        for (label, component) in components {
                            if let Some(storage) = scene.resource_storage_from_label(label) {
                                let mut storage = storage.write().unwrap();
                                if let ResourceStorage::EntityComponent(storage) = &mut *storage {
                                    let string = component.to_string();
                                    let mut x = serde_json::Deserializer::from_str(&string);
                                    storage.insert_serialized(entity_id, &mut <dyn Deserializer>::erase(&mut x)).unwrap();
                                }
                            } else {
                                return Err(Error::new(format!("invalid entity component: {}", label), result::SourceLocation::here()));
                            }
                        }
                    } else {
                        return Err(Error::new("components field not found", result::SourceLocation::here()));
                    }
                }
            } else {
                return Err(Error::new("no entities found in scene JSON", result::SourceLocation::here()));
            }
        }

        Ok(scene)
    }
}
