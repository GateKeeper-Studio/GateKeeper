-- Link admin_users (many per org) and subscriptions (one per org) to organizations.
ALTER TABLE "admin_users" ADD COLUMN organization_id UUID NULL;
ALTER TABLE "subscriptions" ADD COLUMN organization_id UUID NULL;

-- Backfill records that predate organizations. No-op on a fresh database.
DO $$
DECLARE
    v_legacy_org UUID;
    r RECORD;
    v_first BOOLEAN := TRUE;
BEGIN
    IF EXISTS (SELECT 1 FROM admin_users WHERE organization_id IS NULL)
       OR EXISTS (SELECT 1 FROM subscriptions WHERE organization_id IS NULL) THEN
        v_legacy_org := gen_random_uuid();
        INSERT INTO organizations (id, name, description, is_active, created_at)
        VALUES (v_legacy_org, 'Legacy',
                'Auto-created for records that predate organizations', TRUE, now());

        -- All orphan admins join the single Legacy org.
        UPDATE admin_users SET organization_id = v_legacy_org WHERE organization_id IS NULL;

        -- One subscription per organization: the first orphan subscription joins
        -- the Legacy org; each additional one gets its own organization.
        FOR r IN SELECT id FROM subscriptions WHERE organization_id IS NULL ORDER BY created_at LOOP
            IF v_first THEN
                UPDATE subscriptions SET organization_id = v_legacy_org WHERE id = r.id;
                v_first := FALSE;
            ELSE
                DECLARE
                    v_org UUID := gen_random_uuid();
                BEGIN
                    INSERT INTO organizations (id, name, description, is_active, created_at)
                    VALUES (v_org, 'Legacy ' || r.id, 'Auto-created', TRUE, now());
                    UPDATE subscriptions SET organization_id = v_org WHERE id = r.id;
                END;
            END IF;
        END LOOP;
    END IF;
END $$;

ALTER TABLE "admin_users" ALTER COLUMN organization_id SET NOT NULL;
ALTER TABLE "subscriptions" ALTER COLUMN organization_id SET NOT NULL;

ALTER TABLE "admin_users" ADD CONSTRAINT fk_admin_user_organization
    FOREIGN KEY (organization_id) REFERENCES "organizations" (id) ON DELETE CASCADE;
ALTER TABLE "subscriptions" ADD CONSTRAINT fk_subscription_organization
    FOREIGN KEY (organization_id) REFERENCES "organizations" (id) ON DELETE CASCADE;

CREATE INDEX idx_admin_users_organization_id ON admin_users (organization_id);

-- An organization can have at most one subscription.
CREATE UNIQUE INDEX uq_subscriptions_organization_id ON subscriptions (organization_id);
