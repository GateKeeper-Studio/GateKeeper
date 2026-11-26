use std::sync::Arc;

use crate::{
    application::use_cases::trusted_device::{
        list::{query::Query, response::Response},
        shared::TrustedDeviceView,
    },
    domain::{
        errors::DomainError, repositories::trusted_device_repository::TrustedDeviceRepository,
    },
};

// GET /me/trusted-devices and GET /tenants/{t}/users/{u}/trusted-devices.
pub struct UseCase {
    devices: Arc<dyn TrustedDeviceRepository>,
}

impl UseCase {
    pub fn new(devices: Arc<dyn TrustedDeviceRepository>) -> Self {
        Self { devices }
    }

    pub async fn execute(&self, query: Query) -> Result<Response, DomainError> {
        let devices = self
            .devices
            .list_for_user(query.tenant_id, query.user_id)
            .await?;
        Ok(Response {
            devices: devices.iter().map(TrustedDeviceView::from).collect(),
        })
    }
}
