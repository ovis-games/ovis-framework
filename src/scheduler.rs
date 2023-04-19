use std::{
    collections::{HashMap, VecDeque},
    sync::{
        atomic::{AtomicU32, AtomicUsize},
        mpsc::{self, Sender},
        Arc, Condvar, Mutex, MutexGuard, PoisonError, RwLock,
    },
    thread::{self, JoinHandle},
};

use crate::{
    EntityDescriptor, EntityId, Error, Instance, JobFunction, JobId, JobKind, SceneState,
    SourceLocation, Viewport, ViewportId,
};

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

pub struct SystemResources<'a> {
    game_time: f32,
    delta_time: f32,
    entity_spawner: &'a Sender<EntityDescriptor>,
    entity_despawner: &'a Sender<EntityId>,
    viewport: Option<&'a Viewport>,
    pipeline: Option<&'a wgpu::RenderPipeline>,
}

impl SystemResources<'_> {
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

    pub fn viewport(&self) -> Option<&Viewport> {
        self.viewport
    }

    pub fn pipeline(&self) -> Option<&wgpu::RenderPipeline> {
        self.pipeline
    }
}

struct JobState {
    id: JobId,
    function: JobFunction,
    regular_dependency_count: usize,
    per_viewport_dependency_count: usize,
    dependencies_finished: AtomicUsize,
    required_for: Vec<usize>,
    executed_per_viewport: bool,
}

struct ScheduledJob {
    job_index: usize,
    viewport_id: Option<ViewportId>,
}

pub struct Scheduler {
    worker: Vec<JoinHandle<()>>,
    state: Arc<SceneState>,

    // The state of all jobs.
    jobs: Arc<Vec<JobState>>,
    // These are the jobs without any dependencies. They can be enqueued directly at the beginning
    // of each frame.
    jobs_without_dependencies: Vec<usize>,

    // The jobs that are available for executing
    available_jobs: Arc<SimpleCondvar<VecDeque<ScheduledJob>>>,

    jobs_finished: Arc<AtomicUsize>,
    frame_finished_receiver: mpsc::Receiver<crate::Result<()>>,

    delta_time: Arc<AtomicU32>,
    game_time: Arc<AtomicU32>,
    spawned_entities_receiver: mpsc::Receiver<EntityDescriptor>,
    despawned_entities_receiver: mpsc::Receiver<EntityId>,

    pipelines: Arc<RwLock<HashMap<(usize, ViewportId), wgpu::RenderPipeline>>>,
}

impl Scheduler {
    pub fn new(
        instance: &Instance,
        kind: JobKind,
        state: Arc<SceneState>,
        worker_count: usize,
    ) -> Self {
        let mut worker: Vec<JoinHandle<()>> = Vec::with_capacity(worker_count);

        let mut jobs = Vec::<JobState>::new();
        let mut jobs_without_dependencies = Vec::<usize>::new();
        let mut job_state_indices = HashMap::<JobId, usize>::new();

        let mut regular_job_count = 0_usize;
        let mut per_viewport_job_count = 0_usize;

        for (job_id, job) in instance
            .jobs()
            .into_iter()
            .filter(|(_, job)| job.kind() == kind)
        {
            let job_index = jobs.len();
            job_state_indices.insert(job_id, job_index);
            jobs.push(JobState {
                id: job_id,
                function: job.function(),
                regular_dependency_count: 0,
                per_viewport_dependency_count: 0,
                dependencies_finished: AtomicUsize::new(0),
                required_for: vec![],
                executed_per_viewport: true,
            });
            per_viewport_job_count += 1;
            if job.dependencies().len() == 0 {
                jobs_without_dependencies.push(job_index);
            }
        }

        for (job_id, job) in instance
            .jobs()
            .into_iter()
            .filter(|(_, job)| job.kind() == kind)
        {
            for dependency in job.dependencies() {
                if jobs[job_state_indices[dependency]].executed_per_viewport {
                    jobs[job_state_indices[&job_id]].per_viewport_dependency_count += 1;
                } else {
                    jobs[job_state_indices[&job_id]].regular_dependency_count += 1;
                }

                jobs[job_state_indices[dependency]]
                    .required_for
                    .push(job_state_indices[&job_id]);
            }
        }

        let jobs = Arc::new(jobs);
        let pipelines = Arc::new(RwLock::new(HashMap::new()));
        let available_jobs = Arc::new(SimpleCondvar::new(VecDeque::<ScheduledJob>::new()));
        let jobs_finished = Arc::new(AtomicUsize::new(0));
        let game_time = Arc::new(AtomicU32::new(0));
        let delta_time = Arc::new(AtomicU32::new(0));
        let (frame_finished_sender, frame_finished_receiver) = mpsc::channel::<crate::Result<()>>();
        let (spawned_entities_sender, spawned_entities_receiver) =
            mpsc::channel::<EntityDescriptor>();
        let (despawned_entities_sender, despawned_entities_receiver) = mpsc::channel::<EntityId>();

        for i in 0..worker_count {
            let jobs = jobs.clone();
            let state = state.clone();
            let available_jobs = available_jobs.clone();
            let jobs_finished = jobs_finished.clone();
            let game_time = game_time.clone();
            let delta_time = delta_time.clone();
            let frame_finished_sender = frame_finished_sender.clone();
            let spawned_entities_sender = spawned_entities_sender.clone();
            let despawned_entities_sender = despawned_entities_sender.clone();
            let pipelines = pipelines.clone();

            worker.push(thread::spawn(move || {
                println!("[{i}]: spawned");

                loop {
                    let scheduled_job = available_jobs.wait_mut(|jobs| jobs.pop_front());
                    let viewports = state.viewports().read().unwrap();
                    let job_index = scheduled_job.job_index;
                    let viewport_id = scheduled_job.viewport_id;
                    let pipelines = pipelines.read().unwrap();

                    let system_resources = SystemResources {
                        game_time: f32::from_ne_bytes(
                            game_time
                                .load(std::sync::atomic::Ordering::Relaxed)
                                .to_ne_bytes(),
                        ),
                        delta_time: f32::from_ne_bytes(
                            delta_time
                                .load(std::sync::atomic::Ordering::Relaxed)
                                .to_ne_bytes(),
                        ),
                        entity_spawner: &spawned_entities_sender,
                        entity_despawner: &despawned_entities_sender,
                        viewport: scheduled_job
                            .viewport_id
                            .map(|id| viewports.get(id).unwrap()),
                        pipeline: viewport_id.and_then(|id| pipelines.get(&(job_index, id))),
                    };

                    if let Some(viewport_id) = viewport_id {
                        println!("[{i}]: executing job {job_index} for {viewport_id}");
                        // system_resources.viewport = Some(viewports.get(viewport_id).unwrap());
                    } else {
                        println!("[{i}]: executing job {job_index}");
                    }

                    let job = &jobs[job_index];
                    if let Err(error) = (job.function)(&system_resources, &state) {
                        frame_finished_sender
                            .send(Err(error))
                            .expect("channel send failure");
                    } else {
                        let completed_jobs =
                            jobs_finished.fetch_add(1, std::sync::atomic::Ordering::Relaxed) + 1;
                        if completed_jobs
                            == regular_job_count
                                + per_viewport_job_count * state.viewports().read().unwrap().len()
                        {
                            frame_finished_sender
                                .send(Ok(()))
                                .expect("channel send failure");
                        } else {
                            for dependent_job_index in &job.required_for {
                                let dependent_job = &jobs[*dependent_job_index];
                                if dependent_job
                                    .dependencies_finished
                                    .fetch_add(1, std::sync::atomic::Ordering::Relaxed)
                                    == (dependent_job.regular_dependency_count
                                        + dependent_job.per_viewport_dependency_count
                                            * state.viewports().read().unwrap().len())
                                        - 1
                                {
                                    if dependent_job.executed_per_viewport {
                                        for (viewport_id, _) in &*state.viewports().read().unwrap()
                                        {
                                            available_jobs.mutate_and_notify_one(|jobs| {
                                                jobs.push_back(ScheduledJob {
                                                    job_index: *dependent_job_index,
                                                    viewport_id: Some(viewport_id),
                                                });
                                            });
                                        }
                                    } else {
                                        available_jobs.mutate_and_notify_one(|jobs| {
                                            jobs.push_back(ScheduledJob {
                                                job_index: *dependent_job_index,
                                                viewport_id: None,
                                            });
                                        });
                                    }
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
            pipelines,
        };
    }

    pub fn configure_pipelines(&mut self) {
        let mut pipelines = self.pipelines.write().unwrap();
        let viewports = self.state.viewports().read().unwrap();

        pipelines.clear();

        for (job_index, job) in self.jobs.iter().enumerate() {
            for (viewport_id, viewport) in &*viewports {
                let render_pipeline_layout = viewport.gpu().device().create_pipeline_layout(
                    &wgpu::PipelineLayoutDescriptor {
                        label: Some("Render Pipeline Layout"),
                        bind_group_layouts: &[],
                        push_constant_ranges: &[],
                    },
                );

                pipelines.insert(
                    (job_index, viewport_id),
                    viewport.gpu().device().create_render_pipeline(
                        &wgpu::RenderPipelineDescriptor {
                            label: Some("Render Pipeline"),
                            layout: Some(&render_pipeline_layout),
                            vertex: wgpu::VertexState {
                                module: &viewport.gpu().shader_module(),
                                entry_point: "vs_main",
                                buffers: &[],
                            },
                            fragment: Some(wgpu::FragmentState {
                                module: &viewport.gpu().shader_module(),
                                entry_point: "fs_main",
                                targets: &[Some(wgpu::ColorTargetState {
                                    format: viewport.surface_config().format,
                                    blend: Some(wgpu::BlendState::REPLACE),
                                    write_mask: wgpu::ColorWrites::ALL,
                                })],
                            }),
                            primitive: wgpu::PrimitiveState {
                                topology: wgpu::PrimitiveTopology::TriangleList,
                                strip_index_format: None,
                                front_face: wgpu::FrontFace::Ccw,
                                cull_mode: Some(wgpu::Face::Back),
                                polygon_mode: wgpu::PolygonMode::Fill,
                                unclipped_depth: false,
                                conservative: false,
                            },
                            depth_stencil: None,
                            multisample: wgpu::MultisampleState {
                                count: 1,
                                mask: !0,
                                alpha_to_coverage_enabled: false,
                            },
                            multiview: None,
                        },
                    ),
                );
            }
        }
    }

    pub fn run_jobs(&self, game_time: f32, delta_time: f32) -> crate::Result<()> {
        self.game_time.store(
            u32::from_ne_bytes(game_time.to_ne_bytes()),
            std::sync::atomic::Ordering::Relaxed,
        );
        self.delta_time.store(
            u32::from_ne_bytes(delta_time.to_ne_bytes()),
            std::sync::atomic::Ordering::Relaxed,
        );
        self.jobs_finished
            .store(0, std::sync::atomic::Ordering::Relaxed);
        for job in &*self.jobs {
            job.dependencies_finished
                .store(0, std::sync::atomic::Ordering::Relaxed);
        }

        let viewports = self.state.viewports().read().unwrap();

        println!("=== Start Frame ===");
        //
        // let entities = self.state.entities().read().unwrap();

        // Not sure whether the above or this is faster.
        self.available_jobs.mutate_and_notify_all(|jobs| {
            for j in &self.jobs_without_dependencies {
                let job = &self.jobs[*j];
                if job.executed_per_viewport {
                    for (viewport_id, _) in &*viewports {
                        println!("pushing {j} for {viewport_id}");
                        jobs.push_back(ScheduledJob {
                            job_index: *j,
                            viewport_id: Some(viewport_id),
                        });
                    }
                } else {
                    println!("pushing {j}");
                    jobs.push_back(ScheduledJob {
                        job_index: *j,
                        viewport_id: None,
                    });
                }
            }
        });

        match self.frame_finished_receiver.recv() {
            Ok(Ok(_)) => {}
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

        println!("=== End Frame ===");
        return Ok(());
    }
}
