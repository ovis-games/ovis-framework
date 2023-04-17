use crate::{IdMap, Job, JobFunction, JobId, JobKind};

pub struct Instance {
    jobs: IdMap<Job, JobId>,
}

impl Instance {
    pub fn new() -> Self {
        return Self { jobs: IdMap::new() };
    }

    pub fn jobs(&self) -> &IdMap<Job, JobId> {
        return &self.jobs;
    }

    pub fn register_job(&mut self, kind: JobKind, function: JobFunction) -> JobId {
        return self.jobs.insert(Job::new(kind, function)).0;
    }

    pub fn add_job_dependency(&mut self, job_id: JobId, dependency_id: JobId) {
        if let Some(dependency) = self.jobs.get(dependency_id) {
            let dependency_kind = dependency.kind();
            if let Some(job) = self.jobs.get_mut(job_id) {
                if job.kind() == dependency_kind {
                    job.add_dependency(dependency_id);
                }
            }
        }
    }
}
