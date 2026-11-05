#!/bin/sh
# Creates the DML-only runtime role the server connects with.
#
# Runs once, when the Postgres data directory is first initialised (mounted into
# /docker-entrypoint-initdb.d by docker-compose.prod.yml). For an existing
# database, run the same SQL by hand as the owner role (see roles.sql).
#
# Roles:
#   $POSTGRES_USER          owner: creates/alters tables. Used ONLY by the
#                           `migrate` job (DATABASE_MIGRATION_URL).
#   $POSTGRES_APP_USER      runtime: SELECT/INSERT/UPDATE/DELETE, no DDL. Used by
#                           the server (DATABASE_URL). `audit_events` additionally
#                           refuses UPDATE through a trigger (see migrations).
set -eu

psql -v ON_ERROR_STOP=1 \
     --username "$POSTGRES_USER" --dbname "$POSTGRES_DB" \
     -v app_user="$POSTGRES_APP_USER" -v app_password="$POSTGRES_APP_PASSWORD" \
     -v owner="$POSTGRES_USER" -v db="$POSTGRES_DB" \
     -f /opt/gatekeeper/roles.sql
