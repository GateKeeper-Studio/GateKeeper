use std::sync::Arc;

use crate::{
    application::services::templated_email_sender::TemplatedEmailSender,
    testing::{
        fake_email_renderer::FakeEmailRenderer,
        in_memory_repositories::in_memory_email_template_repository::InMemoryEmailTemplateRepository,
        recording_email_sender::RecordingEmailSender,
    },
};

/// Builds a `TemplatedEmailSender` backed by a recording transport, an empty
/// in-memory template store (so the registry defaults are used), and the naive
/// fake renderer. Returns the recording sender too, so tests can assert what was
/// delivered after rendering.
pub fn recording_templated_email_sender() -> (Arc<TemplatedEmailSender>, Arc<RecordingEmailSender>)
{
    let recording = Arc::new(RecordingEmailSender::new());
    let templated = Arc::new(TemplatedEmailSender::new(
        Arc::new(InMemoryEmailTemplateRepository::new()),
        Arc::new(FakeEmailRenderer::new()),
        recording.clone(),
    ));
    (templated, recording)
}
