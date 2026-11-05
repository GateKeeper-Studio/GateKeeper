use std::sync::Arc;

use uuid::Uuid;

use crate::application::use_cases::email_template::reset_email_template::{
    command::Command, use_case::UseCase,
};
use crate::domain::{
    entities::email_template::{EmailTemplate, EmailTemplateType},
    repositories::email_template_repository::EmailTemplateRepository,
};
use crate::testing::in_memory_repositories::in_memory_email_template_repository::InMemoryEmailTemplateRepository;

#[tokio::test]
async fn reset_removes_customization_and_returns_default() {
    let tenant_id = Uuid::now_v7();
    let repo = Arc::new(InMemoryEmailTemplateRepository::new());
    repo.upsert(&EmailTemplate::new(
        tenant_id,
        EmailTemplateType::EmailVerification,
        "Custom".to_string(),
        "<mjml><mj-body></mj-body></mjml>".to_string(),
    ))
    .await
    .unwrap();

    let use_case = UseCase::new(repo.clone());
    let response = use_case
        .execute(Command {
            tenant_id,
            template_type: EmailTemplateType::EmailVerification,
        })
        .await
        .unwrap();

    assert!(!response.is_customized);
    // The override is gone — a subsequent lookup falls back to the default.
    assert!(
        repo.find_by_tenant_and_type(tenant_id, EmailTemplateType::EmailVerification)
            .await
            .unwrap()
            .is_none()
    );
}
