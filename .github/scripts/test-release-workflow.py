#!/usr/bin/env python3
from __future__ import annotations

from pathlib import Path


WORKFLOW = Path(".github/workflows/release.yml")
WORKFLOW_DIR = Path(".github/workflows")


def require(condition: bool, message: str) -> None:
    if not condition:
        raise SystemExit(message)


def require_rust_cache_does_not_restore_bins() -> None:
    for workflow in sorted(WORKFLOW_DIR.glob("*.yml")):
        lines = workflow.read_text(encoding="utf-8").splitlines()
        for index, line in enumerate(lines):
            if "uses: Swatinem/rust-cache@v2" not in line:
                continue

            window = lines[index + 1 : index + 8]
            has_cache_bin_disabled = any(
                "cache-bin:" in candidate and '"false"' in candidate for candidate in window
            )
            require(
                has_cache_bin_disabled,
                f'{workflow}:{index + 1}: rust-cache must set cache-bin: "false"',
            )


def main() -> int:
    workflow = WORKFLOW.read_text(encoding="utf-8")
    require("issues: write" in workflow, "release publisher must be allowed to comment on PRs")
    require(
        "pull-requests: write" in workflow,
        "release publisher must request pull request write permission for PR comments",
    )
    require("name: Comment on source PR" in workflow, "release workflow must comment on the source PR")
    require(
        "if: needs.release-meta.outputs.pr_number != ''" in workflow,
        "release PR comment step must be skipped when the snapshot has no PR number",
    )
    require(
        "proxy-broker:release-success:${releaseTag}" in workflow,
        "release PR comments must carry a stable idempotency marker",
    )
    require(
        "github.rest.issues.updateComment" in workflow and "github.rest.issues.createComment" in workflow,
        "release PR comments must be idempotent across reruns",
    )
    require(
        "release_html_url" in workflow and "RELEASE_URL: ${{ steps.ensure-release.outputs.release_html_url }}" in workflow,
        "release PR comments must link to the published GitHub Release",
    )
    comment_index = workflow.index("name: Comment on source PR")
    mark_released_index = workflow.index("name: Mark snapshot as released")
    require(
        comment_index < mark_released_index,
        "release PR comment must run before marking the snapshot released",
    )
    require_rust_cache_does_not_restore_bins()
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
