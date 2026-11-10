use chrono::{Duration, Utc};

use super::*;
use crate::{
    domain::entities::audit_action::ADMIN_ACCOUNT_LOCKED,
    testing::admin_fixture::{AdminFixture, EMAIL, PASSWORD},
};

fn is_generic_failure(result: &Result<AdminSignIn, DomainError>) -> bool {
    matches!(result, Err(DomainError::AuthenticationError(m)) if m == "invalid credentials")
}

#[tokio::test]
async fn the_right_password_signs_in_whatever_the_email_case() {
    let f = AdminFixture::new();
    let result = f
        .authenticator
        .sign_in("  ADMIN@Example.com ", PASSWORD, None)
        .await
        .unwrap();
    assert!(matches!(result, AdminSignIn::Authenticated(a) if a.id() == f.admin.id()));
}

#[tokio::test]
async fn unknown_email_wrong_password_and_deactivated_fail_the_same_way() {
    let f = AdminFixture::new();
    assert!(is_generic_failure(
        &f.authenticator
            .sign_in("nobody@example.com", PASSWORD, None)
            .await
    ));
    assert!(is_generic_failure(
        &f.authenticator.sign_in(EMAIL, "wrong-password", None).await
    ));

    let mut deactivated = f.stored();
    deactivated.deactivate();
    *f.admins.admins.lock().unwrap().first_mut().unwrap() = deactivated;
    assert!(is_generic_failure(
        &f.authenticator.sign_in(EMAIL, PASSWORD, None).await
    ));
}

#[tokio::test]
async fn a_suspended_organization_cannot_sign_in() {
    let f = AdminFixture::new();
    let suspended = f.stored().with_sign_in_state(false, 0, None);
    *f.admins.admins.lock().unwrap().first_mut().unwrap() = suspended;
    assert!(is_generic_failure(
        &f.authenticator.sign_in(EMAIL, PASSWORD, None).await
    ));
}

#[tokio::test]
async fn consecutive_failures_lock_the_account_even_for_the_right_password() {
    let f = AdminFixture::new();
    for _ in 0..MAX_FAILED_SIGN_INS {
        assert!(is_generic_failure(
            &f.authenticator.sign_in(EMAIL, "wrong-password", None).await
        ));
    }
    let stored = f.stored();
    assert!(stored.is_locked(Utc::now()));
    assert_eq!(stored.failed_sign_in_count(), 0, "the count restarts");
    assert_eq!(f.audit.count_action(ADMIN_ACCOUNT_LOCKED), 1);

    // The lock is indistinguishable from a wrong password.
    assert!(is_generic_failure(
        &f.authenticator.sign_in(EMAIL, PASSWORD, None).await
    ));

    // Once it expires, the right password works and clears the state.
    let expired = stored.with_sign_in_state(true, 0, Some(Utc::now() - Duration::seconds(1)));
    *f.admins.admins.lock().unwrap().first_mut().unwrap() = expired;
    assert!(matches!(
        f.authenticator.sign_in(EMAIL, PASSWORD, None).await,
        Ok(AdminSignIn::Authenticated(_))
    ));
    assert_eq!(f.stored().locked_until(), None);
}

#[tokio::test]
async fn a_success_clears_earlier_failures() {
    let f = AdminFixture::new();
    for _ in 0..MAX_FAILED_SIGN_INS - 1 {
        let _ = f.authenticator.sign_in(EMAIL, "wrong-password", None).await;
    }
    assert_eq!(f.stored().failed_sign_in_count(), MAX_FAILED_SIGN_INS - 1);
    f.authenticator
        .sign_in(EMAIL, PASSWORD, None)
        .await
        .unwrap();
    assert_eq!(f.stored().failed_sign_in_count(), 0);
}

#[tokio::test]
async fn with_a_second_factor_the_password_alone_only_asks_for_the_code() {
    let f = AdminFixture::new();
    f.with_totp().await;

    assert!(matches!(
        f.authenticator.sign_in(EMAIL, PASSWORD, None).await,
        Ok(AdminSignIn::SecondFactorRequired(_))
    ));
    // A wrong password is still the generic failure, never `SecondFactorRequired`.
    assert!(is_generic_failure(
        &f.authenticator.sign_in(EMAIL, "wrong-password", None).await
    ));

    let wrong = f
        .authenticator
        .sign_in(EMAIL, PASSWORD, Some("000000"))
        .await;
    assert!(matches!(wrong, Err(DomainError::MfaError(m)) if m == INVALID_MFA_CODE));
    assert_eq!(f.stored().failed_sign_in_count(), 2, "both failures count");

    let code = f.totp_code();
    assert!(matches!(
        f.authenticator.sign_in(EMAIL, PASSWORD, Some(&code)).await,
        Ok(AdminSignIn::Authenticated(_))
    ));
    // The same code cannot be replayed.
    assert!(matches!(
        f.authenticator.sign_in(EMAIL, PASSWORD, Some(&code)).await,
        Err(DomainError::MfaError(_))
    ));
}

#[tokio::test]
async fn a_recovery_code_works_once_in_any_case_and_format() {
    let f = AdminFixture::new();
    let codes = f.with_totp().await;
    let typed = codes[0].to_lowercase().replace('-', " ");

    assert!(matches!(
        f.authenticator.sign_in(EMAIL, PASSWORD, Some(&typed)).await,
        Ok(AdminSignIn::Authenticated(_))
    ));
    assert!(matches!(
        f.authenticator
            .sign_in(EMAIL, PASSWORD, Some(&codes[0]))
            .await,
        Err(DomainError::MfaError(_))
    ));
    let remaining = f.factors.find(f.admin.id()).await.unwrap().unwrap();
    assert_eq!(
        remaining.recovery_code_hashes().len(),
        RECOVERY_CODE_COUNT - 1
    );
}

#[tokio::test]
async fn step_up_asks_for_the_password_and_counts_wrong_ones() {
    let f = AdminFixture::new();
    assert!(matches!(
        f.authenticator.require_password(&f.admin, None).await,
        Err(DomainError::StepUpRequired(_))
    ));
    assert!(matches!(
        f.authenticator
            .require_password(&f.admin, Some("nope"))
            .await,
        Err(DomainError::Validation(_))
    ));
    assert_eq!(f.stored().failed_sign_in_count(), 1);
    f.authenticator
        .require_password(&f.admin, Some(PASSWORD))
        .await
        .unwrap();
}

#[test]
fn recovery_codes_are_distinct_and_only_their_hashes_are_kept() {
    let (codes, hashes) = generate_recovery_codes();
    assert_eq!(codes.len(), RECOVERY_CODE_COUNT);
    let unique: std::collections::HashSet<_> = codes.iter().collect();
    assert_eq!(unique.len(), RECOVERY_CODE_COUNT);
    for (code, hash) in codes.iter().zip(&hashes) {
        assert_eq!(code.len(), 11);
        assert_eq!(&sha256_hex(&normalize_recovery_code(code)), hash);
    }
}
