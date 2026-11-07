ALTER TABLE auth_sessions DROP COLUMN IF EXISTS risk_login_event_id;

ALTER TABLE tenant_risk_policies
  DROP CONSTRAINT IF EXISTS chk_tenant_risk_policies_scoring_mode,
  DROP COLUMN IF EXISTS scoring_mode;

DROP TABLE IF EXISTS risk_models;
DROP TABLE IF EXISTS login_event_labels;

DROP INDEX IF EXISTS idx_login_events_tenant_asn_created_at;
ALTER TABLE login_events
  DROP COLUMN IF EXISTS risk_model,
  DROP COLUMN IF EXISTS risk_features;
