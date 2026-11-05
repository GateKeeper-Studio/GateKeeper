-- One row per risk signal detected while evaluating a login_event. Keeping signals
-- normalized (rather than a JSON blob on login_events) makes the decision auditable
-- and explainable ("why was this HIGH risk?") and lets the dashboard filter by
-- signal_type. `points` is the signal's contribution to the event's risk_score.
CREATE TABLE IF NOT EXISTS "login_risk_signals" (
    id UUID PRIMARY KEY,
    login_event_id UUID NOT NULL,
    tenant_id UUID NOT NULL,

    signal_type TEXT NOT NULL,
    severity TEXT NOT NULL,
    points INTEGER NOT NULL,
    metadata_json JSONB NULL,

    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),

    CONSTRAINT fk_login_risk_signals_event FOREIGN KEY (login_event_id)
        REFERENCES "login_events" (id) ON DELETE CASCADE
);

-- Load all signals for one event (details page).
CREATE INDEX idx_login_risk_signals_event
    ON login_risk_signals (login_event_id);

-- Filter/aggregate by signal type over time (e.g. password-spraying events).
CREATE INDEX idx_login_risk_signals_tenant_signal_created_at
    ON login_risk_signals (tenant_id, signal_type, created_at DESC);
