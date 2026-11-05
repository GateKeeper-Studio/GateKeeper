use std::sync::Arc;

use uuid::Uuid;

use crate::application::use_cases::email_template::list_email_templates::{
    query::Query, use_case::UseCase,
};
use crate::domain::{
    entities::email_template::{EmailTemplate, EmailTemplateType},
    repositories::email_template_repository::EmailTemplateRepository,
};
use crate::testing::in_memory_repositories::in_memory_email_template_repository::InMemoryEmailTemplateRepository;

#[tokio::test]
async fn lists_every_situation_with_customization_flags() {
    let tenant_id = Uuid::now_v7();
    let repo = Arc::new(InMemoryEmailTemplateRepository::new());
    repo.upsert(&EmailTemplate::new(
        tenant_id,
        EmailTemplateType::PasswordReset,
        "Reset".to_string(),
        "<mjml><mj-body></mj-body></mjml>".to_string(),
    ))
    .await
    .unwrap();

    let use_case = UseCase::new(repo);
    let response = use_case.execute(Query { tenant_id }).await.unwrap();

    assert_eq!(response.items.len(), EmailTemplateType::ALL.len());

    let reset = response
        .items
        .iter()
        .find(|i| i.template_type == "password_reset")
        .unwrap();
    assert!(reset.is_customized);
    assert!(reset.updated_at.is_some());

    let mfa = response
        .items
        .iter()
        .find(|i| i.template_type == "mfa_code")
        .unwrap();
    assert!(!mfa.is_customized);
    assert!(mfa.updated_at.is_none());
    assert!(!mfa.label.is_empty());
}
