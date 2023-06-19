use std::{sync::Arc, time::Instant};
use winit::{
    error::OsError,
    event::*,
    event_loop::{ControlFlow, EventLoop},
    monitor::MonitorHandle,
    window::{Window, WindowBuilder},
};

use crate::{Gpu, Scene};

pub struct Instance {
    wgpu_instance: wgpu::Instance,
    gpus: Vec<Arc<Gpu>>,
    event_loop: EventLoop<()>,
}

impl Instance {
    pub async fn new() -> Self {
        let wgpu_instance = wgpu::Instance::new(wgpu::InstanceDescriptor::default());

        let gpus = vec![Arc::new(Gpu::new(&wgpu_instance, 0).await)];

        let instance = Self {
            event_loop: EventLoop::new(),
            gpus,
            wgpu_instance,
        };

        return instance;
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

    pub fn run<S: IntoIterator<Item = Scene>>(self, scenes: S) {
        let mut scenes = scenes.into_iter().collect::<Vec<_>>();
        let mut last_update = Instant::now();

        self.event_loop.run(move |event, _, control_flow| {
            *control_flow = ControlFlow::Wait;

            println!("{:?}", event);

            match event {
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
            }
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
}
