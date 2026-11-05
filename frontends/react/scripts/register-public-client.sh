#!/usr/bin/env bash
#
# Register this React app as a PUBLIC OAuth client. Public clients do not receive
# or use a client secret; they rely on Authorization Code + PKCE.
#
# Usage:
#   ./scripts/register-public-client.sh <tenant_id> [admin_bearer_token]
#
# Environment overrides:
#   IDP_BASE_URL   (default http://localhost:8080)
#   REDIRECT_URI   (default http://localhost:5175/)
set -euo pipefail

TENANT_ID="${1:-}"
ADMIN_TOKEN="${2:-}"
IDP_BASE_URL="${IDP_BASE_URL:-http://localhost:8080}"
REDIRECT_URI="${REDIRECT_URI:-http://localhost:5175/}"

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
  "name": "React Public Client",
  "client_type": "public",
  "redirect_uris": ["${REDIRECT_URI}"],
  "post_logout_redirect_uris": ["http://localhost:5175"],
  "allowed_scopes": ["openid", "profile", "email"],
  "allowed_grant_types": ["authorization_code", "refresh_token"],
  "require_pkce": true,
  "require_consent": false
}
JSON

echo "Registering public client on ${IDP_BASE_URL} for tenant ${TENANT_ID}..." >&2

RESPONSE="$(curl -fsS -X POST \
  "${IDP_BASE_URL}/tenants/${TENANT_ID}/oauth-clients" \
  -H "Content-Type: application/json" \
  "${AUTH_HEADER[@]}" \
  -d "${BODY}")"

CLIENT_ID="$(printf '%s' "$RESPONSE" | grep -o '"client_id":"[^"]*"' | head -n1 | cut -d'"' -f4)"

if [[ -z "$CLIENT_ID" ]]; then
  echo "Unexpected response:" >&2
  printf '%s\n' "$RESPONSE" >&2
  exit 1
fi

cat <<ENV

Public client registered. Add these to frontends/react/.env :

VITE_GATEKEEPER_TENANT_ID=${TENANT_ID}
VITE_GATEKEEPER_CLIENT_ID=${CLIENT_ID}
VITE_GATEKEEPER_REDIRECT_URI=${REDIRECT_URI}

Remember to add ${REDIRECT_URI%/} to the IdP's CORS_ALLOWED_ORIGINS.

ENV
