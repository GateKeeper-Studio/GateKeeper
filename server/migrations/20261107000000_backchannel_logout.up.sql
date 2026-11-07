-- OIDC Back-Channel Logout 1.0.
--
-- 1. Client registration (OIDC metadata names). NULL = the client is not notified.
--    The URI is validated like a webhook URL (SSRF rules) before it gets here.
ALTER TABLE oauth_clients
  ADD COLUMN backchannel_logout_uri TEXT NULL,
  ADD COLUMN backchannel_logout_session_required BOOLEAN NOT NULL DEFAULT FALSE;

-- 2. Logout notifications travel through the same outbox and delivery table as the
--    user webhooks: same transaction as the sign-out, same retry state machine, same
--    SKIP LOCKED claim. A delivery now targets EITHER a webhook subscription OR an
--    OAuth client's back-channel logout URI — exactly one of the two.
ALTER TABLE webhook_deliveries
  ALTER COLUMN subscription_id DROP NOT NULL,
  ADD COLUMN client_id UUID NULL,
  ADD CONSTRAINT fk_webhook_deliveries_client FOREIGN KEY (client_id) REFERENCES oauth_clients (id) ON DELETE CASCADE,
  ADD CONSTRAINT chk_webhook_deliveries_target CHECK (num_nonnulls(subscription_id, client_id) = 1);

-- Fan-out idempotency for logout deliveries, the twin of uq_webhook_delivery.
CREATE UNIQUE INDEX uq_webhook_delivery_client ON webhook_deliveries (event_id, client_id)
  WHERE client_id IS NOT NULL;
