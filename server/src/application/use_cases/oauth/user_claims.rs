use uuid::Uuid;

use crate::{application::ports::unit_of_work::UnitOfWork, domain::errors::DomainError};

// Resolves the role and group NAMES a user holds in a tenant, for embedding in
// issued tokens. Names (not ids) are used because they are what relying parties
// match against for RBAC decisions and are human-readable in a decoded JWT.
//
// Both lists are always returned (possibly empty) so consumers can rely on the
// `roles` / `groups` claims existing rather than probing for their presence.
pub async fn resolve_user_claims(
    uow: &dyn UnitOfWork,
    tenant_id: Uuid,
    user_id: Uuid,
) -> Result<(Vec<String>, Vec<String>), DomainError> {
    let roles = uow
        .role_repository()
        .find_by_user_id(tenant_id, user_id)
        .await?
        .iter()
        .map(|role| role.name().to_string())
        .collect();

    let groups = uow
        .group_repository()
        .find_by_user_id(tenant_id, user_id)
        .await?
        .iter()
        .map(|group| group.name().to_string())
        .collect();

    Ok((roles, groups))
}
