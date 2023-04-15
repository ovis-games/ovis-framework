use std::time::Instant;

use ovis::{Instance, Scene, JobKind};

fn main() {
    let mut instance = Instance::new();

    for _ in 0..10000 {
        instance.register_job(ovis::JobKind::Update, |_| {});
    }

    instance.register_job(JobKind::Update, |_| {
        // println!("Job X start");
        // thread::sleep(Duration::from_millis(10));
        // println!("Job X End");
    });
    instance.register_job(JobKind::Update, |_| {
        // println!("Job Y start");
        // thread::sleep(Duration::from_millis(10));
        // println!("Job Y End");
    });
    instance.register_job(JobKind::Update, |_| {
        // println!("Job Z start");
        // thread::sleep(Duration::from_millis(10));
        // println!("Job Z End");
    });
    // instance.register_job(ovis::JobKind::Update, |_| println!("Job A"));
    // instance.register_job(ovis::JobKind::Update, |_| println!("Job B"));
    // instance.register_job(ovis::JobKind::Update, |_| println!("Job C"));

    let mut scene = Scene::new(&instance);

    let t0 = Instant::now();
    for _ in 0..100 {
        scene.tick(0.0);
    }
    println!("Elapsed time: {}ms", t0.elapsed().as_millis());
}
