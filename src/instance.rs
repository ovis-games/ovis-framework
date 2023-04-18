use std::{sync::Arc, time::Instant};
use winit::{
    error::OsError,
    event::*,
    event_loop::{ControlFlow, EventLoop},
    window::{Window, WindowBuilder}, monitor::MonitorHandle,
};

use crate::{Error, IdMap, Job, JobFunction, JobId, JobKind, Scene, SceneState, SystemResources};

pub struct Instance {
    jobs: IdMap<Job, JobId>,
    wgpu_instance: wgpu::Instance,
    default_adapter: Arc<wgpu::Adapter>,
    event_loop: EventLoop<()>,
}

impl Instance {
    pub async fn new() -> Self {
        let wgpu_instance = wgpu::Instance::new(wgpu::InstanceDescriptor::default());
        let default_adapter = wgpu_instance
            .request_adapter(&wgpu::RequestAdapterOptionsBase {
                power_preference: wgpu::PowerPreference::default(),
                force_fallback_adapter: false,
                compatible_surface: None,
            })
            .await
            .unwrap();

        let mut instance = Self {
            jobs: IdMap::new(),
            event_loop: EventLoop::new(),
            default_adapter: Arc::new(default_adapter),
            wgpu_instance,
        };

        instance.register_job(JobKind::Update, clear_surface);

        return instance;
    }

    pub fn jobs(&self) -> &IdMap<Job, JobId> {
        return &self.jobs;
    }

    pub fn wgpu(&self) -> &wgpu::Instance {
        return &self.wgpu_instance;
    }

    pub fn default_adapter(&self) -> &Arc<wgpu::Adapter> {
        return &self.default_adapter;
    }

    pub fn primary_monitor(&self) -> Option<MonitorHandle> {
        return self.event_loop.primary_monitor();
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
                scene.add_viewport(0, surface, window.inner_size());

                Ok(window)
            }
            result => result,
        }
    }

    pub fn register_job(&mut self, kind: JobKind, function: JobFunction) -> JobId {
        return self.jobs.insert(Job::new(kind, function)).0;
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
}

pub fn clear_surface(_sr: &mut SystemResources, s: &SceneState) -> Result<(), Error> {
    for (_id, viewport) in &*s.viewports().read().unwrap() {
        let gpu = &s.gpus()[viewport.gpu_index()];
        let mut encoder = gpu
            .device()
            .create_command_encoder(&wgpu::CommandEncoderDescriptor { label: Some("") });
        let color_attachment = viewport
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
            let _render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some(""),
                color_attachments: &[color_attachment],
                depth_stencil_attachment: None,
            });
        }
        gpu.queue().submit(std::iter::once(encoder.finish()));
    }

    Ok(())
}
