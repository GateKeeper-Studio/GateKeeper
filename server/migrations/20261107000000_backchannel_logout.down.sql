-- Logout deliveries cannot be represented once the client target is gone.
DELETE FROM webhook_deliveries WHERE client_id IS NOT NULL;
DELETE FROM outbox_events WHERE event_type = 'oidc.backchannel_logout';

DROP INDEX IF EXISTS uq_webhook_delivery_client;
ALTER TABLE webhook_deliveries
  DROP CONSTRAINT IF EXISTS chk_webhook_deliveries_target,
  DROP CONSTRAINT IF EXISTS fk_webhook_deliveries_client,
  DROP COLUMN IF EXISTS client_id,
  ALTER COLUMN subscription_id SET NOT NULL;

ALTER TABLE oauth_clients
  DROP COLUMN IF EXISTS backchannel_logout_session_required,
  DROP COLUMN IF EXISTS backchannel_logout_uri;
