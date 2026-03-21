#!/usr/bin/env bash
set -euo pipefail

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
python3 - <<'PY' "$repo_root/.github/scripts/release_snapshot.py"
from __future__ import annotations

import argparse
import importlib.util
import json
import os
import subprocess
import sys
import tempfile
from pathlib import Path

script_path = Path(sys.argv[1])
spec = importlib.util.spec_from_file_location("release_snapshot", script_path)
module = importlib.util.module_from_spec(spec)
assert spec is not None and spec.loader is not None
sys.modules[spec.name] = module
spec.loader.exec_module(module)


def run(*args: str, cwd: Path) -> str:
    result = subprocess.run(["git", *args], cwd=cwd, check=True, text=True, capture_output=True)
    return result.stdout.strip()


def make_pr(number: int, title: str, head_sha: str, labels: list[str]) -> dict[str, object]:
    return {
        "number": number,
        "title": title,
        "head": {"sha": head_sha},
        "labels": [{"name": label} for label in labels],
    }


with tempfile.TemporaryDirectory(prefix="release-snapshot-") as tmp:
    repo = Path(tmp)
    run("init", cwd=repo)
    run("config", "user.name", "Test User", cwd=repo)
    run("config", "user.email", "test@example.com", cwd=repo)
    run("checkout", "-b", "main", cwd=repo)
    (repo / "Cargo.toml").write_text('[package]\nname = "demo"\nversion = "0.1.0"\n')
    (repo / "README.md").write_text("base\n")
    run("add", "Cargo.toml", "README.md", cwd=repo)
    run("commit", "-m", "base", cwd=repo)
    run("tag", "v0.1.0", cwd=repo)

    (repo / "README.md").write_text("one\n")
    run("add", "README.md", cwd=repo)
    run("commit", "-m", "one", cwd=repo)
    sha1 = run("rev-parse", "HEAD", cwd=repo)

    (repo / "README.md").write_text("two\n")
    run("add", "README.md", cwd=repo)
    run("commit", "-m", "two", cwd=repo)
    sha2 = run("rev-parse", "HEAD", cwd=repo)

    prs = {
        sha1: make_pr(101, "Patch release", sha1, ["type:patch", "channel:stable"]),
        sha2: make_pr(102, "Minor release", sha2, ["type:minor", "channel:stable"]),
    }

    original_cwd = Path.cwd()
    original_loader = module.load_pr_for_commit
    try:
        os.chdir(repo)
        module.load_pr_for_commit = lambda api_root, repository, token, target_sha, **kwargs: prs[target_sha]

        snapshot1 = module.build_snapshot(
            target_sha=sha1,
            repository="IvanLi-CN/proxy-broker",
            token="token",
            notes_ref=module.DEFAULT_NOTES_REF,
            registry="ghcr.io",
            api_root="https://api.github.com",
        )
        assert snapshot1["status"] == "pending"
        assert snapshot1["next_stable_version"] == "0.1.1"
        assert snapshot1["release_tag"] == "v0.1.1"
        run("notes", f"--ref={module.DEFAULT_NOTES_REF}", "add", "-f", "-m", json.dumps(snapshot1), sha1, cwd=repo)

        snapshot2 = module.build_snapshot(
            target_sha=sha2,
            repository="IvanLi-CN/proxy-broker",
            token="token",
            notes_ref=module.DEFAULT_NOTES_REF,
            registry="ghcr.io",
            api_root="https://api.github.com",
        )
        assert snapshot2["base_stable_version"] == "0.1.1"
        assert snapshot2["next_stable_version"] == "0.2.0"
        assert snapshot2["status"] == "pending"
        run("notes", f"--ref={module.DEFAULT_NOTES_REF}", "add", "-f", "-m", json.dumps(snapshot2), sha2, cwd=repo)

        run("tag", "v0.1.1", sha1, cwd=repo)
        pending = module.pending_release_targets(module.DEFAULT_NOTES_REF, sha2)
        assert pending == [sha1, sha2]
        assert module.publication_tags(snapshot1, notes_ref=module.DEFAULT_NOTES_REF, main_ref=sha2) == (
            "ghcr.io/ivanli-cn/proxy-broker:v0.1.1"
        )
        assert module.publication_tags(snapshot2, notes_ref=module.DEFAULT_NOTES_REF, main_ref=sha2) == (
            "ghcr.io/ivanli-cn/proxy-broker:v0.2.0,ghcr.io/ivanli-cn/proxy-broker:latest"
        )

        try:
            module.mark_released(
                argparse.Namespace(
                    target_sha=sha1,
                    notes_ref=module.DEFAULT_NOTES_REF,
                    published_tags="ghcr.io/ivanli-cn/proxy-broker:v0.1.1",
                    max_attempts=1,
                )
            )
        except module.SnapshotError as exc:
            assert "Failed to mark" in str(exc)
            assert "origin" in str(exc)
        else:
            raise AssertionError("mark_released should fail when the notes ref cannot be pushed")
    finally:
        module.load_pr_for_commit = original_loader
        os.chdir(original_cwd)

with tempfile.TemporaryDirectory(prefix="release-snapshot-tag-reuse-") as tmp:
    repo = Path(tmp)
    run("init", cwd=repo)
    run("config", "user.name", "Test User", cwd=repo)
    run("config", "user.email", "test@example.com", cwd=repo)
    run("checkout", "-b", "main", cwd=repo)
    (repo / "Cargo.toml").write_text('[package]\nname = "demo"\nversion = "0.1.0"\n')
    (repo / "README.md").write_text("base\n")
    run("add", "Cargo.toml", "README.md", cwd=repo)
    run("commit", "-m", "base", cwd=repo)
    run("tag", "v0.2.1", cwd=repo)

    (repo / "README.md").write_text("current target\n")
    run("add", "README.md", cwd=repo)
    run("commit", "-m", "current target", cwd=repo)
    sha = run("rev-parse", "HEAD", cwd=repo)
    run("tag", "v0.3.0", sha, cwd=repo)

    original_cwd = Path.cwd()
    original_loader = module.load_pr_for_commit
    try:
        os.chdir(repo)
        module.load_pr_for_commit = (
            lambda api_root, repository, token, target_sha, **kwargs: make_pr(
                201, "Tagged release", sha, ["type:minor", "channel:stable"]
            )
        )
        snapshot = module.build_snapshot(
            target_sha=sha,
            repository="IvanLi-CN/proxy-broker",
            token="token",
            notes_ref=module.DEFAULT_NOTES_REF,
            registry="ghcr.io",
            api_root="https://api.github.com",
        )
        assert snapshot["release_tag"] == "v0.3.0"
        assert snapshot["app_effective_version"] == "0.3.0"
        assert snapshot["base_stable_version"] == "0.2.1"
        assert snapshot["status"] == "pending"
    finally:
        module.load_pr_for_commit = original_loader
        os.chdir(original_cwd)

with tempfile.TemporaryDirectory(prefix="release-snapshot-rc-base-") as tmp:
    repo = Path(tmp)
    run("init", cwd=repo)
    run("config", "user.name", "Test User", cwd=repo)
    run("config", "user.email", "test@example.com", cwd=repo)
    run("checkout", "-b", "main", cwd=repo)
    (repo / "Cargo.toml").write_text('[package]\nname = "demo"\nversion = "0.1.0"\n')
    (repo / "README.md").write_text("base\n")
    run("add", "Cargo.toml", "README.md", cwd=repo)
    run("commit", "-m", "base", cwd=repo)
    run("tag", "v0.1.0", cwd=repo)

    (repo / "README.md").write_text("rc candidate\n")
    run("add", "README.md", cwd=repo)
    run("commit", "-m", "rc candidate", cwd=repo)
    sha_rc = run("rev-parse", "HEAD", cwd=repo)

    (repo / "README.md").write_text("stable patch\n")
    run("add", "README.md", cwd=repo)
    run("commit", "-m", "stable patch", cwd=repo)
    sha_patch = run("rev-parse", "HEAD", cwd=repo)

    original_cwd = Path.cwd()
    original_loader = module.load_pr_for_commit
    try:
        os.chdir(repo)
        module.load_pr_for_commit = lambda api_root, repository, token, target_sha, **kwargs: {
            sha_rc: make_pr(211, "RC candidate", sha_rc, ["type:minor", "channel:rc"]),
            sha_patch: make_pr(212, "Stable patch", sha_patch, ["type:patch", "channel:stable"]),
        }[target_sha]

        rc_snapshot = module.build_snapshot(
            target_sha=sha_rc,
            repository="IvanLi-CN/proxy-broker",
            token="token",
            notes_ref=module.DEFAULT_NOTES_REF,
            registry="ghcr.io",
            api_root="https://api.github.com",
        )
        assert rc_snapshot["release_tag"].startswith("v0.2.0-rc.")
        run("notes", f"--ref={module.DEFAULT_NOTES_REF}", "add", "-f", "-m", json.dumps(rc_snapshot), sha_rc, cwd=repo)

        stable_snapshot = module.build_snapshot(
            target_sha=sha_patch,
            repository="IvanLi-CN/proxy-broker",
            token="token",
            notes_ref=module.DEFAULT_NOTES_REF,
            registry="ghcr.io",
            api_root="https://api.github.com",
        )
        assert stable_snapshot["base_stable_version"] == "0.1.0"
        assert stable_snapshot["next_stable_version"] == "0.1.1"
        assert stable_snapshot["release_tag"] == "v0.1.1"
    finally:
        module.load_pr_for_commit = original_loader
        os.chdir(original_cwd)

with tempfile.TemporaryDirectory(prefix="release-snapshot-ensure-") as tmp:
    repo = Path(tmp)
    origin = repo / "origin.git"
    run("init", "--bare", str(origin), cwd=repo)

    work = repo / "work"
    work.mkdir()
    run("init", cwd=work)
    run("config", "user.name", "Test User", cwd=work)
    run("config", "user.email", "test@example.com", cwd=work)
    run("remote", "add", "origin", str(origin), cwd=work)
    run("checkout", "-b", "main", cwd=work)
    (work / "Cargo.toml").write_text('[package]\nname = "demo"\nversion = "0.1.0"\n')
    (work / "README.md").write_text("base\n")
    run("add", "Cargo.toml", "README.md", cwd=work)
    run("commit", "-m", "base", cwd=work)
    run("tag", "v0.1.0", cwd=work)

    (work / "README.md").write_text("first patch\n")
    run("add", "README.md", cwd=work)
    run("commit", "-m", "first patch", cwd=work)
    sha1 = run("rev-parse", "HEAD", cwd=work)

    (work / "README.md").write_text("second patch\n")
    run("add", "README.md", cwd=work)
    run("commit", "-m", "second patch", cwd=work)
    sha2 = run("rev-parse", "HEAD", cwd=work)

    (work / "README.md").write_text("docs only\n")
    run("add", "README.md", cwd=work)
    run("commit", "-m", "docs only", cwd=work)
    sha3 = run("rev-parse", "HEAD", cwd=work)

    (work / "README.md").write_text("legacy release\n")
    run("add", "README.md", cwd=work)
    run("commit", "-m", "legacy release", cwd=work)
    sha4 = run("rev-parse", "HEAD", cwd=work)
    run("tag", "v0.1.3", sha4, cwd=work)
    run("push", "origin", "main", "--tags", cwd=work)

    original_cwd = Path.cwd()
    original_load_pr = module.load_pr_for_commit
    original_release_exists = module.github_release_exists
    try:
        os.chdir(work)
        release_tags_with_objects: set[str] = {"v0.1.3"}
        module.load_pr_for_commit = lambda api_root, repository, token, target_sha, **kwargs: {
            sha1: make_pr(301, "First patch", sha1, ["type:patch", "channel:stable"]),
            sha2: make_pr(302, "Second patch", sha2, ["type:patch", "channel:stable"]),
            sha3: make_pr(303, "Docs only", sha3, ["type:docs", "channel:stable"]),
            sha4: make_pr(304, "Legacy release", sha4, ["type:patch", "channel:stable"]),
        }.get(target_sha)
        module.github_release_exists = lambda api_root, repository, token, tag_name: tag_name in release_tags_with_objects

        exit_code = module.ensure_snapshot(
            argparse.Namespace(
                target_sha=sha3,
                github_repository="IvanLi-CN/proxy-broker",
                github_token="token",
                notes_ref=module.DEFAULT_NOTES_REF,
                registry="ghcr.io",
                api_root="https://api.github.com",
                output=str(work / "snapshot.json"),
                max_attempts=1,
                target_only=False,
            )
        )
        assert exit_code == 0
        assert module.read_snapshot(module.DEFAULT_NOTES_REF, sha1)["next_stable_version"] == "0.1.1"
        assert module.read_snapshot(module.DEFAULT_NOTES_REF, sha2)["next_stable_version"] == "0.1.2"
        assert module.read_snapshot(module.DEFAULT_NOTES_REF, sha3)["status"] == "skipped"
        run("tag", "v0.1.2", sha2, cwd=work)
        output = work / "select-target-pending.out"
        exit_code = module.select_dispatch_target(
            argparse.Namespace(
                notes_ref=module.DEFAULT_NOTES_REF,
                requested_sha=sha2,
                github_repository="IvanLi-CN/proxy-broker",
                github_token="token",
                api_root="https://api.github.com",
                github_output=str(output),
            )
        )
        assert exit_code == 0
        assert output.read_text() == f"target_sha={sha1}\nassets_only=false\n"
        output = work / "select-target-skipped.out"
        exit_code = module.select_dispatch_target(
            argparse.Namespace(
                notes_ref=module.DEFAULT_NOTES_REF,
                requested_sha=sha3,
                github_repository="IvanLi-CN/proxy-broker",
                github_token="token",
                api_root="https://api.github.com",
                github_output=str(output),
            )
        )
        assert exit_code == 0
        assert output.read_text() == f"target_sha={sha1}\nassets_only=false\n"

        exit_code = module.mark_released(
            argparse.Namespace(
                target_sha=sha1,
                notes_ref=module.DEFAULT_NOTES_REF,
                published_tags="ghcr.io/ivanli-cn/proxy-broker:v0.1.1",
                max_attempts=1,
            )
        )
        assert exit_code == 0
        assert module.read_snapshot(module.DEFAULT_NOTES_REF, sha1)["status"] == "released"
        pending = module.pending_release_targets(module.DEFAULT_NOTES_REF, sha3)
        assert pending == [sha2]

        exit_code = module.ensure_snapshot(
            argparse.Namespace(
                target_sha=sha4,
                github_repository="IvanLi-CN/proxy-broker",
                github_token="token",
                notes_ref=module.DEFAULT_NOTES_REF,
                registry="ghcr.io",
                api_root="https://api.github.com",
                output=str(work / "legacy-snapshot.json"),
                max_attempts=1,
                target_only=True,
            )
        )
        assert exit_code == 0
        assert module.read_snapshot(module.DEFAULT_NOTES_REF, sha4)["snapshot_source"] == "manual-backfill"
        output = work / "select-target-legacy-backfill.out"
        exit_code = module.select_dispatch_target(
            argparse.Namespace(
                notes_ref=module.DEFAULT_NOTES_REF,
                requested_sha=sha4,
                github_repository="IvanLi-CN/proxy-broker",
                github_token="token",
                api_root="https://api.github.com",
                github_output=str(output),
            )
        )
        assert exit_code == 0
        assert output.read_text() == f"target_sha={sha4}\nassets_only=true\n"

        release_tags_with_objects.add("v0.1.2")
        output = work / "select-target-partial-release.out"
        exit_code = module.select_dispatch_target(
            argparse.Namespace(
                notes_ref=module.DEFAULT_NOTES_REF,
                requested_sha=sha2,
                github_repository="IvanLi-CN/proxy-broker",
                github_token="token",
                api_root="https://api.github.com",
                github_output=str(output),
            )
        )
        assert exit_code == 0
        assert output.read_text() == f"target_sha={sha2}\nassets_only=true\n"

        exit_code = module.mark_released(
            argparse.Namespace(
                target_sha=sha2,
                notes_ref=module.DEFAULT_NOTES_REF,
                published_tags="ghcr.io/ivanli-cn/proxy-broker:v0.1.2,ghcr.io/ivanli-cn/proxy-broker:latest",
                max_attempts=1,
            )
        )
        assert exit_code == 0
        assert module.read_snapshot(module.DEFAULT_NOTES_REF, sha2)["status"] == "released"
        output = work / "select-target-released.out"
        exit_code = module.select_dispatch_target(
            argparse.Namespace(
                notes_ref=module.DEFAULT_NOTES_REF,
                requested_sha=sha2,
                github_repository="IvanLi-CN/proxy-broker",
                github_token="token",
                api_root="https://api.github.com",
                github_output=str(output),
            )
        )
        assert exit_code == 0
        assert output.read_text() == f"target_sha={sha2}\nassets_only=true\n"
    finally:
        module.load_pr_for_commit = original_load_pr
        module.github_release_exists = original_release_exists
        os.chdir(original_cwd)
PY
