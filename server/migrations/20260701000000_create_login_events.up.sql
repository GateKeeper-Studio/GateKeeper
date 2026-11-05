-- Every authentication attempt (successful or failed) evaluated by the Risk Engine.
-- This is the durable audit history for fraud/suspicious-login detection AND the
-- source of truth for the PostgreSQL windowed velocity counters (failed-by-IP,
-- failed-by-user, password spraying) — there is no Redis; counts are computed here.
--
-- PII-minimal: never store raw passwords, MFA codes, tokens or session secrets.
-- `ip_address` is TEXT (matches audit_events and the RequestContext we already
-- capture from X-Forwarded-For/X-Real-IP), not INET. Geo columns are nullable and
-- stay NULL when GeoIP enrichment is unavailable (graceful degradation).
CREATE TABLE IF NOT EXISTS "login_events" (
    id UUID PRIMARY KEY,
    tenant_id UUID NOT NULL,
    user_id UUID NULL,
    client_id UUID NULL,

    username_normalized TEXT NULL,
    username_hash TEXT NULL,

    ip_address TEXT NULL,
    ip_hash TEXT NULL,

    user_agent TEXT NULL,
    user_agent_hash TEXT NULL,

    country_code TEXT NULL,
    region TEXT NULL,
    city TEXT NULL,
    latitude DOUBLE PRECISION NULL,
    longitude DOUBLE PRECISION NULL,
    asn INTEGER NULL,
    isp TEXT NULL,

    -- `credentials_valid` = the password was correct; `success` = the attempt was
    -- allowed to proceed (correct password AND not blocked). They differ on a
    -- valid-but-blocked login.
    credentials_valid BOOLEAN NOT NULL,
    success BOOLEAN NOT NULL,
    failure_reason TEXT NULL,

    risk_score INTEGER NOT NULL DEFAULT 0,
    risk_level TEXT NOT NULL,
    decision TEXT NOT NULL,

    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),

    CONSTRAINT fk_login_events_tenant FOREIGN KEY (tenant_id)
        REFERENCES "tenants" (id) ON DELETE CASCADE
);

-- Dashboard listing (newest first) and the tenant-attack-spike counter.
CREATE INDEX idx_login_events_tenant_created_at
    ON login_events (tenant_id, created_at DESC);

-- Per-user history: new-country, new-device, impossible-travel, failed-by-user.
CREATE INDEX idx_login_events_tenant_user_created_at
    ON login_events (tenant_id, user_id, created_at DESC);

-- Per-IP velocity: failed-by-IP and password-spraying (distinct users per IP).
CREATE INDEX idx_login_events_tenant_ip_created_at
    ON login_events (tenant_id, ip_address, created_at DESC);

-- Risk-level filtering on the dashboard.
CREATE INDEX idx_login_events_tenant_risk_level_created_at
    ON login_events (tenant_id, risk_level, created_at DESC);
