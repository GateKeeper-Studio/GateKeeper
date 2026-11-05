-- Per-tenant tuning of the Risk Engine. A tenant with no row uses built-in defaults
-- (TenantRiskPolicy::defaults_for), so no backfill is needed; the admin PUT endpoint
-- upserts a row. Defaults mirror the spec's "Recommended Defaults for MVP" (§19).
CREATE TABLE IF NOT EXISTS "tenant_risk_policies" (
    id UUID PRIMARY KEY,
    tenant_id UUID NOT NULL UNIQUE,

    enabled BOOLEAN NOT NULL DEFAULT true,

    failed_ip_threshold INTEGER NOT NULL DEFAULT 10,
    failed_user_threshold INTEGER NOT NULL DEFAULT 5,
    password_spraying_user_threshold INTEGER NOT NULL DEFAULT 10,
    tenant_attack_threshold INTEGER NOT NULL DEFAULT 100,

    medium_risk_threshold INTEGER NOT NULL DEFAULT 30,
    high_risk_threshold INTEGER NOT NULL DEFAULT 60,
    critical_risk_threshold INTEGER NOT NULL DEFAULT 90,

    require_mfa_on_high_risk BOOLEAN NOT NULL DEFAULT true,
    block_on_critical_risk BOOLEAN NOT NULL DEFAULT true,

    temporary_block_minutes INTEGER NOT NULL DEFAULT 30,

    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at TIMESTAMPTZ NULL,

    CONSTRAINT fk_tenant_risk_policies_tenant FOREIGN KEY (tenant_id)
        REFERENCES "tenants" (id) ON DELETE CASCADE
);
