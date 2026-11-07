use chrono::{DateTime, Utc};
use uuid::Uuid;

// The verdict on one login event: the ground truth the statistical risk model learns
// from (docs/RISK-SCORING.md).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RiskLabel {
    Legitimate,
    Fraudulent,
}

impl RiskLabel {
    pub fn as_str(&self) -> &'static str {
        match self {
            RiskLabel::Legitimate => "legitimate",
            RiskLabel::Fraudulent => "fraudulent",
        }
    }

    pub fn parse(value: &str) -> Option<Self> {
        match value {
            "legitimate" => Some(RiskLabel::Legitimate),
            "fraudulent" => Some(RiskLabel::Fraudulent),
            _ => None,
        }
    }
}

// Where a label came from. An admin's review is authoritative; the two MFA outcomes
// are weak evidence gathered automatically.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LabelSource {
    Admin,
    // The user passed the step-up MFA the risk engine forced.
    MfaPassed,
    // The forced step-up's challenge ran out of attempts. Could be an attacker, could
    // be a user who mistyped five times: hence the low weight.
    MfaFailed,
}

impl LabelSource {
    pub fn as_str(&self) -> &'static str {
        match self {
            LabelSource::Admin => "admin",
            LabelSource::MfaPassed => "mfa_passed",
            LabelSource::MfaFailed => "mfa_failed",
        }
    }

    pub fn parse(value: &str) -> Option<Self> {
        match value {
            "admin" => Some(LabelSource::Admin),
            "mfa_passed" => Some(LabelSource::MfaPassed),
            "mfa_failed" => Some(LabelSource::MfaFailed),
            _ => None,
        }
    }

    // How much one label of this source counts in training.
    pub fn training_weight(&self) -> f64 {
        match self {
            LabelSource::Admin => 1.0,
            LabelSource::MfaPassed | LabelSource::MfaFailed => 0.3,
        }
    }

    pub fn is_weak(&self) -> bool {
        !matches!(self, LabelSource::Admin)
    }
}

#[derive(Debug, Clone)]
pub struct LoginEventLabel {
    login_event_id: Uuid,
    tenant_id: Uuid,
    label: RiskLabel,
    source: LabelSource,
    labeled_by: Option<Uuid>,
    note: Option<String>,
    created_at: DateTime<Utc>,
    updated_at: DateTime<Utc>,
}

impl LoginEventLabel {
    pub fn by_admin(
        tenant_id: Uuid,
        login_event_id: Uuid,
        label: RiskLabel,
        admin_id: Option<Uuid>,
        note: Option<String>,
    ) -> Self {
        let now = Utc::now();
        LoginEventLabel {
            login_event_id,
            tenant_id,
            label,
            source: LabelSource::Admin,
            labeled_by: admin_id,
            note,
            created_at: now,
            updated_at: now,
        }
    }

    // A weak label from a step-up MFA outcome.
    pub fn from_mfa(tenant_id: Uuid, login_event_id: Uuid, passed: bool) -> Self {
        let now = Utc::now();
        let (label, source) = if passed {
            (RiskLabel::Legitimate, LabelSource::MfaPassed)
        } else {
            (RiskLabel::Fraudulent, LabelSource::MfaFailed)
        };
        LoginEventLabel {
            login_event_id,
            tenant_id,
            label,
            source,
            labeled_by: None,
            note: None,
            created_at: now,
            updated_at: now,
        }
    }

    #[allow(clippy::too_many_arguments)]
    pub fn reconstruct(
        login_event_id: Uuid,
        tenant_id: Uuid,
        label: RiskLabel,
        source: LabelSource,
        labeled_by: Option<Uuid>,
        note: Option<String>,
        created_at: DateTime<Utc>,
        updated_at: DateTime<Utc>,
    ) -> Self {
        LoginEventLabel {
            login_event_id,
            tenant_id,
            label,
            source,
            labeled_by,
            note,
            created_at,
            updated_at,
        }
    }

    // The precedence rule, in one place: an admin label replaces anything; a weak
    // label never replaces an admin one.
    pub fn may_replace(&self, existing: &LoginEventLabel) -> bool {
        !self.source.is_weak() || existing.source.is_weak()
    }

    pub fn login_event_id(&self) -> Uuid {
        self.login_event_id
    }
    pub fn tenant_id(&self) -> Uuid {
        self.tenant_id
    }
    pub fn label(&self) -> RiskLabel {
        self.label
    }
    pub fn source(&self) -> LabelSource {
        self.source
    }
    pub fn labeled_by(&self) -> Option<Uuid> {
        self.labeled_by
    }
    pub fn note(&self) -> Option<&str> {
        self.note.as_deref()
    }
    pub fn created_at(&self) -> DateTime<Utc> {
        self.created_at
    }
    pub fn updated_at(&self) -> DateTime<Utc> {
        self.updated_at
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn an_admin_label_replaces_anything_and_a_weak_one_never_replaces_an_admin() {
        let (tenant, event) = (Uuid::now_v7(), Uuid::now_v7());
        let admin = LoginEventLabel::by_admin(tenant, event, RiskLabel::Fraudulent, None, None);
        let weak = LoginEventLabel::from_mfa(tenant, event, true);

        assert!(admin.may_replace(&weak));
        assert!(admin.may_replace(&admin));
        assert!(!weak.may_replace(&admin));
        assert!(weak.may_replace(&LoginEventLabel::from_mfa(tenant, event, false)));
    }

    #[test]
    fn mfa_outcomes_map_to_weak_labels() {
        let passed = LoginEventLabel::from_mfa(Uuid::now_v7(), Uuid::now_v7(), true);
        let failed = LoginEventLabel::from_mfa(Uuid::now_v7(), Uuid::now_v7(), false);
        assert_eq!(
            (passed.label(), passed.source()),
            (RiskLabel::Legitimate, LabelSource::MfaPassed)
        );
        assert_eq!(
            (failed.label(), failed.source()),
            (RiskLabel::Fraudulent, LabelSource::MfaFailed)
        );
        assert!(passed.source().training_weight() < LabelSource::Admin.training_weight());
    }
}
