#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd -- "$SCRIPT_DIR/../.." && pwd)"
STACK_ENV="${FORWARD_AUTH_STACK_ENV:-$REPO_ROOT/deploy/forward-auth/generated/stack.env}"

if [ ! -f "$STACK_ENV" ]; then
  echo "missing stack env: $STACK_ENV" >&2
  exit 2
fi

set -a
# shellcheck disable=SC1090
source "$STACK_ENV"
set +a

TMP_DIR="$(mktemp -d)"
trap 'rm -rf "$TMP_DIR"' EXIT

basic_header() {
  printf 'Authorization: Basic %s' "$(printf '%s:%s' "$1" "$2" | base64 | tr -d '\n')"
}

curl_common=(
  --silent
  --show-error
  --insecure
  --resolve "${FORWARD_AUTH_AUTHELIA_HOST}:${FORWARD_AUTH_HTTPS_PORT}:127.0.0.1"
  --resolve "${FORWARD_AUTH_BROKER_HOST}:${FORWARD_AUTH_HTTPS_PORT}:127.0.0.1"
  --resolve "${FORWARD_AUTH_BROKER_BASIC_HOST}:${FORWARD_AUTH_HTTPS_PORT}:127.0.0.1"
  --resolve "${FORWARD_AUTH_MACHINE_HOST}:${FORWARD_AUTH_HTTPS_PORT}:127.0.0.1"
)

request_json() {
  local method="$1"
  local url="$2"
  local output="$3"
  shift 3

  curl "${curl_common[@]}" \
    -o "$output" \
    -w '%{http_code}' \
    -X "$method" \
    "$@" \
    "$url"
}

assert_json_field() {
  local json_file="$1"
  local expression="$2"
  local expected="$3"

  python3 - "$json_file" "$expression" "$expected" <<'PY'
import json
import sys

path, expression, expected = sys.argv[1:]
value = json.load(open(path, "r", encoding="utf-8"))
for chunk in expression.split("."):
    if chunk:
        value = value[chunk]
if str(value) != expected:
    raise SystemExit(f"expected {expression}={expected!r}, got {value!r}")
PY
}

wait_for_machine_health() {
  local body="$TMP_DIR/health.json"
  local status=""

  for _ in $(seq 1 90); do
    status="$(request_json \
      GET \
      "https://${FORWARD_AUTH_MACHINE_HOST}:${FORWARD_AUTH_HTTPS_PORT}/healthz" \
      "$body" 2>/dev/null)" || true
    if [ "$status" = "200" ] && grep -q '"status":"ok"' "$body"; then
      return 0
    fi
    sleep 2
  done

  echo "proxy-broker health check did not become ready" >&2
  return 1
}

echo "[smoke] waiting for machine host health"
wait_for_machine_health

echo "[smoke] checking unauthenticated human route is rejected by proxy-broker, not by Traefik"
anonymous_ui="$TMP_DIR/anonymous-ui.json"
status="$(request_json \
  GET \
  "https://${FORWARD_AUTH_BROKER_HOST}:${FORWARD_AUTH_HTTPS_PORT}/" \
  "$anonymous_ui")"
[ "$status" = "401" ]
assert_json_field "$anonymous_ui" "code" "authentication_required"

echo "[smoke] admin can resolve identity and open UI through Authelia basic route"
admin_me="$TMP_DIR/admin-me.json"
status="$(request_json \
  GET \
  "https://${FORWARD_AUTH_BROKER_BASIC_HOST}:${FORWARD_AUTH_HTTPS_PORT}/api/v1/auth/me" \
  "$admin_me" \
  -H "$(basic_header "$FORWARD_AUTH_ADMIN_USER" "$FORWARD_AUTH_ADMIN_PASSWORD")")"
[ "$status" = "200" ]
assert_json_field "$admin_me" "principal_type" "human"
assert_json_field "$admin_me" "subject" "$FORWARD_AUTH_ADMIN_USER"
assert_json_field "$admin_me" "is_admin" "True"

admin_ui="$TMP_DIR/admin-ui.html"
status="$(request_json \
  GET \
  "https://${FORWARD_AUTH_BROKER_BASIC_HOST}:${FORWARD_AUTH_HTTPS_PORT}/" \
  "$admin_ui" \
  -H "$(basic_header "$FORWARD_AUTH_ADMIN_USER" "$FORWARD_AUTH_ADMIN_PASSWORD")")"
[ "$status" = "200" ]
grep -qi '<!doctype html>' "$admin_ui"

echo "[smoke] non-admin human is identified but denied admin surfaces"
viewer_me="$TMP_DIR/viewer-me.json"
status="$(request_json \
  GET \
  "https://${FORWARD_AUTH_BROKER_BASIC_HOST}:${FORWARD_AUTH_HTTPS_PORT}/api/v1/auth/me" \
  "$viewer_me" \
  -H "$(basic_header "$FORWARD_AUTH_VIEWER_USER" "$FORWARD_AUTH_VIEWER_PASSWORD")")"
[ "$status" = "200" ]
assert_json_field "$viewer_me" "subject" "$FORWARD_AUTH_VIEWER_USER"
assert_json_field "$viewer_me" "is_admin" "False"

viewer_profiles="$TMP_DIR/viewer-profiles.json"
status="$(request_json \
  GET \
  "https://${FORWARD_AUTH_BROKER_BASIC_HOST}:${FORWARD_AUTH_HTTPS_PORT}/api/v1/profiles" \
  "$viewer_profiles" \
  -H "$(basic_header "$FORWARD_AUTH_VIEWER_USER" "$FORWARD_AUTH_VIEWER_PASSWORD")")"
[ "$status" = "403" ]
assert_json_field "$viewer_profiles" "code" "admin_required"

viewer_ui="$TMP_DIR/viewer-ui.json"
status="$(request_json \
  GET \
  "https://${FORWARD_AUTH_BROKER_BASIC_HOST}:${FORWARD_AUTH_HTTPS_PORT}/" \
  "$viewer_ui" \
  -H "$(basic_header "$FORWARD_AUTH_VIEWER_USER" "$FORWARD_AUTH_VIEWER_PASSWORD")")"
[ "$status" = "403" ]
assert_json_field "$viewer_ui" "code" "admin_required"

echo "[smoke] creating profiles and a profile API key as admin"
create_default="$TMP_DIR/create-default.json"
status="$(request_json \
  POST \
  "https://${FORWARD_AUTH_BROKER_BASIC_HOST}:${FORWARD_AUTH_HTTPS_PORT}/api/v1/profiles" \
  "$create_default" \
  -H "$(basic_header "$FORWARD_AUTH_ADMIN_USER" "$FORWARD_AUTH_ADMIN_PASSWORD")" \
  -H "Content-Type: application/json" \
  --data '{"profile_id":"default"}')"
[ "$status" = "201" ]
assert_json_field "$create_default" "profile_id" "default"

create_other="$TMP_DIR/create-other.json"
status="$(request_json \
  POST \
  "https://${FORWARD_AUTH_BROKER_BASIC_HOST}:${FORWARD_AUTH_HTTPS_PORT}/api/v1/profiles" \
  "$create_other" \
  -H "$(basic_header "$FORWARD_AUTH_ADMIN_USER" "$FORWARD_AUTH_ADMIN_PASSWORD")" \
  -H "Content-Type: application/json" \
  --data '{"profile_id":"other"}')"
[ "$status" = "201" ]
assert_json_field "$create_other" "profile_id" "other"

create_key="$TMP_DIR/create-key.json"
status="$(request_json \
  POST \
  "https://${FORWARD_AUTH_BROKER_BASIC_HOST}:${FORWARD_AUTH_HTTPS_PORT}/api/v1/profiles/default/api-keys" \
  "$create_key" \
  -H "$(basic_header "$FORWARD_AUTH_ADMIN_USER" "$FORWARD_AUTH_ADMIN_PASSWORD")" \
  -H "Content-Type: application/json" \
  --data '{"name":"smoke-bot"}')"
[ "$status" = "201" ]
assert_json_field "$create_key" "api_key.name" "smoke-bot"

api_key_secret="$(python3 - "$create_key" <<'PY'
import json
import sys
print(json.load(open(sys.argv[1], "r", encoding="utf-8"))["secret"])
PY
)"

echo "[smoke] machine host accepts profile API key and enforces profile scope"
machine_me="$TMP_DIR/machine-me.json"
status="$(request_json \
  GET \
  "https://${FORWARD_AUTH_MACHINE_HOST}:${FORWARD_AUTH_HTTPS_PORT}/api/v1/auth/me" \
  "$machine_me" \
  -H "Authorization: Bearer ${api_key_secret}")"
[ "$status" = "200" ]
assert_json_field "$machine_me" "principal_type" "api_key"
assert_json_field "$machine_me" "profile_id" "default"

machine_default="$TMP_DIR/machine-default.json"
status="$(request_json \
  GET \
  "https://${FORWARD_AUTH_MACHINE_HOST}:${FORWARD_AUTH_HTTPS_PORT}/api/v1/profiles/default/sessions" \
  "$machine_default" \
  -H "Authorization: Bearer ${api_key_secret}")"
[ "$status" = "200" ]

machine_other="$TMP_DIR/machine-other.json"
status="$(request_json \
  GET \
  "https://${FORWARD_AUTH_MACHINE_HOST}:${FORWARD_AUTH_HTTPS_PORT}/api/v1/profiles/other/sessions" \
  "$machine_other" \
  -H "Authorization: Bearer ${api_key_secret}")"
[ "$status" = "403" ]
assert_json_field "$machine_other" "code" "profile_access_denied"

printf '[smoke] success: https://%s:%s, https://%s:%s, https://%s:%s\n' \
  "$FORWARD_AUTH_BROKER_HOST" \
  "$FORWARD_AUTH_HTTPS_PORT" \
  "$FORWARD_AUTH_BROKER_BASIC_HOST" \
  "$FORWARD_AUTH_HTTPS_PORT" \
  "$FORWARD_AUTH_MACHINE_HOST" \
  "$FORWARD_AUTH_HTTPS_PORT"
