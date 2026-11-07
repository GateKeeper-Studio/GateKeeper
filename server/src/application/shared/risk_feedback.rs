// Weak labels for the statistical risk model (docs/RISK-SCORING.md), from the outcome
// of a step-up MFA the risk engine forced. Written through the MFA use case's own
// unit of work, so a label exists only if the outcome it describes committed.
//
// Only sessions whose step-up was risk-forced carry `risk_login_event_id`; MFA a
// client requires anyway says nothing about risk and labels nothing.
use crate::{
    application::ports::unit_of_work::UnitOfWork,
    domain::{
        entities::{
            auth_session::AuthSession, login_event_label::LoginEventLabel,
            mfa_challenge::MfaChallenge,
        },
        errors::DomainError,
    },
};

// The user passed the step-up: weak evidence the login was legitimate.
pub async fn label_mfa_passed(
    uow: &dyn UnitOfWork,
    session: &AuthSession,
) -> Result<(), DomainError> {
    label(uow, session, true).await
}

// Call after counting a failed attempt: labels only the attempt that used up the
// challenge, as weak evidence of fraud (a user can also simply mistype five times).
pub async fn label_if_mfa_exhausted(
    uow: &dyn UnitOfWork,
    session: &AuthSession,
    challenge: &MfaChallenge,
) -> Result<(), DomainError> {
    if challenge.attempts_exhausted() {
        label(uow, session, false).await?;
    }
    Ok(())
}

async fn label(
    uow: &dyn UnitOfWork,
    session: &AuthSession,
    passed: bool,
) -> Result<(), DomainError> {
    let Some(login_event_id) = session.risk_login_event_id() else {
        return Ok(());
    };
    uow.login_event_label_repository()
        .upsert(&LoginEventLabel::from_mfa(
            session.tenant_id(),
            login_event_id,
            passed,
        ))
        .await?;
    Ok(())
}
