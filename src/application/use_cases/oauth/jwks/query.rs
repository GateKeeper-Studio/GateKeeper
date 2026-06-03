use uuid::Uuid;

// `GET /.well-known/jwks.json`. Per-tenant issuer ⇒ the JWKS is tenant-scoped.
#[derive(Debug, Clone)]
pub struct Query {
  pub tenant_id: Uuid,
}
