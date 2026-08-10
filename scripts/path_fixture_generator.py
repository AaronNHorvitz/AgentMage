#!/usr/bin/env python3
"""Generate portable hostile-path fixtures without privileged side effects."""

from __future__ import annotations

import argparse
import hashlib
import json
import os
import shutil
import sys
import tempfile
from dataclasses import dataclass
from pathlib import Path, PurePosixPath
from typing import Any


ROOT = Path(__file__).resolve().parents[1]
PROFILE_PATH = ROOT / "fixtures" / "path-fixture-profile.json"
REPORT_PATH = (
    ROOT
    / "artifacts"
    / "sprints"
    / "sprint-2"
    / "story-2.1"
    / "path-fixture-report.json"
)
EXPECTED_SCENARIOS = (
    "path-traversal",
    "symlink-escape",
    "case-collision",
    "unicode-normalization",
    "mount-change",
    "file-replacement",
    "stale-bookmark",
)
EXPECTED_TRAVERSAL = (
    "../outside/forbidden.txt",
    "nested/../../outside/forbidden.txt",
    "..\\outside\\forbidden.txt",
    "%2e%2e/outside/forbidden.txt",
    "/synthetic-absolute/forbidden.txt",
)
EXPECTED_SYMLINKS = (
    ("sandbox/allowed/workspace/link-out", "../../outside/forbidden.txt"),
    ("sandbox/allowed/workspace/linked-outside", "../../outside"),
)
EXPECTED_SIDE_EFFECTS = {
    "executes_external_commands": False,
    "uses_network": False,
    "performs_mount_operation": False,
    "creates_platform_bookmark": False,
    "writes_outside_requested_destination": False,
    "overwrites_existing_destination": False,
}


@dataclass(frozen=True)
class PathFixtureEntry:
    kind: str
    content: bytes = b""
    target: str = ""


def read_json(path: Path) -> Any:
    return json.loads(path.read_text(encoding="utf-8"))


def canonical_json(value: Any) -> bytes:
    return (json.dumps(value, indent=2, sort_keys=True) + "\n").encode("utf-8")


def sha256_bytes(value: bytes) -> str:
    return hashlib.sha256(value).hexdigest()


def safe_relative(value: str) -> bool:
    path = PurePosixPath(value)
    return bool(value) and not path.is_absolute() and ".." not in path.parts


def _symlink_pairs(records: Any) -> tuple[tuple[str, str], ...]:
    if not isinstance(records, list):
        return ()
    return tuple(
        (item.get("path"), item.get("target"))
        for item in records
        if isinstance(item, dict)
    )


def validate_profile(profile: Any) -> list[str]:
    failures: list[str] = []
    if not isinstance(profile, dict):
        return ["path fixture profile must be an object"]
    if (
        profile.get("schema_version") != 1
        or profile.get("profile_id") != "agentmage-path-fixtures-v1"
        or profile.get("status") != "synthetic-portable-fixture-contract"
    ):
        failures.append("path fixture profile identity is invalid")
    if profile.get("seed") != "agentmage-path-fixtures-synthetic-v1":
        failures.append("path fixture seed is not pinned")
    if profile.get("fixed_timestamp_epoch") != 1704067200:
        failures.append("path fixture timestamp is not pinned")
    if tuple(profile.get("scenario_types", [])) != EXPECTED_SCENARIOS:
        failures.append("path fixture scenario closure drifted")
    if tuple(profile.get("traversal_inputs", [])) != EXPECTED_TRAVERSAL:
        failures.append("path traversal fixture closure drifted")
    if _symlink_pairs(profile.get("symlink_fixtures")) != EXPECTED_SYMLINKS:
        failures.append("symlink fixture closure drifted")
    if profile.get("case_collision_logical_paths") != [
        "Notes/Case.md",
        "Notes/case.md",
    ]:
        failures.append("case-collision fixture closure drifted")
    if profile.get("unicode_collision_logical_paths") != [
        "Notes/caf\u00e9.md",
        "Notes/cafe\u0301.md",
    ]:
        failures.append("Unicode-normalization fixture closure drifted")
    if profile.get("side_effect_contract") != EXPECTED_SIDE_EFFECTS:
        failures.append("path fixture side-effect contract was weakened")
    if profile.get("macos_bookmark_claim") != "synthetic-record-only":
        failures.append("path fixture cannot claim a real macOS bookmark")
    if profile.get("product_path_safety_claim") != "none":
        failures.append("path fixture cannot claim product path safety")
    return failures


def add_file(entries: dict[str, PathFixtureEntry], path: str, content: bytes | str) -> None:
    if not safe_relative(path) or path in entries:
        raise ValueError(f"invalid or duplicate path fixture file: {path}")
    encoded = content.encode("utf-8") if isinstance(content, str) else content
    entries[path] = PathFixtureEntry(kind="file", content=encoded)


def add_symlink(entries: dict[str, PathFixtureEntry], path: str, target: str) -> None:
    if not safe_relative(path) or path in entries:
        raise ValueError(f"invalid or duplicate path fixture link: {path}")
    if PurePosixPath(target).is_absolute() or not target:
        raise ValueError(f"unsafe path fixture link target: {target}")
    entries[path] = PathFixtureEntry(kind="symlink", target=target)


def scenario_manifest(profile: dict[str, Any]) -> dict[str, Any]:
    seed = profile["seed"]
    identity = lambda name: sha256_bytes(f"{seed}:{name}".encode("utf-8"))
    original = b"synthetic original content\n"
    replacement = b"synthetic replacement content\n"
    return {
        "schema_version": 1,
        "profile_id": profile["profile_id"],
        "scenarios": [
            {
                "id": "path-traversal",
                "inputs": profile["traversal_inputs"],
                "expected": "reject-before-filesystem-access",
            },
            {
                "id": "symlink-escape",
                "links": profile["symlink_fixtures"],
                "approved_root": "sandbox/allowed/workspace",
                "expected": "reject-target-outside-approved-root",
            },
            {
                "id": "case-collision",
                "logical_paths": profile["case_collision_logical_paths"],
                "payloads": [
                    "portable/case-collision/upper.payload",
                    "portable/case-collision/lower.payload",
                ],
                "expected": "normalization-ambiguity",
            },
            {
                "id": "unicode-normalization",
                "logical_paths": profile["unicode_collision_logical_paths"],
                "payloads": [
                    "portable/unicode-collision/nfc.payload",
                    "portable/unicode-collision/nfd.payload",
                ],
                "expected": "normalization-ambiguity",
            },
            {
                "id": "mount-change",
                "path": "sandbox/allowed/workspace/mount-target/data.txt",
                "identity_before": identity("mount-before"),
                "identity_after": identity("mount-after"),
                "operation_performed": False,
                "expected": "stale-filesystem-identity",
            },
            {
                "id": "file-replacement",
                "path": "sandbox/allowed/workspace/replacement/current.txt",
                "original_sha256": sha256_bytes(original),
                "replacement_sha256": sha256_bytes(replacement),
                "expected": "stale-file-identity",
            },
            {
                "id": "stale-bookmark",
                "bookmark_kind": "synthetic-record",
                "bookmark_id": identity("bookmark-id"),
                "observed_resource_id": identity("bookmark-before"),
                "current_resource_id": identity("bookmark-after"),
                "platform_bookmark_created": False,
                "expected": "stale-bookmark-refusal",
            },
        ],
        "product_path_safety_claim": "none",
        "macos_bookmark_claim": "synthetic-record-only",
    }


def materialize_specs(profile: dict[str, Any]) -> dict[str, PathFixtureEntry]:
    entries: dict[str, PathFixtureEntry] = {}
    add_file(entries, "sandbox/allowed/workspace/docs/inside.txt", "authorized synthetic content\n")
    add_file(entries, "sandbox/outside/forbidden.txt", "prohibited synthetic content\n")
    add_file(
        entries,
        "sandbox/allowed/workspace/mount-target/data.txt",
        "synthetic mount identity payload\n",
    )
    add_file(
        entries,
        "sandbox/allowed/workspace/replacement/current.txt",
        "synthetic original content\n",
    )
    add_file(
        entries,
        "sandbox/allowed/workspace/replacement/replacement.txt",
        "synthetic replacement content\n",
    )
    add_file(entries, "portable/case-collision/upper.payload", "upper-case logical path\n")
    add_file(entries, "portable/case-collision/lower.payload", "lower-case logical path\n")
    add_file(entries, "portable/unicode-collision/nfc.payload", "NFC logical path\n")
    add_file(entries, "portable/unicode-collision/nfd.payload", "NFD logical path\n")
    for record in profile["symlink_fixtures"]:
        add_symlink(entries, record["path"], record["target"])
    add_file(entries, "path-scenarios.json", canonical_json(scenario_manifest(profile)))
    return dict(sorted(entries.items()))


def corpus_identity(entries: dict[str, PathFixtureEntry]) -> str:
    digest = hashlib.sha256()
    for path, entry in sorted(entries.items()):
        encoded_path = path.encode("utf-8")
        encoded_kind = entry.kind.encode("ascii")
        payload = entry.content if entry.kind == "file" else entry.target.encode("utf-8")
        for value in (encoded_path, encoded_kind, payload):
            digest.update(len(value).to_bytes(8, "big"))
            digest.update(value)
    return digest.hexdigest()


def generate(profile: dict[str, Any], destination: Path) -> dict[str, Any]:
    failures = validate_profile(profile)
    if failures:
        raise ValueError("; ".join(failures))
    if destination.exists():
        raise FileExistsError("path fixture destination already exists")
    destination.parent.mkdir(parents=True, exist_ok=True)
    staging = Path(tempfile.mkdtemp(prefix=".agentmage-paths-", dir=destination.parent))
    entries = materialize_specs(profile)
    try:
        for relative, entry in entries.items():
            target = staging / relative
            target.parent.mkdir(parents=True, exist_ok=True)
            if entry.kind == "file":
                target.write_bytes(entry.content)
                target.chmod(0o644)
                os.utime(target, (profile["fixed_timestamp_epoch"],) * 2)
            else:
                target.symlink_to(entry.target)
                os.utime(
                    target,
                    (profile["fixed_timestamp_epoch"],) * 2,
                    follow_symlinks=False,
                )
        os.replace(staging, destination)
    except BaseException:
        shutil.rmtree(staging, ignore_errors=True)
        raise
    return {
        "corpus_sha256": corpus_identity(entries),
        "entry_count": len(entries),
        "file_count": sum(item.kind == "file" for item in entries.values()),
        "symlink_count": sum(item.kind == "symlink" for item in entries.values()),
    }


def build_report(root: Path = ROOT) -> dict[str, Any]:
    profile_path = root / "fixtures/path-fixture-profile.json"
    profile = read_json(profile_path)
    failures = validate_profile(profile)
    if failures:
        raise ValueError("; ".join(failures))
    entries = materialize_specs(profile)
    return {
        "schema_version": 1,
        "task_id": "2.1.1.2",
        "status": "pass",
        "profile_sha256": sha256_bytes(profile_path.read_bytes()),
        "preview": {
            "persisted": False,
            "corpus_sha256": corpus_identity(entries),
            "scenario_count": len(EXPECTED_SCENARIOS),
            "entry_count": len(entries),
            "symlink_count": sum(item.kind == "symlink" for item in entries.values()),
        },
        "side_effect_contract": profile["side_effect_contract"],
        "macos_bookmark_claim": "synthetic-record-only",
        "product_path_safety_claim": "none",
        "versioned_corpus_status": "fulfilled-by-agentmage-versioned-synthetic-corpus-v1",
    }


def validate_report(report: Any, root: Path = ROOT) -> list[str]:
    if not isinstance(report, dict):
        return ["path fixture report must be an object"]
    failures = []
    if report.get("schema_version") != 1 or report.get("task_id") != "2.1.1.2":
        failures.append("path fixture report identity is invalid")
    if report.get("status") != "pass":
        failures.append("path fixture generator did not pass")
    if report.get("macos_bookmark_claim") != "synthetic-record-only":
        failures.append("path fixture report claimed a real macOS bookmark")
    if report.get("product_path_safety_claim") != "none":
        failures.append("path fixture report claimed product path safety")
    if (
        report.get("versioned_corpus_status")
        != "fulfilled-by-agentmage-versioned-synthetic-corpus-v1"
    ):
        failures.append("path fixture report lost its versioned corpus disposition")
    if report != build_report(root):
        failures.append("path fixture report is stale or non-deterministic")
    return failures


def write_report(root: Path = ROOT) -> None:
    output = root / REPORT_PATH.relative_to(ROOT)
    output.parent.mkdir(parents=True, exist_ok=True)
    output.write_bytes(canonical_json(build_report(root)))


def check_report(root: Path = ROOT) -> list[str]:
    try:
        report = read_json(root / REPORT_PATH.relative_to(ROOT))
    except (OSError, json.JSONDecodeError) as error:
        return [f"cannot read path fixture report: {error}"]
    return validate_report(report, root)


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--output", type=Path)
    parser.add_argument("--write-report", action="store_true")
    args = parser.parse_args()
    try:
        profile = read_json(PROFILE_PATH)
        if args.output is not None:
            print(json.dumps(generate(profile, args.output), indent=2, sort_keys=True))
        if args.write_report:
            write_report()
        failures = check_report()
    except (OSError, ValueError) as error:
        print(f"path fixture generator failed: {error}", file=sys.stderr)
        return 1
    if failures:
        for failure in failures:
            print(f"path fixture generator failed: {failure}", file=sys.stderr)
        return 1
    if args.output is None:
        print("portable path, collision, replacement, mount, and bookmark fixtures validated")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
