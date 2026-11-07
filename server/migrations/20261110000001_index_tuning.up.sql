-- Index review against every query the code runs (EXPLAIN of all cached queries).

-- Sessions list (/me/sessions, admin Sessions tab) reads a user's whole token history,
-- revoked rows included, which the partial active-token index cannot serve; it also
-- backs the ON DELETE CASCADE from users (a tenant deletion deletes every user).
CREATE INDEX idx_refresh_tokens_user ON refresh_tokens (user_id);
-- The hourly retention purge: DELETE ... WHERE expires_at < now().
CREATE INDEX idx_refresh_tokens_expires_at ON refresh_tokens (expires_at);

-- The audit list always filters by tenant; the action/actor filters must be scoped
-- to it instead of walking that action or actor across every tenant.
CREATE INDEX idx_audit_tenant_action_created ON audit_events (tenant_id, action, created_at DESC);
CREATE INDEX idx_audit_tenant_actor_created ON audit_events (tenant_id, actor_id, created_at DESC);
DROP INDEX idx_audit_action;
DROP INDEX idx_audit_actor;

-- Redundant: covered by the leading columns of a unique index on the same table.
DROP INDEX idx_eip_oauth_client;   -- uq_eip_client_provider (oauth_client_id, provider)
DROP INDEX idx_pa_user;            -- uq_pa_user_doc_version (user_id, document_type, document_version)
