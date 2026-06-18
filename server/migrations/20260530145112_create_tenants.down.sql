-- Add down migration script here
DROP TABLE IF EXISTS "tenants";

CREATE INDEX uq_tenants_name ON tenants (name);