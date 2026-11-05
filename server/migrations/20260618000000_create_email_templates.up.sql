-- Per-tenant editable transactional email templates (email confirmation,
-- password reset, MFA code, …). One row per (tenant, template_type). When no row
-- exists the application serves a built-in default, so a tenant only stores a row
-- once it customizes a template — mirroring `legal_documents`.
--
-- `body_mjml` holds the admin-authored MJML source; it is compiled to
-- Outlook-safe HTML, sanitized, and inlined at render/send time (never trusted on
-- read). `subject` is a plain-text template that may contain {{placeholders}}.
CREATE TABLE IF NOT EXISTS email_templates (
  tenant_id     UUID        NOT NULL,
  template_type VARCHAR(48) NOT NULL,
  subject       TEXT        NOT NULL,
  body_mjml     TEXT        NOT NULL,
  updated_at    TIMESTAMPTZ NOT NULL,
  PRIMARY KEY (tenant_id, template_type),
  CONSTRAINT fk_email_templates_tenant FOREIGN KEY (tenant_id)
    REFERENCES tenants (id) ON DELETE CASCADE
);
