// Who is performing a use case, as the application layer sees it.
//
// The HTTP layer already authorizes every request (`require_admin`,
// `TenantScope`), but a check that lives only there protects only HTTP: a new
// handler that forgets the extractor, a CLI, or a background job could call the
// use case with any `tenant_id`. Commands for sensitive operations therefore
// carry an `Actor`, and the use case authorizes it against the resource it loaded
// — so the rule holds no matter who calls.
use uuid::Uuid;

use crate::domain::{
    entities::tenant::Tenant, errors::DomainError,
    repositories::tenant_repository::TenantRepository,
};

fn denied() -> DomainError {
    DomainError::AccessDenied("not authorized for this tenant".to_string())
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Actor {
    /// An end user acting on their own account (`/me/*`).
    User { tenant_id: Uuid, user_id: Uuid },
    /// A tenant-scoped administrator (tenant token with an admin role).
    TenantAdmin { tenant_id: Uuid, user_id: Uuid },
    /// A platform administrator, bound to exactly one organization.
    PlatformAdmin {
        admin_id: Uuid,
        organization_id: Uuid,
    },
    /// The server itself (background jobs, operator CLIs).
    System,
}

impl Actor {
    /// The id to attribute an audit event to.
    pub fn id(&self) -> Option<Uuid> {
        match self {
            Actor::User { user_id, .. } | Actor::TenantAdmin { user_id, .. } => Some(*user_id),
            Actor::PlatformAdmin { admin_id, .. } => Some(*admin_id),
            Actor::System => None,
        }
    }

    /// May this actor administer `tenant`? Tenant admins: only their own tenant.
    /// Platform admins: any tenant of their organization. End users: never.
    pub fn authorize_tenant_admin(&self, tenant: &Tenant) -> Result<(), DomainError> {
        let allowed = match self {
            Actor::TenantAdmin { tenant_id, .. } => *tenant_id == tenant.id(),
            Actor::PlatformAdmin {
                organization_id, ..
            } => *organization_id == tenant.organization_id(),
            Actor::System => true,
            Actor::User { .. } => false,
        };
        if allowed { Ok(()) } else { Err(denied()) }
    }

    /// `authorize_tenant_admin` for a tenant known only by id. The tenant is loaded
    /// only when the answer depends on it (a platform admin's organization); a
    /// missing tenant is `NotFound` for them, as it is in the use cases.
    pub async fn authorize_tenant_admin_for(
        &self,
        tenants: &dyn TenantRepository,
        tenant_id: Uuid,
    ) -> Result<(), DomainError> {
        match self {
            Actor::System => Ok(()),
            Actor::TenantAdmin { tenant_id: own, .. } if *own == tenant_id => Ok(()),
            Actor::PlatformAdmin { .. } => {
                let tenant = tenants.find_by_id(tenant_id).await?.ok_or_else(|| {
                    DomainError::NotFound(format!("tenant {tenant_id} not found"))
                })?;
                self.authorize_tenant_admin(&tenant)
            }
            Actor::TenantAdmin { .. } | Actor::User { .. } => Err(denied()),
        }
    }

    /// May this actor manage `organization_id` (its tenant records, its settings)?
    /// Only a platform admin of that organization, or the system.
    pub fn authorize_organization(&self, organization_id: Uuid) -> Result<(), DomainError> {
        match self {
            Actor::System => Ok(()),
            Actor::PlatformAdmin {
                organization_id: own,
                ..
            } if *own == organization_id => Ok(()),
            _ => Err(DomainError::AccessDenied(
                "not authorized for this organization".to_string(),
            )),
        }
    }

    /// May this actor act on `user_id`'s account in `tenant`? The user themself, or
    /// anyone who may administer the tenant.
    pub fn authorize_account(&self, tenant: &Tenant, user_id: Uuid) -> Result<(), DomainError> {
        match self {
            Actor::User {
                tenant_id,
                user_id: me,
            } if *tenant_id == tenant.id() && *me == user_id => Ok(()),
            Actor::User { .. } => Err(DomainError::AccessDenied(
                "not authorized for this account".to_string(),
            )),
            _ => self.authorize_tenant_admin(tenant),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn tenant(organization_id: Uuid) -> Tenant {
        Tenant::new(
            "Acme".to_string(),
            None,
            "pepper".to_string(),
            true,
            true,
            true,
            organization_id,
        )
    }

    #[test]
    fn tenant_admins_are_confined_to_their_tenant() {
        let t = tenant(Uuid::now_v7());
        let own = Actor::TenantAdmin {
            tenant_id: t.id(),
            user_id: Uuid::now_v7(),
        };
        let other = Actor::TenantAdmin {
            tenant_id: Uuid::now_v7(),
            user_id: Uuid::now_v7(),
        };
        assert!(own.authorize_tenant_admin(&t).is_ok());
        assert!(other.authorize_tenant_admin(&t).is_err());
    }

    #[test]
    fn platform_admins_are_confined_to_their_organization() {
        let org = Uuid::now_v7();
        let t = tenant(org);
        let own = Actor::PlatformAdmin {
            admin_id: Uuid::now_v7(),
            organization_id: org,
        };
        let other = Actor::PlatformAdmin {
            admin_id: Uuid::now_v7(),
            organization_id: Uuid::now_v7(),
        };
        assert!(own.authorize_tenant_admin(&t).is_ok());
        assert!(other.authorize_tenant_admin(&t).is_err());
    }

    #[test]
    fn end_users_act_only_on_their_own_account() {
        let t = tenant(Uuid::now_v7());
        let me = Uuid::now_v7();
        let user = Actor::User {
            tenant_id: t.id(),
            user_id: me,
        };
        assert!(user.authorize_account(&t, me).is_ok());
        assert!(user.authorize_account(&t, Uuid::now_v7()).is_err());
        assert!(user.authorize_tenant_admin(&t).is_err());
    }

    #[test]
    fn organizations_are_managed_only_by_their_own_platform_admins() {
        let org = Uuid::now_v7();
        let own = Actor::PlatformAdmin {
            admin_id: Uuid::now_v7(),
            organization_id: org,
        };
        let other = Actor::PlatformAdmin {
            admin_id: Uuid::now_v7(),
            organization_id: Uuid::now_v7(),
        };
        let tenant_admin = Actor::TenantAdmin {
            tenant_id: Uuid::now_v7(),
            user_id: Uuid::now_v7(),
        };
        assert!(own.authorize_organization(org).is_ok());
        assert!(other.authorize_organization(org).is_err());
        assert!(tenant_admin.authorize_organization(org).is_err());
        assert!(Actor::System.authorize_organization(org).is_ok());
    }

    #[tokio::test]
    async fn tenant_checks_by_id_load_the_tenant_only_for_platform_admins() {
        use crate::testing::in_memory_repositories::in_memory_tenant_repository::InMemoryTenantRepository;

        let org = Uuid::now_v7();
        let t = tenant(org);
        let repo = InMemoryTenantRepository::new();
        repo.memory.data.lock().unwrap().insert(t.id(), t.clone());

        let own_platform = Actor::PlatformAdmin {
            admin_id: Uuid::now_v7(),
            organization_id: org,
        };
        let other_platform = Actor::PlatformAdmin {
            admin_id: Uuid::now_v7(),
            organization_id: Uuid::now_v7(),
        };
        assert!(
            own_platform
                .authorize_tenant_admin_for(&repo, t.id())
                .await
                .is_ok()
        );
        assert!(matches!(
            other_platform
                .authorize_tenant_admin_for(&repo, t.id())
                .await,
            Err(DomainError::AccessDenied(_))
        ));
        // Unknown tenant: NotFound for a platform admin, as in the use cases.
        assert!(matches!(
            own_platform
                .authorize_tenant_admin_for(&repo, Uuid::now_v7())
                .await,
            Err(DomainError::NotFound(_))
        ));
    }

    // Guards the NEXT use case someone writes. Every tenant-scoped write use case
    // (its command has a `tenant_id` and it opens a Unit of Work) must take
    // `actor: &Actor` and authorize it. Self-service and protocol flows identify
    // the caller from the token itself and are listed as exempt.
    #[test]
    fn every_tenant_scoped_write_use_case_takes_an_actor() {
        const EXEMPT: &[&str] = &[
            // End user acting on their own account; ids come from the token.
            "compliance/record_policy_acceptance",
            "mfa/confirm_totp_enrollment",
            "mfa/delete_my_mfa_enrollment",
            "mfa/finish_webauthn_enrollment",
            "mfa/rename_my_mfa_enrollment",
            "mfa/set_preferred_mfa_method",
            "mfa/start_totp_enrollment",
            "mfa/start_webauthn_enrollment",
            // OAuth protocol: the client authenticates itself.
            "oauth/token_exchange",
            "oauth/token_refresh",
        ];
        let root =
            std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("src/application/use_cases");
        let mut missing = Vec::new();
        let mut stack = vec![root.clone()];
        while let Some(dir) = stack.pop() {
            for entry in std::fs::read_dir(&dir).unwrap() {
                let path = entry.unwrap().path();
                if path.is_dir() {
                    stack.push(path);
                    continue;
                }
                let name = path.file_name().unwrap().to_str().unwrap();
                let (use_case, command) = if name == "use_case.rs" {
                    let dir = path.parent().unwrap();
                    (path.clone(), dir.join("command.rs"))
                } else if dir.ends_with("security") && name.ends_with(".rs") {
                    (path.clone(), path.clone())
                } else {
                    continue;
                };
                let Ok(command_src) = std::fs::read_to_string(&command) else {
                    continue;
                };
                let src = std::fs::read_to_string(&use_case).unwrap();
                let is_tenant_write = command_src.contains("pub struct Command")
                    && command_src.contains("pub tenant_id")
                    && src.contains("unit_of_work_factory.begin()");
                if !is_tenant_write {
                    continue;
                }
                let rel = use_case
                    .strip_prefix(&root)
                    .unwrap()
                    .to_string_lossy()
                    .trim_end_matches("/use_case.rs")
                    .to_string();
                if EXEMPT.contains(&rel.as_str()) {
                    continue;
                }
                if !src.contains("actor: &Actor") {
                    missing.push(rel);
                }
            }
        }
        missing.sort();
        assert!(
            missing.is_empty(),
            "these tenant-scoped write use cases do not take `actor: &Actor`: {missing:?}\n\
             Add `actor: &Actor` to `execute` and call `actor.authorize_tenant_admin_for(..)` \
             (or add the use case to EXEMPT with a reason)."
        );
    }
}
