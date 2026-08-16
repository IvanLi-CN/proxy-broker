#!/usr/bin/env bash
set -euo pipefail

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"

assert_workflow_group() {
  local workflow_path="$1"
  local expected_group="$2"
  grep -Fqx "  group: ${expected_group}" "${workflow_path}"
  grep -Fqx "  cancel-in-progress: true" "${workflow_path}"
  grep -Fq "edited" "${workflow_path}"
}

assert_workflow_group "${repo_root}/.github/workflows/ci-pr.yml" 'ci-pr-${{ github.event_name == '"'"'pull_request'"'"' && github.event.action == '"'"'edited'"'"' && format('"'"'metadata-{0}-{1}'"'"', github.event.pull_request.number, github.run_id) || github.event.pull_request.number || github.ref }}'
assert_workflow_group "${repo_root}/.github/workflows/label-gate.yml" 'label-gate-${{ github.event_name == '"'"'pull_request'"'"' && github.event.action == '"'"'edited'"'"' && format('"'"'metadata-{0}-{1}'"'"', github.event.pull_request.number, github.run_id) || github.event.pull_request.number || github.run_id }}'

echo "test-workflow-concurrency: all checks passed"
