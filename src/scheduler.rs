use std::{
    collections::VecDeque,
    sync::{atomic::AtomicUsize, Arc, Condvar, Mutex, MutexGuard, PoisonError},
    thread::{self, JoinHandle},
};

use crate::{Instance, JobFunction, JobId, JobKind, SceneState, VersionedIndexId};

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

struct JobState {
    id: JobId,
    function: JobFunction,
    dependency_count: usize,
    dependencies_finished: AtomicUsize,
    required_for: Vec<JobId>,
}

pub struct Scheduler {
    worker: Vec<JoinHandle<()>>,

    // These are the jobs without any dependencies. They can be enqueued directly at the beginning
    // of each frame.
    jobs_without_dependencies: Vec<JobId>,

    jobs: Arc<Vec<Option<JobState>>>,

    // The number of jobs. This can be different that jobs.len() if there are holes in the array.
    job_count: usize,

    // The jobs that are available for executing
    available_jobs: Arc<SimpleCondvar<VecDeque<JobId>>>,

    // The number of jobs that have been sucessfully executed. This is used to determine when there
    // frame is complete. TODO: currently the "main" thread is woken up after each completed job.
    // This should be changed to only be woken up by the last job.
    jobs_finished: Arc<SimpleCondvar<usize>>,
    // state: Arc<SceneState>,
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
        let jobs_finished = Arc::new(SimpleCondvar::new(0));
        // let available_jobs = Arc::new(Queue::<JobId>::new());

        for i in 0..worker_count {
            let jobs = jobs.clone();
            // let jobs_finished_barrier = jobs_finished_barrier.clone();
            // let frame_number = frame_number.clone();
            let state = state.clone();
            let available_jobs = available_jobs.clone();
            let jobs_finished = jobs_finished.clone();
            // let available_jobs = available_jobs.clone();

            let jobs_finished = jobs_finished.clone();

            worker.push(thread::spawn(move || {
                println!("[{i}]: spawned");
                loop {
                    // println!("[{i}]: waiting for job");
                    let job_id = available_jobs.wait_mut(|jobs| jobs.pop_front());

                    // println!("[{i}]: executing job {}", job_id);
                    let job = unsafe { jobs[job_id.index()].as_ref().unwrap_unchecked() };
                    (job.function)(&state);
                    jobs_finished.mutate_and_notify_all(|c| *c += 1);

                    for dependent_job_id in &job.required_for {
                        let dependent_job =
                            unsafe { jobs[dependent_job_id.index()].as_ref().unwrap_unchecked() };
                        if dependent_job
                            .dependencies_finished
                            .fetch_add(1, std::sync::atomic::Ordering::Relaxed)
                            == dependent_job.dependency_count - 1
                        {
                            // print!("[{i}]: push {}", *dependent_job_id);
                            available_jobs
                                .mutate_and_notify_one(|jobs| jobs.push_back(*dependent_job_id));
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
            // state,
            jobs_finished,
            job_count,
        };
    }

    pub fn run_jobs(&self) {
        *self.jobs_finished.get_mut().unwrap() = 0;
        for job in &*self.jobs {
            if let Some(job) = job {
                job.dependencies_finished
                    .store(0, std::sync::atomic::Ordering::Relaxed);
            }
        }

        // println!("=== Start Frame ===");

        // for id in &self.jobs_without_dependencies {
        //     // println!("push: {}", *id);
        //     self.available_jobs.mutate_and_notify_one(|jobs| jobs.push_back(*id));
        // }
        // Not sure whether the above or this is faster.
        self.available_jobs
            .mutate_and_notify_all(|jobs| jobs.extend(self.jobs_without_dependencies.iter()));

        self.jobs_finished.wait(|c| *c == self.job_count);
        // println!("=== End Frame ===");
    }
}
