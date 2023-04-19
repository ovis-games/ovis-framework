use ovis::{Instance, Scene};
use pollster::block_on;
use winit::window::{WindowBuilder, Fullscreen};

async fn run() {
    env_logger::init();
    let instance = Instance::new().await;

    let mut scene = Scene::new(&instance).await;
    let _window = instance.build_window(&mut scene, WindowBuilder::new().with_fullscreen(Some(Fullscreen::Borderless(instance.primary_monitor())))).unwrap();
    let _window2 = instance.build_window(&mut scene, WindowBuilder::new()).unwrap();

    instance.run([scene]);
}

fn main() {
    block_on(run());
}
