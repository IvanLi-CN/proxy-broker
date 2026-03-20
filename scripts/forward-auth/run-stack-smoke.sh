#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd -- "$SCRIPT_DIR/../.." && pwd)"
STACK_DIR="$REPO_ROOT/deploy/forward-auth"
KEEP_RUN=0
COMPOSE_PROJECT=""

usage() {
  cat <<'EOF'
Usage: run-stack-smoke.sh --compose-project <name> --http-port <port> --https-port <port> [--keep-run]
EOF
}

while [ "$#" -gt 0 ]; do
  case "$1" in
    --compose-project)
      COMPOSE_PROJECT="$2"
      shift 2
      ;;
    --http-port)
      export FORWARD_AUTH_HTTP_PORT="$2"
      shift 2
      ;;
    --https-port)
      export FORWARD_AUTH_HTTPS_PORT="$2"
      shift 2
      ;;
    --keep-run)
      KEEP_RUN=1
      shift
      ;;
    -h|--help)
      usage
      exit 0
      ;;
    *)
      echo "unknown argument: $1" >&2
      usage >&2
      exit 2
      ;;
  esac
done

if [ -z "$COMPOSE_PROJECT" ] || [ -z "${FORWARD_AUTH_HTTP_PORT:-}" ] || [ -z "${FORWARD_AUTH_HTTPS_PORT:-}" ]; then
  usage >&2
  exit 2
fi

"$REPO_ROOT/scripts/forward-auth/render-stack.sh"

set -a
# shellcheck disable=SC1091
source "$STACK_DIR/generated/stack.env"
set +a

ENV_FILE="$STACK_DIR/generated/stack.env"
CAPS_OVERRIDE="$STACK_DIR/generated/caps-compat.yaml"
COMPOSE_FILE="$STACK_DIR/compose.yaml"
COMPOSE_CMD=(docker compose --env-file "$ENV_FILE" -p "$COMPOSE_PROJECT" -f "$COMPOSE_FILE" -f "$CAPS_OVERRIDE")

generate_caps_override() {
  local services
  services="$(docker compose --env-file "$ENV_FILE" -f "$COMPOSE_FILE" config --services)"
  {
    echo 'services:'
    while IFS= read -r service; do
      [ -n "$service" ] || continue
      cat <<EOF
  ${service}:
    cap_drop:
      - ALL
    cap_add:
      - CHOWN
      - DAC_OVERRIDE
      - FSETID
      - FOWNER
      - MKNOD
      - NET_RAW
      - SETGID
      - SETUID
      - SETPCAP
      - NET_BIND_SERVICE
      - SYS_CHROOT
      - KILL
      - AUDIT_WRITE
EOF
    done <<<"$services"
  } > "$CAPS_OVERRIDE"
}

print_failure_context() {
  echo "[stack] docker compose ps" >&2
  "${COMPOSE_CMD[@]}" ps >&2 || true
  echo "[stack] docker compose logs --tail=200" >&2
  "${COMPOSE_CMD[@]}" logs --tail=200 >&2 || true
}

cleanup() {
  if [ "$KEEP_RUN" -eq 0 ]; then
    "${COMPOSE_CMD[@]}" down -v --remove-orphans || true
  fi
}

on_exit() {
  local exit_code="$1"
  if [ "$exit_code" -ne 0 ]; then
    print_failure_context
  fi
  cleanup
}

trap 'on_exit $?' EXIT

generate_caps_override

echo "[stack] building and starting compose project $COMPOSE_PROJECT"
"${COMPOSE_CMD[@]}" up -d --build
"${COMPOSE_CMD[@]}" ps

echo "[stack] running smoke tests"
"$REPO_ROOT/scripts/forward-auth/smoke.sh"

if [ "$KEEP_RUN" -eq 1 ]; then
  echo "[stack] keep-run enabled; leaving compose project $COMPOSE_PROJECT running"
fi
