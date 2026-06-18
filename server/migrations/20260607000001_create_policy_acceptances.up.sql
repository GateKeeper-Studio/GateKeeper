-- Versioned acceptance of legal documents (Terms of Service, Privacy Policy).
-- Backs consent/lawful-basis evidence (GDPR Arts. 6/7, LGPD Arts. 7-9, SOC 2 Privacy):
-- one row per (user, document_type, version) records WHEN a subject accepted WHICH
-- version, so a version bump can force re-acceptance and we can prove acceptance.
CREATE TABLE IF NOT EXISTS "policy_acceptances" (
  id               UUID PRIMARY KEY,
  tenant_id        UUID NOT NULL,
  user_id          UUID NOT NULL,
  document_type    VARCHAR(32)  NOT NULL,   -- 'terms_of_service' | 'privacy_policy'
  document_version VARCHAR(32)  NOT NULL,
  accepted_at      TIMESTAMPTZ  NOT NULL,
  ip_address       TEXT NULL,
  CONSTRAINT fk_pa_tenant FOREIGN KEY (tenant_id) REFERENCES "tenants" (id) ON DELETE CASCADE,
  CONSTRAINT fk_pa_user   FOREIGN KEY (user_id)   REFERENCES "users" (id)   ON DELETE CASCADE,
  CONSTRAINT uq_pa_user_doc_version UNIQUE (user_id, document_type, document_version)
);

CREATE INDEX IF NOT EXISTS idx_pa_user ON policy_acceptances (user_id);
