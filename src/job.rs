use crate::{Result, SceneState, StandardVersionedIndexId, SystemResources, IdMap, ResourceId};
use lazy_static::lazy_static;
use std::{collections::HashSet, sync::{RwLock, RwLockReadGuard}};

// A `Job` corresponds to a `System` in the classical ECS terminology.
// More concrete, a job is a function that operates on the state of of a scene (scene components,
// entities and their components, events, ...).
// There are two kind of jobs: setup and update jobs. Setup-jobs run once when the scene is
// created. Those can be used to set the initial state of the scene. Update jobs run on every frame
// of the scene.

pub type JobId = StandardVersionedIndexId;
pub type JobFunction = fn(&SystemResources, &SceneState) -> Result<()>;

// The kind of job
#[derive(Copy, Clone, PartialEq, Eq)]
pub enum JobKind {
    Setup,
    Update,
}

#[derive(Copy, Clone, PartialEq, Eq)]
pub enum ResourceAccess {
    Read(ResourceId),
    // Write(ResourceId),
    // ReadWrite(ResourceId),
}

pub struct Job {
    kind: JobKind,
    function: JobFunction,
    dependencies: HashSet<JobId>,
    resource_access: Vec<ResourceAccess>,
}

impl Job {
    pub fn new(kind: JobKind, function: JobFunction, resource_access: &[ResourceAccess]) -> Self {
        return Self {
            kind,
            function,
            dependencies: HashSet::new(),
            resource_access: resource_access.to_vec(),
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

    pub fn resource_access(&self) -> &[ResourceAccess] {
        &self.resource_access
    }
}

lazy_static! {
    static ref REGISTERED_JOBS: RwLock<IdMap<JobId, Job>> = RwLock::new(IdMap::new());
}

pub fn register_job(kind: JobKind, function: JobFunction, resource_access: &[ResourceAccess]) -> JobId {
    return REGISTERED_JOBS.write().unwrap().insert(Job::new(kind, function, resource_access)).0;
}

pub fn add_job_dependency(job_id: JobId, dependency_id: JobId) {
    let mut jobs = REGISTERED_JOBS.write().unwrap();
    if let Some(dependency) = jobs.get(dependency_id) {
        let dependency_kind = dependency.kind();
        if let Some(job) = jobs.get_mut(job_id) {
            if job.kind() == dependency_kind {
                job.add_dependency(dependency_id);
            }
        }
    }
}

pub fn jobs() -> RwLockReadGuard<'static, IdMap<JobId, Job>> {
    return REGISTERED_JOBS.read().unwrap();

}
