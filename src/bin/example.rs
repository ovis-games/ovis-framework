use std::{time::{Instant, Duration}, thread};

use ovis::{Instance, Scene, JobKind};
use rand::Rng;

fn main() {
    let mut instance = Instance::new();

    let job_a = instance.register_job(JobKind::Update, |_| {
        let sleep_range = 0u64..100u64;
        // println!("A: start");
        // thread::sleep(Duration::from_micros(rand::thread_rng().gen_range(sleep_range)));
        // println!("A: end");
    });
    let job_b = instance.register_job(JobKind::Update, |_| {
        let sleep_range = 0u64..100u64;
        // println!("B: start");
        // thread::sleep(Duration::from_micros(rand::thread_rng().gen_range(sleep_range)));
        // println!("B: end");
    });
    let job_c = instance.register_job(JobKind::Update, |_| {
        let sleep_range = 0u64..100u64;
        // println!("C: start");
        // thread::sleep(Duration::from_micros(rand::thread_rng().gen_range(sleep_range)));
        // println!("C: end");
    });

    let job_x = instance.register_job(JobKind::Update, |_| {
        let sleep_range = 0u64..100u64;
        // println!("X: start");
        // thread::sleep(Duration::from_micros(rand::thread_rng().gen_range(sleep_range)));
        // println!("X: end");
    });
    let job_y = instance.register_job(JobKind::Update, |_| {
        let sleep_range = 0u64..100u64;
        // println!("Y: start");
        // thread::sleep(Duration::from_micros(rand::thread_rng().gen_range(sleep_range)));
        // println!("Y: end");
    });
    let job_z = instance.register_job(JobKind::Update, |_| {
        let sleep_range = 0u64..100u64;
        // println!("z: start");
        // thread::sleep(Duration::from_micros(rand::thread_rng().gen_range(sleep_range)));
        // println!("Z: end");
    });

    // instance.add_job_dependency(job_x, job_z);

    let mut scene = Scene::new(&instance);

    thread::sleep(Duration::from_millis(100));

    let t0 = Instant::now();
    // loop {
    //     scene.tick(0.0);
    // }
    for _ in 0..1000 {
        scene.tick(0.0);
    }
    println!("Elapsed time: {}ms", t0.elapsed().as_millis());
}
