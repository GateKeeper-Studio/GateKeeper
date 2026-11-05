-- Outbound event delivery: transactional outbox + signed webhooks.
--
-- Consumers of this IdP are SEPARATE services that keep their own user tables.
-- They need to learn when a user appears, changes, or is removed. The problem
-- this schema solves is not "how do we make an HTTP call" — it is the dual write:
-- calling an endpoint after COMMIT loses the event if the process dies in between;
-- calling it before COMMIT announces a user that a rollback then erases.
--
-- The fix is that outbox_events is written in the SAME transaction as the change
-- it describes. An event exists if and only if the change committed.

-- The outbox. Written by use cases inside their own UnitOfWork transaction; read
-- by the dispatcher worker, which fans each event out to matching subscriptions.
CREATE TABLE IF NOT EXISTS outbox_events (
  id           UUID PRIMARY KEY,               -- uuid_v7: time-sortable, and doubles as
                                               -- the idempotency key consumers dedupe on
  tenant_id    UUID NOT NULL,
  event_type   VARCHAR(64)  NOT NULL,          -- 'user.created' | 'user.updated' | 'user.deleted'
  payload      JSONB        NOT NULL,
  occurred_at  TIMESTAMPTZ  NOT NULL,
  published_at TIMESTAMPTZ  NULL,              -- NULL = not yet fanned out
  CONSTRAINT fk_outbox_events_tenant FOREIGN KEY (tenant_id) REFERENCES tenants (id) ON DELETE CASCADE
);

-- The fan-out claim reads only unpublished rows, oldest first. A partial index
-- keeps it small no matter how much history accumulates.
CREATE INDEX IF NOT EXISTS idx_outbox_unpublished ON outbox_events (id) WHERE published_at IS NULL;

-- Tenant-scoped delivery targets. Mirrors tenant_emails: tenant-owned, secret
-- encrypted at rest, is_active toggle.
CREATE TABLE IF NOT EXISTS webhook_subscriptions (
  id               UUID PRIMARY KEY,
  tenant_id        UUID NOT NULL,
  name             VARCHAR(255) NOT NULL,
  url              TEXT NOT NULL,
  secret_encrypted TEXT NOT NULL,              -- base64(nonce || AES-256-GCM); never exposed by any API
  event_types      TEXT[] NOT NULL DEFAULT '{}', -- filter; empty array = every event type
  is_active        BOOLEAN NOT NULL DEFAULT TRUE,
  created_at       TIMESTAMPTZ NOT NULL,
  updated_at       TIMESTAMPTZ NULL,
  CONSTRAINT fk_webhook_subscriptions_tenant FOREIGN KEY (tenant_id) REFERENCES tenants (id) ON DELETE CASCADE
);

CREATE INDEX IF NOT EXISTS idx_webhook_subscriptions_tenant ON webhook_subscriptions (tenant_id);

-- One row per (event, subscription). This is where retry state lives, so a
-- subscription that is down cannot hold up delivery to the others.
CREATE TABLE IF NOT EXISTS webhook_deliveries (
  id              UUID PRIMARY KEY,
  event_id        UUID NOT NULL,
  subscription_id UUID NOT NULL,
  tenant_id       UUID NOT NULL,               -- denormalised so the admin delivery log
                                               -- can be tenant-scoped without a join
  status          VARCHAR(16) NOT NULL,        -- 'pending' | 'delivered' | 'dead'
  attempts        INTEGER NOT NULL DEFAULT 0,
  next_attempt_at TIMESTAMPTZ NOT NULL,
  last_error      TEXT NULL,
  last_status     INTEGER NULL,                -- last HTTP status seen (NULL on transport error)
  created_at      TIMESTAMPTZ NOT NULL,
  delivered_at    TIMESTAMPTZ NULL,
  CONSTRAINT fk_webhook_deliveries_event FOREIGN KEY (event_id) REFERENCES outbox_events (id) ON DELETE CASCADE,
  CONSTRAINT fk_webhook_deliveries_subscription FOREIGN KEY (subscription_id) REFERENCES webhook_subscriptions (id) ON DELETE CASCADE,
  -- Makes fan-out idempotent: if the worker dies after inserting deliveries but
  -- before stamping published_at, re-running the phase inserts nothing new.
  CONSTRAINT uq_webhook_delivery UNIQUE (event_id, subscription_id)
);

-- The delivery claim reads rows that are due and still pending.
CREATE INDEX IF NOT EXISTS idx_webhook_deliveries_due ON webhook_deliveries (next_attempt_at)
  WHERE status = 'pending';
-- Admin delivery log, newest first.
CREATE INDEX IF NOT EXISTS idx_webhook_deliveries_tenant ON webhook_deliveries (tenant_id, created_at DESC);
