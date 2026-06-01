#[derive(Debug, serde::Serialize)]
pub struct PagedContent<T> {
  pub items: Vec<T>,
  pub total: u32,
  pub page: u32,
  pub page_size: u32,
}