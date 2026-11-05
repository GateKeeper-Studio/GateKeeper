use serde::Serialize;

use crate::application::use_cases::webhook_subscription::subscription_view::SubscriptionView;

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Response {
    #[serde(flatten)]
    pub subscription: SubscriptionView,
    // Present ONLY when rotate_secret was requested — the one chance to copy it.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub secret: Option<String>,
}
