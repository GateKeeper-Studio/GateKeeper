ALTER TABLE auth_sessions
    DROP CONSTRAINT chk_auth_sessions_primary_factor,
    DROP COLUMN primary_factor;
