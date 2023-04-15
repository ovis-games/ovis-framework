use crate::{IdStorage, Instance, Scheduler, JobKind};

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
    state: SceneState,
    scheduler: Scheduler,
}

impl Scene {
    pub fn new(instance: &Instance) -> Self {
        return Self {
            state: SceneState::new(),
            scheduler: Scheduler::new(instance, JobKind::Update, 2),
        };
    }

    pub fn entities(&self) -> &IdStorage {
        return &self.state.entities;
    }

    pub fn viewports(&self) -> &IdStorage {
        return &self.state.viewports;
    }

    pub fn tick(&mut self, _delta_time: f32) -> bool {
        self.scheduler.run_jobs(&mut self.state);
        return true;
    }
}
