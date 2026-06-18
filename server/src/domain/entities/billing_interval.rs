use serde::{Deserialize, Serialize};

use crate::domain::errors::DomainError;

// How often a subscription plan bills. Persisted as a lowercase string so the
// column stays human-readable and stable across renames of the Rust variants.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum BillingInterval {
    Monthly,
    Yearly,
}

impl BillingInterval {
    pub fn as_str(&self) -> &'static str {
        match self {
            BillingInterval::Monthly => "monthly",
            BillingInterval::Yearly => "yearly",
        }
    }

    pub fn from_str(value: &str) -> Result<Self, DomainError> {
        match value {
            "monthly" => Ok(BillingInterval::Monthly),
            "yearly" => Ok(BillingInterval::Yearly),
            other => Err(DomainError::InvalidRequest(format!(
                "Invalid billing interval: '{other}' (expected 'monthly' or 'yearly')"
            ))),
        }
    }
}
