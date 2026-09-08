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
    require(
        "issues: write" not in workflow,
        "release publisher must not request issue write permission for source PR comments",
    )
    require(
        "pull-requests: write" not in workflow,
        "release publisher must not request pull request write permission for source PR comments",
    )
    require(
        "name: Comment on source PR" not in workflow,
        "release workflow must not comment on the source PR",
    )
    require(
        "github.rest.issues." not in workflow,
        "release workflow must not call the issue comment API",
    )
    require(
        "pr_number: ${{ steps.snapshot.outputs.pr_number }}" not in workflow,
        "release workflow must not expose source PR metadata for post-publish comments",
    )
    assets_index = workflow.index("name: Upload binary release assets")
    mark_released_index = workflow.index("name: Mark snapshot as released")
    require(
        assets_index < mark_released_index,
        "release snapshot must be marked after binary assets are uploaded",
    )
    require(
        "python3 .github/scripts/release_snapshot.py mark-released" in workflow[mark_released_index:],
        "release workflow must mark the snapshot as released after publication",
    )
    require_rust_cache_does_not_restore_bins()
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
