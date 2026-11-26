// The whole life of an admin's second factor: enroll, confirm, sign in with it,
// regenerate recovery codes, turn it off.
use std::sync::Arc;

use crate::{
    application::{
        services::admin_authenticator::{AdminSignIn, RECOVERY_CODE_COUNT},
        use_cases::admin_mfa::{
            confirm_totp, disable, get_status, regenerate_recovery_codes, start_totp,
        },
    },
    domain::{errors::DomainError, repositories::admin_mfa_repository::AdminMfaRepository},
    testing::admin_fixture::{AdminFixture, EMAIL, PASSWORD, current_code},
};

fn start(f: &AdminFixture) -> start_totp::use_case::UseCase {
    start_totp::use_case::UseCase::new(f.admins.clone(), f.factors.clone(), f.authenticator.clone())
}

fn confirm(f: &AdminFixture) -> confirm_totp::use_case::UseCase {
    confirm_totp::use_case::UseCase::new(f.factors.clone(), f.authenticator.clone())
}

fn status(f: &AdminFixture) -> get_status::use_case::UseCase {
    get_status::use_case::UseCase::new(f.factors.clone())
}

async fn enroll(f: &AdminFixture) -> (String, Vec<String>) {
    let started = start(f)
        .execute(start_totp::command::Command {
            admin_id: f.admin.id(),
            current_password: Some(PASSWORD.to_string()),
        })
        .await
        .unwrap();
    assert!(
        started
            .otpauth_url
            .starts_with("otpauth://totp/GateKeeper:admin@example.com?")
    );
    let codes = confirm(f)
        .execute(confirm_totp::command::Command {
            admin_id: f.admin.id(),
            code: current_code(&started.secret_base32),
        })
        .await
        .unwrap()
        .recovery_codes;
    (started.secret_base32, codes)
}

#[tokio::test]
async fn enrolling_needs_the_password_and_a_code_from_the_app() {
    let f = AdminFixture::new();
    let refused = start(&f)
        .execute(start_totp::command::Command {
            admin_id: f.admin.id(),
            current_password: None,
        })
        .await;
    assert!(matches!(refused, Err(DomainError::StepUpRequired(_))));

    let started = start(&f)
        .execute(start_totp::command::Command {
            admin_id: f.admin.id(),
            current_password: Some(PASSWORD.to_string()),
        })
        .await
        .unwrap();
    // Pending: sign-in does not ask for a code yet.
    assert!(
        !status(&f)
            .execute(get_status::query::Query {
                admin_id: f.admin.id()
            })
            .await
            .unwrap()
            .totp_enabled
    );
    assert!(matches!(
        f.authenticator.sign_in(EMAIL, PASSWORD, None).await,
        Ok(AdminSignIn::Authenticated(_))
    ));

    let wrong = confirm(&f)
        .execute(confirm_totp::command::Command {
            admin_id: f.admin.id(),
            code: "000000".to_string(),
        })
        .await;
    assert!(matches!(wrong, Err(DomainError::Validation(_))));

    let codes = confirm(&f)
        .execute(confirm_totp::command::Command {
            admin_id: f.admin.id(),
            code: current_code(&started.secret_base32),
        })
        .await
        .unwrap()
        .recovery_codes;
    assert_eq!(codes.len(), RECOVERY_CODE_COUNT);

    let now_on = status(&f)
        .execute(get_status::query::Query {
            admin_id: f.admin.id(),
        })
        .await
        .unwrap();
    assert!(now_on.totp_enabled);
    assert_eq!(now_on.recovery_codes_remaining, RECOVERY_CODE_COUNT);
    assert!(matches!(
        f.authenticator.sign_in(EMAIL, PASSWORD, None).await,
        Ok(AdminSignIn::SecondFactorRequired(_))
    ));
}

#[tokio::test]
async fn an_enabled_factor_is_never_replaced_by_a_new_enrollment() {
    let f = AdminFixture::new();
    let (seed, _) = enroll(&f).await;
    let again = start(&f)
        .execute(start_totp::command::Command {
            admin_id: f.admin.id(),
            current_password: Some(PASSWORD.to_string()),
        })
        .await;
    assert!(matches!(again, Err(DomainError::Conflict(_))));
    let stored = f.factors.find(f.admin.id()).await.unwrap().unwrap();
    assert_eq!(stored.totp_secret_enc(), format!("enc:{seed}"));
}

#[tokio::test]
async fn regenerating_replaces_every_recovery_code() {
    let f = AdminFixture::new();
    let (_, old_codes) = enroll(&f).await;
    let new_codes = regenerate_recovery_codes::use_case::UseCase::new(
        f.admins.clone(),
        f.factors.clone(),
        f.authenticator.clone(),
    )
    .execute(regenerate_recovery_codes::command::Command {
        admin_id: f.admin.id(),
        current_password: Some(PASSWORD.to_string()),
    })
    .await
    .unwrap()
    .recovery_codes;
    assert_ne!(old_codes, new_codes);
    assert!(
        f.authenticator
            .sign_in(EMAIL, PASSWORD, Some(&old_codes[0]))
            .await
            .is_err()
    );
    assert!(matches!(
        f.authenticator
            .sign_in(EMAIL, PASSWORD, Some(&new_codes[0]))
            .await,
        Ok(AdminSignIn::Authenticated(_))
    ));
}

#[tokio::test]
async fn turning_it_off_needs_the_password_and_a_code() {
    let f = AdminFixture::new();
    let (_, codes) = enroll(&f).await;
    let disable = Arc::new(disable::use_case::UseCase::new(
        f.admins.clone(),
        f.factors.clone(),
        f.authenticator.clone(),
    ));

    let without_code = disable
        .execute(disable::command::Command {
            admin_id: f.admin.id(),
            current_password: Some(PASSWORD.to_string()),
            code: None,
        })
        .await;
    assert!(matches!(without_code, Err(DomainError::Validation(_))));
    let without_password = disable
        .execute(disable::command::Command {
            admin_id: f.admin.id(),
            current_password: None,
            code: Some(codes[0].clone()),
        })
        .await;
    assert!(matches!(
        without_password,
        Err(DomainError::StepUpRequired(_))
    ));

    let removed = disable
        .execute(disable::command::Command {
            admin_id: f.admin.id(),
            current_password: Some(PASSWORD.to_string()),
            code: Some(codes[1].clone()),
        })
        .await
        .unwrap();
    assert!(removed);
    assert!(f.factors.find(f.admin.id()).await.unwrap().is_none());
    assert!(matches!(
        f.authenticator.sign_in(EMAIL, PASSWORD, None).await,
        Ok(AdminSignIn::Authenticated(_))
    ));
}
