#!/usr/bin/env bash
set -euo pipefail

TESTBOX="${TESTBOX:-codex-testbox}"
KEEP_RUN=0
HTTP_PORT=""
HTTPS_PORT=""
STACK_VARIANT="${FORWARD_AUTH_STACK_VARIANT:-build}"

usage() {
  cat <<'EOF'
Usage: run-shared-testbox.sh [--http-port <port>] [--https-port <port>] [--variant <build|ghcr>] [--keep-run]
EOF
}

while [ "$#" -gt 0 ]; do
  case "$1" in
    --http-port)
      HTTP_PORT="$2"
      shift 2
      ;;
    --https-port)
      HTTPS_PORT="$2"
      shift 2
      ;;
    --variant)
      STACK_VARIANT="$2"
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

case "$STACK_VARIANT" in
  build|ghcr)
    ;;
  *)
    echo "unsupported variant: $STACK_VARIANT (expected build|ghcr)" >&2
    exit 2
    ;;
esac

if REPO_ROOT="$(git rev-parse --show-toplevel 2>/dev/null)"; then
  :
else
  REPO_ROOT="$(pwd)"
fi

REPO_ROOT="$(python3 - "$REPO_ROOT" <<'PY'
import os
import sys
print(os.path.realpath(sys.argv[1]))
PY
)"
REPO_NAME="$(basename "$REPO_ROOT")"
PATH_HASH8="$(python3 - "$REPO_ROOT" <<'PY'
import hashlib
import os
import sys
print(hashlib.sha256(os.path.realpath(sys.argv[1]).encode()).hexdigest()[:8])
PY
)"
GIT_SHA="$(git -C "$REPO_ROOT" rev-parse --short HEAD 2>/dev/null || echo nogit)"
RUN_ID="$(date -u +%Y%m%d_%H%M%S)_${GIT_SHA}"
WORKSPACE_SLUG="${REPO_NAME}__${PATH_HASH8}"
REMOTE_BASE="/srv/codex/workspaces/${USER}"
REMOTE_WORKSPACE="${REMOTE_BASE}/${WORKSPACE_SLUG}"
REMOTE_RUN="${REMOTE_WORKSPACE}/runs/${RUN_ID}"
COMPOSE_PROJECT_RAW="codex_${WORKSPACE_SLUG}_${RUN_ID}"
COMPOSE_PROJECT="$(python3 - "$COMPOSE_PROJECT_RAW" <<'PY'
import re
import sys
s = re.sub(r'[^a-z0-9_-]+', '_', sys.argv[1].lower()).strip('_')
print(s[:63] if len(s) > 63 else s)
PY
)"

if [ -z "$HTTP_PORT" ] || [ -z "$HTTPS_PORT" ]; then
  read -r picked_http picked_https <<EOF
$(ssh -o BatchMode=yes "$TESTBOX" "python3 - <<'PY'
import socket
ports = []
for _ in range(2):
    sock = socket.socket()
    sock.bind(('127.0.0.1', 0))
    ports.append(str(sock.getsockname()[1]))
    sock.close()
print(' '.join(ports))
PY")
EOF
  : "${HTTP_PORT:=$picked_http}"
  : "${HTTPS_PORT:=$picked_https}"
fi

cleanup_remote_run() {
  if [ "$KEEP_RUN" -eq 0 ]; then
    ssh -o BatchMode=yes "$TESTBOX" "rm -rf '$REMOTE_RUN'" >/dev/null 2>&1 || true
  fi
}

trap cleanup_remote_run EXIT

CREATED_UTC="$(date -u +%Y-%m-%dT%H:%M:%SZ)"
ssh -o BatchMode=yes "$TESTBOX" "mkdir -p '$REMOTE_RUN' && cat > '$REMOTE_WORKSPACE/workspace.txt'" <<EOF
local_repo_root=${REPO_ROOT}
created_utc=${CREATED_UTC}
EOF

rsync -az --delete \
  --exclude '.git/' \
  --exclude 'node_modules/' \
  --exclude 'target/' \
  --exclude 'dist/' \
  --exclude 'build/' \
  --exclude '.next/' \
  --exclude '.venv/' \
  --exclude 'deploy/forward-auth/generated/' \
  "$REPO_ROOT/" "$TESTBOX:$REMOTE_RUN/"

REMOTE_ARGS=(
  ./scripts/forward-auth/run-stack-smoke.sh
  --compose-project "$COMPOSE_PROJECT"
  --http-port "$HTTP_PORT"
  --https-port "$HTTPS_PORT"
  --variant "$STACK_VARIANT"
)
if [ "$KEEP_RUN" -eq 1 ]; then
  REMOTE_ARGS+=(--keep-run)
fi

remote_cmd="$(printf '%q ' "${REMOTE_ARGS[@]}")"
ssh -o BatchMode=yes "$TESTBOX" "set -euo pipefail; cd '$REMOTE_RUN'; ${remote_cmd}"

echo "shared-testbox smoke passed"
echo "variant=$STACK_VARIANT"
echo "remote_run=$REMOTE_RUN"
echo "compose_project=$COMPOSE_PROJECT"
echo "https_port=$HTTPS_PORT"
echo "http_port=$HTTP_PORT"
echo "broker_host=broker.forward-auth.test"
echo "broker_basic_host=broker-basic.forward-auth.test"
echo "machine_host=machine-broker.forward-auth.test"
