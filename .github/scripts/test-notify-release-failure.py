#!/usr/bin/env python3
"""Check the release-failure notification workflow's migration contract."""

from __future__ import annotations

import re
from pathlib import Path


WORKFLOW = Path(".github/workflows/notify-release-failure.yml")
OIDRUNE_REFERENCE = (
    "IvanLi-CN/oidrune/.github/workflows/notify.yml@"
    "e48822f99c6402a753ed86557ea029754cbab20b"
)
LEGACY_REFERENCE = "IvanLi-CN/github-workflows/.github/workflows/release-failure-telegram.yml@main"


def require(condition: bool, message: str) -> None:
    if not condition:
        raise SystemExit(message)


def job_section(workflow: str, job_name: str, next_job_name: str | None = None) -> str:
    start = workflow.index(f"  {job_name}:")
    end = workflow.index(f"  {next_job_name}:", start) if next_job_name else len(workflow)
    return workflow[start:end]


def with_keys(section: str) -> set[str]:
    with_block = section[section.index("    with:") :]
    return set(re.findall(r"^      ([a-z_]+):", with_block, flags=re.MULTILINE))


def main() -> int:
    workflow = WORKFLOW.read_text(encoding="utf-8")
    failure = job_section(workflow, "notify_failure", "smoke_test")
    smoke = job_section(workflow, "smoke_test")

    require(LEGACY_REFERENCE not in workflow, "legacy Telegram reusable workflow reference remains")
    require(workflow.count(OIDRUNE_REFERENCE) == 2, "both notification paths must pin Oidrune at the trusted SHA")
    require(workflow.count("id-token: write") == 2, "each Oidrune caller must grant id-token: write")
    require("secrets:" not in workflow, "legacy Telegram secret forwarding must be removed")
    require("gateway_url:" not in workflow, "caller must use Oidrune's default gateway")
    require("oidc_audience:" not in workflow, "caller must use Oidrune's default OIDC audience")

    require(
        "    workflows:\n      - Release\n    types:\n      - completed\n    branches:\n      - main\n" in workflow,
        "workflow_run filter must remain bound to failed Release runs on main",
    )
    require("  workflow_dispatch:\n" in workflow, "manual workflow_dispatch smoke entrypoint is missing")
    require(
        "if: ${{ github.event_name == 'workflow_run' && github.event.workflow_run.conclusion == 'failure' }}" in failure,
        "release notification must only run for failed workflow_run events",
    )
    require("needs:\n      - resolve_release_context\n" in failure, "failure path must keep metadata resolution")
    require("if: ${{ github.event_name == 'workflow_dispatch' }}" in smoke, "smoke path must remain manual-only")

    for section, job_name in ((failure, "notify_failure"), (smoke, "smoke_test")):
        require(f"uses: {OIDRUNE_REFERENCE}" in section, f"{job_name} must call the pinned Oidrune workflow")
        require(
            "permissions:\n      id-token: write\n" in section,
            f"{job_name} must grant id-token: write to its reusable workflow call",
        )

    require(with_keys(failure) == {"outcome", "summary"}, "failure path must pass only Oidrune inputs")
    require(with_keys(smoke) == {"outcome", "summary"}, "smoke path must pass only Oidrune inputs")
    for section, title in (
        (failure, "🚨 Release Failed · ${{ github.repository }}"),
        (smoke, "🧪 Smoke Test · ${{ github.repository }}"),
    ):
        require(title in section, f"notification title is missing: {title}")
        require("target_sha:" in section, "summary must include the resolved target SHA")
        require("run_url:" in section, "summary must include the failed or smoke run URL")
        require("status:" in section, "summary must include status")
        for field in ("workflow:", "event:", "ref:", "attempt:", "actor:", "details:"):
            require(field in section, f"summary must include caller metadata: {field}")

    require(
        "outcome: ${{ github.event.workflow_run.conclusion }}" in failure,
        "failure outcome must preserve workflow conclusion",
    )
    require("outcome: failure" in smoke, "smoke outcome must remain failure")
    require("status: smoke test" in smoke, "smoke summary must preserve its distinct status")
    require(
        "target_sha: ${{ needs.resolve_release_context.outputs.head_sha }}" in failure,
        "failure summary must use resolved release target SHA",
    )
    require("target_sha: ${{ github.sha }}" in smoke, "smoke summary must use the dispatch commit SHA")
    require(
        "details: ${{ needs.resolve_release_context.outputs.extra_details }}" in failure,
        "failure resolver details must remain visible",
    )
    require("details: manual notifier smoke test" in smoke, "smoke details must remain visible")
    print("test-notify-release-failure: all checks passed")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
