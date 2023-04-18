use std::{
    sync::{Arc, RwLock},
    thread,
};

use wgpu::{Adapter, Device, Queue};
use winit::dpi::PhysicalSize;

use crate::{IdStorage, Instance, JobKind, Result, Scheduler, StandardVersionedIndexId, IdMap};

pub type EntityId = StandardVersionedIndexId<8>;
pub type ViewportId = StandardVersionedIndexId<8>;

pub struct EntityDescriptor {}

impl EntityDescriptor {
    pub fn new() -> Self {
        EntityDescriptor {}
    }
}

pub struct Viewport {
    gpu_index: usize,
    surface: wgpu::Surface,
    texture: Option<wgpu::SurfaceTexture>,
    texture_view: Option<wgpu::TextureView>,
}

impl Viewport {
    pub fn gpu_index(&self) -> usize {
        self.gpu_index
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
}

pub struct Gpu {
    adapter: Arc<Adapter>,
    shader_module: wgpu::ShaderModule,
    device: Device,
    queue: Queue,
}

impl Gpu {
    pub fn device(&self) -> &Device {
        &self.device
    }

    pub fn queue(&self) -> &Queue {
        &self.queue
    }
}

pub struct SceneState {
    gpus: Arc<Vec<Gpu>>,
    entities: Arc<RwLock<IdStorage<EntityId>>>,
    viewports: Arc<RwLock<IdMap<Viewport, ViewportId>>>,
}

impl SceneState {
    pub fn new(gpus: Vec<Gpu>) -> Self {
        return Self {
            gpus: Arc::new(gpus),
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

    pub fn gpus(&self) -> &[Gpu] {
        self.gpus.as_ref()
    }
}

pub struct Scene {
    game_time: f32,
    state: Arc<SceneState>,
    scheduler: Scheduler,
}

impl Scene {
    pub async fn new<A: IntoIterator<Item = Arc<Adapter>>>(instance: &Instance, adapters: A) -> Self {
        let mut gpus = vec![];
        for adapter in adapters {
            let (device, queue) = adapter
                .request_device(
                    &wgpu::DeviceDescriptor {
                        features: wgpu::Features::empty(),
                        limits: wgpu::Limits::default(),
                        label: None,
                    },
                    None,
                )
                .await
                .unwrap();

            let shader_module = device.create_shader_module(wgpu::include_wgsl!("shader.wgsl"));
            gpus.push(Gpu { device, queue, adapter, shader_module });
        }
        let state = Arc::new(SceneState::new(gpus));

        return Self {
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

    pub fn add_viewport(&mut self, adapter_index: usize, surface: wgpu::Surface, size: PhysicalSize<u32>) -> ViewportId {
        let gpu = &self.state.gpus()[0];
        let surface_caps = surface.get_capabilities(&gpu.adapter);
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
        self.viewports().write().unwrap().insert(Viewport { gpu_index: adapter_index, surface, texture: None, texture_view: None }).0
    }

    pub fn entities(&self) -> &Arc<RwLock<IdStorage>> {
        return &self.state.entities;
    }

    pub fn viewports(&self) -> &Arc<RwLock<IdMap<Viewport, ViewportId>>> {
        return &self.state.viewports;
    }

    pub fn tick(&mut self, delta_time: f32) -> Result<()> {
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
