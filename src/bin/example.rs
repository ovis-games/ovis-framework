#![feature(trait_upcasting)]

use ovis_runtime::{load_runtime, Position};
use ovis_core::{Instance, Scene};
use pollster::block_on;
use ovis_core::winit::window::WindowBuilder;

async fn run() {
    env_logger::init();

    load_runtime();

    let instance = Instance::new().await;

    let mut scene = Scene::new(&instance).await;
    let _window = instance
        .build_window(&mut scene, WindowBuilder::new().with_title("Example"))
        .unwrap();
    let _window2 = instance
        .build_window(&mut scene, WindowBuilder::new())
        .unwrap();

    {
        let mut entities = scene.entities().write().unwrap();
        for i in 0..2 {
            let entity_id = entities.reserve();

            scene.state().resource_storage_mut::<Position>().unwrap().insert(
                entity_id,
                Position {
                    x: 1.0 - i as f32,
                    y: i as f32,
                },
            );
        }
    }

    instance.run([scene]);
}

fn main() {
    block_on(run());
}
