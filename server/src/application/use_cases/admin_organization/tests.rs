// The admins of an organization managing each other, end to end over the in-memory
// stores.
use std::sync::Arc;

use chrono::{Duration, Utc};
use uuid::Uuid;

use crate::{
    application::{
        services::admin_authenticator::AdminSignIn,
        shared::actor::Actor,
        use_cases::admin_organization::{
            accept_invitation, invite_admin, list_admins, list_invitations, lookup_invitation,
            reset_admin_mfa, revoke_invitation, set_admin_active, set_mfa_policy,
        },
    },
    domain::{
        entities::{
            admin_mfa::AdminMfa, admin_web_session::AdminWebSession, organization::Organization,
        },
        errors::DomainError,
        repositories::{
            admin_mfa_repository::AdminMfaRepository,
            organization_repository::OrganizationRepository,
        },
    },
    testing::{
        admin_fixture::{AdminFixture, PASSWORD},
        in_memory_repositories::{
            in_memory_admin_invitation_repository::InMemoryAdminInvitationRepository,
            in_memory_organization_repository::InMemoryOrganizationRepository,
        },
    },
};

// The in-memory store writes to a draft that a Unit of Work would commit; this
// policy use case writes through the repository directly, as it does on a pool.
fn flush(organizations: &InMemoryOrganizationRepository) {
    let drafted: Vec<_> = organizations.memory.draft.lock().unwrap().drain().collect();
    organizations.memory.data.lock().unwrap().extend(drafted);
}

struct Org {
    f: AdminFixture,
    organizations: Arc<InMemoryOrganizationRepository>,
    invitations: Arc<InMemoryAdminInvitationRepository>,
    actor: Actor,
}

async fn org() -> Org {
    let f = AdminFixture::new();
    let organizations = Arc::new(InMemoryOrganizationRepository::new());
    let organization = Organization::reconstruct(
        f.admin.organization_id(),
        "Acme".to_string(),
        None,
        true,
        Utc::now(),
        None,
    );
    organizations
        .memory
        .data
        .lock()
        .unwrap()
        .insert(organization.id(), organization);
    let invitations = Arc::new(InMemoryAdminInvitationRepository::new(f.admins.clone()));
    let actor = Actor::PlatformAdmin {
        admin_id: f.admin.id(),
        organization_id: f.admin.organization_id(),
    };
    Org {
        f,
        organizations,
        invitations,
        actor,
    }
}

fn invite(o: &Org) -> invite_admin::use_case::UseCase {
    invite_admin::use_case::UseCase::new(
        o.f.admins.clone(),
        o.invitations.clone(),
        o.f.authenticator.clone(),
    )
}

fn accept(o: &Org) -> accept_invitation::use_case::UseCase {
    accept_invitation::use_case::UseCase::new(o.invitations.clone(), o.f.authenticator.clone())
}

fn invite_command(o: &Org, email: &str, password: Option<&str>) -> invite_admin::command::Command {
    invite_admin::command::Command {
        organization_id: o.f.admin.organization_id(),
        email: email.to_string(),
        current_password: password.map(str::to_string),
    }
}

fn accept_command(token: &str, password: &str) -> accept_invitation::command::Command {
    accept_invitation::command::Command {
        token: token.to_string(),
        name: "New Admin".to_string(),
        password: password.to_string(),
    }
}

const NEW_PASSWORD: &str = "a-long-enough-password";

#[tokio::test]
async fn an_invitation_link_creates_the_admin_once() {
    let o = org().await;
    let refused = invite(&o)
        .execute(&o.actor, invite_command(&o, "new@example.com", None))
        .await;
    assert!(matches!(refused, Err(DomainError::StepUpRequired(_))));

    let invited = invite(&o)
        .execute(
            &o.actor,
            invite_command(&o, "New@Example.com", Some(PASSWORD)),
        )
        .await
        .unwrap();
    assert_eq!(invited.email, "new@example.com");

    let lookup =
        lookup_invitation::use_case::UseCase::new(o.invitations.clone(), o.organizations.clone());
    let shown = lookup
        .execute(lookup_invitation::query::Query {
            token: invited.token.clone(),
        })
        .await
        .unwrap();
    assert_eq!(shown.organization_name, "Acme");

    // The password policy applies, and a refusal leaves the link usable.
    let weak = accept(&o)
        .execute(accept_command(&invited.token, "short"))
        .await;
    assert!(matches!(weak, Err(DomainError::Validation(_))));

    let accepted = accept(&o)
        .execute(accept_command(&invited.token, NEW_PASSWORD))
        .await
        .unwrap();
    assert_eq!(accepted.organization_id, o.f.admin.organization_id());
    assert!(matches!(
        o.f.authenticator
            .sign_in("new@example.com", NEW_PASSWORD, None)
            .await,
        Ok(AdminSignIn::Authenticated(a)) if a.id() == accepted.admin_id
    ));

    let again = accept(&o)
        .execute(accept_command(&invited.token, NEW_PASSWORD))
        .await;
    assert!(matches!(again, Err(DomainError::NotFound(_))), "single use");
    assert!(
        lookup
            .execute(lookup_invitation::query::Query {
                token: invited.token
            })
            .await
            .is_err()
    );
}

#[tokio::test]
async fn re_inviting_replaces_the_link_and_existing_admins_cannot_be_invited() {
    let o = org().await;
    let first = invite(&o)
        .execute(
            &o.actor,
            invite_command(&o, "new@example.com", Some(PASSWORD)),
        )
        .await
        .unwrap();
    let second = invite(&o)
        .execute(
            &o.actor,
            invite_command(&o, "new@example.com", Some(PASSWORD)),
        )
        .await
        .unwrap();
    assert!(
        accept(&o)
            .execute(accept_command(&first.token, NEW_PASSWORD))
            .await
            .is_err()
    );

    let open = list_invitations::use_case::UseCase::new(o.f.admins.clone(), o.invitations.clone())
        .execute(
            &o.actor,
            list_invitations::query::Query {
                organization_id: o.f.admin.organization_id(),
            },
        )
        .await
        .unwrap();
    assert_eq!(open.invitations.len(), 1);
    assert_eq!(open.invitations[0].id, second.invitation_id);

    revoke_invitation::use_case::UseCase::new(o.f.admins.clone(), o.invitations.clone())
        .execute(
            &o.actor,
            revoke_invitation::command::Command {
                organization_id: o.f.admin.organization_id(),
                invitation_id: second.invitation_id,
            },
        )
        .await
        .unwrap();
    assert!(
        accept(&o)
            .execute(accept_command(&second.token, NEW_PASSWORD))
            .await
            .is_err()
    );

    let existing = invite(&o)
        .execute(
            &o.actor,
            invite_command(&o, "ADMIN@example.com", Some(PASSWORD)),
        )
        .await;
    assert!(matches!(existing, Err(DomainError::Conflict(_))));
}

#[tokio::test]
async fn an_admin_of_another_organization_is_refused() {
    let o = org().await;
    let outsider = Actor::PlatformAdmin {
        admin_id: Uuid::now_v7(),
        organization_id: Uuid::now_v7(),
    };
    let listed = list_admins::use_case::UseCase::new(o.f.admins.clone())
        .execute(
            &outsider,
            list_admins::query::Query {
                organization_id: o.f.admin.organization_id(),
            },
        )
        .await;
    assert!(matches!(listed, Err(DomainError::AccessDenied(_))));
}

fn set_active(o: &Org) -> set_admin_active::use_case::UseCase {
    set_admin_active::use_case::UseCase::new(
        o.f.admins.clone(),
        o.f.authenticator.clone(),
        o.f.refresh_tokens.clone(),
        o.f.web_sessions.clone(),
    )
}

fn active_command(o: &Org, admin_id: Uuid, active: bool) -> set_admin_active::command::Command {
    set_admin_active::command::Command {
        organization_id: o.f.admin.organization_id(),
        admin_id,
        active,
        current_password: Some(PASSWORD.to_string()),
    }
}

#[tokio::test]
async fn deactivating_a_colleague_ends_their_sessions_and_reactivation_restores_access() {
    let o = org().await;
    let colleague = o.f.add_colleague("colleague@example.com");
    o.f.web_sessions
        .sessions
        .lock()
        .unwrap()
        .push(AdminWebSession::new(
            colleague.id(),
            "s".into(),
            Utc::now(),
            Duration::hours(1),
        ));

    let own = set_active(&o)
        .execute(&o.actor, active_command(&o, o.f.admin.id(), false))
        .await;
    assert!(matches!(own, Err(DomainError::Validation(_))));

    assert!(
        set_active(&o)
            .execute(&o.actor, active_command(&o, colleague.id(), false))
            .await
            .unwrap()
    );
    assert!(o.f.web_sessions.sessions.lock().unwrap().is_empty());
    assert!(
        o.f.authenticator
            .sign_in("colleague@example.com", PASSWORD, None)
            .await
            .is_err()
    );

    assert!(
        set_active(&o)
            .execute(&o.actor, active_command(&o, colleague.id(), true))
            .await
            .unwrap()
    );
    assert!(
        o.f.authenticator
            .sign_in("colleague@example.com", PASSWORD, None)
            .await
            .is_ok()
    );
}

#[tokio::test]
async fn a_colleague_can_reset_a_lost_second_factor_but_not_their_own() {
    let o = org().await;
    let colleague = o.f.add_colleague("colleague@example.com");
    let now = Utc::now();
    o.f.factors
        .save_pending(&AdminMfa::pending(colleague.id(), "enc:SEED".into(), now))
        .await
        .unwrap();
    o.f.factors
        .confirm(colleague.id(), 1, &[], now)
        .await
        .unwrap();

    let reset = reset_admin_mfa::use_case::UseCase::new(
        o.f.admins.clone(),
        o.f.factors.clone(),
        o.f.authenticator.clone(),
        o.f.refresh_tokens.clone(),
        o.f.web_sessions.clone(),
    );
    let command = |admin_id| reset_admin_mfa::command::Command {
        organization_id: o.f.admin.organization_id(),
        admin_id,
        current_password: Some(PASSWORD.to_string()),
    };
    assert!(matches!(
        reset.execute(&o.actor, command(o.f.admin.id())).await,
        Err(DomainError::Validation(_))
    ));
    assert!(
        reset
            .execute(&o.actor, command(colleague.id()))
            .await
            .unwrap()
    );
    assert!(o.f.factors.find(colleague.id()).await.unwrap().is_none());
}

#[tokio::test]
async fn requiring_two_step_needs_it_on_the_acting_admin_and_restricts_the_others() {
    let o = org().await;
    let colleague = o.f.add_colleague("colleague@example.com");
    let policy = set_mfa_policy::use_case::UseCase::new(
        o.f.admins.clone(),
        o.organizations.clone(),
        o.f.authenticator.clone(),
    );
    let command = |required| set_mfa_policy::command::Command {
        organization_id: o.f.admin.organization_id(),
        required,
        current_password: Some(PASSWORD.to_string()),
    };

    let refused = policy.execute(&o.actor, command(true)).await;
    assert!(matches!(refused, Err(DomainError::Validation(_))));

    o.f.with_totp().await;
    assert!(policy.execute(&o.actor, command(true)).await.unwrap());
    flush(&o.organizations);
    assert!(
        !policy.execute(&o.actor, command(true)).await.unwrap(),
        "no change"
    );
    let stored = o
        .organizations
        .find_by_id(o.f.admin.organization_id())
        .await
        .unwrap()
        .unwrap();
    assert!(stored.require_admin_mfa());

    // The colleague (no second factor) is flagged; the acting admin is not.
    let with_policy = |admin: crate::domain::entities::admin_user::AdminUser| {
        admin.with_account_state(true, None)
    };
    assert!(
        o.f.authenticator
            .mfa_setup_required(&with_policy(colleague))
            .await
            .unwrap()
    );
    assert!(
        !o.f.authenticator
            .mfa_setup_required(&with_policy(o.f.stored()))
            .await
            .unwrap()
    );
}
