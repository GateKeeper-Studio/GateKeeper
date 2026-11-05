-- Add up migration script here
CREATE TABLE IF NOT EXISTS tenant_custom_fields (
  id UUID PRIMARY KEY,
  tenant_id UUID NOT NULL,
  field_key VARCHAR(64) NOT NULL,                -- immutable slug, e.g. "shirt_size"
  label VARCHAR(150) NOT NULL,
  description VARCHAR(500) NULL,
  field_type VARCHAR(20) NOT NULL,                -- 'text' | 'number' | 'boolean' | 'date' | 'select'
  options TEXT[] NULL,                            -- only meaningful for 'select'
  is_required BOOLEAN NOT NULL DEFAULT FALSE,
  display_order INT NOT NULL DEFAULT 0,
  is_active BOOLEAN NOT NULL DEFAULT TRUE,
  created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
  updated_at TIMESTAMPTZ NOT NULL DEFAULT now(),
  CONSTRAINT fk_tenant_custom_fields_tenant FOREIGN KEY (tenant_id) REFERENCES tenants (id) ON DELETE CASCADE
);
-- field_key must be unique within a tenant; lookups/listing are ordered by display_order.
CREATE UNIQUE INDEX uq_tenant_custom_fields_tenant_key ON tenant_custom_fields (tenant_id, field_key);
CREATE INDEX ix_tenant_custom_fields_tenant_order ON tenant_custom_fields (tenant_id, display_order);
