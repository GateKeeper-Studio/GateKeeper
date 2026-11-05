use std::sync::Arc;

use uuid::Uuid;

use crate::application::use_cases::email_template::get_email_template::{
    query::Query, use_case::UseCase,
};
use crate::domain::{
    entities::email_template::{EmailTemplate, EmailTemplateType},
    repositories::email_template_repository::EmailTemplateRepository,
};
use crate::testing::in_memory_repositories::in_memory_email_template_repository::InMemoryEmailTemplateRepository;

#[tokio::test]
async fn returns_default_when_not_customized() {
    let repo = Arc::new(InMemoryEmailTemplateRepository::new());
    let use_case = UseCase::new(repo);

    let response = use_case
        .execute(Query {
            tenant_id: Uuid::now_v7(),
            template_type: EmailTemplateType::EmailVerification,
        })
        .await
        .unwrap();

    assert!(!response.is_customized);
    assert!(response.updated_at.is_none());
    assert_eq!(response.template_type, "email_verification");
    assert!(response.body_mjml.contains("{{code}}"));
    assert!(response.variables.contains(&"code".to_string()));
}

#[tokio::test]
async fn returns_customized_when_present() {
    let tenant_id = Uuid::now_v7();
    let repo = Arc::new(InMemoryEmailTemplateRepository::new());
    repo.upsert(&EmailTemplate::new(
        tenant_id,
        EmailTemplateType::MfaCode,
        "Custom subject".to_string(),
        "<mjml><mj-body></mj-body></mjml>".to_string(),
    ))
    .await
    .unwrap();

    let use_case = UseCase::new(repo);
    let response = use_case
        .execute(Query {
            tenant_id,
            template_type: EmailTemplateType::MfaCode,
        })
        .await
        .unwrap();

    assert!(response.is_customized);
    assert_eq!(response.subject, "Custom subject");
    assert!(response.updated_at.is_some());
}
