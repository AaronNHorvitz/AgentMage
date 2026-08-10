#!/usr/bin/env python3
"""Build and validate the reviewable Story 1.1 architecture artifact index."""

from __future__ import annotations

import argparse
import hashlib
import json
import sys
from pathlib import Path, PurePosixPath
from typing import Any


ROOT = Path(__file__).resolve().parents[1]
OUTPUT_DIR = ROOT / "artifacts" / "sprints" / "sprint-1" / "story-1.1"
INDEX_PATH = OUTPUT_DIR / "artifact-index.json"
EXPECTED_ARTIFACTS = {
    "1.1.2.1": (
        "repository-skeleton",
        (
            "Cargo.toml",
            "architecture/module-inventory.json",
            "capabilities/read-only/Cargo.toml",
            "kernel/contracts/Cargo.toml",
            "kernel/engine/Cargo.toml",
            "platforms/linux/Cargo.toml",
            "platforms/macos/Package.swift",
            "release/xtask/Cargo.toml",
            "shells/host/Cargo.toml",
            "shells/vscode/package.json",
        ),
    ),
    "1.1.2.2": (
        "architecture-decision-and-diagrams",
        (
            "architecture/dependency-rules.json",
            "architecture/language-build-matrix.json",
            "docs/architecture/dependency-direction.md",
            "docs/decisions/0004-language-and-build-system-architecture.md",
        ),
    ),
    "1.1.2.3": (
        "locked-dependency-graph-and-sbom",
        (
            "Cargo.lock",
            "package-lock.json",
            "platforms/macos/Package.resolved",
            "supply-chain/dependency-hashes.sha256",
            "supply-chain/dependency-provenance.json",
            "supply-chain/sbom.cdx.json",
        ),
    ),
    "1.1.2.4": (
        "clean-development-commands",
        (
            "architecture/build-contract.json",
            "package.json",
            "rust-toolchain.toml",
            "rustfmt.toml",
            "shells/vscode/eslint.config.mjs",
            "shells/vscode/tsconfig.json",
        ),
    ),
}
EXPECTED_COMMANDS = (
    "npm run product:build",
    "npm run product:format-check",
    "npm run product:lint",
    "npm run product:test",
)


def sha256_file(path: Path) -> str:
    return hashlib.sha256(path.read_bytes()).hexdigest()


def safe_path(raw_path: Any) -> bool:
    if not isinstance(raw_path, str) or not raw_path:
        return False
    path = PurePosixPath(raw_path)
    return not path.is_absolute() and ".." not in path.parts and str(path) == raw_path


def build_index(root: Path = ROOT) -> dict[str, Any]:
    artifacts = []
    for task_id, (artifact_id, paths) in EXPECTED_ARTIFACTS.items():
        artifacts.append(
            {
                "task_id": task_id,
                "artifact_id": artifact_id,
                "files": [
                    {"path": path, "sha256": sha256_file(root / path)} for path in paths
                ],
            }
        )
    return {
        "schema_version": 1,
        "story_id": "1.1",
        "status": "reviewable-blocked-macos",
        "platform_status": {
            "shared_linux": "verified-local",
            "macos_structure": "interface-only",
            "macos_build": "blocked-macos",
            "macos_verification": "blocked-macos",
        },
        "commands": list(EXPECTED_COMMANDS),
        "artifacts": artifacts,
    }


def validate_index(index: Any, root: Path = ROOT) -> list[str]:
    failures: list[str] = []
    if not isinstance(index, dict):
        return ["artifact index must be an object"]
    if index.get("schema_version") != 1 or index.get("story_id") != "1.1":
        failures.append("artifact index identity is invalid")
    if index.get("status") != "reviewable-blocked-macos":
        failures.append("artifact index must retain blocked macOS status")
    if index.get("platform_status") != {
        "shared_linux": "verified-local",
        "macos_structure": "interface-only",
        "macos_build": "blocked-macos",
        "macos_verification": "blocked-macos",
    }:
        failures.append("artifact platform status is invalid")
    if tuple(index.get("commands", [])) != EXPECTED_COMMANDS:
        failures.append("artifact command surface is incomplete or reordered")

    raw_artifacts = index.get("artifacts")
    if not isinstance(raw_artifacts, list):
        return [*failures, "artifacts must be an array"]
    artifacts = {
        item.get("task_id"): item for item in raw_artifacts if isinstance(item, dict)
    }
    if len(artifacts) != len(raw_artifacts) or set(artifacts) != set(EXPECTED_ARTIFACTS):
        failures.append("artifact task closure is incomplete or duplicated")
    for task_id, (expected_id, expected_paths) in EXPECTED_ARTIFACTS.items():
        artifact = artifacts.get(task_id, {})
        if artifact.get("artifact_id") != expected_id:
            failures.append(f"artifact id does not match task {task_id}")
        raw_files = artifact.get("files")
        if not isinstance(raw_files, list):
            failures.append(f"artifact files must be an array: {task_id}")
            continue
        files = {item.get("path"): item for item in raw_files if isinstance(item, dict)}
        if len(files) != len(raw_files) or set(files) != set(expected_paths):
            failures.append(f"artifact file closure is invalid: {task_id}")
        for path, record in files.items():
            if not safe_path(path):
                failures.append(f"unsafe artifact path: {path}")
                continue
            destination = root / path
            if not destination.is_file():
                failures.append(f"missing artifact file: {path}")
                continue
            if record.get("sha256") != sha256_file(destination):
                failures.append(f"artifact checksum mismatch: {path}")
    if index != build_index(root):
        failures.append("artifact index is stale or non-deterministic")
    return failures


def write_index(root: Path = ROOT) -> None:
    output = root / "artifacts" / "sprints" / "sprint-1" / "story-1.1"
    output.mkdir(parents=True, exist_ok=True)
    (output / "artifact-index.json").write_text(
        json.dumps(build_index(root), indent=2, sort_keys=True) + "\n",
        encoding="utf-8",
    )


def check_index(root: Path = ROOT) -> list[str]:
    try:
        index = json.loads(
            (root / "artifacts/sprints/sprint-1/story-1.1/artifact-index.json").read_text(
                encoding="utf-8"
            )
        )
    except (OSError, json.JSONDecodeError) as error:
        return [f"cannot read Story 1.1 artifact index: {error}"]
    return validate_index(index, root)


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--write", action="store_true")
    args = parser.parse_args()
    try:
        if args.write:
            write_index()
        failures = check_index()
    except OSError as error:
        print(f"Story 1.1 artifact validation failed: {error}", file=sys.stderr)
        return 1
    if failures:
        for failure in failures:
            print(f"Story 1.1 artifact validation failed: {failure}", file=sys.stderr)
        return 1
    print("Story 1.1 review artifacts validated")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
