-- Per-tenant editable legal documents (Terms of Service / Privacy Policy).
-- One row per (tenant, document_type). When no row exists the application serves
-- a built-in default, so a tenant only stores a row once it customizes the text.
CREATE TABLE IF NOT EXISTS legal_documents (
  tenant_id UUID NOT NULL,
  document_type VARCHAR(32) NOT NULL, -- 'terms_of_service' | 'privacy_policy'
  markdown_content TEXT NOT NULL,
  updated_at TIMESTAMPTZ NOT NULL,
  PRIMARY KEY (tenant_id, document_type),
  CONSTRAINT fk_legal_documents_tenant FOREIGN KEY (tenant_id)
    REFERENCES tenants (id) ON DELETE CASCADE
);
