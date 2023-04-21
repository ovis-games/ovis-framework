use std::collections::HashSet;
use crate::{SceneState, StandardVersionedIndexId, Result, SystemResources};

// A `Job` corresponds to a `System` in the classical ECS terminology.
// More concrete, a job is a function that operates on the state of of a scene (scene components,
// entities and their components, events, ...).
// There are two kind of jobs: setup and update jobs. Setup-jobs run once when the scene is
// created. Those can be used to set the initial state of the scene. Update jobs run on every frame
// of the scene.

pub type JobId = StandardVersionedIndexId<>;
pub type JobFunction = fn(&SystemResources, &SceneState) -> Result<()>;

// The kind of job
#[derive(Copy, Clone, PartialEq, Eq)]
pub enum JobKind {
    Setup,
    Update,
}

pub struct Job {
    kind: JobKind,
    function: JobFunction,
    dependencies: HashSet<JobId>,
    // render_pass_descriptor: Option<RenderPassJobDescriptor>,
}

impl Job {
    pub fn new(kind: JobKind, function: JobFunction) -> Self {
        return Self {
            kind,
            function,
            dependencies: HashSet::new(),
            // render_pass_descriptor,
        };
    }

    pub fn dependencies(&self) -> &HashSet<JobId> {
        return &self.dependencies;
    }

    pub fn add_dependency(&mut self, dependency: JobId) {
        self.dependencies.insert(dependency);
    }

    pub fn function(&self) -> JobFunction {
        self.function
    }

    pub fn kind(&self) -> JobKind {
        self.kind
    }
}
