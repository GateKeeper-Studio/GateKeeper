use crate::application::ports::task_spawner::{BackgroundTask, TaskSpawner};

// Production `TaskSpawner`: a detached tokio task. Nobody awaits it, so a failure can
// only be logged — the request that spawned it has already been answered.
pub struct TokioTaskSpawner;

impl TaskSpawner for TokioTaskSpawner {
    fn spawn(&self, name: &'static str, task: BackgroundTask) {
        tokio::spawn(async move {
            if let Err(error) = task.await {
                tracing::warn!(task = name, error = %error, "background task failed");
            }
        });
    }
}
