#![feature(trait_upcasting)]

use ovis_runtime::load_runtime;
use ovis_core::{Instance, Scene};
use pollster::block_on;
use ovis_core::winit::window::WindowBuilder;
use serde_json::json;

async fn run() {
    env_logger::init();

    load_runtime();

    let instance = Instance::new().await;
    let mut scene = Scene::from_json(&instance, json! {
        {
            "entities": [
                {
                    "components": {
                        "ovis_runtime_VertexPosition": [
                            [-1, 0, 0],
                            [1, 0, 0],
                            [0, 1, 0]
                        ],
                        "ovis_runtime_VertexColor": [
                            [1, 0, 0, 1],
                            [0, 1, 0, 1],
                            [0, 0, 1, 1]
                        ],
                        "ovis_runtime_Transform": {
                            "translation": [0, 0, 0],
                            "rotation": [0, 0, 0, 1],
                            "scaling": [1, 1, 1]
                        }
                    }
                }
            ]
        }
    }).await.unwrap();

    // let mut scene = Scene::new(&instance).await;
    let _window = instance
        .build_window(&mut scene, WindowBuilder::new().with_title("Example"))
        .unwrap();
    let _window2 = instance
        .build_window(&mut scene, WindowBuilder::new())
        .unwrap();

    // {
    //     let mut entities = scene.entities().write().unwrap();
    //     for i in 0..2 {
    //         let entity_id = entities.reserve();

    //         // scene.state().resource_storage_mut::<Position>().unwrap().insert(
    //         //     entity_id,
    //         //     Position {
    //         //         x: 1.0 - i as f32,
    //         //         y: i as f32,
    //         //     },
    //         // );
    //     }
    // }

    instance.run([scene]);
}

fn main() {
    block_on(run());
}
