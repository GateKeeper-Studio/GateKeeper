use std::sync::Arc;

use crate::application::{
  shared::paged_content::PagedContent,
  use_cases::tenant::list_tenants::{query::Query, response::Response},
};
use crate::domain::{
  errors::DomainError,
  repositories::tenant_repository::TenantRepository,
};

pub struct UseCase {
  tenant_repository: Arc<dyn TenantRepository>,
}

impl UseCase {
  pub fn new(tenant_repository: Arc<dyn TenantRepository>) -> Self {
    Self { tenant_repository }
  }

  pub async fn execute(&self, query: Query) -> Result<PagedContent<Response>, DomainError> {
    // Guard the pagination math: page must be >= 1 (offset = (page-1)*size) and
    // page_size is bounded so a request can't ask for an unbounded result set.
    let page = query.page.unwrap_or(1).max(1);
    let page_size = query.page_size.unwrap_or(10).clamp(1, 100);

    let tenants = self.tenant_repository.find_all(page, page_size).await?;
    let total = self.tenant_repository.count_all().await?;

    Ok(PagedContent {
      items: tenants.into_iter().map(|t| Response {
        tenant_id: t.id(),
        name: t.name().to_string(),
        description: t.description().map(str::to_string),
        is_active: t.is_active(),
        can_self_register: t.can_self_register(),
        can_self_forgot_password: t.can_self_forgot_password(),
        created_at: t.created_at(),
        updated_at: t.updated_at(),
      }).collect(),
      total,
      page,
      page_size,
    })
  }
}
