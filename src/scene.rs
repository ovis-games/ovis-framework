use std::{thread, sync::Arc};

use crate::{IdStorage, Instance, Scheduler, JobKind, Result};

pub struct SceneState {
    entities: IdStorage,
    viewports: IdStorage,
}
impl SceneState {
    pub fn new() -> Self {
        return Self {
            entities: IdStorage::new(),
            viewports: IdStorage::new(),
        };
    }
}

pub struct Scene {
    state: Arc<SceneState>,
    scheduler: Scheduler,
}

impl Scene {
    pub fn new(instance: &Instance) -> Self {
        let state = Arc::new(SceneState::new());
        return Self {
            scheduler: Scheduler::new(instance, JobKind::Update, state.clone(), thread::available_parallelism().map(|c| -> usize {c.into()}).unwrap_or(4)),
            state,
        };
    }

    pub fn entities(&self) -> &IdStorage {
        return &self.state.entities;
    }

    pub fn viewports(&self) -> &IdStorage {
        return &self.state.viewports;
    }

    pub fn tick(&mut self, _delta_time: f32) -> Result<()> {
        return self.scheduler.run_jobs();
    }
}
