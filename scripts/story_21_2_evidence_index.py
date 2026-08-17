#!/usr/bin/env python3
"""Build and verify the hashed Story 21.2 requirement-to-evidence index."""

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
from typing import Any


ROOT = Path(__file__).resolve().parents[1]
REPORT_PATH = ROOT / "artifacts/sprints/sprint-21/story-21.2/evidence-index.json"
ZERO_SHA256 = "0" * 64
TASK_PATTERN = re.compile(
    r"^\s*- \[(?P<checked>[ x])\] \*\*Sub-task "
    r"(?P<task_id>21\.2\.[123]\.[1-6])(?:[^*]*)?:\*\* (?P<statement>.+)$"
)

EVIDENCE_PATHS = (
    "artifacts/sprints/sprint-21/story-21.2/crash-matrix.json",
    "artifacts/sprints/sprint-21/story-21.2/crash-matrix.log",
    "artifacts/sprints/sprint-21/story-21.2/pressure-report.json",
    "artifacts/sprints/sprint-21/story-21.2/pressure-report.log",
    "artifacts/sprints/sprint-50/story-50.2-runtime-load-worker/report.json",
    "docs/architecture/runtime-event-journal.md",
    "docs/verification/story-21-2-local-results.md",
    "fixtures/runtime-hardening/v1/linux-reference-load-profile.json",
    "kernel/contracts/src/runtime_event.rs",
    "kernel/engine/src/authority_transaction.rs",
    "kernel/engine/src/operational_store.rs",
    "kernel/engine/src/runtime_artifact.rs",
    "kernel/engine/src/runtime_event.rs",
    "kernel/engine/src/runtime_journal.rs",
    "kernel/engine/src/runtime_loop.rs",
    "kernel/engine/src/runtime_loop_tests.rs",
    "kernel/engine/src/runtime_projection.rs",
    "schemas/runtime/examples/runtime-event.valid.json",
    "schemas/runtime/runtime-event.schema.json",
    "scripts/runtime_hardening_load.py",
    "scripts/story_21_2_crash_evidence.py",
    "scripts/story_21_2_evidence_index.py",
    "scripts/story_21_2_pressure_evidence.py",
    "shells/host/src/cli_runtime.rs",
    "shells/host/src/linux_coding_runtime.rs",
    "tests/test_story_21_2_crash_evidence.py",
    "tests/test_story_21_2_evidence_index.py",
    "tests/test_story_21_2_pressure_evidence.py",
)

MAPPINGS: tuple[dict[str, Any], ...] = (
    {
        "task_id": "21.2.1.1",
        "status": "complete",
        "code": [
            "kernel/contracts/src/runtime_event.rs",
            "kernel/engine/src/runtime_event.rs",
            "schemas/runtime/runtime-event.schema.json",
        ],
        "tests": [
            "story_21_2_runtime_event_contract_is_closed_and_hash_bound",
            "story_21_2_json_fixture_uses_the_rust_canonical_digest",
        ],
        "evidence": ["schemas/runtime/examples/runtime-event.valid.json"],
    },
    {
        "task_id": "21.2.1.2",
        "status": "complete",
        "code": ["kernel/engine/src/runtime_event.rs"],
        "tests": [
            "story_21_2_every_event_family_has_a_legal_transition_path",
            "story_21_2_closed_event_families_have_one_canonical_persistence_class",
        ],
        "evidence": ["docs/architecture/runtime-event-journal.md"],
    },
    {
        "task_id": "21.2.1.3",
        "status": "complete",
        "code": ["kernel/engine/src/runtime_projection.rs"],
        "tests": [
            "story_50_2_verified_runtime_exchange_projects_valid_non_authoritative_turns",
            "story_50_2_metrics_and_diagnostics_are_content_free_event_bound_and_bounded",
        ],
        "evidence": ["docs/architecture/runtime-event-journal.md"],
    },
    {
        "task_id": "21.2.1.4",
        "status": "complete",
        "code": [
            "kernel/engine/src/authority_transaction.rs",
            "kernel/engine/src/operational_store.rs",
            "kernel/engine/src/runtime_journal.rs",
            "kernel/engine/src/runtime_loop.rs",
            "shells/host/src/linux_coding_runtime.rs",
        ],
        "tests": [
            "correctness_event_and_parent_grant_survive_one_atomic_reopen",
            "correctness_event_insert_failure_rolls_back_parent_grant_and_generation",
            "effect_start_and_terminal_receipt_events_reopen_with_exact_authority",
            "rejected_terminal_event_requires_recovery_without_a_false_completion_event",
            "correctness_event_checkpoint_and_binding_survive_one_atomic_reopen",
            "correctness_event_insert_failure_rolls_back_checkpoint_and_binding",
            "durable_coding_run_persists_continuation_artifact_and_checkpoint",
        ],
        "evidence": [
            "docs/architecture/runtime-event-journal.md",
            "docs/verification/story-21-2-local-results.md",
        ],
    },
    {
        "task_id": "21.2.1.5",
        "status": "complete",
        "code": ["kernel/engine/src/runtime_journal.rs"],
        "tests": [
            "correctness_flushes_every_ordered_predecessor_and_reopens_exactly",
            "story_50_2_dedicated_writer_saturation_is_bounded_and_recoverable",
        ],
        "evidence": ["docs/architecture/runtime-event-journal.md"],
    },
    {
        "task_id": "21.2.1.6",
        "status": "complete",
        "code": ["kernel/engine/src/runtime_event.rs"],
        "tests": [
            "story_21_2_publisher_is_ordered_bounded_and_nonblocking",
            "story_21_2_subscriber_bounds_and_disconnect_are_non_authoritative",
        ],
        "evidence": ["docs/architecture/runtime-event-journal.md"],
    },
    {
        "task_id": "21.2.2.1",
        "status": "complete",
        "code": [
            "kernel/contracts/src/runtime_event.rs",
            "schemas/runtime/runtime-event.schema.json",
        ],
        "tests": ["story_21_2_json_fixture_uses_the_rust_canonical_digest"],
        "evidence": [
            "docs/architecture/runtime-event-journal.md",
            "schemas/runtime/examples/runtime-event.valid.json",
        ],
    },
    {
        "task_id": "21.2.2.2",
        "status": "complete",
        "code": ["kernel/engine/src/runtime_projection.rs"],
        "tests": [
            "story_50_2_transcript_projection_obeys_opt_in_safe_mode_sensitivity_and_budget",
            "story_50_2_safe_mode_and_restricted_events_never_collect_optional_telemetry",
        ],
        "evidence": ["docs/architecture/runtime-event-journal.md"],
    },
    {
        "task_id": "21.2.2.3",
        "status": "complete",
        "code": [
            "kernel/engine/src/runtime_journal.rs",
            "kernel/engine/src/operational_store.rs",
        ],
        "tests": ["story_21_2_dedicated_writer_keeps_progress_and_clients_off_slow_store"],
        "evidence": ["docs/architecture/runtime-event-journal.md"],
    },
    {
        "task_id": "21.2.2.4",
        "status": "complete",
        "code": ["scripts/story_21_2_evidence_index.py"],
        "tests": ["test_current_index_is_exact_and_hash_bound"],
        "evidence": ["docs/verification/story-21-2-local-results.md"],
    },
    {
        "task_id": "21.2.3.1",
        "status": "complete",
        "code": ["kernel/engine/src/runtime_event.rs"],
        "tests": [
            "story_21_2_every_event_family_has_a_legal_transition_path",
            "story_21_2_sequence_rejects_reorder_replay_binding_and_post_terminal_events",
            "story_21_2_every_stream_binding_dimension_is_immutable",
        ],
        "evidence": ["docs/verification/story-21-2-local-results.md"],
    },
    {
        "task_id": "21.2.3.2",
        "status": "complete",
        "code": [
            "kernel/engine/src/runtime_journal.rs",
            "scripts/story_21_2_crash_evidence.py",
        ],
        "tests": [
            "story_21_2_process_stop_matrix_preserves_one_truthful_replay",
            "test_current_report_and_raw_trace_are_hash_bound",
        ],
        "evidence": [
            "artifacts/sprints/sprint-21/story-21.2/crash-matrix.json",
            "artifacts/sprints/sprint-21/story-21.2/crash-matrix.log",
            "docs/verification/story-21-2-local-results.md",
        ],
    },
    {
        "task_id": "21.2.3.3",
        "status": "partial",
        "code": [
            "kernel/engine/src/runtime_journal.rs",
            "kernel/engine/src/runtime_loop.rs",
            "kernel/engine/src/runtime_loop_tests.rs",
            "scripts/story_21_2_pressure_evidence.py",
        ],
        "tests": [
            "queue_pressure_flushes_in_declared_batches_without_growth",
            "story_21_2_worker_enforces_the_exact_canonical_byte_boundary",
            "story_21_2_durable_model_progress_and_cancellation_survive_delayed_sqlcipher",
            "story_50_2_dedicated_writer_saturation_is_bounded_and_recoverable",
            "test_current_report_and_raw_trace_are_hash_bound",
        ],
        "evidence": [
            "artifacts/sprints/sprint-21/story-21.2/pressure-report.json",
            "artifacts/sprints/sprint-21/story-21.2/pressure-report.log",
            "artifacts/sprints/sprint-50/story-50.2-runtime-load-worker/report.json",
            "docs/verification/story-21-2-local-results.md",
            "fixtures/runtime-hardening/v1/linux-reference-load-profile.json",
        ],
    },
    {
        "task_id": "21.2.3.4",
        "status": "complete",
        "code": [
            "kernel/engine/src/runtime_artifact.rs",
            "kernel/engine/src/runtime_projection.rs",
        ],
        "tests": [
            "story_21_2_artifact_canaries_require_an_exact_owner_bound_payload_read",
            "story_21_2_projection_runtime_has_no_external_telemetry_dependency",
            "story_21_2_sensitive_canaries_are_excluded_or_restricted_in_every_projection",
        ],
        "evidence": ["docs/verification/story-21-2-local-results.md"],
    },
    {
        "task_id": "21.2.3.5",
        "status": "partial",
        "code": [
            "kernel/engine/src/runtime_journal.rs",
            "scripts/runtime_hardening_load.py",
        ],
        "tests": ["story_50_2_reference_hardware_load_is_bounded_recoverable_and_nonblocking"],
        "evidence": [
            "artifacts/sprints/sprint-50/story-50.2-runtime-load-worker/report.json",
            "fixtures/runtime-hardening/v1/linux-reference-load-profile.json",
        ],
    },
    {
        "task_id": "21.2.3.6",
        "status": "open",
        "code": [],
        "tests": [],
        "evidence": ["docs/verification/story-21-2-local-results.md"],
    },
)

LIMITATIONS = (
    "Integrated physical-effect recovery-to-terminal-event reconciliation remains open.",
    "Physical filesystem/device latency, power-loss, and storage-failure campaigns remain open.",
    "Only one retained Fedora source-host benchmark profile exists.",
    "Installed-client, supported-platform, and independent-review evidence remains open.",
    "Product security mapping and deferred manual fuzzing remain open.",
)


def canonical_json(value: Any) -> bytes:
    return (json.dumps(value, indent=2, sort_keys=True) + "\n").encode("utf-8")


def sha256_bytes(value: bytes) -> str:
    return hashlib.sha256(value).hexdigest()


def sha256_file(path: Path) -> str:
    return sha256_bytes(path.read_bytes())


def git_revision(revision: str = "HEAD", root: Path = ROOT) -> str:
    return subprocess.run(
        ["git", "rev-parse", f"{revision}^{{commit}}"],
        cwd=root,
        check=True,
        capture_output=True,
        text=True,
    ).stdout.strip()


def task_anchors(root: Path = ROOT) -> dict[str, dict[str, Any]]:
    anchors: dict[str, dict[str, Any]] = {}
    for line in (root / "TASKS.md").read_text(encoding="utf-8").splitlines():
        match = TASK_PATTERN.match(line)
        if match is None:
            continue
        task_id = match.group("task_id")
        if task_id in anchors:
            raise ValueError(f"duplicate Story 21.2 sub-task: {task_id}")
        statement = match.group("statement")
        anchors[task_id] = {
            "checked": match.group("checked") == "x",
            "statement_sha256": sha256_bytes(statement.encode("utf-8")),
        }
    expected = [mapping["task_id"] for mapping in MAPPINGS]
    if list(anchors) != expected:
        raise ValueError("Story 21.2 task anchors are missing or reordered")
    for mapping in MAPPINGS:
        checked = anchors[mapping["task_id"]]["checked"]
        if checked != (mapping["status"] == "complete"):
            raise ValueError(f"Story 21.2 task status drifted: {mapping['task_id']}")
    return anchors


def safe_relative_path(value: str) -> bool:
    path = PurePosixPath(value)
    return bool(value) and not path.is_absolute() and ".." not in path.parts and str(path) == value


def source_text(root: Path, relative: str) -> str:
    return (root / relative).read_text(encoding="utf-8")


def validate_mapping_sources(root: Path = ROOT) -> None:
    evidence_paths = set(EVIDENCE_PATHS)
    for relative in EVIDENCE_PATHS:
        if not safe_relative_path(relative) or not (root / relative).is_file():
            raise ValueError(f"Story 21.2 evidence path is unavailable: {relative}")
    searchable = "\n".join(
        source_text(root, relative)
        for relative in EVIDENCE_PATHS
        if Path(relative).suffix in {".py", ".rs"}
    )
    for mapping in MAPPINGS:
        for relative in (*mapping["code"], *mapping["evidence"]):
            if relative not in evidence_paths:
                raise ValueError(f"unindexed Story 21.2 evidence path: {relative}")
        for test_id in mapping["tests"]:
            if test_id not in searchable:
                raise ValueError(f"Story 21.2 test identity is unresolved: {test_id}")


def evidence_records(root: Path = ROOT) -> list[dict[str, Any]]:
    return [
        {
            "path": relative,
            "bytes": (root / relative).stat().st_size,
            "sha256": sha256_file(root / relative),
        }
        for relative in EVIDENCE_PATHS
    ]


def build_index(source_revision: str, root: Path = ROOT) -> dict[str, Any]:
    if re.fullmatch(r"[0-9a-f]{40}", source_revision) is None:
        raise ValueError("Story 21.2 source revision is invalid")
    anchors = task_anchors(root)
    validate_mapping_sources(root)
    mappings = []
    for mapping in MAPPINGS:
        mappings.append({**copy.deepcopy(mapping), **anchors[mapping["task_id"]]})
    counts = {
        status: sum(mapping["status"] == status for mapping in MAPPINGS)
        for status in ("complete", "partial", "open")
    }
    index: dict[str, Any] = {
        "schema_version": 1,
        "record_type": "story_21_2_requirement_code_test_evidence_index",
        "story_id": "21.2",
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
        return ["Story 21.2 evidence index is not an object"]
    revision = index.get("source_revision")
    if not isinstance(revision, str):
        return ["Story 21.2 evidence index source revision is absent"]
    try:
        expected = build_index(revision, root)
    except (OSError, UnicodeError, ValueError) as error:
        return [str(error)]
    return [] if index == expected else ["Story 21.2 evidence index differs from exact inputs"]


def check_index(root: Path = ROOT) -> list[str]:
    path = root / REPORT_PATH.relative_to(ROOT)
    try:
        index = json.loads(path.read_text(encoding="utf-8"))
    except (OSError, UnicodeError, json.JSONDecodeError):
        return ["Story 21.2 evidence index is unreadable"]
    return validate_index(index, root)


def write_atomic(path: Path, content: bytes) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    descriptor, temporary_name = tempfile.mkstemp(prefix=".story-21-2-", dir=path.parent)
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
            print(f"Story 21.2 evidence index failed: {failure}", file=sys.stderr)
        return 1 if failures else 0
    revision = git_revision(arguments.source_revision)
    write_atomic(REPORT_PATH, canonical_json(build_index(revision)))
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
