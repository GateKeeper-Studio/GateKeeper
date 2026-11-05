-- Temporary IP/user blocks. Replaces the "Redis key with a TTL" the spec suggests:
-- a block is an active row whose `expires_at` is in the future. The pre-auth check
-- looks up an active IP block before spending an Argon2 verification, and the Risk
-- Engine inserts rows here as a side effect when velocity thresholds are exceeded
-- (`source = 'system'`) or when an admin blocks an IP manually (`source = 'admin'`).
--
-- We prefer temporary IP blocks over hard account lockout (lockout is a DoS vector).
CREATE TABLE IF NOT EXISTS "security_blocks" (
    id UUID PRIMARY KEY,
    tenant_id UUID NOT NULL,

    block_type TEXT NOT NULL,        -- 'ip' | 'user'
    ip_address TEXT NULL,
    user_id UUID NULL,

    reason TEXT NOT NULL,
    source TEXT NOT NULL,            -- 'system' | 'admin'

    expires_at TIMESTAMPTZ NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    created_by UUID NULL,

    CONSTRAINT fk_security_blocks_tenant FOREIGN KEY (tenant_id)
        REFERENCES "tenants" (id) ON DELETE CASCADE
);

-- Fast active-IP-block lookup on the pre-auth path.
CREATE INDEX idx_security_blocks_tenant_ip_expires
    ON security_blocks (tenant_id, ip_address, expires_at);

-- Active-user-block lookup.
CREATE INDEX idx_security_blocks_tenant_user_expires
    ON security_blocks (tenant_id, user_id, expires_at);
