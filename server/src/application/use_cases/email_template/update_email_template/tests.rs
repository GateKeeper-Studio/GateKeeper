use std::sync::Arc;

use uuid::Uuid;

use crate::application::use_cases::email_template::update_email_template::{
    command::Command, use_case::UseCase,
};
use crate::domain::entities::email_template::EmailTemplateType;
use crate::testing::{
    fake_email_renderer::FakeEmailRenderer,
    in_memory_repositories::in_memory_email_template_repository::InMemoryEmailTemplateRepository,
};

fn use_case() -> (UseCase, Arc<InMemoryEmailTemplateRepository>) {
    let repo = Arc::new(InMemoryEmailTemplateRepository::new());
    let uc = UseCase::new(repo.clone(), Arc::new(FakeEmailRenderer::new()));
    (uc, repo)
}

fn command(template_type: EmailTemplateType, body: &str) -> Command {
    Command {
        tenant_id: Uuid::now_v7(),
        template_type,
        subject: "Confirm {{app_name}}".to_string(),
        body_mjml: body.to_string(),
    }
}

#[tokio::test]
async fn stores_a_valid_template() {
    let (uc, _repo) = use_case();
    let response = uc
        .execute(command(
            EmailTemplateType::EmailVerification,
            "<mjml><mj-body>{{code}} {{app_name}}</mj-body></mjml>",
        ))
        .await
        .unwrap();

    assert!(response.is_customized);
    assert!(response.updated_at.is_some());
}

#[tokio::test]
async fn rejects_unknown_placeholder() {
    let (uc, _repo) = use_case();
    let err = uc
        .execute(command(
            EmailTemplateType::EmailVerification,
            "<mjml><mj-body>{{password}}</mj-body></mjml>",
        ))
        .await
        .unwrap_err();

    assert!(matches!(
        err,
        crate::domain::errors::DomainError::EmailTemplateError(_)
    ));
}

#[tokio::test]
async fn rejects_inline_script() {
    let (uc, _repo) = use_case();
    let err = uc
        .execute(command(
            EmailTemplateType::EmailVerification,
            "<mjml><mj-body><script>alert(1)</script>{{code}}</mj-body></mjml>",
        ))
        .await
        .unwrap_err();

    assert!(matches!(
        err,
        crate::domain::errors::DomainError::EmailTemplateError(_)
    ));
}
