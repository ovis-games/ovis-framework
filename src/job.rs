use std::collections::HashSet;

use crate::{SceneState, StandardVersionedIndexId};

pub type JobId = StandardVersionedIndexId<>;
pub type JobFunction = fn(&SceneState);

#[derive(Copy, Clone, PartialEq, Eq)]
pub enum JobKind {
    Setup,
    Update,
}

pub struct Job {
    kind: JobKind,
    function: JobFunction,
    dependencies: HashSet<JobId>,
}

impl Job {
    pub fn new(kind: JobKind, function: JobFunction) -> Self {
        return Self {
            kind,
            function,
            dependencies: HashSet::new(),
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
