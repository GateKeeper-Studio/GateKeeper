#!/usr/bin/env bash
#
# Register this Next.js app as a CONFIDENTIAL OAuth client on the Rust Identity
# Provider, and print the credentials to drop into frontends/nextjs/.env.
#
# The client secret is Argon2-hashed with the tenant pepper and returned only once,
# at creation — so it must be created through the admin endpoint, not seeded in SQL.
#
# Usage:
#   ./scripts/register-client.sh <tenant_id> [admin_bearer_token]
#
# Environment overrides:
#   IDP_BASE_URL   (default http://localhost:8080)
#   REDIRECT_URI   (default http://localhost:5179/auth/callback)
set -euo pipefail

TENANT_ID="${1:-}"
ADMIN_TOKEN="${2:-}"
IDP_BASE_URL="${IDP_BASE_URL:-http://localhost:8080}"
REDIRECT_URI="${REDIRECT_URI:-http://localhost:5179/auth/callback}"

if [[ -z "$TENANT_ID" ]]; then
  echo "usage: $0 <tenant_id> [admin_bearer_token]" >&2
  exit 1
fi

AUTH_HEADER=()
if [[ -n "$ADMIN_TOKEN" ]]; then
  AUTH_HEADER=(-H "Authorization: Bearer ${ADMIN_TOKEN}")
fi

read -r -d '' BODY <<JSON || true
{
  "name": "Next.js BFF App",
  "client_type": "confidential",
  "redirect_uris": ["${REDIRECT_URI}"],
  "post_logout_redirect_uris": ["http://localhost:5179"],
  "allowed_scopes": ["openid", "profile", "email"],
  "allowed_grant_types": ["authorization_code", "refresh_token"],
  "require_pkce": true,
  "require_consent": false
}
JSON

echo "Registering confidential client on ${IDP_BASE_URL} for tenant ${TENANT_ID}..." >&2

RESPONSE="$(curl -fsS -X POST \
  "${IDP_BASE_URL}/tenants/${TENANT_ID}/oauth-clients" \
  -H "Content-Type: application/json" \
  "${AUTH_HEADER[@]}" \
  -d "${BODY}")"

CLIENT_ID="$(printf '%s' "$RESPONSE" | grep -o '"client_id":"[^"]*"' | head -n1 | cut -d'"' -f4)"
CLIENT_SECRET="$(printf '%s' "$RESPONSE" | grep -o '"client_secret":"[^"]*"' | head -n1 | cut -d'"' -f4)"

if [[ -z "$CLIENT_ID" || -z "$CLIENT_SECRET" ]]; then
  echo "Unexpected response:" >&2
  printf '%s\n' "$RESPONSE" >&2
  exit 1
fi

cat <<ENV

✅ Client registered. Add these to frontends/nextjs/.env :

OAUTH_TENANT_ID=${TENANT_ID}
OAUTH_CLIENT_ID=${CLIENT_ID}
OAUTH_CLIENT_SECRET=${CLIENT_SECRET}
OAUTH_REDIRECT_URI=${REDIRECT_URI}

Also set SESSION_SECRET (openssl rand -hex 32). No CORS needed (server→server).

ENV
