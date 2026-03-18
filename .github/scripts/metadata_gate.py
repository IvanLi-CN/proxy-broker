#!/usr/bin/env python3
from __future__ import annotations

import argparse
import json
import os
import sys
import urllib.error
import urllib.request
from pathlib import Path

API_VERSION = "2022-11-28"
ALLOWED_TYPE_LABELS = {
    "type:docs",
    "type:skip",
    "type:patch",
    "type:minor",
    "type:major",
}
ALLOWED_CHANNEL_LABELS = {"channel:stable", "channel:rc"}


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser(description="Validate release intent labels for a PR.")
    parser.add_argument("gate", choices=["label"])
    parser.add_argument("--repo", default=os.environ.get("GITHUB_REPOSITORY", ""))
    parser.add_argument("--api-root", default=os.environ.get("GITHUB_API_URL", "https://api.github.com"))
    parser.add_argument("--token", default=os.environ.get("GITHUB_TOKEN", ""))
    parser.add_argument("--event-path", default=os.environ.get("GITHUB_EVENT_PATH", ""))
    parser.add_argument("--pull-number", type=int, default=None)
    return parser.parse_args()


def load_event_payload(event_path: str) -> dict:
    if not event_path:
        return {}
    path = Path(event_path)
    if not path.is_file():
        return {}
    return json.loads(path.read_text())


def split_repo(full_name: str) -> tuple[str, str]:
    owner, sep, repo = full_name.partition("/")
    if not sep or not owner or not repo:
        raise SystemExit("Repository must be in owner/name form")
    return owner, repo


def resolve_pull_number(args: argparse.Namespace, payload: dict) -> int:
    if args.pull_number:
        return args.pull_number
    pr = payload.get("pull_request")
    if isinstance(pr, dict):
        number = pr.get("number")
        if isinstance(number, int) and number > 0:
            return number
    raise SystemExit("Missing pull request number for label gate")


def github_json(api_root: str, token: str, url_path: str):
    headers = {
        "Accept": "application/vnd.github+json",
        "User-Agent": "proxy-broker-label-gate",
        "X-GitHub-Api-Version": API_VERSION,
    }
    if token:
        headers["Authorization"] = f"Bearer {token}"
    req = urllib.request.Request(f"{api_root.rstrip('/')}{url_path}", headers=headers)
    with urllib.request.urlopen(req, timeout=30) as resp:
        return json.loads(resp.read().decode("utf-8"))


def describe(labels: list[str]) -> str:
    if not labels:
        return "(none)"
    return ", ".join(sorted(set(labels)))


def validate_labels(labels: list[str]) -> tuple[bool, str]:
    type_labels = sorted({label for label in labels if label.startswith("type:")})
    channel_labels = sorted({label for label in labels if label.startswith("channel:")})
    unknown_type = [label for label in type_labels if label not in ALLOWED_TYPE_LABELS]
    unknown_channel = [label for label in channel_labels if label not in ALLOWED_CHANNEL_LABELS]
    selected_type = [label for label in type_labels if label in ALLOWED_TYPE_LABELS]
    selected_channel = [label for label in channel_labels if label in ALLOWED_CHANNEL_LABELS]

    problems: list[str] = []
    if unknown_type:
      problems.append(f"unknown type label(s): {', '.join(unknown_type)}")
    if unknown_channel:
      problems.append(f"unknown channel label(s): {', '.join(unknown_channel)}")
    if len(selected_type) != 1:
      problems.append(f"expected exactly 1 type:* label, got {len(selected_type)}")
    if len(selected_channel) != 1:
      problems.append(f"expected exactly 1 channel:* label, got {len(selected_channel)}")

    if problems:
        return False, f"{'; '.join(problems)} | labels={describe(labels)}"
    return True, f"labels OK: {selected_type[0]} + {selected_channel[0]}"


def write_summary(lines: list[str]) -> None:
    step_summary = os.environ.get("GITHUB_STEP_SUMMARY", "")
    if not step_summary:
        return
    with open(step_summary, "a", encoding="utf-8") as handle:
        handle.write("\n".join(lines) + "\n")


def main() -> int:
    args = parse_args()
    owner, repo = split_repo(args.repo)
    payload = load_event_payload(args.event_path)
    pull_number = resolve_pull_number(args, payload)
    issue = github_json(
        args.api_root,
        args.token,
        f"/repos/{owner}/{repo}/issues/{pull_number}",
    )
    raw_labels = issue.get("labels") if isinstance(issue, dict) else []
    names = sorted(
        {
            str(label.get("name"))
            for label in raw_labels
            if isinstance(label, dict) and label.get("name")
        }
    )
    passed, description = validate_labels(names)
    write_summary(
        [
            "## PR label gate",
            f"- PR #{pull_number}: {'pass' if passed else 'fail'}",
            f"- detail: {description}",
        ]
    )
    if not passed:
        print(f"PR #{pull_number}: {description}", file=sys.stderr)
        return 1
    print(f"PR #{pull_number}: {description}")
    return 0


if __name__ == "__main__":
    try:
        raise SystemExit(main())
    except urllib.error.HTTPError as exc:
        detail = exc.read().decode("utf-8", errors="replace")
        raise SystemExit(f"GitHub API error: {exc.code} {detail or exc.reason}") from exc
