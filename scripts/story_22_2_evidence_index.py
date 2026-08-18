#!/usr/bin/env python3
"""Build and verify the hashed Story 22.2 requirement-to-evidence index."""

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
REPORT_PATH = ROOT / "artifacts/sprints/sprint-22/story-22.2/evidence-index.json"
ZERO_SHA256 = "0" * 64
TASK_PATTERN = re.compile(
    r"^\s*- \[(?P<checked>[ x])\] \*\*Sub-task "
    r"(?P<task_id>22\.2\.[123]\.[1-7])(?:[^*]*)?:\*\* (?P<statement>.+)$"
)

EVIDENCE_PATHS = (
    "artifacts/sprints/sprint-22/story-22.2/artifact-boundary-review.json",
    "artifacts/sprints/sprint-22/story-22.2/native-artifact-crash-matrix.json",
    "artifacts/sprints/sprint-22/story-22.2/native-artifact-crash-matrix.log",
    "artifacts/sprints/sprint-22/story-22.2/native-artifact-integrity.json",
    "artifacts/sprints/sprint-22/story-22.2/native-artifact-integrity.log",
    "artifacts/sprints/sprint-22/story-22.2/native-artifact-pressure.json",
    "artifacts/sprints/sprint-22/story-22.2/native-artifact-pressure.log",
    "artifacts/sprints/sprint-22/story-22.2/native-artifact-resume.json",
    "artifacts/sprints/sprint-22/story-22.2/native-artifact-resume.log",
    "artifacts/sprints/sprint-22/story-22.2/security-evidence-map.json",
    "artifacts/sprints/sprint-22/story-22.2/security-evidence.log",
    "docs/architecture/runtime-artifact-lifecycle.md",
    "docs/verification/task-22-2-3-6-product-security-evidence.md",
    "kernel/contracts/src/runtime_artifact.rs",
    "kernel/engine/src/operational_store.rs",
    "kernel/engine/src/runtime_artifact.rs",
    "kernel/engine/src/runtime_loop.rs",
    "kernel/engine/src/runtime_loop_tests.rs",
    "platforms/linux/src/platform.rs",
    "platforms/linux/src/runtime_artifact_store.rs",
    "schemas/runtime/examples/runtime-artifact-manifest.valid.json",
    "schemas/runtime/examples/runtime-artifact-operator-view.valid.json",
    "schemas/runtime/examples/runtime-artifact-reference.valid.json",
    "schemas/runtime/examples/runtime-resume-binding.valid.json",
    "schemas/runtime/runtime-artifact-manifest.schema.json",
    "schemas/runtime/runtime-artifact-operator-view.schema.json",
    "schemas/runtime/runtime-artifact-reference.schema.json",
    "schemas/runtime/runtime-resume-binding.schema.json",
    "scripts/story_22_2_artifact_crash_evidence.py",
    "scripts/story_22_2_artifact_integrity_evidence.py",
    "scripts/story_22_2_artifact_pressure_evidence.py",
    "scripts/story_22_2_artifact_resume_evidence.py",
    "scripts/story_22_2_evidence_index.py",
    "scripts/runtime_artifact_boundary_review.py",
    "scripts/story_22_2_security_evidence.py",
    "shells/host/src/coding_session.rs",
    "shells/host/src/linux_coding_runtime.rs",
    "tests/test_planning_schemas.mjs",
    "tests/test_story_22_2_artifact_crash_evidence.py",
    "tests/test_story_22_2_artifact_integrity_evidence.py",
    "tests/test_story_22_2_artifact_pressure_evidence.py",
    "tests/test_story_22_2_artifact_resume_evidence.py",
    "tests/test_story_22_2_evidence_index.py",
    "tests/test_runtime_artifact_boundary_review.py",
    "tests/test_story_22_2_security_evidence.py",
)

MAPPINGS: tuple[dict[str, Any], ...] = (
    {
        "task_id": "22.2.1.1",
        "status": "complete",
        "code": [
            "kernel/contracts/src/runtime_artifact.rs",
            "kernel/engine/src/runtime_artifact.rs",
            "schemas/runtime/runtime-artifact-reference.schema.json",
            "schemas/runtime/runtime-artifact-manifest.schema.json",
        ],
        "tests": [
            "size_media_retention_and_preview_bounds_are_closed",
            "manifest_digest_or_reference_drift_fails_closed",
        ],
        "evidence": [
            "schemas/runtime/examples/runtime-artifact-reference.valid.json",
            "schemas/runtime/examples/runtime-artifact-manifest.valid.json",
        ],
    },
    {
        "task_id": "22.2.1.2",
        "status": "complete",
        "code": [
            "kernel/engine/src/operational_store.rs",
            "platforms/linux/src/platform.rs",
            "platforms/linux/src/runtime_artifact_store.rs",
        ],
        "tests": [
            "stage_place_read_inventory_and_dedup_are_content_addressed",
            "staging_and_objects_are_encrypted_randomized_and_key_bound",
        ],
        "evidence": [
            "artifacts/sprints/sprint-22/story-22.2/native-artifact-integrity.json"
        ],
    },
    {
        "task_id": "22.2.1.3",
        "status": "complete",
        "code": [
            "kernel/engine/src/runtime_artifact.rs",
            "platforms/linux/src/runtime_artifact_store.rs",
        ],
        "tests": [
            "publication_rejects_mismatched_producer_authority_before_staging",
            "partial_and_identifier_colliding_publications_preserve_canonical_state",
            "staging_is_bounded_and_interrupted_objects_are_cleaned",
        ],
        "evidence": [
            "artifacts/sprints/sprint-22/story-22.2/native-artifact-integrity.json",
            "artifacts/sprints/sprint-22/story-22.2/native-artifact-integrity.log",
        ],
    },
    {
        "task_id": "22.2.1.4",
        "status": "complete",
        "code": [
            "kernel/engine/src/runtime_loop.rs",
            "kernel/engine/src/runtime_loop_tests.rs",
            "shells/host/src/linux_coding_runtime.rs",
        ],
        "tests": [
            "durable_large_model_output_is_artifact_backed_and_event_referenced",
            "durable_tool_outputs_use_only_the_declared_closed_artifact_kind",
            "story_22_2_linux_generated_file_is_published_and_checkpoint_bound",
        ],
        "evidence": [
            "artifacts/sprints/sprint-22/story-22.2/native-artifact-integrity.json"
        ],
    },
    {
        "task_id": "22.2.1.5",
        "status": "complete",
        "code": [
            "kernel/engine/src/runtime_artifact.rs",
            "platforms/linux/src/runtime_artifact_store.rs",
        ],
        "tests": [
            "publication_deduplicates_without_broadening_owner_or_reference_state",
            "current_checkpoint_reference_prevents_release_and_collection",
            "hard_links_and_open_handle_collection_do_not_disclose_plaintext",
        ],
        "evidence": [
            "artifacts/sprints/sprint-22/story-22.2/native-artifact-integrity.json"
        ],
    },
    {
        "task_id": "22.2.1.6",
        "status": "complete",
        "code": [
            "kernel/engine/src/runtime_artifact.rs",
            "shells/host/src/linux_coding_runtime.rs",
        ],
        "tests": [
            "checkpoint_cursor_and_artifact_set_publish_atomically_and_reopen",
            "failed_resume_binding_rolls_back_the_checkpoint_and_requires_reopen",
            "story_22_2_linux_restart_restores_checkpoint_without_replaying_the_tool",
        ],
        "evidence": [
            "artifacts/sprints/sprint-22/story-22.2/native-artifact-resume.json",
            "artifacts/sprints/sprint-22/story-22.2/native-artifact-resume.log",
        ],
    },
    {
        "task_id": "22.2.1.7",
        "status": "complete",
        "code": ["platforms/linux/src/runtime_artifact_store.rs"],
        "tests": ["repository_evidence_namespace_is_never_private_artifact_authority"],
        "evidence": ["docs/architecture/runtime-artifact-lifecycle.md"],
    },
    {
        "task_id": "22.2.2.1",
        "status": "complete",
        "code": [
            "schemas/runtime/runtime-artifact-reference.schema.json",
            "schemas/runtime/runtime-artifact-manifest.schema.json",
            "schemas/runtime/runtime-artifact-operator-view.schema.json",
            "schemas/runtime/runtime-resume-binding.schema.json",
        ],
        "tests": ["runtimeArtifactTypes"],
        "evidence": ["docs/architecture/runtime-artifact-lifecycle.md"],
    },
    {
        "task_id": "22.2.2.2",
        "status": "complete",
        "code": ["kernel/engine/src/runtime_artifact.rs"],
        "tests": [
            "checkpoint_cursor_and_artifact_set_publish_atomically_and_reopen",
            "missing_and_corrupt_payloads_quarantine_every_active_reference",
        ],
        "evidence": [
            "docs/architecture/runtime-artifact-lifecycle.md",
            "artifacts/sprints/sprint-22/story-22.2/native-artifact-resume.json",
        ],
    },
    {
        "task_id": "22.2.2.3",
        "status": "complete",
        "code": [
            "kernel/contracts/src/runtime_artifact.rs",
            "kernel/engine/src/runtime_artifact.rs",
            "schemas/runtime/runtime-artifact-operator-view.schema.json",
        ],
        "tests": ["operator_view_is_path_free_complete_and_tracks_cleanup_state"],
        "evidence": [
            "docs/architecture/runtime-artifact-lifecycle.md",
            "schemas/runtime/examples/runtime-artifact-operator-view.valid.json",
        ],
    },
    {
        "task_id": "22.2.2.4",
        "status": "complete",
        "code": ["scripts/story_22_2_evidence_index.py"],
        "tests": ["test_current_index_is_exact_and_hash_bound"],
        "evidence": ["docs/architecture/runtime-artifact-lifecycle.md"],
    },
    {
        "task_id": "22.2.3.1",
        "status": "complete",
        "code": [
            "kernel/engine/src/runtime_artifact.rs",
            "platforms/linux/src/runtime_artifact_store.rs",
            "scripts/story_22_2_artifact_integrity_evidence.py",
        ],
        "tests": [
            "test_coverage_names_every_required_integrity_class",
            "test_current_report_and_raw_trace_are_hash_bound",
        ],
        "evidence": [
            "artifacts/sprints/sprint-22/story-22.2/native-artifact-integrity.json",
            "artifacts/sprints/sprint-22/story-22.2/native-artifact-integrity.log",
        ],
    },
    {
        "task_id": "22.2.3.2",
        "status": "partial",
        "code": [
            "platforms/linux/src/runtime_artifact_store.rs",
            "scripts/story_22_2_artifact_crash_evidence.py",
        ],
        "tests": [
            "story_22_2_native_crash_matrix_reconciles_every_artifact_boundary",
            "test_closed_matrix_has_every_native_boundary_position_once",
        ],
        "evidence": [
            "artifacts/sprints/sprint-22/story-22.2/native-artifact-crash-matrix.json",
            "artifacts/sprints/sprint-22/story-22.2/native-artifact-crash-matrix.log",
        ],
    },
    {
        "task_id": "22.2.3.3",
        "status": "complete",
        "code": [
            "shells/host/src/coding_session.rs",
            "shells/host/src/linux_coding_runtime.rs",
            "scripts/story_22_2_artifact_resume_evidence.py",
        ],
        "tests": [
            "story_22_2_linux_restart_recovers_large_terminal_model_artifact",
            "story_22_2_linux_restart_restores_large_command_and_test_artifacts_without_replay",
            "story_22_2_linux_restart_restores_checkpoint_without_replaying_the_tool",
            "test_closed_metrics_cover_exact_resume_drift_and_payload_loss",
        ],
        "evidence": [
            "artifacts/sprints/sprint-22/story-22.2/native-artifact-resume.json",
            "artifacts/sprints/sprint-22/story-22.2/native-artifact-resume.log",
        ],
    },
    {
        "task_id": "22.2.3.4",
        "status": "complete",
        "code": [
            "kernel/engine/src/runtime_artifact.rs",
            "platforms/linux/src/runtime_artifact_store.rs",
        ],
        "tests": [
            "fixed_namespace_substitution_fails_before_the_next_store_effect",
            "concurrent_file_races_deduplicate_or_fail_closed_without_deletion_or_disclosure",
            "story_21_2_artifact_canaries_require_an_exact_owner_bound_payload_read",
            "repository_evidence_namespace_is_never_private_artifact_authority",
        ],
        "evidence": [
            "artifacts/sprints/sprint-22/story-22.2/native-artifact-integrity.json"
        ],
    },
    {
        "task_id": "22.2.3.5",
        "status": "partial",
        "code": [
            "platforms/linux/src/runtime_artifact_store.rs",
            "scripts/story_22_2_artifact_pressure_evidence.py",
        ],
        "tests": [
            "story_22_2_native_artifact_pressure_reaches_declared_ceilings",
            "test_exact_ceiling_metrics_pass",
        ],
        "evidence": [
            "artifacts/sprints/sprint-22/story-22.2/native-artifact-pressure.json",
            "artifacts/sprints/sprint-22/story-22.2/native-artifact-pressure.log",
        ],
    },
    {
        "task_id": "22.2.3.6",
        "status": "complete",
        "code": [
            "scripts/runtime_artifact_boundary_review.py",
            "scripts/story_22_2_security_evidence.py",
        ],
        "tests": [
            "test_current_committed_boundary_passes_every_independent_check",
            "test_retained_report_and_log_are_hash_bound",
        ],
        "evidence": [
            "artifacts/sprints/sprint-22/story-22.2/artifact-boundary-review.json",
            "artifacts/sprints/sprint-22/story-22.2/native-artifact-crash-matrix.json",
            "artifacts/sprints/sprint-22/story-22.2/native-artifact-integrity.json",
            "artifacts/sprints/sprint-22/story-22.2/native-artifact-pressure.json",
            "artifacts/sprints/sprint-22/story-22.2/native-artifact-resume.json",
            "artifacts/sprints/sprint-22/story-22.2/security-evidence-map.json",
            "artifacts/sprints/sprint-22/story-22.2/security-evidence.log",
        ],
    },
)

LIMITATIONS = (
    "Stops inside filesystem or SQLite syscalls and physical storage-fault campaigns remain open.",
    "Quarantined-payload operator recovery, long mixed-artifact sessions, and concurrent collection campaigns remain open.",
    "Larger unique-object and installed-interface pressure evidence remains open.",
    "Windows native artifact storage and supported-platform package evidence remains open.",
    "Independent automated artifact-boundary review passes; independent human and cryptographic review remains open.",
    "Manual fuzzing remains deliberately deferred and open.",
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
            raise ValueError(f"duplicate Story 22.2 sub-task: {task_id}")
        anchors[task_id] = {
            "checked": match.group("checked") == "x",
            "statement_sha256": sha256_bytes(
                match.group("statement").encode("utf-8")
            ),
        }
    expected = [mapping["task_id"] for mapping in MAPPINGS]
    if list(anchors) != expected:
        raise ValueError("Story 22.2 task anchors are missing or reordered")
    for mapping in MAPPINGS:
        checked = anchors[mapping["task_id"]]["checked"]
        if checked != (mapping["status"] == "complete"):
            raise ValueError(f"Story 22.2 task status drifted: {mapping['task_id']}")
    return anchors


def safe_relative_path(value: str) -> bool:
    path = PurePosixPath(value)
    return bool(value) and not path.is_absolute() and ".." not in path.parts and str(path) == value


def validate_mapping_sources(root: Path = ROOT) -> None:
    evidence_paths = set(EVIDENCE_PATHS)
    for relative in EVIDENCE_PATHS:
        if not safe_relative_path(relative) or not (root / relative).is_file():
            raise ValueError(f"Story 22.2 evidence path is unavailable: {relative}")
    searchable = "\n".join(
        (root / relative).read_text(encoding="utf-8")
        for relative in EVIDENCE_PATHS
        if Path(relative).suffix in {".mjs", ".py", ".rs"}
    )
    for mapping in MAPPINGS:
        for relative in (*mapping["code"], *mapping["evidence"]):
            if relative not in evidence_paths:
                raise ValueError(f"unindexed Story 22.2 evidence path: {relative}")
        for test_id in mapping["tests"]:
            if test_id not in searchable:
                raise ValueError(f"Story 22.2 test identity is unresolved: {test_id}")


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
        raise ValueError("Story 22.2 source revision is invalid")
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
        "record_type": "story_22_2_requirement_code_test_evidence_index",
        "story_id": "22.2",
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
        return ["Story 22.2 evidence index is not an object"]
    revision = index.get("source_revision")
    if not isinstance(revision, str):
        return ["Story 22.2 evidence index source revision is absent"]
    try:
        expected = build_index(revision, root)
    except (OSError, UnicodeError, ValueError) as error:
        return [str(error)]
    return [] if index == expected else ["Story 22.2 evidence index differs from exact inputs"]


def check_index(root: Path = ROOT) -> list[str]:
    path = root / REPORT_PATH.relative_to(ROOT)
    try:
        index = json.loads(path.read_text(encoding="utf-8"))
    except (OSError, UnicodeError, json.JSONDecodeError):
        return ["Story 22.2 evidence index is unreadable"]
    return validate_index(index, root)


def write_atomic(path: Path, content: bytes) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    descriptor, temporary_name = tempfile.mkstemp(prefix=".story-22-2-", dir=path.parent)
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
            print(f"Story 22.2 evidence index failed: {failure}", file=sys.stderr)
        return 1 if failures else 0
    revision = git_revision(arguments.source_revision)
    write_atomic(REPORT_PATH, canonical_json(build_index(revision)))
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
