-- Runtime role with data access only (no DDL). psql variables:
--   :app_user  :app_password  :owner  :db
-- Run as the owner role:
--   psql -v app_user=gatekeeper_app -v app_password=... -v owner=gatekeeper \
--        -v db=gatekeeper -f roles.sql
CREATE ROLE :"app_user" LOGIN PASSWORD :'app_password';

GRANT CONNECT ON DATABASE :"db" TO :"app_user";
GRANT USAGE ON SCHEMA public TO :"app_user";

-- Tables and sequences that already exist...
GRANT SELECT, INSERT, UPDATE, DELETE ON ALL TABLES IN SCHEMA public TO :"app_user";
GRANT USAGE, SELECT ON ALL SEQUENCES IN SCHEMA public TO :"app_user";

-- ...and every one the owner creates later (migrations).
ALTER DEFAULT PRIVILEGES FOR ROLE :"owner" IN SCHEMA public
    GRANT SELECT, INSERT, UPDATE, DELETE ON TABLES TO :"app_user";
ALTER DEFAULT PRIVILEGES FOR ROLE :"owner" IN SCHEMA public
    GRANT USAGE, SELECT ON SEQUENCES TO :"app_user";

-- Optional hardening once the first migration has run (the table does not exist
-- at init time):
--   REVOKE ALL ON TABLE public._sqlx_migrations FROM gatekeeper_app;
