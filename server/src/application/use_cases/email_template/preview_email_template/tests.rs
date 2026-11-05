use std::sync::Arc;

use crate::application::use_cases::email_template::preview_email_template::{
    command::Command, use_case::UseCase,
};
use crate::domain::entities::email_template::EmailTemplateType;
use crate::testing::fake_email_renderer::FakeEmailRenderer;

#[tokio::test]
async fn renders_draft_with_sample_values() {
    let use_case = UseCase::new(Arc::new(FakeEmailRenderer::new()));

    let response = use_case
        .execute(Command {
            template_type: EmailTemplateType::EmailVerification,
            subject: "Hello {{app_name}}".to_string(),
            body_mjml: "<mjml><mj-body>{{code}}</mj-body></mjml>".to_string(),
        })
        .await
        .unwrap();

    // Sample context substitutes app_name -> "Example App" and code -> "123456".
    assert_eq!(response.subject, "Hello Example App");
    assert!(response.html.contains("123456"));
}
