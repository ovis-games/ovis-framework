use std::{
    sync::{
        atomic::{AtomicPtr, AtomicUsize},
        Arc, Barrier, Condvar, Mutex, MutexGuard, PoisonError,
    },
    thread::{self, JoinHandle},
};

use crate::{JobFunction, JobId, JobKind, Instance, SceneState};

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

    fn get_mut(&self) -> Result<MutexGuard<'_, T>, PoisonError<MutexGuard<'_, T>>> {
        self.mutex.lock()
    }

    fn wait<P: FnMut(&T) -> bool>(&self, mut p: P) {
        let mut guard = self.mutex.lock().unwrap();
        while !p(&guard) {
            guard = self.cond_var.wait(guard).unwrap();
        }
    }

    fn notify_all(&self) {
        self.cond_var.notify_all();
    }
}

struct SchedulerJob {
    id: JobId,
    finished: Arc<SimpleCondvar<bool>>,
    function: JobFunction,
}

#[derive(PartialEq)]
enum SchedulerState {
    WaitingToRun,
    Running,
    Finished,
}

pub struct Scheduler {
    worker: Vec<JoinHandle<()>>,
    schedulder_state_cvar: Arc<SimpleCondvar<SchedulerState>>,
    jobs: Arc<Vec<SchedulerJob>>,
    jobs_finished_barrier: Arc<Barrier>,
    current_job: Arc<AtomicUsize>,
    state: Arc<AtomicPtr<SceneState>>,
    frame_number: Arc<SimpleCondvar<usize>>,
}

impl Scheduler {
    pub fn new(instance: &Instance, kind: JobKind, worker_count: usize) -> Self {
        let state = Arc::new(AtomicPtr::default());
        let mut worker: Vec<JoinHandle<()>> = Vec::with_capacity(worker_count);
        let schedulder_state_cvar = Arc::new(SimpleCondvar::new(SchedulerState::WaitingToRun));

        let jobs = instance
            .jobs()
            .into_iter()
            .map(|(id, job)| SchedulerJob {
                id,
                finished: Arc::new(SimpleCondvar::new(false)),
                function: job.function(),
            })
            .collect::<Vec<_>>();
        let jobs = Arc::new(jobs);
        let current_job = Arc::new(AtomicUsize::new(0));
        let jobs_finished_barrier = Arc::new(Barrier::new(worker_count + 1));
        let frame_number = Arc::new(SimpleCondvar::new(0));

        for i in 0..worker_count {
            let schedulder_state_cvar = schedulder_state_cvar.clone();
            let jobs = jobs.clone();
            let current_job = current_job.clone();
            let jobs_finished_barrier = jobs_finished_barrier.clone();
            let frame_number = frame_number.clone();
            let state = state.clone();

            worker.push(thread::spawn(move || {
                let mut current_frame_number = 0;
                println!("[{i}] Spawned");

                loop {
                    // println!("[{i}] Wait for next frame");
                    frame_number.wait(|n| {
                        if current_frame_number != *n {
                            current_frame_number = *n;
                            return true;
                        } else {
                            return false;
                        }
                    });
                    // println!("[{i}] Start frame {current_frame_number}");
                    let state: &SceneState = unsafe { &*state.load(std::sync::atomic::Ordering::Acquire) };

                    loop {
                        let job_index =
                            current_job.fetch_add(1, std::sync::atomic::Ordering::AcqRel);

                        if job_index < jobs.len() {
                            // println!("[{i}] Run job {}", jobs[job_index].id);
                            (jobs[job_index].function)(state);
                        } else {
                            break;
                        }
                    }

                    // println!("[{i}] Wait for frame finish");
                    jobs_finished_barrier.wait();
                }
            }));
        }

        return Self {
            worker,
            schedulder_state_cvar,
            jobs,
            jobs_finished_barrier,
            current_job,
            frame_number,
            state,
        };
    }

    pub fn run_jobs(&self, state: &mut SceneState) {
        self.state
            .store(state as *mut SceneState, std::sync::atomic::Ordering::Release);
        self.current_job
            .store(0, std::sync::atomic::Ordering::Release);
        // println!(
        //     "=== Start Frame {} ===",
        //     self.frame_number.get_mut().unwrap()
        // );
        self.frame_number
            .mutate_and_notify_all(|number| *number = number.wrapping_add(1));

        // for job in self.jobs.iter() {
        //     job.finished.wait(|finished| *finished);
        //     println!("job {} finished", job.id);
        // }
        self.jobs_finished_barrier.wait();
        // println!("=== End Frame {} ===", self.frame_number.get_mut().unwrap());
    }
}
