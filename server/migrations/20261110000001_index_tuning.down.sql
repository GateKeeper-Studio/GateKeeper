CREATE INDEX idx_pa_user ON policy_acceptances (user_id);
CREATE INDEX idx_eip_oauth_client ON external_identity_providers (oauth_client_id);
CREATE INDEX idx_audit_actor ON audit_events (actor_id, created_at DESC);
CREATE INDEX idx_audit_action ON audit_events (action, created_at DESC);
DROP INDEX idx_audit_tenant_actor_created;
DROP INDEX idx_audit_tenant_action_created;
DROP INDEX idx_refresh_tokens_expires_at;
DROP INDEX idx_refresh_tokens_user;
