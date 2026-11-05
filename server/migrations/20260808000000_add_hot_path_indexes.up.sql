-- Indexes for three query shapes that were doing sequential scans.
--
-- Each one matches a predicate the code actually issues; the column order follows
-- the equality columns first, then the ordering column, so Postgres can satisfy
-- both the filter and the sort from the index.
--
-- CONCURRENTLY is deliberately NOT used: sqlx runs each migration inside a
-- transaction, and CREATE INDEX CONCURRENTLY cannot run in one. These tables are
-- small enough today that the brief ACCESS SHARE lock is fine. If they have grown
-- before you apply this, build the indexes by hand with CONCURRENTLY first — the
-- IF NOT EXISTS below then makes this migration a no-op.

-- `find_by_tenant_id` (oauth_client_repository.rs:305) filters on tenant_id and
-- orders by created_at DESC; `count_by_tenant_id` (:360) filters on tenant_id
-- alone and is served by the leading column. Previously only client_id was
-- indexed, so every admin "list OAuth clients" page scanned the table twice.
CREATE INDEX IF NOT EXISTS idx_oauth_clients_tenant_id_created_at
    ON oauth_clients (tenant_id, created_at DESC);

-- `revoke_by_user_and_client` (refresh_token_repository.rs:154) filters
-- (tenant_id, user_id, client_id) and `revoke_by_user` (:174) filters
-- (tenant_id, user_id) — a prefix of the same index. Both only ever touch live
-- tokens, so the partial index keeps it small: revoked rows accumulate but are
-- never matched here.
CREATE INDEX IF NOT EXISTS idx_refresh_tokens_tenant_user_client_active
    ON refresh_tokens (tenant_id, user_id, client_id)
    WHERE revoked_at IS NULL;

-- `delete_by_user_id` (auth_session_repository.rs:201) — runs on password change,
-- "log out everywhere" and account erasure, and had no index at all.
CREATE INDEX IF NOT EXISTS idx_auth_sessions_user_id
    ON auth_sessions (user_id);
