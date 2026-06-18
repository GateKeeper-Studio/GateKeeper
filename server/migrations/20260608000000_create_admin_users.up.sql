-- Platform admin accounts (global, not tenant-scoped). These operators sign in to
-- the dashboard and act across all tenants. Authentication is independent of the
-- per-tenant OAuth user/credential model: admin login issues an HS256 token signed
-- with a server-wide secret, so admins never need a tenant signing key.
CREATE TABLE admin_users (
  id UUID PRIMARY KEY,
  email VARCHAR(128) NOT NULL,
  password_hash VARCHAR(255) NOT NULL,
  password_algorithm VARCHAR(50) NOT NULL,
  is_active BOOLEAN NOT NULL DEFAULT TRUE,
  created_at TIMESTAMPTZ NOT NULL,
  updated_at TIMESTAMPTZ NULL
);

CREATE UNIQUE INDEX uq_admin_users_email ON admin_users (email);
