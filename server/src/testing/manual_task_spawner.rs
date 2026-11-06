use std::sync::Mutex;

use crate::{
    application::ports::task_spawner::{BackgroundTask, TaskSpawner},
    domain::errors::DomainError,
};

// Test `TaskSpawner`: queues every task until the test calls `run_all`. Lets a test
// assert what the caller did BEFORE the background work ran (e.g. that a request
// answered without sending any email), and then what the background work did.
#[derive(Default)]
pub struct ManualTaskSpawner {
    queue: Mutex<Vec<(&'static str, BackgroundTask)>>,
}

impl ManualTaskSpawner {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn pending(&self) -> usize {
        self.queue.lock().unwrap().len()
    }

    // Runs every queued task in order and returns their results.
    pub async fn run_all(&self) -> Vec<(&'static str, Result<(), DomainError>)> {
        let tasks: Vec<_> = std::mem::take(&mut *self.queue.lock().unwrap());
        let mut results = Vec::with_capacity(tasks.len());
        for (name, task) in tasks {
            results.push((name, task.await));
        }
        results
    }
}

impl TaskSpawner for ManualTaskSpawner {
    fn spawn(&self, name: &'static str, task: BackgroundTask) {
        self.queue.lock().unwrap().push((name, task));
    }
}
