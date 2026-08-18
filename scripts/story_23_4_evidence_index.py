#!/usr/bin/env python3
"""Build and verify the hashed Story 23.4 requirement-to-evidence index."""

from __future__ import annotations

import argparse
import copy
import hashlib
import json
import os
import re
import subprocess
import sys
import tempfile
from pathlib import Path, PurePosixPath
from typing import Any, Final


ROOT: Final = Path(__file__).resolve().parents[1]
REPORT_PATH: Final = (
    ROOT / "artifacts/sprints/sprint-23/story-23.4/evidence-index.json"
)
ZERO_SHA256: Final = "0" * 64
TASK_PATTERN: Final = re.compile(
    r"^\s*- \[(?P<checked>[ x])\] \*\*Sub-task "
    r"(?P<task_id>23\.4\.[123]\.\d+)(?:[^*]*)?:\*\* (?P<statement>.+)$"
)

EVIDENCE_PATHS: Final = (
    "artifacts/sprints/sprint-21/story-21.2/pressure-report.json",
    "artifacts/sprints/sprint-23/story-23.4/coordinator-boundary-review.json",
    "artifacts/sprints/sprint-23/story-23.4/runtime-evidence.json",
    "artifacts/sprints/sprint-23/story-23.4/runtime-evidence.log",
    "artifacts/sprints/sprint-23/story-23.4/security-evidence-map.json",
    "artifacts/sprints/sprint-23/story-23.4/security-evidence.log",
    "artifacts/sprints/sprint-50/story-50.2-runtime-load-worker/report.json",
    "docs/architecture/reusable-runtime-coordinator.md",
    "docs/verification/story-23-4-local-results.md",
    "kernel/contracts/src/runtime_event.rs",
    "kernel/contracts/src/runtime_run.rs",
    "kernel/engine/src/runtime_coordinator.rs",
    "kernel/engine/src/runtime_loop.rs",
    "kernel/engine/src/runtime_loop_tests.rs",
    "scripts/dependency_rules.py",
    "scripts/effect_boundary.py",
    "scripts/runtime_coordinator_boundary_review.py",
    "scripts/story_23_4_evidence_index.py",
    "scripts/story_23_4_runtime_evidence.py",
    "scripts/story_23_4_security_evidence.py",
    "shells/host/src/coding_client.rs",
    "shells/host/src/cli_runtime.rs",
    "shells/host/src/native_chat_runtime.rs",
    "shells/host/src/runtime_parity_tests.rs",
    "shells/host/src/runtime_read_tests.rs",
    "shells/host/src/runtime_tools.rs",
    "shells/vscode/src/provider.ts",
    "shells/vscode/src/runtime_transport.ts",
    "shells/vscode/test/host_bridge.test.ts",
    "shells/vscode/test/provider.test.ts",
    "shells/vscode/test/runtime_transport.test.ts",
    "tests/test_runtime_coordinator_boundary_review.py",
    "tests/test_story_23_4_evidence_index.py",
    "tests/test_story_23_4_runtime_evidence.py",
    "tests/test_story_23_4_security_evidence.py",
)


def entry(
    task_id: str,
    status: str,
    code: list[str],
    tests: list[str],
    evidence: list[str],
) -> dict[str, Any]:
    return {
        "task_id": task_id,
        "status": status,
        "code": code,
        "tests": tests,
        "evidence": evidence,
    }


MAPPINGS: Final = (
    entry(
        "23.4.1.1",
        "complete",
        ["kernel/contracts/src/runtime_run.rs", "kernel/engine/src/runtime_coordinator.rs"],
        [
            "story_23_4_request_and_outcome_are_closed_hash_bound_contracts",
            "story_23_4_request_rejects_binding_catalog_context_cursor_and_limit_drift",
        ],
        ["docs/architecture/reusable-runtime-coordinator.md"],
    ),
    entry(
        "23.4.1.2",
        "complete",
        ["kernel/engine/src/runtime_loop.rs"],
        ["story_23_4_direct_answer_is_verifier_backed_and_streamed_in_exact_order"],
        ["docs/architecture/reusable-runtime-coordinator.md"],
    ),
    entry(
        "23.4.1.3",
        "complete",
        ["kernel/engine/src/runtime_loop.rs"],
        [
            "story_23_4_ask_pauses_before_effect_and_exact_allow_resumes_once",
            "story_23_4_deny_closes_without_launching_the_tool",
        ],
        ["artifacts/sprints/sprint-23/story-23.4/runtime-evidence.json"],
    ),
    entry(
        "23.4.1.4",
        "complete",
        ["kernel/engine/src/runtime_loop.rs", "kernel/engine/src/runtime_loop_tests.rs"],
        ["story_23_4_repeat_no_progress_and_turn_budgets_stop_truthfully"],
        ["artifacts/sprints/sprint-23/story-23.4/runtime-evidence.json"],
    ),
    entry(
        "23.4.1.5",
        "complete",
        ["kernel/engine/src/runtime_loop.rs"],
        ["test_representative_shortcuts_fail_their_named_boundary"],
        [
            "artifacts/sprints/sprint-23/story-23.4/coordinator-boundary-review.json",
            "docs/architecture/reusable-runtime-coordinator.md",
        ],
    ),
    entry(
        "23.4.1.6",
        "complete",
        [
            "shells/host/src/runtime_read_tests.rs",
            "shells/host/src/runtime_tools.rs",
        ],
        [
            "story_23_4_fake_model_uses_existing_native_read_tool_then_verifies_completion",
            "story_23_4_fake_model_composes_multiple_reads_search_and_read_only_git",
        ],
        ["artifacts/sprints/sprint-23/story-23.4/runtime-evidence.json"],
    ),
    entry(
        "23.4.1.7",
        "partial",
        [
            "shells/host/src/native_chat_runtime.rs",
            "shells/vscode/src/provider.ts",
            "shells/vscode/src/runtime_transport.ts",
        ],
        [
            "completed_shared_runtime_is_prepared_started_and_replayed_by_exact_cursor",
            "native Chat renders one complete shared-runtime stream and outcome",
            "stream substitution reordering and payload mutation fail closed",
        ],
        ["docs/verification/story-23-4-local-results.md"],
    ),
    entry(
        "23.4.2.1",
        "complete",
        ["kernel/engine/src/runtime_loop.rs"],
        ["test_current_committed_boundary_passes_every_automated_check"],
        ["docs/architecture/reusable-runtime-coordinator.md"],
    ),
    entry(
        "23.4.2.2",
        "complete",
        ["shells/host/src/coding_client.rs", "shells/host/src/runtime_read_tests.rs"],
        ["story_50_2_read_only_and_coding_packets_are_equal_across_all_three_callers"],
        ["docs/architecture/reusable-runtime-coordinator.md"],
    ),
    entry(
        "23.4.2.3",
        "complete",
        ["scripts/runtime_coordinator_boundary_review.py"],
        ["test_current_committed_boundary_passes_every_automated_check"],
        [
            "artifacts/sprints/sprint-23/story-23.4/coordinator-boundary-review.json",
            "docs/architecture/reusable-runtime-coordinator.md",
        ],
    ),
    entry(
        "23.4.2.4",
        "complete",
        ["scripts/story_23_4_evidence_index.py"],
        ["test_current_index_is_exact_and_hash_bound"],
        ["docs/verification/story-23-4-local-results.md"],
    ),
    entry(
        "23.4.3.1",
        "complete",
        [
            "kernel/engine/src/runtime_loop_tests.rs",
            "shells/host/src/runtime_read_tests.rs",
        ],
        [
            "story_23_4_direct_answer_is_verifier_backed_and_streamed_in_exact_order",
            "story_23_4_fake_model_composes_multiple_reads_search_and_read_only_git",
            "story_23_4_model_dependency_failures_close_without_effect_or_retry",
        ],
        [
            "artifacts/sprints/sprint-23/story-23.4/runtime-evidence.json",
            "artifacts/sprints/sprint-23/story-23.4/runtime-evidence.log",
        ],
    ),
    entry(
        "23.4.3.2",
        "complete",
        [
            "scripts/dependency_rules.py",
            "scripts/effect_boundary.py",
            "scripts/runtime_coordinator_boundary_review.py",
        ],
        [
            "test_representative_shortcuts_fail_their_named_boundary",
            "story_50_2_confusion_and_interface_bypasses_never_present_or_complete",
            "story_50_2_narrow_workflow_authority_rejects_every_broadening_without_execution",
        ],
        ["artifacts/sprints/sprint-23/story-23.4/coordinator-boundary-review.json"],
    ),
    entry(
        "23.4.3.3",
        "partial",
        [
            "kernel/engine/src/runtime_loop.rs",
            "kernel/engine/src/runtime_loop_tests.rs",
            "shells/host/src/native_chat_runtime.rs",
        ],
        [
            "story_23_4_model_dependency_failures_close_without_effect_or_retry",
            "story_23_4_terminal_tool_failures_have_one_receipt_and_no_hidden_retry",
            "protected_approval_and_cancellation_preserve_exact_runtime_identity",
        ],
        [
            "artifacts/sprints/sprint-23/story-23.4/runtime-evidence.json",
            "docs/verification/story-23-4-local-results.md",
        ],
    ),
    entry(
        "23.4.3.4",
        "complete",
        [
            "shells/host/src/coding_client.rs",
            "shells/host/src/cli_runtime.rs",
            "shells/host/src/runtime_parity_tests.rs",
        ],
        ["story_50_2_read_only_and_coding_packets_are_equal_across_all_three_callers"],
        ["artifacts/sprints/sprint-23/story-23.4/runtime-evidence.json"],
    ),
    entry(
        "23.4.3.5",
        "complete",
        [
            "kernel/engine/src/runtime_loop_tests.rs",
            "scripts/story_23_4_runtime_evidence.py",
        ],
        [
            "story_23_4_ephemeral_runtime_profile_is_bounded",
            "test_current_report_and_raw_trace_are_hash_bound",
        ],
        [
            "artifacts/sprints/sprint-21/story-21.2/pressure-report.json",
            "artifacts/sprints/sprint-23/story-23.4/runtime-evidence.json",
            "artifacts/sprints/sprint-50/story-50.2-runtime-load-worker/report.json",
        ],
    ),
    entry(
        "23.4.3.6",
        "partial",
        ["scripts/story_23_4_security_evidence.py"],
        [
            "test_mapping_is_closed_complete_for_declared_scope_and_path_bound",
            "story_23_4_runtime_events_exclude_raw_model_and_tool_canaries",
        ],
        [
            "artifacts/sprints/sprint-23/story-23.4/security-evidence-map.json",
            "artifacts/sprints/sprint-23/story-23.4/security-evidence.log",
            "docs/verification/story-23-4-local-results.md",
        ],
    ),
)

LIMITATIONS: Final = (
    "The installed host still lacks a production runtime factory and admitted local model.",
    "The complete boundary-by-boundary ephemeral failure injection matrix remains open.",
    "Independent coordinator and installed accessibility review evidence remains open.",
    "Persistent crash/restart and physical dependency faults remain owned by later durable campaigns.",
    "Manual fuzzing remains deliberately deferred and open.",
    "No supported-platform, release, or deployment approval follows from this index.",
)


def canonical_json(value: Any) -> bytes:
    return (json.dumps(value, indent=2, sort_keys=True) + "\n").encode("utf-8")


def sha256_bytes(value: bytes) -> str:
    return hashlib.sha256(value).hexdigest()


def git_revision(revision: str = "HEAD", root: Path = ROOT) -> str:
    result = subprocess.run(
        ["git", "rev-parse", f"{revision}^{{commit}}"],
        cwd=root,
        check=False,
        capture_output=True,
        text=True,
        timeout=30,
    )
    resolved = result.stdout.strip()
    if result.returncode or re.fullmatch(r"[0-9a-f]{40}", resolved) is None:
        raise ValueError("Story 23.4 source revision is invalid")
    return resolved


def safe_relative_path(value: str) -> bool:
    path = PurePosixPath(value)
    return bool(value) and not path.is_absolute() and ".." not in path.parts and str(path) == value


def task_anchors(root: Path = ROOT) -> dict[str, dict[str, Any]]:
    anchors: dict[str, dict[str, Any]] = {}
    for line in (root / "TASKS.md").read_text(encoding="utf-8").splitlines():
        match = TASK_PATTERN.match(line)
        if match is None:
            continue
        task_id = match.group("task_id")
        if task_id in anchors:
            raise ValueError(f"duplicate Story 23.4 sub-task: {task_id}")
        anchors[task_id] = {
            "checked": match.group("checked") == "x",
            "statement_sha256": sha256_bytes(match.group("statement").encode("utf-8")),
        }
    expected = [mapping["task_id"] for mapping in MAPPINGS]
    if list(anchors) != expected:
        raise ValueError("Story 23.4 task anchors are missing or reordered")
    for mapping in MAPPINGS:
        checked = anchors[mapping["task_id"]]["checked"]
        if checked != (mapping["status"] == "complete"):
            raise ValueError(f"Story 23.4 task status drifted: {mapping['task_id']}")
    return anchors


def validate_mapping_sources(root: Path = ROOT) -> None:
    evidence_paths = set(EVIDENCE_PATHS)
    for relative in EVIDENCE_PATHS:
        if not safe_relative_path(relative) or not (root / relative).is_file():
            raise ValueError(f"Story 23.4 evidence path is unavailable: {relative}")
    searchable = "\n".join(
        (root / relative).read_text(encoding="utf-8")
        for relative in EVIDENCE_PATHS
        if Path(relative).suffix in {".py", ".rs", ".ts"}
    )
    for mapping in MAPPINGS:
        for relative in (*mapping["code"], *mapping["evidence"]):
            if relative not in evidence_paths:
                raise ValueError(f"unindexed Story 23.4 evidence path: {relative}")
        for test_id in mapping["tests"]:
            if test_id not in searchable:
                raise ValueError(f"Story 23.4 test identity is unresolved: {test_id}")


def evidence_records(root: Path = ROOT) -> list[dict[str, Any]]:
    return [
        {
            "path": relative,
            "bytes": (root / relative).stat().st_size,
            "sha256": sha256_bytes((root / relative).read_bytes()),
        }
        for relative in EVIDENCE_PATHS
    ]


def build_index(source_revision: str, root: Path = ROOT) -> dict[str, Any]:
    if re.fullmatch(r"[0-9a-f]{40}", source_revision) is None:
        raise ValueError("Story 23.4 source revision is invalid")
    anchors = task_anchors(root)
    validate_mapping_sources(root)
    mappings = [
        {**copy.deepcopy(mapping), **anchors[mapping["task_id"]]}
        for mapping in MAPPINGS
    ]
    counts = {
        status: sum(mapping["status"] == status for mapping in MAPPINGS)
        for status in ("complete", "partial", "open")
    }
    index: dict[str, Any] = {
        "schema_version": 1,
        "record_type": "story_23_4_requirement_code_test_evidence_index",
        "story_id": "23.4",
        "source_revision": source_revision,
        "disposition": "partial-local-evidence",
        "mappings": mappings,
        "artifacts": evidence_records(root),
        "summary": {
            "mapping_count": len(MAPPINGS),
            "complete_count": counts["complete"],
            "partial_count": counts["partial"],
            "open_count": counts["open"],
            "story_complete": False,
            "sprint_complete": False,
            "release_approved": False,
        },
        "limitations": list(LIMITATIONS),
        "index_sha256": ZERO_SHA256,
    }
    index["index_sha256"] = sha256_bytes(canonical_json(index))
    return index


def validate_index(index: Any, root: Path = ROOT) -> list[str]:
    if not isinstance(index, dict):
        return ["Story 23.4 evidence index is not an object"]
    revision = index.get("source_revision")
    if not isinstance(revision, str):
        return ["Story 23.4 evidence index source revision is absent"]
    try:
        expected = build_index(revision, root)
    except (OSError, UnicodeError, ValueError) as error:
        return [str(error)]
    return [] if index == expected else ["Story 23.4 evidence index differs from exact inputs"]


def check_index(root: Path = ROOT) -> list[str]:
    path = root / REPORT_PATH.relative_to(ROOT)
    try:
        index = json.loads(path.read_text(encoding="utf-8"))
    except (OSError, UnicodeError, json.JSONDecodeError):
        return ["Story 23.4 evidence index is unreadable"]
    return validate_index(index, root)


def write_atomic(path: Path, content: bytes) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    descriptor, temporary_name = tempfile.mkstemp(prefix=".story-23-4-", dir=path.parent)
    temporary = Path(temporary_name)
    try:
        with os.fdopen(descriptor, "wb") as handle:
            handle.write(content)
            handle.flush()
            os.fsync(handle.fileno())
        temporary.chmod(0o644)
        os.replace(temporary, path)
    except BaseException:
        temporary.unlink(missing_ok=True)
        raise


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--check", action="store_true")
    parser.add_argument("--source-revision", default="HEAD")
    arguments = parser.parse_args()
    if arguments.check:
        failures = check_index()
        for failure in failures:
            print(f"Story 23.4 evidence index failed: {failure}", file=sys.stderr)
        return 1 if failures else 0
    revision = git_revision(arguments.source_revision)
    write_atomic(REPORT_PATH, canonical_json(build_index(revision)))
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
