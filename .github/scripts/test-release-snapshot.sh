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
    run("push", "origin", "main", "--tags", cwd=work)

    original_cwd = Path.cwd()
    original_load_pr = module.load_pr_for_commit
    try:
        os.chdir(work)
        module.load_pr_for_commit = lambda api_root, repository, token, target_sha, **kwargs: {
            sha1: make_pr(301, "First patch", sha1, ["type:patch", "channel:stable"]),
            sha2: make_pr(302, "Second patch", sha2, ["type:patch", "channel:stable"]),
        }.get(target_sha)

        exit_code = module.ensure_snapshot(
            argparse.Namespace(
                target_sha=sha2,
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
        pending = module.pending_release_targets(module.DEFAULT_NOTES_REF, sha2)
        assert pending == [sha2]
    finally:
        module.load_pr_for_commit = original_load_pr
        os.chdir(original_cwd)
PY
