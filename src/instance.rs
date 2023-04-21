use std::{sync::Arc, time::Instant, any::Any};
use winit::{
    error::OsError,
    event::*,
    event_loop::{ControlFlow, EventLoop},
    monitor::MonitorHandle,
    window::{Window, WindowBuilder},
};

use crate::{
    Error, IdMap, Job, JobFunction, JobId, JobKind, Scene, SceneState,
    SystemResources, ResourceKind, ResourceStorage, IdMappedResourceStorage, EntityId, VersionedIndexId, Resource, ResourceId, 
};

pub struct Gpu {
    adapter: wgpu::Adapter,
    shader_module: wgpu::ShaderModule,
    device: wgpu::Device,
    queue: wgpu::Queue,
}

impl Gpu {
    pub fn device(&self) -> &wgpu::Device {
        &self.device
    }

    pub fn queue(&self) -> &wgpu::Queue {
        &self.queue
    }
    
    pub fn adapter(&self) -> &wgpu::Adapter {
        &self.adapter
    }

    pub fn shader_module(&self) -> &wgpu::ShaderModule {
        &self.shader_module
    }
}


struct ResourceRegistration {
    kind: ResourceKind,
    storage_factory: fn() -> Box<dyn ResourceStorage>,
}


pub struct Instance {
    jobs: IdMap<JobId, Job>,
    registered_resources: IdMap<JobId, ResourceRegistration>,
    wgpu_instance: wgpu::Instance,
    gpus: Vec<Arc<Gpu>>,
    event_loop: EventLoop<()>,
    position_id: ResourceId,
}

impl Instance {
    pub async fn new() -> Self {
        let wgpu_instance = wgpu::Instance::new(wgpu::InstanceDescriptor::default());
        let adapter = wgpu_instance
            .request_adapter(&wgpu::RequestAdapterOptionsBase {
                power_preference: wgpu::PowerPreference::default(),
                force_fallback_adapter: false,
                compatible_surface: None,
            })
            .await
            .unwrap();

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

        let gpus = vec![Arc::new(Gpu {
            device,
            queue,
            adapter,
            shader_module,
        })];

        let mut instance = Self {
            jobs: IdMap::new(),
            registered_resources: IdMap::new(),
            event_loop: EventLoop::new(),
            gpus,
            wgpu_instance,
            position_id: ResourceId::from_index(0),
        };

        instance.register_job(JobKind::Update, clear_surface);
        instance.position_id = instance.register_entity_component::<Position>();

        return instance;
    }

    pub fn jobs(&self) -> &IdMap<JobId, Job> {
        return &self.jobs;
    }

    pub fn make_resource_storages(&self) -> Vec<Option<Box<dyn ResourceStorage>>> {
        let mut vec = Vec::new();

        for (resource_id, resource) in &self.registered_resources {
            if resource_id.index() >= vec.len() {
                vec.resize_with(resource_id.index() + 1, || None);
            }
            vec[resource_id.index()] = Some((resource.storage_factory)());
        }

        return vec;
    }

    pub fn wgpu(&self) -> &wgpu::Instance {
        return &self.wgpu_instance;
    }

    pub fn gpus(&self) -> &Vec<Arc<Gpu>> {
        return &self.gpus;
    }

    pub fn primary_monitor(&self) -> Option<MonitorHandle> {
        return self.event_loop.primary_monitor();
    }

    pub fn position_id(&self) -> ResourceId {
        return self.position_id;
    }

    pub fn run<S: IntoIterator<Item = Scene>>(self, scenes: S) {
        let mut scenes = scenes.into_iter().collect::<Vec<_>>();
        let mut last_update = Instant::now();

        self.event_loop
            .run(move |event, _, control_flow| match event {
                Event::WindowEvent {
                    ref event,
                    window_id: _,
                } => match event {
                    WindowEvent::CloseRequested
                    | WindowEvent::KeyboardInput {
                        input:
                            KeyboardInput {
                                state: ElementState::Pressed,
                                virtual_keycode: Some(VirtualKeyCode::Escape),
                                ..
                            },
                        ..
                    } => *control_flow = ControlFlow::Exit,
                    _ => {}
                },
                Event::MainEventsCleared => {
                    let now = Instant::now();
                    let diff = (now - last_update).as_nanos() as f64 / 1000.0 / 1000.0 / 1000.0;
                    last_update = now;

                    for scene in &mut scenes {
                        if let Err(error) = scene.tick(diff as f32) {
                            println!("{error}")
                        }
                    }
                }
                _ => {}
            });
    }

    pub fn build_window(
        &self,
        scene: &mut Scene,
        window_builder: WindowBuilder,
    ) -> Result<Window, OsError> {
        match window_builder.build(&self.event_loop) {
            Ok(window) => {
                let surface = unsafe { self.wgpu_instance.create_surface(&window).unwrap() };
                scene.add_viewport(self.gpus()[0].clone(), surface, window.inner_size());

                Ok(window)
            }
            result => result,
        }
    }

    pub fn register_job(
        &mut self,
        kind: JobKind,
        function: JobFunction,
    ) -> JobId {
        return self
            .jobs
            .insert(Job::new(kind, function))
            .0;
    }

    pub fn add_job_dependency(&mut self, job_id: JobId, dependency_id: JobId) {
        if let Some(dependency) = self.jobs.get(dependency_id) {
            let dependency_kind = dependency.kind();
            if let Some(job) = self.jobs.get_mut(job_id) {
                if job.kind() == dependency_kind {
                    job.add_dependency(dependency_id);
                }
            }
        }
    }

    pub fn register_entity_component<C: Resource + 'static>(&mut self) -> ResourceId {
        return self.registered_resources.insert(ResourceRegistration {
            kind: ResourceKind::EntityComponent,
            storage_factory: IdMappedResourceStorage::<EntityId, C>::factory,
        }).0;
    }
}

pub struct Position {
    pub x: f32,
    pub y: f32,
}

impl Resource for Position {}

pub fn clear_surface(sr: &SystemResources, s: &SceneState) -> Result<(), Error> {
    let viewport = sr.viewport().unwrap();

    let gpu = viewport.gpu();
    let mut encoder = gpu
        .device()
        .create_command_encoder(&wgpu::CommandEncoderDescriptor { label: Some("") });
    let color_attachment =
        viewport
            .texture_view()
            .map(|view| wgpu::RenderPassColorAttachment {
                view,
                resolve_target: None,
                ops: wgpu::Operations {
                    load: wgpu::LoadOp::Clear(wgpu::Color {
                        r: 0.1,
                        g: 0.2,
                        b: 0.3,
                        a: 1.0,
                    }),
                    store: true,
                },
            });
    {
        let mut position_storage = s.resource_storage(ResourceId::from_index_and_version(0, 0)).unwrap().write().unwrap();
        let position_storage = (&mut **position_storage as &mut dyn Any).downcast_mut::<IdMappedResourceStorage<EntityId, Position>>().unwrap();

        for (id, p) in position_storage.iter() {
            println!("{}: ({}, {})", id, p.x, p.y);
        }

        let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some(""),
            color_attachments: &[color_attachment],
            depth_stencil_attachment: None,
        });
        render_pass.set_pipeline(sr.pipeline().unwrap());
        render_pass.draw(0..3, 0..1);
    }
    gpu.queue().submit(std::iter::once(encoder.finish()));

    Ok(())
}
