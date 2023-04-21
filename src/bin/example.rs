#![feature(trait_upcasting)]

use ovis::{Instance, Scene, IdMappedResourceStorage, EntityId, Position};
use pollster::block_on;
use winit::window::{WindowBuilder, Fullscreen};
use std::any::Any;

async fn run() {
    env_logger::init();
    let instance = Instance::new().await;

    let mut scene = Scene::new(&instance).await;
    let _window = instance.build_window(&mut scene, WindowBuilder::new().with_fullscreen(Some(Fullscreen::Borderless(instance.primary_monitor())))).unwrap();
    let _window2 = instance.build_window(&mut scene, WindowBuilder::new()).unwrap();

    {
        let mut entities = scene.entities().write().unwrap();
        let entity_id = entities.reserve();
        let mut position_storage = scene.resource_storage(instance.position_id()).unwrap().write().unwrap();
        let position_storage = (&mut **position_storage as &mut dyn Any).downcast_mut::<IdMappedResourceStorage<EntityId, Position>>().unwrap();
        position_storage.insert(entity_id, Position { x: 123.0, y: 133.0});
    }

    instance.run([scene]);
}

fn main() {
    block_on(run());
}
