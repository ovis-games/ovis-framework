use std::{time::{Instant, Duration}, thread};

use ovis::{Instance, Scene, JobKind, Error, SourceLocation};
use rand::Rng;

fn main() {
    let mut instance = Instance::new();

    let job_a = instance.register_job(JobKind::Update, |_| {
        let sleep_range = 0u64..100u64;
        // println!("A: start");
        // thread::sleep(Duration::from_micros(rand::thread_rng().gen_range(sleep_range)));
        // println!("A: end");
        Ok(())
    });
    let job_b = instance.register_job(JobKind::Update, |_| {
        let sleep_range = 0u64..100u64;
        // println!("B: start");
        // thread::sleep(Duration::from_micros(rand::thread_rng().gen_range(sleep_range)));
        // println!("B: end");
        Ok(())
    });
    let job_c = instance.register_job(JobKind::Update, |_| {
        let sleep_range = 0u64..100u64;
        // println!("C: start");
        // thread::sleep(Duration::from_micros(rand::thread_rng().gen_range(sleep_range)));
        // println!("C: end");
        Ok(())
    });

    let job_x = instance.register_job(JobKind::Update, |_| {
        let sleep_range = 0u64..100u64;
        // println!("X: start");
        // thread::sleep(Duration::from_micros(rand::thread_rng().gen_range(sleep_range)));
        // println!("X: end");
        Ok(())
    });
    let job_y = instance.register_job(JobKind::Update, |_| {
        let sleep_range = 0u64..100u64;
        // println!("Y: start");
        // thread::sleep(Duration::from_micros(rand::thread_rng().gen_range(sleep_range)));
        // println!("Y: end");
        Ok(())
    });
    let job_z = instance.register_job(JobKind::Update, |_| {
        let sleep_range = 0u64..100u64;
        // println!("z: start");
        // thread::sleep(Duration::from_micros(rand::thread_rng().gen_range(sleep_range)));
        // println!("Z: end");
        Err(Error::new("something went wrong", SourceLocation::here()))
    });

    // instance.add_job_dependency(job_x, job_z);

    let mut scene = Scene::new(&instance);

    thread::sleep(Duration::from_millis(100));

    let t0 = Instant::now();
    // loop {
    //     scene.tick(0.0);
    // }
    for _ in 0..1000 {
        if let Err(error) = scene.tick(0.0) {
            println!("{error}")
        }
    }
    println!("Elapsed time: {}ms", t0.elapsed().as_millis());
}
