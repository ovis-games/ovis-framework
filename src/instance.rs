use crate::{IdMap, Job, JobKind, JobFunction, JobId};

pub struct Instance {
    jobs: IdMap<Job>,
}

impl Instance {
    pub fn new() -> Self {
        return Self { jobs: IdMap::new() };
    }

    pub fn jobs(&self) -> &IdMap<Job> {
        return &self.jobs;
    }

    pub fn register_job(&mut self, kind: JobKind, function: JobFunction) -> JobId {
        return self.jobs.insert(Job::new(kind, function)).0;
    }
}
