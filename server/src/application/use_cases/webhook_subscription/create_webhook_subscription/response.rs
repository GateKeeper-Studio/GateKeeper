use serde::Serialize;

use crate::application::use_cases::webhook_subscription::subscription_view::SubscriptionView;

// The ONLY time the raw signing secret is ever returned. It is stored encrypted and
// is unrecoverable afterwards — same contract as an OAuth client secret.
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Response {
    #[serde(flatten)]
    pub subscription: SubscriptionView,
    pub secret: String,
}
