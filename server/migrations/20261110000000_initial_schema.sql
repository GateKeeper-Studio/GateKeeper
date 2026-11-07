-- GateKeeper database schema: the baseline.
--
-- The 71 migrations written from 2026-05-30 to 2026-11-09 were squashed into this
-- one file: it creates the schema those migrations produced, in one step. It was
-- generated from `pg_dump --schema-only` of a database migrated through all of
-- them, and checked to reproduce that schema exactly. Their history (with the
-- reasoning behind each table) is in git: `git log --follow -- server/migrations/`.
--
-- Irreversible on purpose (no .down.sql): reverting the baseline would drop every
-- table. A database created before the squash is adopted at startup instead of
-- migrated (infraestructure/database/migrations.rs).
--
-- New schema changes go in new migration files after this one:
--   sqlx migrate add -r <description>

-- ── Functions ────────────────────────────────────────────────────────────────
CREATE FUNCTION audit_events_reject_update() RETURNS trigger
    LANGUAGE plpgsql
    AS $$
BEGIN
    IF current_setting('gatekeeper.audit_erasure', true) = 'on' THEN
        RETURN NEW;
    END IF;
    RAISE EXCEPTION 'audit_events is append-only (UPDATE refused)'
        USING ERRCODE = 'insufficient_privilege';
END;
$$;

-- ── Organizations and platform admins ───────────────────────────────────────────
-- The operators of the platform: their organization, accounts, refresh-token
-- families (bearer API) and console browser sessions (cookie).

CREATE TABLE organizations (
    id          UUID NOT NULL,
    name        VARCHAR(255) NOT NULL,
    description VARCHAR(255),
    is_active   BOOLEAN DEFAULT true NOT NULL,
    created_at  TIMESTAMPTZ NOT NULL,
    updated_at  TIMESTAMPTZ,
    CONSTRAINT organizations_pkey PRIMARY KEY (id)
);

CREATE UNIQUE INDEX uq_organizations_name ON organizations (name);

CREATE TABLE admin_users (
    id                 UUID NOT NULL,
    email              VARCHAR(128) NOT NULL,
    password_hash      VARCHAR(255) NOT NULL,
    password_algorithm VARCHAR(50) NOT NULL,
    is_active          BOOLEAN DEFAULT true NOT NULL,
    created_at         TIMESTAMPTZ NOT NULL,
    updated_at         TIMESTAMPTZ,
    organization_id    UUID NOT NULL,
    name               VARCHAR(128) DEFAULT ''::character varying NOT NULL,
    CONSTRAINT admin_users_pkey PRIMARY KEY (id),
    CONSTRAINT fk_admin_user_organization FOREIGN KEY (organization_id) REFERENCES organizations(id) ON DELETE CASCADE
);

CREATE INDEX idx_admin_users_organization_id ON admin_users (organization_id);
CREATE UNIQUE INDEX uq_admin_users_email ON admin_users (email);

CREATE TABLE admin_refresh_tokens (
    id         UUID NOT NULL,
    admin_id   UUID NOT NULL,
    token_hash VARCHAR(64) NOT NULL,
    family_id  UUID NOT NULL,
    rotated_at TIMESTAMPTZ,
    revoked_at TIMESTAMPTZ,
    expires_at TIMESTAMPTZ NOT NULL,
    created_at TIMESTAMPTZ NOT NULL,
    CONSTRAINT admin_refresh_tokens_pkey PRIMARY KEY (id),
    CONSTRAINT admin_refresh_tokens_admin_id_fkey FOREIGN KEY (admin_id) REFERENCES admin_users(id) ON DELETE CASCADE
);

CREATE INDEX ix_admin_refresh_tokens_admin ON admin_refresh_tokens (admin_id);
CREATE INDEX ix_admin_refresh_tokens_family ON admin_refresh_tokens (family_id);
CREATE UNIQUE INDEX uq_admin_refresh_tokens_token_hash ON admin_refresh_tokens (token_hash);

CREATE TABLE admin_web_sessions (
    id           UUID NOT NULL,
    admin_id     UUID NOT NULL,
    token_hash   VARCHAR(64) NOT NULL,
    created_at   TIMESTAMPTZ NOT NULL,
    last_seen_at TIMESTAMPTZ NOT NULL,
    expires_at   TIMESTAMPTZ NOT NULL,
    CONSTRAINT admin_web_sessions_pkey PRIMARY KEY (id),
    CONSTRAINT admin_web_sessions_admin_id_fkey FOREIGN KEY (admin_id) REFERENCES admin_users(id) ON DELETE CASCADE
);

CREATE INDEX ix_admin_web_sessions_admin ON admin_web_sessions (admin_id);
CREATE INDEX ix_admin_web_sessions_expires ON admin_web_sessions (expires_at);
CREATE UNIQUE INDEX uq_admin_web_sessions_token_hash ON admin_web_sessions (token_hash);

-- ── Tenants ─────────────────────────────────────────────────────────────────────
-- A tenant is an isolated user directory with its own keys, emails, legal
-- documents, custom sign-up fields and risk policy.

CREATE TABLE tenants (
    id                          UUID NOT NULL,
    name                        VARCHAR(255) NOT NULL,
    description                 VARCHAR(255),
    can_self_register           BOOLEAN DEFAULT false NOT NULL,
    can_self_forgot_password    BOOLEAN DEFAULT false NOT NULL,
    is_active                   BOOLEAN DEFAULT true NOT NULL,
    password_hash_secret        TEXT NOT NULL,
    created_at                  TIMESTAMPTZ NOT NULL,
    updated_at                  TIMESTAMPTZ,
    appearance                  JSONB,
    organization_id             UUID NOT NULL,
    default_language            VARCHAR(10) DEFAULT 'en'::character varying NOT NULL,
    password_hash_secret_sealed BOOLEAN DEFAULT false NOT NULL,
    CONSTRAINT tenants_pkey PRIMARY KEY (id),
    CONSTRAINT fk_tenant_organization FOREIGN KEY (organization_id) REFERENCES organizations(id) ON DELETE CASCADE
);

CREATE INDEX idx_tenants_organization_id ON tenants (organization_id);
CREATE UNIQUE INDEX uq_tenants_name ON tenants (name);

CREATE TABLE signing_keys (
    id                    UUID NOT NULL,
    tenant_id             UUID NOT NULL,
    kid                   VARCHAR(128) NOT NULL,
    algorithm             VARCHAR(8) NOT NULL,
    public_jwk            JSONB NOT NULL,
    private_key_encrypted TEXT NOT NULL,
    status                VARCHAR(16) DEFAULT 'active'::character varying NOT NULL,
    rotates_at            TIMESTAMPTZ,
    created_at            TIMESTAMPTZ NOT NULL,
    CONSTRAINT signing_keys_pkey PRIMARY KEY (id),
    CONSTRAINT fk_signing_keys_tenant FOREIGN KEY (tenant_id) REFERENCES tenants(id) ON DELETE CASCADE
);

CREATE INDEX idx_signing_keys_tenant_status ON signing_keys (tenant_id, status);
CREATE UNIQUE INDEX uq_signing_keys_kid ON signing_keys (kid);

CREATE TABLE tenant_emails (
    id                 UUID NOT NULL,
    tenant_id          UUID NOT NULL,
    name               VARCHAR(255) NOT NULL,
    host               VARCHAR(255) NOT NULL,
    port               INTEGER NOT NULL,
    username           VARCHAR(255) NOT NULL,
    password_encrypted TEXT NOT NULL,
    from_email         VARCHAR(255) NOT NULL,
    from_name          VARCHAR(255),
    encryption         VARCHAR(16) NOT NULL,
    is_default         BOOLEAN DEFAULT false NOT NULL,
    is_active          BOOLEAN DEFAULT true NOT NULL,
    created_at         TIMESTAMPTZ NOT NULL,
    updated_at         TIMESTAMPTZ,
    CONSTRAINT tenant_emails_pkey PRIMARY KEY (id),
    CONSTRAINT fk_tenant_emails_tenant FOREIGN KEY (tenant_id) REFERENCES tenants(id) ON DELETE CASCADE
);

CREATE INDEX idx_tenant_emails_tenant_id ON tenant_emails (tenant_id);
CREATE UNIQUE INDEX uq_tenant_emails_default ON tenant_emails (tenant_id) WHERE is_default;

CREATE TABLE email_templates (
    tenant_id     UUID NOT NULL,
    template_type VARCHAR(48) NOT NULL,
    subject       TEXT NOT NULL,
    body_mjml     TEXT NOT NULL,
    updated_at    TIMESTAMPTZ NOT NULL,
    CONSTRAINT email_templates_pkey PRIMARY KEY (tenant_id, template_type),
    CONSTRAINT fk_email_templates_tenant FOREIGN KEY (tenant_id) REFERENCES tenants(id) ON DELETE CASCADE
);

CREATE TABLE legal_documents (
    tenant_id        UUID NOT NULL,
    document_type    VARCHAR(32) NOT NULL,
    markdown_content TEXT NOT NULL,
    updated_at       TIMESTAMPTZ NOT NULL,
    CONSTRAINT legal_documents_pkey PRIMARY KEY (tenant_id, document_type),
    CONSTRAINT fk_legal_documents_tenant FOREIGN KEY (tenant_id) REFERENCES tenants(id) ON DELETE CASCADE
);

CREATE TABLE tenant_custom_fields (
    id            UUID NOT NULL,
    tenant_id     UUID NOT NULL,
    field_key     VARCHAR(64) NOT NULL,
    field_type    VARCHAR(20) NOT NULL,
    is_required   BOOLEAN DEFAULT false NOT NULL,
    display_order INTEGER DEFAULT 0 NOT NULL,
    is_active     BOOLEAN DEFAULT true NOT NULL,
    created_at    TIMESTAMPTZ DEFAULT now() NOT NULL,
    updated_at    TIMESTAMPTZ DEFAULT now() NOT NULL,
    label         JSONB NOT NULL,
    description   JSONB,
    options       JSONB,
    CONSTRAINT tenant_custom_fields_pkey PRIMARY KEY (id),
    CONSTRAINT fk_tenant_custom_fields_tenant FOREIGN KEY (tenant_id) REFERENCES tenants(id) ON DELETE CASCADE
);

CREATE INDEX ix_tenant_custom_fields_tenant_order ON tenant_custom_fields (tenant_id, display_order);
CREATE UNIQUE INDEX uq_tenant_custom_fields_tenant_key ON tenant_custom_fields (tenant_id, field_key);

CREATE TABLE tenant_risk_policies (
    id                               UUID NOT NULL,
    tenant_id                        UUID NOT NULL,
    enabled                          BOOLEAN DEFAULT true NOT NULL,
    failed_ip_threshold              INTEGER DEFAULT 10 NOT NULL,
    failed_user_threshold            INTEGER DEFAULT 5 NOT NULL,
    password_spraying_user_threshold INTEGER DEFAULT 10 NOT NULL,
    tenant_attack_threshold          INTEGER DEFAULT 100 NOT NULL,
    medium_risk_threshold            INTEGER DEFAULT 30 NOT NULL,
    high_risk_threshold              INTEGER DEFAULT 60 NOT NULL,
    critical_risk_threshold          INTEGER DEFAULT 90 NOT NULL,
    require_mfa_on_high_risk         BOOLEAN DEFAULT true NOT NULL,
    block_on_critical_risk           BOOLEAN DEFAULT true NOT NULL,
    temporary_block_minutes          INTEGER DEFAULT 30 NOT NULL,
    created_at                       TIMESTAMPTZ DEFAULT now() NOT NULL,
    updated_at                       TIMESTAMPTZ,
    scoring_mode                     TEXT DEFAULT 'rules'::text NOT NULL,
    CONSTRAINT tenant_risk_policies_pkey PRIMARY KEY (id),
    CONSTRAINT tenant_risk_policies_tenant_id_key UNIQUE (tenant_id),
    CONSTRAINT chk_tenant_risk_policies_scoring_mode CHECK ((scoring_mode = ANY (ARRAY['rules'::text, 'shadow'::text, 'hybrid'::text]))),
    CONSTRAINT fk_tenant_risk_policies_tenant FOREIGN KEY (tenant_id) REFERENCES tenants(id) ON DELETE CASCADE
);

-- ── OAuth clients ───────────────────────────────────────────────────────────────
-- The applications that sign users in through a tenant, and their social
-- identity providers.

CREATE TABLE oauth_clients (
    id                                  UUID NOT NULL,
    tenant_id                           UUID NOT NULL,
    client_id                           UUID NOT NULL,
    client_secret_hash                  VARCHAR(255),
    client_type                         VARCHAR(16) NOT NULL,
    name                                VARCHAR(255) NOT NULL,
    redirect_uris                       TEXT[] NOT NULL,
    post_logout_redirect_uris           TEXT[] DEFAULT '{}'::text[] NOT NULL,
    allowed_scopes                      TEXT[] NOT NULL,
    allowed_grant_types                 TEXT[] NOT NULL,
    require_pkce                        BOOLEAN DEFAULT true NOT NULL,
    require_consent                     BOOLEAN DEFAULT true NOT NULL,
    is_active                           BOOLEAN DEFAULT true NOT NULL,
    created_at                          TIMESTAMPTZ NOT NULL,
    updated_at                          TIMESTAMPTZ,
    email_provider_id                   UUID,
    mfa_required                        BOOLEAN DEFAULT false NOT NULL,
    mfa_email_enabled                   BOOLEAN DEFAULT true NOT NULL,
    mfa_totp_enabled                    BOOLEAN DEFAULT true NOT NULL,
    mfa_webauthn_enabled                BOOLEAN DEFAULT true NOT NULL,
    tos_uri                             TEXT,
    policy_uri                          TEXT,
    login_appearance                    JSONB,
    initiate_login_uri                  TEXT,
    allowed_languages                   TEXT[] DEFAULT ARRAY['en'::text] NOT NULL,
    default_language                    VARCHAR(10) DEFAULT 'en'::character varying NOT NULL,
    external_login_only                 BOOLEAN DEFAULT false NOT NULL,
    magic_link_enabled                  BOOLEAN DEFAULT false NOT NULL,
    backchannel_logout_uri              TEXT,
    backchannel_logout_session_required BOOLEAN DEFAULT false NOT NULL,
    CONSTRAINT oauth_clients_pkey PRIMARY KEY (id),
    CONSTRAINT fk_oauth_clients_tenant FOREIGN KEY (tenant_id) REFERENCES tenants(id) ON DELETE CASCADE,
    CONSTRAINT oauth_clients_email_provider_id_fkey FOREIGN KEY (email_provider_id) REFERENCES tenant_emails(id) ON DELETE SET NULL
);

CREATE INDEX idx_oauth_clients_tenant_id_created_at ON oauth_clients (tenant_id, created_at DESC);
CREATE UNIQUE INDEX uq_oauth_clients_client_id ON oauth_clients (client_id);

CREATE TABLE external_identity_providers (
    id                               UUID NOT NULL,
    oauth_client_id                  UUID NOT NULL,
    tenant_id                        UUID NOT NULL,
    provider                         VARCHAR(32) NOT NULL,
    provider_client_id               TEXT NOT NULL,
    provider_client_secret_encrypted TEXT CONSTRAINT external_identity_providers_provider_client_secret_enc_not_null NOT NULL,
    redirect_uri                     TEXT NOT NULL,
    is_active                        BOOLEAN DEFAULT true NOT NULL,
    created_at                       TIMESTAMPTZ NOT NULL,
    updated_at                       TIMESTAMPTZ,
    CONSTRAINT external_identity_providers_pkey PRIMARY KEY (id),
    CONSTRAINT fk_eip_oauth_client FOREIGN KEY (oauth_client_id) REFERENCES oauth_clients(id) ON DELETE CASCADE,
    CONSTRAINT fk_eip_tenant FOREIGN KEY (tenant_id) REFERENCES tenants(id) ON DELETE CASCADE
);

CREATE INDEX idx_eip_oauth_client ON external_identity_providers (oauth_client_id);
CREATE UNIQUE INDEX uq_eip_client_provider ON external_identity_providers (oauth_client_id, provider);

-- ── Users ───────────────────────────────────────────────────────────────────────
-- Tenant users: identity, profile, password credentials, linked social
-- identities and accepted policies.

CREATE TABLE users (
    id                   UUID NOT NULL,
    tenant_id            UUID NOT NULL,
    email                VARCHAR(128) NOT NULL,
    created_at           TIMESTAMPTZ NOT NULL,
    updated_at           TIMESTAMPTZ,
    is_active            BOOLEAN DEFAULT true NOT NULL,
    is_email_verified    BOOLEAN DEFAULT false NOT NULL,
    preferred_2fa_method VARCHAR(24),
    anonymized_at        TIMESTAMPTZ,
    legal_hold           BOOLEAN DEFAULT false NOT NULL,
    CONSTRAINT users_pkey PRIMARY KEY (id),
    CONSTRAINT fk_user_tenant FOREIGN KEY (tenant_id) REFERENCES tenants(id) ON DELETE CASCADE
);

CREATE INDEX idx_users_tenant_id_created_at ON users (tenant_id, created_at DESC);
CREATE UNIQUE INDEX uq_users_tenant_lower_email ON users (tenant_id, lower((email)::text));

CREATE TABLE user_profiles (
    user_id             UUID NOT NULL,
    display_name        VARCHAR(255) NOT NULL,
    first_name          VARCHAR(255) NOT NULL,
    last_name           VARCHAR(255) NOT NULL,
    phone_number        VARCHAR(255),
    address             VARCHAR(255),
    photo_url           VARCHAR(255),
    custom_field_values JSONB,
    CONSTRAINT user_profiles_pkey PRIMARY KEY (user_id),
    CONSTRAINT fk_user_profile_user FOREIGN KEY (user_id) REFERENCES users(id) ON DELETE CASCADE
);

CREATE TABLE user_credentials (
    id                 UUID NOT NULL,
    user_id            UUID NOT NULL,
    password_hash      VARCHAR(255) NOT NULL,
    password_algorithm VARCHAR(50) NOT NULL,
    should_change_pass BOOLEAN DEFAULT false NOT NULL,
    updated_at         TIMESTAMPTZ,
    created_at         TIMESTAMPTZ NOT NULL,
    CONSTRAINT user_credentials_pkey PRIMARY KEY (id),
    CONSTRAINT fk_user_credentials_tenant_user FOREIGN KEY (user_id) REFERENCES users(id) ON DELETE CASCADE
);

CREATE UNIQUE INDEX uq_user_credentials_user_id ON user_credentials (user_id);

CREATE TABLE social_identities (
    id               UUID NOT NULL,
    user_id          UUID NOT NULL,
    tenant_id        UUID NOT NULL,
    provider         VARCHAR(32) NOT NULL,
    provider_subject TEXT NOT NULL,
    email            TEXT,
    created_at       TIMESTAMPTZ NOT NULL,
    updated_at       TIMESTAMPTZ,
    CONSTRAINT social_identities_pkey PRIMARY KEY (id),
    CONSTRAINT fk_social_tenant FOREIGN KEY (tenant_id) REFERENCES tenants(id) ON DELETE CASCADE,
    CONSTRAINT fk_social_user FOREIGN KEY (user_id) REFERENCES users(id) ON DELETE CASCADE
);

CREATE INDEX idx_social_user ON social_identities (user_id);
CREATE UNIQUE INDEX uq_social_provider_subject ON social_identities (tenant_id, provider, provider_subject);

CREATE TABLE policy_acceptances (
    id               UUID NOT NULL,
    tenant_id        UUID NOT NULL,
    user_id          UUID NOT NULL,
    document_type    VARCHAR(32) NOT NULL,
    document_version VARCHAR(32) NOT NULL,
    accepted_at      TIMESTAMPTZ NOT NULL,
    ip_address       TEXT,
    CONSTRAINT policy_acceptances_pkey PRIMARY KEY (id),
    CONSTRAINT uq_pa_user_doc_version UNIQUE (user_id, document_type, document_version),
    CONSTRAINT fk_pa_tenant FOREIGN KEY (tenant_id) REFERENCES tenants(id) ON DELETE CASCADE,
    CONSTRAINT fk_pa_user FOREIGN KEY (user_id) REFERENCES users(id) ON DELETE CASCADE
);

CREATE INDEX idx_pa_user ON policy_acceptances (user_id);

-- ── Roles and groups ────────────────────────────────────────────────────────────
-- Tenant-scoped authorization.

CREATE TABLE roles (
    id          UUID NOT NULL,
    tenant_id   UUID NOT NULL,
    name        VARCHAR(128) NOT NULL,
    description TEXT,
    created_at  TIMESTAMPTZ NOT NULL,
    updated_at  TIMESTAMPTZ,
    CONSTRAINT roles_pkey PRIMARY KEY (id),
    CONSTRAINT ck_roles_name_not_reserved CHECK ((lower(btrim((name)::text)) <> ALL (ARRAY['admin'::text, 'tenant_admin'::text]))),
    CONSTRAINT fk_role_tenant FOREIGN KEY (tenant_id) REFERENCES tenants(id) ON DELETE CASCADE
);

CREATE INDEX idx_roles_tenant_id_created_at ON roles (tenant_id, created_at DESC);
CREATE UNIQUE INDEX uq_roles_tenant_name ON roles (tenant_id, name);

CREATE TABLE role_assignments (
    role_id    UUID NOT NULL,
    user_id    UUID NOT NULL,
    created_at TIMESTAMPTZ NOT NULL,
    CONSTRAINT role_assignments_pkey PRIMARY KEY (role_id, user_id),
    CONSTRAINT fk_ra_role FOREIGN KEY (role_id) REFERENCES roles(id) ON DELETE CASCADE,
    CONSTRAINT fk_ra_user FOREIGN KEY (user_id) REFERENCES users(id) ON DELETE CASCADE
);

CREATE INDEX idx_role_assignments_user_id ON role_assignments (user_id);

CREATE TABLE groups (
    id          UUID NOT NULL,
    tenant_id   UUID NOT NULL,
    name        VARCHAR(128) NOT NULL,
    description TEXT,
    created_at  TIMESTAMPTZ NOT NULL,
    updated_at  TIMESTAMPTZ,
    CONSTRAINT groups_pkey PRIMARY KEY (id),
    CONSTRAINT fk_group_tenant FOREIGN KEY (tenant_id) REFERENCES tenants(id) ON DELETE CASCADE
);

CREATE INDEX idx_groups_tenant_id_created_at ON groups (tenant_id, created_at DESC);
CREATE UNIQUE INDEX uq_groups_tenant_name ON groups (tenant_id, name);

CREATE TABLE group_members (
    group_id   UUID NOT NULL,
    user_id    UUID NOT NULL,
    created_at TIMESTAMPTZ NOT NULL,
    CONSTRAINT group_members_pkey PRIMARY KEY (group_id, user_id),
    CONSTRAINT fk_gm_group FOREIGN KEY (group_id) REFERENCES groups(id) ON DELETE CASCADE,
    CONSTRAINT fk_gm_user FOREIGN KEY (user_id) REFERENCES users(id) ON DELETE CASCADE
);

CREATE INDEX idx_group_members_user_id ON group_members (user_id);

-- ── Sign-in, tokens and consent ─────────────────────────────────────────────────
-- The hosted login flow (auth sessions), its short-lived codes and links, OAuth
-- codes and refresh-token families, and per-client consent.

CREATE TABLE auth_sessions (
    id                            UUID NOT NULL,
    tenant_id                     UUID NOT NULL,
    session_token_hash            VARCHAR(64) NOT NULL,
    user_id                       UUID,
    authenticated_at              TIMESTAMPTZ,
    pending_client_id             UUID NOT NULL,
    pending_redirect_uri          TEXT NOT NULL,
    pending_scopes                TEXT[] NOT NULL,
    pending_state                 VARCHAR(255) NOT NULL,
    pending_code_challenge        VARCHAR(128) NOT NULL,
    pending_code_challenge_method VARCHAR(8) NOT NULL,
    pending_nonce                 VARCHAR(255),
    expires_at                    TIMESTAMPTZ NOT NULL,
    created_at                    TIMESTAMPTZ NOT NULL,
    mfa_required                  BOOLEAN DEFAULT false NOT NULL,
    mfa_verified_at               TIMESTAMPTZ,
    mfa_method                    VARCHAR(16),
    absolute_expires_at           TIMESTAMPTZ NOT NULL,
    primary_factor                VARCHAR(16) DEFAULT 'password'::character varying NOT NULL,
    risk_login_event_id           UUID,
    CONSTRAINT auth_sessions_pkey PRIMARY KEY (id),
    CONSTRAINT chk_auth_sessions_primary_factor CHECK (primary_factor IN ('password', 'passkey', 'magic_link')),
    CONSTRAINT fk_auth_sessions_client FOREIGN KEY (pending_client_id) REFERENCES oauth_clients(id) ON DELETE CASCADE,
    CONSTRAINT fk_auth_sessions_tenant FOREIGN KEY (tenant_id) REFERENCES tenants(id) ON DELETE CASCADE,
    CONSTRAINT fk_auth_sessions_user FOREIGN KEY (user_id) REFERENCES users(id) ON DELETE CASCADE
);

CREATE INDEX idx_auth_sessions_expires_at ON auth_sessions (expires_at);
CREATE INDEX idx_auth_sessions_user_id ON auth_sessions (user_id);
CREATE UNIQUE INDEX uq_auth_sessions_token_hash ON auth_sessions (session_token_hash);

CREATE TABLE authorization_codes (
    id                    UUID NOT NULL,
    tenant_id             UUID NOT NULL,
    code_hash             VARCHAR(64) NOT NULL,
    client_id             UUID NOT NULL,
    user_id               UUID NOT NULL,
    redirect_uri          TEXT NOT NULL,
    scopes                TEXT[] NOT NULL,
    code_challenge        VARCHAR(128) NOT NULL,
    code_challenge_method VARCHAR(8) NOT NULL,
    nonce                 VARCHAR(255),
    consumed_at           TIMESTAMPTZ,
    expires_at            TIMESTAMPTZ NOT NULL,
    created_at            TIMESTAMPTZ NOT NULL,
    CONSTRAINT authorization_codes_pkey PRIMARY KEY (id),
    CONSTRAINT fk_authcode_client FOREIGN KEY (client_id) REFERENCES oauth_clients(id) ON DELETE CASCADE,
    CONSTRAINT fk_authcode_user FOREIGN KEY (user_id) REFERENCES users(id) ON DELETE CASCADE
);

CREATE INDEX idx_authorization_codes_expires_at ON authorization_codes (expires_at);
CREATE UNIQUE INDEX uq_authorization_codes_code_hash ON authorization_codes (code_hash);

CREATE TABLE refresh_tokens (
    id         UUID NOT NULL,
    tenant_id  UUID NOT NULL,
    token_hash VARCHAR(64) NOT NULL,
    client_id  UUID NOT NULL,
    user_id    UUID NOT NULL,
    scopes     TEXT[] NOT NULL,
    family_id  UUID NOT NULL,
    rotated_at TIMESTAMPTZ,
    revoked_at TIMESTAMPTZ,
    expires_at TIMESTAMPTZ NOT NULL,
    created_at TIMESTAMPTZ NOT NULL,
    CONSTRAINT refresh_tokens_pkey PRIMARY KEY (id),
    CONSTRAINT fk_refresh_client FOREIGN KEY (client_id) REFERENCES oauth_clients(id) ON DELETE CASCADE,
    CONSTRAINT fk_refresh_user FOREIGN KEY (user_id) REFERENCES users(id) ON DELETE CASCADE
);

CREATE INDEX idx_refresh_tokens_family ON refresh_tokens (family_id);
CREATE INDEX idx_refresh_tokens_tenant_user_client_active ON refresh_tokens (tenant_id, user_id, client_id) WHERE (revoked_at IS NULL);
CREATE UNIQUE INDEX uq_refresh_tokens_token_hash ON refresh_tokens (token_hash);

CREATE TABLE consent_grants (
    id         UUID NOT NULL,
    tenant_id  UUID NOT NULL,
    user_id    UUID NOT NULL,
    client_id  UUID NOT NULL,
    scopes     TEXT[] NOT NULL,
    revoked_at TIMESTAMPTZ,
    created_at TIMESTAMPTZ NOT NULL,
    updated_at TIMESTAMPTZ,
    CONSTRAINT consent_grants_pkey PRIMARY KEY (id),
    CONSTRAINT fk_consent_client FOREIGN KEY (client_id) REFERENCES oauth_clients(id) ON DELETE CASCADE,
    CONSTRAINT fk_consent_user FOREIGN KEY (user_id) REFERENCES users(id) ON DELETE CASCADE
);

CREATE UNIQUE INDEX uq_consent_grants_user_client ON consent_grants (user_id, client_id);

CREATE TABLE external_login_flows (
    id                           UUID NOT NULL,
    state_hash                   VARCHAR(64) NOT NULL,
    external_provider_id         UUID NOT NULL,
    oauth_client_id              UUID NOT NULL,
    tenant_id                    UUID NOT NULL,
    provider                     VARCHAR(32) NOT NULL,
    client_redirect_uri          TEXT NOT NULL,
    client_response_type         VARCHAR(32) NOT NULL,
    client_scope                 TEXT NOT NULL,
    client_state                 TEXT NOT NULL,
    client_nonce                 TEXT,
    client_code_challenge        TEXT NOT NULL,
    client_code_challenge_method VARCHAR(16) NOT NULL,
    expires_at                   TIMESTAMPTZ NOT NULL,
    created_at                   TIMESTAMPTZ NOT NULL,
    CONSTRAINT external_login_flows_pkey PRIMARY KEY (id),
    CONSTRAINT fk_elf_oauth_client FOREIGN KEY (oauth_client_id) REFERENCES oauth_clients(id) ON DELETE CASCADE,
    CONSTRAINT fk_elf_provider FOREIGN KEY (external_provider_id) REFERENCES external_identity_providers(id) ON DELETE CASCADE
);

CREATE UNIQUE INDEX uq_elf_state_hash ON external_login_flows (state_hash);

CREATE TABLE magic_link_tokens (
    id              UUID NOT NULL,
    tenant_id       UUID NOT NULL,
    user_id         UUID NOT NULL,
    auth_session_id UUID NOT NULL,
    token_hash      VARCHAR(64) NOT NULL,
    consumed_at     TIMESTAMPTZ,
    expires_at      TIMESTAMPTZ NOT NULL,
    created_at      TIMESTAMPTZ NOT NULL,
    CONSTRAINT magic_link_tokens_pkey PRIMARY KEY (id),
    CONSTRAINT fk_magic_link_tokens_session FOREIGN KEY (auth_session_id) REFERENCES auth_sessions(id) ON DELETE CASCADE,
    CONSTRAINT fk_magic_link_tokens_tenant FOREIGN KEY (tenant_id) REFERENCES tenants(id) ON DELETE CASCADE,
    CONSTRAINT fk_magic_link_tokens_user FOREIGN KEY (user_id) REFERENCES users(id) ON DELETE CASCADE
);

CREATE INDEX idx_magic_link_tokens_auth_session ON magic_link_tokens (auth_session_id);
CREATE INDEX idx_magic_link_tokens_user_created ON magic_link_tokens (user_id, created_at DESC);

CREATE TABLE email_verification_tokens (
    id          UUID NOT NULL,
    tenant_id   UUID NOT NULL,
    user_id     UUID NOT NULL,
    token_hash  VARCHAR(64) NOT NULL,
    consumed_at TIMESTAMPTZ,
    expires_at  TIMESTAMPTZ NOT NULL,
    created_at  TIMESTAMPTZ NOT NULL,
    attempts    INTEGER DEFAULT 0 NOT NULL,
    CONSTRAINT email_verification_tokens_pkey PRIMARY KEY (id),
    CONSTRAINT fk_email_verification_tokens_tenant FOREIGN KEY (tenant_id) REFERENCES tenants(id) ON DELETE CASCADE,
    CONSTRAINT fk_email_verification_tokens_user FOREIGN KEY (user_id) REFERENCES users(id) ON DELETE CASCADE
);

CREATE INDEX idx_email_verification_tokens_expires_at ON email_verification_tokens (expires_at);
CREATE INDEX idx_email_verification_tokens_user_id ON email_verification_tokens (user_id);

CREATE TABLE password_reset_tokens (
    id          UUID NOT NULL,
    tenant_id   UUID NOT NULL,
    user_id     UUID NOT NULL,
    token_hash  VARCHAR(64) NOT NULL,
    consumed_at TIMESTAMPTZ,
    expires_at  TIMESTAMPTZ NOT NULL,
    created_at  TIMESTAMPTZ NOT NULL,
    CONSTRAINT password_reset_tokens_pkey PRIMARY KEY (id),
    CONSTRAINT fk_password_reset_tokens_tenant FOREIGN KEY (tenant_id) REFERENCES tenants(id) ON DELETE CASCADE,
    CONSTRAINT fk_password_reset_tokens_user FOREIGN KEY (user_id) REFERENCES users(id) ON DELETE CASCADE
);

CREATE INDEX idx_password_reset_tokens_expires_at ON password_reset_tokens (expires_at);
CREATE INDEX idx_password_reset_tokens_user_id ON password_reset_tokens (user_id);

CREATE TABLE password_change_tokens (
    id          UUID NOT NULL,
    tenant_id   UUID NOT NULL,
    user_id     UUID NOT NULL,
    token_hash  VARCHAR(64) NOT NULL,
    consumed_at TIMESTAMPTZ,
    expires_at  TIMESTAMPTZ NOT NULL,
    created_at  TIMESTAMPTZ NOT NULL,
    CONSTRAINT password_change_tokens_pkey PRIMARY KEY (id),
    CONSTRAINT fk_password_change_tokens_tenant FOREIGN KEY (tenant_id) REFERENCES tenants(id) ON DELETE CASCADE,
    CONSTRAINT fk_password_change_tokens_user FOREIGN KEY (user_id) REFERENCES users(id) ON DELETE CASCADE
);

CREATE INDEX idx_password_change_tokens_expires_at ON password_change_tokens (expires_at);
CREATE INDEX idx_password_change_tokens_user_id ON password_change_tokens (user_id);

-- ── Multi-factor authentication ─────────────────────────────────────────────────
-- Enrolled second factors and pending challenges.

CREATE TABLE mfa_enrollments (
    id             UUID NOT NULL,
    tenant_id      UUID NOT NULL,
    user_id        UUID NOT NULL,
    method         VARCHAR(16) NOT NULL,
    secret_enc     TEXT,
    totp_last_step BIGINT,
    passkey_json   JSONB,
    credential_id  TEXT,
    name           VARCHAR(100),
    confirmed_at   TIMESTAMPTZ,
    last_used_at   TIMESTAMPTZ,
    created_at     TIMESTAMPTZ NOT NULL,
    CONSTRAINT mfa_enrollments_pkey PRIMARY KEY (id),
    CONSTRAINT fk_mfa_enrollments_tenant FOREIGN KEY (tenant_id) REFERENCES tenants(id) ON DELETE CASCADE,
    CONSTRAINT fk_mfa_enrollments_user FOREIGN KEY (user_id) REFERENCES users(id) ON DELETE CASCADE
);

CREATE INDEX idx_mfa_enrollments_user ON mfa_enrollments (user_id);
CREATE UNIQUE INDEX uq_mfa_enrollments_cred ON mfa_enrollments (credential_id) WHERE (credential_id IS NOT NULL);
CREATE UNIQUE INDEX uq_mfa_enrollments_totp ON mfa_enrollments (user_id) WHERE ((method)::text = 'totp'::text);

CREATE TABLE mfa_challenges (
    id              UUID NOT NULL,
    tenant_id       UUID NOT NULL,
    user_id         UUID NOT NULL,
    auth_session_id UUID,
    purpose         VARCHAR(16) NOT NULL,
    method          VARCHAR(16) NOT NULL,
    code_hash       VARCHAR(64),
    state_json      JSONB,
    attempts        INTEGER DEFAULT 0 NOT NULL,
    consumed_at     TIMESTAMPTZ,
    expires_at      TIMESTAMPTZ NOT NULL,
    created_at      TIMESTAMPTZ NOT NULL,
    CONSTRAINT mfa_challenges_pkey PRIMARY KEY (id),
    CONSTRAINT fk_mfa_challenges_session FOREIGN KEY (auth_session_id) REFERENCES auth_sessions(id) ON DELETE CASCADE,
    CONSTRAINT fk_mfa_challenges_tenant FOREIGN KEY (tenant_id) REFERENCES tenants(id) ON DELETE CASCADE,
    CONSTRAINT fk_mfa_challenges_user FOREIGN KEY (user_id) REFERENCES users(id) ON DELETE CASCADE
);

CREATE INDEX idx_mfa_challenges_session ON mfa_challenges (auth_session_id);
CREATE INDEX idx_mfa_challenges_user ON mfa_challenges (user_id);

-- ── Risk engine ─────────────────────────────────────────────────────────────────
-- Login events and their signals, IP blocks, labels for the statistical model,
-- and the trained models themselves.

CREATE TABLE login_events (
    id                  UUID NOT NULL,
    tenant_id           UUID NOT NULL,
    user_id             UUID,
    client_id           UUID,
    username_normalized TEXT,
    username_hash       TEXT,
    ip_address          TEXT,
    ip_hash             TEXT,
    user_agent          TEXT,
    user_agent_hash     TEXT,
    country_code        TEXT,
    region              TEXT,
    city                TEXT,
    latitude            DOUBLE PRECISION,
    longitude           DOUBLE PRECISION,
    asn                 INTEGER,
    isp                 TEXT,
    credentials_valid   BOOLEAN NOT NULL,
    success             BOOLEAN NOT NULL,
    failure_reason      TEXT,
    risk_score          INTEGER DEFAULT 0 NOT NULL,
    risk_level          TEXT NOT NULL,
    decision            TEXT NOT NULL,
    created_at          TIMESTAMPTZ DEFAULT now() NOT NULL,
    risk_features       JSONB,
    risk_model          JSONB,
    CONSTRAINT login_events_pkey PRIMARY KEY (id),
    CONSTRAINT fk_login_events_tenant FOREIGN KEY (tenant_id) REFERENCES tenants(id) ON DELETE CASCADE
);

CREATE INDEX idx_login_events_tenant_asn_created_at ON login_events (tenant_id, asn, created_at DESC) WHERE (success AND (asn IS NOT NULL));
CREATE INDEX idx_login_events_tenant_created_at ON login_events (tenant_id, created_at DESC);
CREATE INDEX idx_login_events_tenant_ip_created_at ON login_events (tenant_id, ip_address, created_at DESC);
CREATE INDEX idx_login_events_tenant_risk_level_created_at ON login_events (tenant_id, risk_level, created_at DESC);
CREATE INDEX idx_login_events_tenant_user_created_at ON login_events (tenant_id, user_id, created_at DESC);

CREATE TABLE login_risk_signals (
    id             UUID NOT NULL,
    login_event_id UUID NOT NULL,
    tenant_id      UUID NOT NULL,
    signal_type    TEXT NOT NULL,
    severity       TEXT NOT NULL,
    points         INTEGER NOT NULL,
    metadata_json  JSONB,
    created_at     TIMESTAMPTZ DEFAULT now() NOT NULL,
    CONSTRAINT login_risk_signals_pkey PRIMARY KEY (id),
    CONSTRAINT fk_login_risk_signals_event FOREIGN KEY (login_event_id) REFERENCES login_events(id) ON DELETE CASCADE
);

CREATE INDEX idx_login_risk_signals_event ON login_risk_signals (login_event_id);
CREATE INDEX idx_login_risk_signals_tenant_signal_created_at ON login_risk_signals (tenant_id, signal_type, created_at DESC);

CREATE TABLE login_event_labels (
    login_event_id UUID NOT NULL,
    tenant_id      UUID NOT NULL,
    label          TEXT NOT NULL,
    source         TEXT NOT NULL,
    labeled_by     UUID,
    note           TEXT,
    created_at     TIMESTAMPTZ NOT NULL,
    updated_at     TIMESTAMPTZ NOT NULL,
    CONSTRAINT login_event_labels_pkey PRIMARY KEY (login_event_id),
    CONSTRAINT chk_login_event_labels_label CHECK ((label = ANY (ARRAY['legitimate'::text, 'fraudulent'::text]))),
    CONSTRAINT chk_login_event_labels_source CHECK ((source = ANY (ARRAY['admin'::text, 'mfa_passed'::text, 'mfa_failed'::text]))),
    CONSTRAINT fk_login_event_labels_event FOREIGN KEY (login_event_id) REFERENCES login_events(id) ON DELETE CASCADE
);

CREATE INDEX idx_login_event_labels_tenant_created_at ON login_event_labels (tenant_id, created_at DESC);

CREATE TABLE security_blocks (
    id         UUID NOT NULL,
    tenant_id  UUID NOT NULL,
    block_type TEXT NOT NULL,
    ip_address TEXT,
    user_id    UUID,
    reason     TEXT NOT NULL,
    source     TEXT NOT NULL,
    expires_at TIMESTAMPTZ NOT NULL,
    created_at TIMESTAMPTZ DEFAULT now() NOT NULL,
    created_by UUID,
    CONSTRAINT security_blocks_pkey PRIMARY KEY (id),
    CONSTRAINT fk_security_blocks_tenant FOREIGN KEY (tenant_id) REFERENCES tenants(id) ON DELETE CASCADE
);

CREATE INDEX idx_security_blocks_tenant_ip_expires ON security_blocks (tenant_id, ip_address, expires_at);
CREATE INDEX idx_security_blocks_tenant_user_expires ON security_blocks (tenant_id, user_id, expires_at);

CREATE TABLE risk_models (
    id             UUID NOT NULL,
    version        TEXT NOT NULL,
    feature_schema INTEGER NOT NULL,
    model          JSONB NOT NULL,
    metrics        JSONB NOT NULL,
    label_counts   JSONB NOT NULL,
    trained_from   TIMESTAMPTZ NOT NULL,
    trained_to     TIMESTAMPTZ NOT NULL,
    status         TEXT NOT NULL,
    created_at     TIMESTAMPTZ NOT NULL,
    promoted_at    TIMESTAMPTZ,
    retired_at     TIMESTAMPTZ,
    CONSTRAINT risk_models_pkey PRIMARY KEY (id),
    CONSTRAINT risk_models_version_key UNIQUE (version),
    CONSTRAINT chk_risk_models_status CHECK ((status = ANY (ARRAY['candidate'::text, 'active'::text, 'retired'::text])))
);

CREATE UNIQUE INDEX uq_risk_models_single_active ON risk_models (status) WHERE (status = 'active'::text);

-- ── Audit log ───────────────────────────────────────────────────────────────────
-- Append-only: UPDATE is refused by trigger outside a sanctioned erasure.

CREATE TABLE audit_events (
    id            UUID NOT NULL,
    tenant_id     UUID,
    actor_type    VARCHAR(32) NOT NULL,
    actor_id      UUID,
    action        VARCHAR(128) NOT NULL,
    resource_type VARCHAR(64),
    resource_id   UUID,
    outcome       VARCHAR(16) NOT NULL,
    ip_address    TEXT,
    user_agent    TEXT,
    metadata      JSONB DEFAULT '{}'::jsonb NOT NULL,
    created_at    TIMESTAMPTZ DEFAULT now() NOT NULL,
    CONSTRAINT audit_events_pkey PRIMARY KEY (id)
);

CREATE INDEX idx_audit_action ON audit_events (action, created_at DESC);
CREATE INDEX idx_audit_actor ON audit_events (actor_id, created_at DESC);
CREATE INDEX idx_audit_tenant_created ON audit_events (tenant_id, created_at DESC);
CREATE TRIGGER audit_events_append_only BEFORE UPDATE ON audit_events FOR EACH ROW EXECUTE FUNCTION audit_events_reject_update();

-- ── Outbox and webhooks ─────────────────────────────────────────────────────────
-- Domain events written in the same transaction as the change, and their
-- deliveries to webhook subscriptions and back-channel logout endpoints.

CREATE TABLE outbox_events (
    id           UUID NOT NULL,
    tenant_id    UUID NOT NULL,
    event_type   VARCHAR(64) NOT NULL,
    payload      JSONB NOT NULL,
    occurred_at  TIMESTAMPTZ NOT NULL,
    published_at TIMESTAMPTZ,
    CONSTRAINT outbox_events_pkey PRIMARY KEY (id),
    CONSTRAINT fk_outbox_events_tenant FOREIGN KEY (tenant_id) REFERENCES tenants(id) ON DELETE CASCADE
);

CREATE INDEX idx_outbox_unpublished ON outbox_events (id) WHERE (published_at IS NULL);

CREATE TABLE webhook_subscriptions (
    id               UUID NOT NULL,
    tenant_id        UUID NOT NULL,
    name             VARCHAR(255) NOT NULL,
    url              TEXT NOT NULL,
    secret_encrypted TEXT NOT NULL,
    event_types      TEXT[] DEFAULT '{}'::text[] NOT NULL,
    is_active        BOOLEAN DEFAULT true NOT NULL,
    created_at       TIMESTAMPTZ NOT NULL,
    updated_at       TIMESTAMPTZ,
    CONSTRAINT webhook_subscriptions_pkey PRIMARY KEY (id),
    CONSTRAINT fk_webhook_subscriptions_tenant FOREIGN KEY (tenant_id) REFERENCES tenants(id) ON DELETE CASCADE
);

CREATE INDEX idx_webhook_subscriptions_tenant ON webhook_subscriptions (tenant_id);

CREATE TABLE webhook_deliveries (
    id              UUID NOT NULL,
    event_id        UUID NOT NULL,
    subscription_id UUID,
    tenant_id       UUID NOT NULL,
    status          VARCHAR(16) NOT NULL,
    attempts        INTEGER DEFAULT 0 NOT NULL,
    next_attempt_at TIMESTAMPTZ NOT NULL,
    last_error      TEXT,
    last_status     INTEGER,
    created_at      TIMESTAMPTZ NOT NULL,
    delivered_at    TIMESTAMPTZ,
    client_id       UUID,
    CONSTRAINT webhook_deliveries_pkey PRIMARY KEY (id),
    CONSTRAINT uq_webhook_delivery UNIQUE (event_id, subscription_id),
    CONSTRAINT chk_webhook_deliveries_target CHECK ((num_nonnulls(subscription_id, client_id) = 1)),
    CONSTRAINT fk_webhook_deliveries_client FOREIGN KEY (client_id) REFERENCES oauth_clients(id) ON DELETE CASCADE,
    CONSTRAINT fk_webhook_deliveries_event FOREIGN KEY (event_id) REFERENCES outbox_events(id) ON DELETE CASCADE,
    CONSTRAINT fk_webhook_deliveries_subscription FOREIGN KEY (subscription_id) REFERENCES webhook_subscriptions(id) ON DELETE CASCADE
);

CREATE INDEX idx_webhook_deliveries_due ON webhook_deliveries (next_attempt_at) WHERE ((status)::text = 'pending'::text);
CREATE INDEX idx_webhook_deliveries_tenant ON webhook_deliveries (tenant_id, created_at DESC);
CREATE UNIQUE INDEX uq_webhook_delivery_client ON webhook_deliveries (event_id, client_id) WHERE (client_id IS NOT NULL);
