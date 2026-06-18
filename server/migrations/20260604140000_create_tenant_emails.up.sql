-- Per-tenant SMTP email providers. An OAuth client points at one of these to
-- choose how its users' emails (verification codes, password-reset links) are
-- delivered. The SMTP password is stored AES-256-GCM encrypted (never plaintext);
-- it is decrypted only at send time and never exposed through any API.
CREATE TABLE IF NOT EXISTS tenant_emails (
  id UUID PRIMARY KEY,
  tenant_id UUID NOT NULL,
  name VARCHAR(255) NOT NULL,                 -- e.g. "SMTP Gmail"
  host VARCHAR(255) NOT NULL,                 -- e.g. smtp.gmail.com
  port INTEGER NOT NULL,                      -- e.g. 587
  username VARCHAR(255) NOT NULL,
  password_encrypted TEXT NOT NULL,           -- base64(nonce || AES-256-GCM ciphertext)
  from_email VARCHAR(255) NOT NULL,
  from_name VARCHAR(255) NULL,
  encryption VARCHAR(16) NOT NULL,            -- 'starttls' | 'tls' | 'none'
  is_default BOOLEAN NOT NULL DEFAULT FALSE,
  is_active BOOLEAN NOT NULL DEFAULT TRUE,
  created_at TIMESTAMPTZ NOT NULL,
  updated_at TIMESTAMPTZ NULL,
  CONSTRAINT fk_tenant_emails_tenant FOREIGN KEY (tenant_id) REFERENCES tenants (id) ON DELETE CASCADE
);

-- Admin listing is tenant-scoped.
CREATE INDEX idx_tenant_emails_tenant_id ON tenant_emails (tenant_id);
-- At most one default provider per tenant.
CREATE UNIQUE INDEX uq_tenant_emails_default ON tenant_emails (tenant_id) WHERE is_default;

-- An OAuth client may bind to a specific provider. NULL means "use the tenant
-- default" (and ultimately the dev log sender if there is no default).
ALTER TABLE oauth_clients
  ADD COLUMN email_provider_id UUID NULL REFERENCES tenant_emails (id) ON DELETE SET NULL;
