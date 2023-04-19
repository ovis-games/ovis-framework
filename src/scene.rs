use std::{
    sync::{Arc, RwLock},
    thread,
};

use winit::dpi::PhysicalSize;

use crate::{IdStorage, Instance, JobKind, Result, Scheduler, StandardVersionedIndexId, IdMap, Gpu};

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

pub struct SceneState {
    entities: Arc<RwLock<IdStorage<EntityId>>>,
    viewports: Arc<RwLock<IdMap<Viewport, ViewportId>>>,
}

impl SceneState {
    pub fn new() -> Self {
        return Self {
            entities: Arc::new(RwLock::new(IdStorage::new())),
            viewports: Arc::new(RwLock::new(IdMap::new())),
        };
    }

    pub fn entities(&self) -> &RwLock<IdStorage<StandardVersionedIndexId<8>>> {
        self.entities.as_ref()
    }

    pub fn viewports(&self) -> &RwLock<IdMap<Viewport, ViewportId>> {
        self.viewports.as_ref()
    }
}

pub struct Scene {
    game_time: f32,
    state: Arc<SceneState>,
    scheduler: Scheduler,
    viewports_changed: bool,
}

impl Scene {
    pub async fn new(instance: &Instance) -> Self {
        let state = Arc::new(SceneState::new());
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

    pub fn add_viewport(&mut self, gpu: Arc<Gpu>, surface: wgpu::Surface, size: PhysicalSize<u32>) -> ViewportId {
        let surface_caps = surface.get_capabilities(&gpu.adapter());
        let surface_format = surface_caps.formats.iter()
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
        self.viewports().write().unwrap().insert(Viewport { gpu, surface, texture: None, texture_view: None, surface_config: config }).0
    }

    pub fn entities(&self) -> &Arc<RwLock<IdStorage>> {
        return &self.state.entities;
    }

    pub fn viewports(&self) -> &Arc<RwLock<IdMap<Viewport, ViewportId>>> {
        return &self.state.viewports;
    }

    pub fn tick(&mut self, delta_time: f32) -> Result<()> {
        if self.viewports_changed {
            self.scheduler.configure_pipelines();
        }

        for (_id, viewport) in &mut *self.viewports().write().unwrap() {
            let texture = viewport.surface().get_current_texture().unwrap();
            viewport.texture_view = Some(texture.texture.create_view(&wgpu::TextureViewDescriptor::default()));
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
}
