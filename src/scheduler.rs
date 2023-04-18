use std::{
    collections::VecDeque,
    sync::{atomic::{AtomicUsize, AtomicU32}, Arc, Condvar, Mutex, MutexGuard, PoisonError, RwLockReadGuard, mpsc::{self, Sender}},
    thread::{self, JoinHandle},
};

use wgpu::CommandEncoderDescriptor;

use crate::{Instance, JobFunction, JobId, JobKind, SceneState, VersionedIndexId, Error, SourceLocation, EntityDescriptor, EntityId};

struct SimpleCondvar<T> {
    mutex: Mutex<T>,
    cond_var: Condvar,
}

impl<T> SimpleCondvar<T> {
    fn new(initial_value: T) -> Self {
        return Self {
            mutex: Mutex::new(initial_value),
            cond_var: Condvar::new(),
        };
    }

    fn mutate_and_notify_all<F: Fn(&mut T)>(&self, f: F) {
        f(&mut self.mutex.lock().unwrap());
        self.cond_var.notify_all();
    }

    fn mutate_and_notify_one<F: Fn(&mut T)>(&self, f: F) {
        f(&mut self.mutex.lock().unwrap());
        self.cond_var.notify_one();
    }

    fn get_mut(&self) -> Result<MutexGuard<'_, T>, PoisonError<MutexGuard<'_, T>>> {
        self.mutex.lock()
    }

    fn wait<P: FnMut(&T) -> bool>(&self, mut p: P) {
        let mut guard = self.mutex.lock().unwrap();
        while !p(&guard) {
            guard = self.cond_var.wait(guard).unwrap();
        }
    }

    fn wait_mut<V, P: FnMut(&mut T) -> Option<V>>(&self, mut p: P) -> V {
        let mut guard = self.mutex.lock().unwrap();
        loop {
            if let Some(value) = p(&mut guard) {
                return value;
            }
            guard = self.cond_var.wait(guard).unwrap();
        }
    }

    fn notify_one(&self) {
        self.cond_var.notify_one();
    }

    fn notify_all(&self) {
        self.cond_var.notify_all();
    }
}

pub struct SystemResources {
    game_time: f32,
    delta_time: f32,
    entity_spawner: Sender<EntityDescriptor>,
    entity_despawner: Sender<EntityId>,
    // command_encoders: Vec<wgpu::CommandEncoder>,
}

impl SystemResources {
    pub fn game_time(&self) -> f32 {
        self.game_time
    }

    pub fn delta_time(&self) -> f32 {
        self.delta_time
    }

    pub fn entity_despawner(&self) -> &Sender<EntityId> {
        &self.entity_despawner
    }

    pub fn entity_spawner(&self) -> &Sender<EntityDescriptor> {
        &self.entity_spawner
    }

    // pub fn command_encoders(&mut self) -> &mut [wgpu::CommandEncoder] {
    //     self.command_encoders.as_mut()
    // }
}

struct JobState {
    id: JobId,
    function: JobFunction,
    dependency_count: usize,
    dependencies_finished: AtomicUsize,
    required_for: Vec<JobId>,
}

pub struct Scheduler {
    worker: Vec<JoinHandle<()>>,
    state: Arc<SceneState>,

    // These are the jobs without any dependencies. They can be enqueued directly at the beginning
    // of each frame.
    jobs_without_dependencies: Vec<JobId>,

    jobs: Arc<Vec<Option<JobState>>>,

    // The jobs that are available for executing
    available_jobs: Arc<SimpleCondvar<VecDeque<JobId>>>,

    jobs_finished: Arc<AtomicUsize>,
    frame_finished_receiver: mpsc::Receiver<crate::Result<()>>,

    delta_time: Arc<AtomicU32>,
    game_time: Arc<AtomicU32>,
    spawned_entities_receiver: mpsc::Receiver<EntityDescriptor>,
    despawned_entities_receiver: mpsc::Receiver<EntityId>,
}

impl Scheduler {
    pub fn new(
        instance: &Instance,
        kind: JobKind,
        state: Arc<SceneState>,
        worker_count: usize,
    ) -> Self {
        let mut worker: Vec<JoinHandle<()>> = Vec::with_capacity(worker_count);

        let mut job_count = 0;
        let mut jobs = Vec::<Option<JobState>>::new();
        let mut jobs_without_dependencies = Vec::<JobId>::new();
        for (job_id, job) in instance
            .jobs()
            .into_iter()
            .filter(|(_, job)| job.kind() == kind)
        {
            if job_id.index() >= jobs.len() {
                jobs.resize_with(job_id.index() + 1, || None);
            }
            jobs[job_id.index()] = Some(JobState {
                id: job_id,
                function: job.function(),
                dependency_count: job.dependencies().len(),
                dependencies_finished: AtomicUsize::new(0),
                required_for: vec![],
            });
            if job.dependencies().len() == 0 {
                jobs_without_dependencies.push(job_id);
            }
            job_count += 1;
        }

        for (job_id, job) in instance
            .jobs()
            .into_iter()
            .filter(|(_, job)| job.kind() == kind)
        {
            for dependency in job.dependencies() {
                jobs[dependency.index()]
                    .as_mut()
                    .unwrap()
                    .required_for
                    .push(job_id);
            }
        }

        let jobs = Arc::new(jobs);
        let available_jobs = Arc::new(SimpleCondvar::new(VecDeque::<JobId>::new()));
        let jobs_finished = Arc::new(AtomicUsize::new(0));
        let game_time = Arc::new(AtomicU32::new(0));
        let delta_time = Arc::new(AtomicU32::new(0));
        let (frame_finished_sender, frame_finished_receiver) = mpsc::channel::<crate::Result<()>>();
        let (spawned_entities_sender, spawned_entities_receiver) = mpsc::channel::<EntityDescriptor>();
        let (despawned_entities_sender, despawned_entities_receiver) = mpsc::channel::<EntityId>();

        for i in 0..worker_count {
            let jobs = jobs.clone();
            let state = state.clone();
            let available_jobs = available_jobs.clone();
            let jobs_finished = jobs_finished.clone();
            let game_time = game_time.clone();
            let delta_time = delta_time.clone();
            let job_count = job_count.clone();
            let frame_finished_sender = frame_finished_sender.clone();
            let spawned_entities_sender = spawned_entities_sender.clone();
            let despawned_entities_sender = despawned_entities_sender.clone();
            // let encoders = state.gpus().into_iter().map(|gpu| gpu.device().create_command_encoder(&CommandEncoderDescriptor { label: Some("") })).collect();

            worker.push(thread::spawn(move || {
                println!("[{i}]: spawned");
                let mut system_resources = SystemResources {
                    game_time: 0.0,
                    delta_time: 0.0,
                    entity_spawner: spawned_entities_sender,
                    entity_despawner: despawned_entities_sender,
                    // command_encoders: encoders,
                };

                loop {
                    // println!("[{i}]: waiting for job");
                    let job_id = available_jobs.wait_mut(|jobs| jobs.pop_front());

                    system_resources.game_time = f32::from_ne_bytes(game_time.load(std::sync::atomic::Ordering::Relaxed).to_ne_bytes());
                    system_resources.delta_time = f32::from_ne_bytes(delta_time.load(std::sync::atomic::Ordering::Relaxed).to_ne_bytes());

                    // println!("[{i}]: executing job {}", job_id);
                    let job = unsafe { jobs[job_id.index()].as_ref().unwrap_unchecked() };
                    if let Err(error) = (job.function)(&mut system_resources, &state) {
                        frame_finished_sender.send(Err(error)).expect("channel send failure");
                    } else {
                        if jobs_finished.fetch_add(1, std::sync::atomic::Ordering::Relaxed) == job_count - 1 {
                            frame_finished_sender.send(Ok(())).expect("channel send failure");
                        } else {
                            for dependent_job_id in &job.required_for {
                                let dependent_job = unsafe {
                                    jobs[dependent_job_id.index()].as_ref().unwrap_unchecked()
                                };
                                if dependent_job
                                    .dependencies_finished
                                    .fetch_add(1, std::sync::atomic::Ordering::Relaxed)
                                    == dependent_job.dependency_count - 1
                                {
                                    // print!("[{i}]: push {}", *dependent_job_id);
                                    available_jobs.mutate_and_notify_one(|jobs| {
                                        jobs.push_back(*dependent_job_id)
                                    });
                                }
                            }
                        }
                    }
                }
            }));
        }

        return Self {
            jobs_without_dependencies,
            worker,
            jobs,
            available_jobs,
            jobs_finished,
            frame_finished_receiver,
            game_time,
            delta_time,
            spawned_entities_receiver,
            despawned_entities_receiver,
            state,
        };
    }

    pub fn run_jobs(&self, game_time: f32, delta_time: f32) -> crate::Result<()> {
        self.game_time.store(u32::from_ne_bytes(game_time.to_ne_bytes()), std::sync::atomic::Ordering::Relaxed);
        self.delta_time.store(u32::from_ne_bytes(delta_time.to_ne_bytes()), std::sync::atomic::Ordering::Relaxed);
        self.jobs_finished.store(0, std::sync::atomic::Ordering::Relaxed);
        for job in &*self.jobs {
            if let Some(job) = job {
                job.dependencies_finished
                    .store(0, std::sync::atomic::Ordering::Relaxed);
            }
        }

        // println!("=== Start Frame ===");
        //
        // let entities = self.state.entities().read().unwrap();

        // for id in &self.jobs_without_dependencies {
        //     // println!("push: {}", *id);
        //     self.available_jobs.mutate_and_notify_one(|jobs| jobs.push_back(*id));
        // }
        // Not sure whether the above or this is faster.
        self.available_jobs
            .mutate_and_notify_all(|jobs| jobs.extend(self.jobs_without_dependencies.iter()));

        match self.frame_finished_receiver.recv() {
            Ok(Ok(_)) => {},
            Ok(Err(error)) => return Err(error),
            Err(error) => return Err(Error::new(error.to_string(), SourceLocation::here())),
        };

        let mut entities = self.state.entities().write().unwrap();

        for entity_to_remove in self.despawned_entities_receiver.try_iter() {
            println!("despawn entity: {}", entity_to_remove);
            entities.free(entity_to_remove);
        }
        for _entity_to_spawn in self.spawned_entities_receiver.try_iter() {
            println!("spawned entity {}", entities.reserve());
        }

        // println!("=== End Frame ===");
        return Ok(());
    }
}
