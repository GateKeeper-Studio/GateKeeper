-- Add up migration script here
CREATE TABLE IF NOT EXISTS signing_keys (
  id UUID PRIMARY KEY,
  tenant_id UUID NOT NULL,                         -- drop this column for a single global issuer
  kid VARCHAR(128) NOT NULL,                       -- key id embedded in JWT headers
  algorithm VARCHAR(8) NOT NULL,                   -- 'RS256' | 'ES256'
  public_jwk JSONB NOT NULL,                       -- served verbatim at /jwks.json
  private_key_encrypted TEXT NOT NULL,             -- sealed with a master key; never served
  status VARCHAR(16) NOT NULL DEFAULT 'active',    -- 'active' | 'retiring' | 'retired'
  rotates_at TIMESTAMPTZ NULL,
  created_at TIMESTAMPTZ NOT NULL,
  CONSTRAINT fk_signing_keys_tenant FOREIGN KEY (tenant_id) REFERENCES tenants (id) ON DELETE CASCADE
);
CREATE UNIQUE INDEX uq_signing_keys_kid ON signing_keys (kid);
-- find_active() / find_published() filter on status (and tenant_id).
CREATE INDEX idx_signing_keys_tenant_status ON signing_keys (tenant_id, status);