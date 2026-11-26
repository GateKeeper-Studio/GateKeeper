#!/usr/bin/env bash
#
# Sourced by the example apps' register scripts. Prints a short-lived (15 min)
# platform-admin access token for the identity provider at $1.
#
# Signs in through POST /admin/login, the admin API's bearer sign-in (the console
# itself uses a cookie session). Reads ADMIN_EMAIL from the environment or asks for
# it; always asks for the password without echoing it; asks for a two-step code when
# the account has one. Needs `node` (every example already does) to build JSON safely.
#
# Instead of a script you can create the client in the console:
#   {IDP_BASE_URL}/console -> tenant -> OAuth clients -> New.

gatekeeper_admin_token() {
  local idp="$1"
  local email="${ADMIN_EMAIL:-}" password code body response token

  if [[ -z "$email" ]]; then
    read -r -p "Admin email: " email </dev/tty
  fi
  read -r -s -p "Admin password: " password </dev/tty
  echo >&2

  body="$(node -e 'process.stdout.write(JSON.stringify({email: process.argv[1], password: process.argv[2]}))' "$email" "$password")"
  response="$(curl -sS -X POST "${idp}/admin/login" -H "Content-Type: application/json" -d "$body")"

  if [[ "$response" == *'"mfa_required"'* ]]; then
    read -r -p "Two-step verification code (or a recovery code): " code </dev/tty
    body="$(node -e 'process.stdout.write(JSON.stringify({email: process.argv[1], password: process.argv[2], mfa_code: process.argv[3]}))' "$email" "$password" "$code")"
    response="$(curl -sS -X POST "${idp}/admin/login" -H "Content-Type: application/json" -d "$body")"
  fi
  unset password

  token="$(printf '%s' "$response" | grep -o '"access_token":"[^"]*"' | head -n1 | cut -d'"' -f4)"
  if [[ -z "$token" ]]; then
    echo "Admin sign-in failed: ${response}" >&2
    return 1
  fi
  printf '%s' "$token"
}
