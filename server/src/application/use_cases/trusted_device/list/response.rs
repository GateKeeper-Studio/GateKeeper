use serde::Serialize;

use crate::application::use_cases::trusted_device::shared::TrustedDeviceView;

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Response {
    pub devices: Vec<TrustedDeviceView>,
}
