use crate::{SceneState, StandardVersionedIndexId};

pub type JobId = StandardVersionedIndexId<>;
pub type JobFunction = fn(&SceneState);

#[derive(Copy, Clone)]
pub enum JobKind {
    Setup,
    Update,
}

pub struct Job {
    kind: JobKind,
    function: JobFunction,
}

impl Job {
    pub fn new(kind: JobKind, function: JobFunction) -> Self {
        return Self {
            kind,
            function,
        };
    }

    pub fn function(&self) -> JobFunction {
        self.function
    }

    pub fn kind(&self) -> JobKind {
        self.kind
    }
}
