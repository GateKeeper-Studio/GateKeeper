-- Append-only security/audit event log (WI-02).
--
-- This table is the evidentiary backbone for SOC 2 (CC7.2/CC7.3), ISO 27001
-- A.8.15/A.8.16, GDPR Art. 5(2)/30 and LGPD Art. 37. Rows are WRITE-ONCE: the
-- application never UPDATEs or DELETEs them (retention purges of OTHER tables
-- never touch this one). Enforce that at the DB grant level in production:
--   REVOKE UPDATE, DELETE ON audit_events FROM <app_role>;
-- (captured in docs/compliance/crypto-key-management.md, least-privilege section).
CREATE TABLE IF NOT EXISTS "audit_events" (
  id            UUID PRIMARY KEY,                 -- uuid_v7, time-sortable
  tenant_id     UUID NULL,                        -- NULL for platform-level events
  actor_type    VARCHAR(32)  NOT NULL,            -- 'user' | 'client' | 'system' | 'admin'
  actor_id      UUID NULL,                        -- who performed it (NULL for anonymous/system)
  action        VARCHAR(128) NOT NULL,            -- e.g. 'user.login.success', 'consent.revoked'
  resource_type VARCHAR(64)  NULL,                -- 'user' | 'oauth_client' | 'role' | ...
  resource_id   UUID NULL,
  outcome       VARCHAR(16)  NOT NULL,            -- 'success' | 'failure'
  ip_address    TEXT NULL,                        -- client IP as text (no INET ops needed)
  user_agent    TEXT NULL,
  metadata      JSONB NOT NULL DEFAULT '{}',      -- action-specific, MUST stay PII-minimal
  created_at    TIMESTAMPTZ  NOT NULL DEFAULT now()
);

CREATE INDEX IF NOT EXISTS idx_audit_tenant_created ON audit_events (tenant_id, created_at DESC);
CREATE INDEX IF NOT EXISTS idx_audit_actor          ON audit_events (actor_id, created_at DESC);
CREATE INDEX IF NOT EXISTS idx_audit_action         ON audit_events (action, created_at DESC);
