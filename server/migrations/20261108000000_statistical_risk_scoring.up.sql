-- Statistical risk scoring next to the deterministic rules (docs/RISK-SCORING.md).

-- 1. What the hybrid engine saw and concluded, per evaluated attempt.
--    risk_features: {"v": <schema>, "values": {<feature>: <number>}} — derived numbers
--      only (counts, ratios, flags), never identifiers. This is the training dataset.
--    risk_model: the model's verdict and its explanation (version, probability,
--      threshold, mode, applied, top per-feature contributions). NULL when no model
--      scored the attempt.
ALTER TABLE login_events
  ADD COLUMN risk_features JSONB NULL,
  ADD COLUMN risk_model JSONB NULL;

-- asn_tenant_share: successful logins of a tenant from one ASN over a window.
CREATE INDEX idx_login_events_tenant_asn_created_at
  ON login_events (tenant_id, asn, created_at DESC)
  WHERE success AND asn IS NOT NULL;

-- 2. Labels: the feedback loop the model learns from. One per event. An admin label
--    is authoritative; weak labels come from step-up MFA outcomes and never replace an
--    admin's verdict (enforced by the repository's upsert).
CREATE TABLE IF NOT EXISTS login_event_labels (
  login_event_id UUID PRIMARY KEY,
  tenant_id UUID NOT NULL,
  label TEXT NOT NULL,                      -- 'legitimate' | 'fraudulent'
  source TEXT NOT NULL,                     -- 'admin' | 'mfa_passed' | 'mfa_failed'
  labeled_by UUID NULL,                     -- the admin, for source = 'admin'
  note TEXT NULL,
  created_at TIMESTAMPTZ NOT NULL,
  updated_at TIMESTAMPTZ NOT NULL,
  CONSTRAINT fk_login_event_labels_event FOREIGN KEY (login_event_id)
    REFERENCES login_events (id) ON DELETE CASCADE,
  CONSTRAINT chk_login_event_labels_label CHECK (label IN ('legitimate', 'fraudulent')),
  CONSTRAINT chk_login_event_labels_source CHECK (source IN ('admin', 'mfa_passed', 'mfa_failed'))
);

CREATE INDEX idx_login_event_labels_tenant_created_at
  ON login_event_labels (tenant_id, created_at DESC);

-- 3. Trained models. Global (one platform model); kept forever as the audit trail of
--    what scored logins when. At most one is active.
CREATE TABLE IF NOT EXISTS risk_models (
  id UUID PRIMARY KEY,
  version TEXT NOT NULL UNIQUE,
  feature_schema INTEGER NOT NULL,
  model JSONB NOT NULL,                     -- feature names, means, stds, weights, intercept, threshold
  metrics JSONB NOT NULL,                   -- validation AUC, FPR/precision/recall at threshold, gates
  label_counts JSONB NOT NULL,
  trained_from TIMESTAMPTZ NOT NULL,
  trained_to TIMESTAMPTZ NOT NULL,
  status TEXT NOT NULL,                     -- 'candidate' | 'active' | 'retired'
  created_at TIMESTAMPTZ NOT NULL,
  promoted_at TIMESTAMPTZ NULL,
  retired_at TIMESTAMPTZ NULL,
  CONSTRAINT chk_risk_models_status CHECK (status IN ('candidate', 'active', 'retired'))
);

CREATE UNIQUE INDEX uq_risk_models_single_active ON risk_models ((status)) WHERE status = 'active';

-- 4. Per-tenant opt-in: 'rules' (default, today's behaviour), 'shadow' (scored and
--    shown, never acted on), 'hybrid' (may raise a login to step-up MFA, never block).
ALTER TABLE tenant_risk_policies
  ADD COLUMN scoring_mode TEXT NOT NULL DEFAULT 'rules',
  ADD CONSTRAINT chk_tenant_risk_policies_scoring_mode
    CHECK (scoring_mode IN ('rules', 'shadow', 'hybrid'));

-- 5. The login event whose risk decision forced this session's step-up MFA, so the
--    MFA outcome can label it.
ALTER TABLE auth_sessions
  ADD COLUMN risk_login_event_id UUID NULL;
