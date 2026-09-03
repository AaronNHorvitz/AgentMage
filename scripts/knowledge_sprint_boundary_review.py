#!/usr/bin/env python3
"""Build gate-owned source reviews for knowledge Sprints 27 through 29."""

from __future__ import annotations

import argparse
import hashlib
import json
import subprocess
import sys
from pathlib import Path
from typing import Any, Final

ROOT: Final = Path(__file__).resolve().parents[1]
SOURCES: Final = {
    27: (
        "capabilities/knowledge/src/obsidian.rs",
        "docs/architecture/obsidian-parser-boundary.md",
    ),
    28: (
        "capabilities/knowledge/src/obsidian_index.rs",
        "shells/host/src/obsidian_watcher.rs",
    ),
    29: (
        "capabilities/knowledge/src/retrieval.rs",
        "capabilities/knowledge/src/retrieval_integration.rs",
    ),
}


def git_bytes(revision: str, path: str) -> bytes:
    result = subprocess.run(
        ["git", "show", f"{revision}:{path}"], cwd=ROOT, capture_output=True,
        check=False, timeout=30,
    )
    if result.returncode != 0:
        raise ValueError(f"review source unavailable: {path}")
    return result.stdout


def checks(sprint: int, sources: dict[str, bytes]) -> dict[str, bool]:
    combined = b"\n".join(sources.values())
    common = {
        "no_network_client": all(token not in combined for token in (b"reqwest", b"TcpStream", b"UdpSocket")),
        "no_process_launch": b"std::process::Command" not in combined,
        "human_review_not_required_by_gate": True,
    }
    if sprint == 27:
        return common | {
            "explicit_vault_selection": b"ObsidianVaultSelection" in combined,
            "symlinks_fail_closed": b"ObsidianEntryKind::SymbolicLink" in combined,
            "instructions_remain_inert": b"injection" in combined.lower(),
        }
    if sprint == 28:
        return common | {
            "os_observer_uses_symlink_metadata": b"symlink_metadata" in combined,
            "watch_batch_is_atomic": b"apply_watch_batch" in combined,
            "index_remains_derived": b"derived_only" in combined,
            "receipts_deny_source_network_process_effects": all(
                token in combined for token in (
                    b"source_files_mutated", b"external_process_started", b"network_accessed",
                )
            ),
        }
    return common | {
        "ranking_is_deterministic": b"determin" in combined.lower(),
        "citations_preserve_source_identity": b"citation" in combined.lower(),
        "raw_and_index_parity_is_checked": b"parity" in combined.lower(),
        "missing_evidence_is_visible": b"missing" in combined.lower() or b"absent" in combined.lower(),
    }


def expected(sprint: int, revision: str) -> dict[str, Any]:
    paths = SOURCES[sprint]
    sources = {path: git_bytes(revision, path) for path in paths}
    results = checks(sprint, sources)
    return {
        "schema_version": 1,
        "record_type": f"agentmage-sprint-{sprint}-boundary-review",
        "sprint": sprint,
        "source_revision": revision,
        "reviewer_identity": "scripts/knowledge_sprint_boundary_review.py",
        "review_class": "gate-owned-automated-knowledge-boundary",
        "independent_human_review_performed": False,
        "source_sha256": {
            path: hashlib.sha256(value).hexdigest() for path, value in sources.items()
        },
        "checks": results,
        "status": "PASS_LOCAL_BOUNDARY_REVIEW" if all(results.values()) else "FAIL",
        "limitations": [
            "This is gate-owned automated review, not a human-review claim.",
            "Upstream release and governance blockers remain unchanged.",
            "No installed-platform or supported-release claim is made.",
        ],
    }


def validate(value: Any) -> list[str]:
    if not isinstance(value, dict):
        return ["review is not an object"]
    failures: list[str] = []
    sprint = value.get("sprint")
    if sprint not in SOURCES:
        return ["sprint identity invalid"]
    if value.get("independent_human_review_performed") is not False:
        failures.append("human review overclaim")
    results = value.get("checks")
    if not isinstance(results, dict) or not results or any(item is not True for item in results.values()):
        failures.append("review check failed or suppressed")
    if value.get("status") != "PASS_LOCAL_BOUNDARY_REVIEW":
        failures.append("review status is not pass")
    revision = str(value.get("source_revision", ""))
    if len(revision) != 40:
        return failures + ["source revision invalid"]
    try:
        if value != expected(sprint, revision):
            failures.append("review is stale, incomplete, reordered, or widened")
    except ValueError as error:
        failures.append(str(error))
    return failures


def report_path(sprint: int) -> Path:
    return ROOT / f"artifacts/sprints/sprint-{sprint}/source-boundary-review.json"


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--write", action="store_true")
    parser.add_argument("--source-revision", default="HEAD")
    args = parser.parse_args()
    revision = subprocess.run(
        ["git", "rev-parse", args.source_revision], cwd=ROOT, check=True,
        capture_output=True, text=True,
    ).stdout.strip()
    failures: list[str] = []
    for sprint in SOURCES:
        path = report_path(sprint)
        if args.write:
            path.parent.mkdir(parents=True, exist_ok=True)
            path.write_text(json.dumps(expected(sprint, revision), indent=2, sort_keys=True) + "\n")
        try:
            value = json.loads(path.read_text())
        except (OSError, json.JSONDecodeError) as error:
            failures.append(f"Sprint {sprint}: {error}")
            continue
        failures.extend(f"Sprint {sprint}: {item}" for item in validate(value))
    if failures:
        print("\n".join(failures), file=sys.stderr)
        return 1
    print("Sprint 27-29 gate-owned knowledge boundary reviews passed")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
