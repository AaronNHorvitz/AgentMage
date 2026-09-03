#!/usr/bin/env python3
"""Independently evaluate Story 11.2 without widening bounded evidence."""

from __future__ import annotations

import argparse
import hashlib
import json
import os
import subprocess
import sys
import tempfile
from pathlib import Path
from typing import Any, Final


ROOT: Final = Path(__file__).resolve().parents[1]
EVIDENCE_DIR: Final = ROOT / "artifacts/sprints/sprint-11/story-11.2"
REPORT_PATH: Final = EVIDENCE_DIR / "story-gate-report.json"
RAW_PATH: Final = EVIDENCE_DIR / "story-gate-results.log"
REVIEWED_COMMIT: Final = "cae200123296ca8071c2ed0527b4cd718e9e5971"
REVIEWED_TREE: Final = "6b0f2441cc1fa8b3f31d65b4b3bba95132bc9494"
VALIDATOR_COMMANDS: Final = (
    ("python3", "scripts/story_11_2_ac1_evidence.py"),
    ("python3", "scripts/story_11_2_ac2_evidence.py"),
    ("python3", "scripts/story_11_2_ac3_evidence.py"),
)
VALIDATOR_MARKERS: Final = (
    "Story acceptance criterion 11.2.AC1 crash recovery validated",
    "Story acceptance criterion 11.2.AC2 invalidation and stale-result refusal validated",
    "Story acceptance criterion 11.2.AC3 lifecycle and restricted-data reconciliation validated",
)
REVIEWED_PATHS: Final = (
    "kernel/engine/migrations/operational-store/0012-source-artifact-materializations.sql",
    "kernel/engine/migrations/operational-store/0013-source-content-deduplication.sql",
    "kernel/engine/migrations/operational-store/0014-source-lifecycle-transactions.sql",
    "kernel/engine/migrations/operational-store/0015-workflow-materializations.sql",
    "kernel/engine/migrations/operational-store/0016-workflow-attempt-invariants.sql",
    "kernel/engine/src/operational_store.rs",
    "kernel/engine/src/runtime_artifact.rs",
    "kernel/engine/src/runtime_loop.rs",
    "kernel/engine/src/source_lifecycle.rs",
    "artifacts/sprints/sprint-11/story-11.2/source-materialization-schema-report.json",
    "artifacts/sprints/sprint-11/story-11.2/source-content-deduplication-report.json",
    "artifacts/sprints/sprint-11/story-11.2/source-lifecycle-transactions-report.json",
    "artifacts/sprints/sprint-11/story-11.2/workflow-materialization-schema-report.json",
    "artifacts/sprints/sprint-11/story-11.2/workflow-attempt-invariants-report.json",
    "artifacts/sprints/sprint-11/story-11.2/workflow-atomic-publication-report.json",
    "artifacts/sprints/sprint-11/story-11.2/storage-migration-compatibility-report.json",
    "artifacts/sprints/sprint-11/story-11.2/storage-downgrade-refusal-report.json",
    "artifacts/sprints/sprint-11/story-11.2/storage-new-family-lifecycle-report.json",
    "artifacts/sprints/sprint-11/story-11.2/storage-crash-boundary-report.json",
    "artifacts/sprints/sprint-11/story-11.2/storage-crash-exact-state-report.json",
    "artifacts/sprints/sprint-11/story-11.2/storage-security-evidence-report.json",
    "artifacts/sprints/sprint-11/story-11.2/story-ac1-crash-acceptance-report.json",
    "artifacts/sprints/sprint-11/story-11.2/story-ac2-invalidation-acceptance-report.json",
    "artifacts/sprints/sprint-11/story-11.2/story-ac3-lifecycle-acceptance-report.json",
)
REQUIRED_TASK_MARKERS: Final = (
    "- [x] **Task 11.2.1 - Add source-artifact materializations**",
    "- [x] **Task 11.2.2 - Add workflow materializations**",
    "- [x] **Task 11.2.3 - Implement migrations and compatibility**",
    "- [x] **Task 11.2.4 - Verify crash and recovery semantics**",
    *(f"  - [x] **Sub-task 11.2.{task}.{sub}:" for task in range(1, 5) for sub in range(1, 4)),
    *(f"- [x] **Story AC 11.2.AC{criterion}:" for criterion in range(1, 4)),
)
G_DOD_IDS: Final = tuple(f"G-DOD-{index:02d}" for index in range(1, 14)) + (
    "G-DOD-21",
    "G-DOD-22",
)
BLOCKERS: Final = (
    "story-5.2-acceptance-gate-open",
    "story-11.1-acceptance-gate-open",
    "supported-platform-installed-product-evidence-incomplete",
    "integrated-client-runtime-removal-accessibility-evidence-incomplete",
)


def canonical_json(value: Any) -> bytes:
    return (json.dumps(value, indent=2, sort_keys=True) + "\n").encode("utf-8")


def sha256_bytes(value: bytes) -> str:
    return hashlib.sha256(value).hexdigest()


def read_json(path: Path) -> Any:
    return json.loads(path.read_text(encoding="utf-8"))


def write_atomic(path: Path, content: bytes) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    descriptor, temporary_name = tempfile.mkstemp(prefix=".agentmage-story-11-2-gate-", dir=path.parent)
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


def git_output(*arguments: str, binary: bool = False) -> bytes | str:
    result = subprocess.run(
        ["git", *arguments], cwd=ROOT, stdin=subprocess.DEVNULL, stdout=subprocess.PIPE,
        stderr=subprocess.PIPE, check=False, timeout=15,
    )
    if result.returncode != 0:
        raise ValueError(f"git review operation failed: {' '.join(arguments)}")
    return result.stdout if binary else result.stdout.decode("utf-8").strip()


def reviewed_artifacts() -> list[dict[str, Any]]:
    if git_output("rev-parse", REVIEWED_COMMIT) != REVIEWED_COMMIT:
        raise ValueError("Story 11.2 reviewed commit is unavailable")
    if git_output("show", "-s", "--format=%T", REVIEWED_COMMIT) != REVIEWED_TREE:
        raise ValueError("Story 11.2 reviewed tree changed")
    records = []
    for path in REVIEWED_PATHS:
        committed = git_output("show", f"{REVIEWED_COMMIT}:{path}", binary=True)
        current = ROOT / path
        if not current.is_file() or current.read_bytes() != committed:
            raise ValueError(f"reviewed Story 11.2 artifact changed after review: {path}")
        records.append({"path": path, "byte_length": len(committed), "sha256": sha256_bytes(committed)})
    return records


def task_completion(tasks_text: str) -> list[str]:
    return [marker for marker in REQUIRED_TASK_MARKERS if marker not in tasks_text]


def dependency_state(tasks_text: str) -> list[dict[str, str]]:
    expected = (
        ("1.2", "#### [x] Story 1.2 -", "complete"),
        ("5.2", "#### [ ] Story 5.2 -", "blocked-open-acceptance"),
        ("11.1", "#### [ ] Story 11.1 -", "blocked-open-acceptance"),
    )
    states = []
    for story_id, marker, status in expected:
        if marker not in tasks_text:
            raise ValueError(f"Story 11.2 dependency state changed: {story_id}")
        states.append({"story_id": story_id, "status": status})
    return states


def universal_dod() -> list[dict[str, str]]:
    statuses = {control_id: "pass-current-story-scope" for control_id in G_DOD_IDS}
    statuses["G-DOD-06"] = "pass-exact-receipt-linkage-no-new-tool-authority"
    statuses["G-DOD-08"] = "pass-no-user-owned-file-effects"
    statuses["G-DOD-10"] = "blocked-supported-platform-installed-product-matrix"
    statuses["G-DOD-12"] = "pass-automated-independent-aggregate-review"
    statuses["G-DOD-22"] = "blocked-later-integrated-client-runtime-evidence"
    return [{"control_id": control_id, "status": statuses[control_id]} for control_id in G_DOD_IDS]


def capture_validators() -> tuple[str, int]:
    chunks = []
    for command in VALIDATOR_COMMANDS:
        result = subprocess.run(command, cwd=ROOT, stdout=subprocess.PIPE, stderr=subprocess.STDOUT, text=True, check=False)
        chunks.append(f"$ {' '.join(command)}\n{result.stdout.rstrip()}\n")
        if result.returncode != 0:
            return "".join(chunks), result.returncode
    return "".join(chunks), 0


def validate_raw(value: str) -> list[str]:
    failures = [f"Story 11.2 gate results missing marker: {marker}" for marker in VALIDATOR_MARKERS if marker not in value]
    for prohibited in ("Traceback", "FAILED", "validation failed"):
        if prohibited in value:
            failures.append(f"Story 11.2 gate results contain prohibited marker: {prohibited}")
    return failures


def build_report() -> dict[str, Any]:
    tasks_text = (ROOT / "TASKS.md").read_text(encoding="utf-8")
    incomplete = task_completion(tasks_text)
    if incomplete:
        raise ValueError(f"Story 11.2 task or criterion is incomplete: {incomplete[0]}")
    raw = RAW_PATH.read_text(encoding="utf-8")
    raw_failures = validate_raw(raw)
    if raw_failures:
        raise ValueError("; ".join(raw_failures))
    criteria = []
    for name in (
        "story-ac1-crash-acceptance-report.json",
        "story-ac2-invalidation-acceptance-report.json",
        "story-ac3-lifecycle-acceptance-report.json",
    ):
        report = read_json(EVIDENCE_DIR / name)
        criteria.append({
            "criterion_id": report["criterion_id"],
            "status": report["status"],
            "acceptance_truth_sha256": sha256_bytes(canonical_json(report["acceptance_truth"])),
        })
    return {
        "schema_version": 1,
        "story_id": "11.2",
        "status": "blocked-open-dependencies-and-platform-integration",
        "reviewed_commit": REVIEWED_COMMIT,
        "reviewed_tree": REVIEWED_TREE,
        "task_count": 4,
        "sub_task_count": 12,
        "acceptance_criteria": criteria,
        "dependencies": dependency_state(tasks_text),
        "universal_definition_of_done": universal_dod(),
        "blocking_controls": ["G-DOD-10", "G-DOD-22"],
        "blockers": list(BLOCKERS),
        "independent_review": {
            "reviewer_id": "agentmage-story-11.2-independent-gate-v1",
            "review_type": "automated-independent-aggregate-review",
            "reviewed_commit": REVIEWED_COMMIT,
            "reviewed_tree": REVIEWED_TREE,
            "artifacts": reviewed_artifacts(),
            "finding_count": 0,
            "findings": [],
            "external_human_review_status": "not-performed",
        },
        "validator_results_sha256": sha256_bytes(raw.encode("utf-8")),
        "current_linux_store_and_runtime_scope_complete": True,
        "story_checkbox_complete": False,
        "dependency_substitution_permitted": False,
        "platform_evidence_substituted": False,
        "installed_product_claim": "none",
        "product_acceptance_claim": "none",
        "sprint_completion_claim": False,
        "release_claim": "none",
    }


def validate_report(value: Any, *, verify_current: bool = True) -> list[str]:
    if not isinstance(value, dict):
        return ["Story 11.2 gate report must be an object"]
    failures = []
    if value.get("story_id") != "11.2" or value.get("status") != "blocked-open-dependencies-and-platform-integration":
        failures.append("Story 11.2 gate identity or status is invalid")
    criteria = value.get("acceptance_criteria", [])
    if [item.get("criterion_id") for item in criteria] != ["11.2.AC1", "11.2.AC2", "11.2.AC3"] or not all(
        item.get("status", "").startswith("pass-local-current-") for item in criteria
    ):
        failures.append("Story 11.2 acceptance criterion closure is invalid")
    if value.get("universal_definition_of_done") != universal_dod() or value.get("blocking_controls") != ["G-DOD-10", "G-DOD-22"]:
        failures.append("Story 11.2 Definition-of-Done disposition is invalid")
    if value.get("blockers") != list(BLOCKERS):
        failures.append("Story 11.2 blocker set is invalid")
    if value.get("dependencies") != [
        {"story_id": "1.2", "status": "complete"},
        {"story_id": "5.2", "status": "blocked-open-acceptance"},
        {"story_id": "11.1", "status": "blocked-open-acceptance"},
    ]:
        failures.append("Story 11.2 dependency disposition is invalid")
    review = value.get("independent_review", {})
    if (
        review.get("reviewer_id") != "agentmage-story-11.2-independent-gate-v1"
        or review.get("review_type") != "automated-independent-aggregate-review"
        or review.get("reviewed_commit") != REVIEWED_COMMIT
        or review.get("reviewed_tree") != REVIEWED_TREE
        or review.get("finding_count") != 0
        or review.get("findings") != []
        or review.get("external_human_review_status") != "not-performed"
        or len(review.get("artifacts", [])) != len(REVIEWED_PATHS)
    ):
        failures.append("Story 11.2 independent review is invalid")
    if (
        value.get("story_checkbox_complete") is not False
        or value.get("dependency_substitution_permitted") is not False
        or value.get("platform_evidence_substituted") is not False
        or value.get("installed_product_claim") != "none"
        or value.get("product_acceptance_claim") != "none"
        or value.get("sprint_completion_claim") is not False
        or value.get("release_claim") != "none"
    ):
        failures.append("Story 11.2 gate made an unsupported completion claim")
    if verify_current:
        try:
            expected = build_report()
        except (OSError, ValueError, KeyError, TypeError, json.JSONDecodeError) as error:
            failures.append(f"cannot rebuild Story 11.2 gate report: {error}")
        else:
            if value != expected:
                failures.append("Story 11.2 gate report is stale, incomplete, or widened")
    return failures


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--write", action="store_true")
    args = parser.parse_args()
    try:
        if args.write:
            raw, returncode = capture_validators()
            if returncode != 0:
                sys.stderr.write(raw)
                return 1
            raw_failures = validate_raw(raw)
            if raw_failures:
                raise ValueError("; ".join(raw_failures))
            write_atomic(RAW_PATH, raw.encode("utf-8"))
            write_atomic(REPORT_PATH, canonical_json(build_report()))
        report = read_json(REPORT_PATH)
        failures = validate_report(report)
    except (OSError, ValueError, KeyError, TypeError, json.JSONDecodeError) as error:
        print(f"Story 11.2 gate failed: {error}", file=sys.stderr)
        return 1
    if failures:
        for failure in failures:
            print(f"Story 11.2 gate failed: {failure}", file=sys.stderr)
        return 1
    print("Story 11.2 current Linux scope passed with dependency and platform blockers preserved")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
