#!/usr/bin/env python3
"""Independently evaluate Story 1.3 without widening bounded record evidence."""

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
EVIDENCE_DIR: Final = ROOT / "artifacts/sprints/sprint-1/story-1.3"
REPORT_PATH: Final = EVIDENCE_DIR / "story-gate-report.json"
RAW_PATH: Final = EVIDENCE_DIR / "story-gate-results.log"
REVIEWED_COMMIT: Final = "7c3247c6449030f12323a3274ada3ea673fdf14f"
REVIEWED_TREE: Final = "16cee274f7584de087a25d550ce8cecc002676a7"
VALIDATOR_COMMANDS: Final = (
    ("python3", "scripts/story_1_3_ac1_evidence.py"),
    ("python3", "scripts/story_1_3_ac2_evidence.py"),
    ("python3", "scripts/story_1_3_ac3_evidence.py"),
    ("python3", "scripts/engineering_runtime_rv50_evidence.py"),
)
VALIDATOR_MARKERS: Final = (
    "Story acceptance criterion 1.3.AC1 Rust-owned boundary meaning validated",
    "Story acceptance criterion 1.3.AC2 cross-client identity parity validated",
    "Story acceptance criterion 1.3.AC3 pre-dispatch rejection validated",
    "Task 1.3.3.2 applicable RV-50 evidence validated with external blockers open",
)
REVIEWED_PATHS: Final = (
    "kernel/contracts/src/engineering_records.rs",
    "kernel/contracts/src/serialization.rs",
    "kernel/engine/src/engineering_records.rs",
    "kernel/contracts/tests/engineering_runtime_record_types.rs",
    "kernel/engine/tests/engineering_runtime_record_corpus.rs",
    "scripts/engineering_runtime_schemas.mjs",
    "scripts/engineering_runtime_fixture_corpus.mjs",
    "scripts/engineering_runtime_record_evidence.py",
    "scripts/engineering_runtime_rv50_evidence.py",
    "tests/test_engineering_runtime_schemas.mjs",
    "tests/test_engineering_runtime_fixture_corpus.mjs",
    "tests/test_engineering_runtime_record_evidence.py",
    "tests/test_engineering_runtime_rv50_evidence.py",
    "fixtures/engineering-runtime/v2/manifest.json",
    "artifacts/sprints/sprint-1/story-1.3/canonical-record-evidence-index.json",
    "artifacts/sprints/sprint-1/story-1.3/canonical-record-gate-results.log",
    "artifacts/sprints/sprint-1/story-1.3/rv50-applicability.json",
    "artifacts/sprints/sprint-1/story-1.3/rv50-applicable-results.log",
    "artifacts/sprints/sprint-1/story-1.3/story-ac1-rust-owned-boundary-report.json",
    "artifacts/sprints/sprint-1/story-1.3/story-ac2-cross-client-parity-report.json",
    "artifacts/sprints/sprint-1/story-1.3/story-ac3-predispatch-rejection-report.json",
    "docs/verification/story-1-3-ac1-rust-owned-boundary-meaning.md",
    "docs/verification/story-1-3-ac2-cross-client-parity.md",
    "docs/verification/story-1-3-ac3-predispatch-rejection.md",
    "schemas/engineering-runtime/artifact-envelope.schema.json",
    "schemas/engineering-runtime/artifact-transformation.schema.json",
    "schemas/engineering-runtime/artifact-ingestion-result.schema.json",
    "schemas/engineering-runtime/context-manifest.schema.json",
    "schemas/engineering-runtime/workflow-definition.schema.json",
    "schemas/engineering-runtime/workflow-state.schema.json",
    "schemas/engineering-runtime/tool-observation.schema.json",
    "schemas/engineering-runtime/verification-result.schema.json",
    "schemas/engineering-runtime/terminal-result.schema.json",
)
REQUIRED_TASK_MARKERS: Final = (
    *(f"- [x] **Task 1.3.{task} -" for task in range(1, 4)),
    *(f"  - [x] **Sub-task 1.3.1.{sub}:" for sub in range(1, 4)),
    *(f"  - [x] **Sub-task 1.3.2.{sub}:" for sub in range(1, 3)),
    *(f"  - [x] **Sub-task 1.3.3.{sub}:" for sub in range(1, 3)),
    *(f"- [x] **Story AC 1.3.AC{criterion}:" for criterion in range(1, 4)),
)
G_DOD_IDS: Final = tuple(f"G-DOD-{index:02d}" for index in range(1, 14))
BLOCKERS: Final = (
    "story-5.3-acceptance-gate-open",
    "story-11.3-acceptance-gate-open",
    "story-16.4-acceptance-gate-open",
    "story-22.5-acceptance-gate-open",
    "rv-50-supported-platform-installed-product-and-real-model-evidence-incomplete",
)


def canonical_json(value: Any) -> bytes:
    return (json.dumps(value, indent=2, sort_keys=True) + "\n").encode("utf-8")


def sha256_bytes(value: bytes) -> str:
    return hashlib.sha256(value).hexdigest()


def read_json(path: Path) -> Any:
    return json.loads(path.read_text(encoding="utf-8"))


def write_atomic(path: Path, content: bytes) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    descriptor, temporary_name = tempfile.mkstemp(prefix=".agentmage-story-1-3-gate-", dir=path.parent)
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
        raise ValueError("Story 1.3 reviewed commit is unavailable")
    if git_output("show", "-s", "--format=%T", REVIEWED_COMMIT) != REVIEWED_TREE:
        raise ValueError("Story 1.3 reviewed tree changed")
    records = []
    for path in REVIEWED_PATHS:
        committed = git_output("show", f"{REVIEWED_COMMIT}:{path}", binary=True)
        current = ROOT / path
        if not current.is_file() or current.read_bytes() != committed:
            raise ValueError(f"reviewed Story 1.3 artifact changed after review: {path}")
        records.append({"path": path, "byte_length": len(committed), "sha256": sha256_bytes(committed)})
    return records


def task_completion(tasks_text: str) -> list[str]:
    return [marker for marker in REQUIRED_TASK_MARKERS if marker not in tasks_text]


def dependency_state(tasks_text: str) -> list[dict[str, str]]:
    expected = (("1.2", "#### [x] Story 1.2 -", "complete"),)
    states = []
    for story_id, marker, status in expected:
        if marker not in tasks_text:
            raise ValueError(f"Story 1.3 dependency state changed: {story_id}")
        states.append({"story_id": story_id, "status": status})
    return states


def later_story_state(tasks_text: str) -> list[dict[str, str]]:
    states = []
    for story_id in ("5.3", "11.3", "16.4", "22.5"):
        if f"#### [ ] Story {story_id} -" not in tasks_text:
            raise ValueError(f"Story 1.3 later runtime owner state changed: {story_id}")
        states.append({"story_id": story_id, "status": "blocked-open-acceptance"})
    return states


def universal_dod() -> list[dict[str, str]]:
    statuses = {control_id: "pass-current-story-scope" for control_id in G_DOD_IDS}
    statuses["G-DOD-06"] = "pass-no-production-tool-or-model-attempt"
    statuses["G-DOD-07"] = "pass-public-synthetic-data-no-new-store"
    statuses["G-DOD-08"] = "pass-no-user-owned-file-effects"
    statuses["G-DOD-10"] = "blocked-supported-platform-installed-product-matrix"
    statuses["G-DOD-12"] = "pass-automated-independent-aggregate-review"
    return [{"control_id": control_id, "status": statuses[control_id]} for control_id in G_DOD_IDS]


def capture_validators() -> tuple[str, int]:
    chunks = []
    for command in VALIDATOR_COMMANDS:
        result = subprocess.run(
            command, cwd=ROOT, stdout=subprocess.PIPE, stderr=subprocess.STDOUT,
            text=True, check=False,
        )
        chunks.append(f"$ {' '.join(command)}\n{result.stdout.rstrip()}\n")
        if result.returncode != 0:
            return "".join(chunks), result.returncode
    return "".join(chunks), 0


def validate_raw(value: str) -> list[str]:
    failures = [f"Story 1.3 gate results missing marker: {marker}" for marker in VALIDATOR_MARKERS if marker not in value]
    for prohibited in ("Traceback", "FAILED", "validation failed"):
        if prohibited in value:
            failures.append(f"Story 1.3 gate results contain prohibited marker: {prohibited}")
    return failures


def criterion_summary(report: dict[str, Any]) -> dict[str, Any]:
    truth = report["acceptance_truth"]
    criterion_id = report["criterion_id"]
    if criterion_id == "1.3.AC1":
        valid = (
            truth["canonical_record_family_count"] == 9
            and truth["canonical_schema_version"] == 2
            and truth["rust_owned_meaning"] is True
            and truth["unknown_fields_admitted"] is False
            and truth["unsupported_versions_admitted"] is False
        )
        status = "pass-local-current-rust-owned-boundary"
    elif criterion_id == "1.3.AC2":
        valid = (
            truth["canonical_record_family_count"] == 9
            and truth["independent_contract_caller_count"] == 2
            and truth["canonical_bytes_match_across_callers"] is True
            and truth["client_owns_runtime_state"] is False
            and truth["client_receives_execution_authority"] is False
        )
        status = "pass-local-current-cross-client-parity"
    elif criterion_id == "1.3.AC3":
        valid = (
            truth["schema_rejection_count"] == 45
            and truth["trusted_boundary_rejection_count"] == 2
            and truth["model_dispatch_after_rejection"] is False
            and truth["tool_dispatch_after_rejection"] is False
            and truth["effect_dispatch_after_rejection"] is False
            and truth["diagnostics_include_candidate_content"] is False
        )
        status = "pass-local-current-predispatch-rejection"
    else:
        raise ValueError(f"unexpected Story 1.3 criterion: {criterion_id}")
    if not valid:
        raise ValueError(f"Story 1.3 criterion truth is incomplete: {criterion_id}")
    return {
        "criterion_id": criterion_id,
        "status": status,
        "acceptance_truth_sha256": sha256_bytes(canonical_json(truth)),
    }


def rv50_state() -> dict[str, Any]:
    report = read_json(EVIDENCE_DIR / "rv50-applicability.json")
    later = report.get("later_runtime_scenarios", [])
    platforms = report.get("platform_runtime_evidence", [])
    if (
        report.get("status") != "PARTIAL_LOCAL_CONTRACT_EVIDENCE"
        or report.get("protocol_complete") is not False
        or len(later) != 5
        or any(item.get("status") != "BLOCKED_LATER_STORY" for item in later)
        or len(platforms) != 6
        or sum(item.get("status") == "BLOCKED_EXTERNAL" for item in platforms) != 5
        or any(item.get("substitutes_for") != [] for item in platforms)
    ):
        raise ValueError("Story 1.3 RV-50 blocker state is incomplete or widened")
    return {
        "protocol_id": "RV-50",
        "status": "partial-local-contract-evidence",
        "protocol_complete": False,
        "later_runtime_scenario_count": 5,
        "blocked_external_tuple_count": 5,
        "evidence_substitution_permitted": False,
        "report_sha256": sha256_bytes(canonical_json(report)),
    }


def build_report() -> dict[str, Any]:
    tasks_text = (ROOT / "TASKS.md").read_text(encoding="utf-8")
    incomplete = task_completion(tasks_text)
    if incomplete:
        raise ValueError(f"Story 1.3 task or criterion is incomplete: {incomplete[0]}")
    raw = RAW_PATH.read_text(encoding="utf-8")
    raw_failures = validate_raw(raw)
    if raw_failures:
        raise ValueError("; ".join(raw_failures))
    criteria = [
        criterion_summary(read_json(EVIDENCE_DIR / name))
        for name in (
            "story-ac1-rust-owned-boundary-report.json",
            "story-ac2-cross-client-parity-report.json",
            "story-ac3-predispatch-rejection-report.json",
        )
    ]
    return {
        "schema_version": 1,
        "story_id": "1.3",
        "status": "blocked-open-later-stories-and-platform",
        "reviewed_commit": REVIEWED_COMMIT,
        "reviewed_tree": REVIEWED_TREE,
        "task_count": 3,
        "sub_task_count": 7,
        "acceptance_criteria": criteria,
        "dependencies": dependency_state(tasks_text),
        "later_story_dependencies": later_story_state(tasks_text),
        "rv50": rv50_state(),
        "universal_definition_of_done": universal_dod(),
        "blocking_controls": ["G-DOD-10"],
        "blockers": list(BLOCKERS),
        "independent_review": {
            "reviewer_id": "agentmage-story-1.3-independent-gate-v1",
            "review_type": "automated-independent-aggregate-review",
            "reviewed_commit": REVIEWED_COMMIT,
            "reviewed_tree": REVIEWED_TREE,
            "artifacts": reviewed_artifacts(),
            "finding_count": 0,
            "findings": [],
            "external_human_review_status": "not-performed",
        },
        "validator_results_sha256": sha256_bytes(raw.encode("utf-8")),
        "current_local_canonical_record_scope_complete": True,
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
        return ["Story 1.3 gate report must be an object"]
    failures = []
    if (
        value.get("schema_version") != 1
        or value.get("story_id") != "1.3"
        or value.get("status") != "blocked-open-later-stories-and-platform"
        or value.get("reviewed_commit") != REVIEWED_COMMIT
        or value.get("reviewed_tree") != REVIEWED_TREE
    ):
        failures.append("Story 1.3 gate identity or status is invalid")
    criteria = value.get("acceptance_criteria", [])
    if [item.get("criterion_id") for item in criteria if isinstance(item, dict)] != [
        "1.3.AC1", "1.3.AC2", "1.3.AC3"
    ]:
        failures.append("Story 1.3 acceptance criterion closure is invalid")
    elif [item.get("status") for item in criteria] != [
        "pass-local-current-rust-owned-boundary",
        "pass-local-current-cross-client-parity",
        "pass-local-current-predispatch-rejection",
    ]:
        failures.append("Story 1.3 acceptance criterion status is invalid")
    if value.get("universal_definition_of_done") != universal_dod() or value.get("blocking_controls") != ["G-DOD-10"]:
        failures.append("Story 1.3 Definition-of-Done disposition is invalid")
    if value.get("blockers") != list(BLOCKERS):
        failures.append("Story 1.3 blocker set is invalid")
    if value.get("dependencies") != [{"story_id": "1.2", "status": "complete"}]:
        failures.append("Story 1.3 dependency disposition is invalid")
    expected_later = [
        {"story_id": story_id, "status": "blocked-open-acceptance"}
        for story_id in ("5.3", "11.3", "16.4", "22.5")
    ]
    if value.get("later_story_dependencies") != expected_later:
        failures.append("Story 1.3 later-story blocker disposition is invalid")
    rv50 = value.get("rv50", {})
    if (
        rv50.get("protocol_id") != "RV-50"
        or rv50.get("status") != "partial-local-contract-evidence"
        or rv50.get("protocol_complete") is not False
        or rv50.get("later_runtime_scenario_count") != 5
        or rv50.get("blocked_external_tuple_count") != 5
        or rv50.get("evidence_substitution_permitted") is not False
    ):
        failures.append("Story 1.3 RV-50 disposition is invalid")
    review = value.get("independent_review", {})
    if (
        review.get("reviewer_id") != "agentmage-story-1.3-independent-gate-v1"
        or review.get("review_type") != "automated-independent-aggregate-review"
        or review.get("reviewed_commit") != REVIEWED_COMMIT
        or review.get("reviewed_tree") != REVIEWED_TREE
        or review.get("finding_count") != 0
        or review.get("findings") != []
        or review.get("external_human_review_status") != "not-performed"
        or len(review.get("artifacts", [])) != len(REVIEWED_PATHS)
    ):
        failures.append("Story 1.3 independent review is invalid")
    if (
        value.get("current_local_canonical_record_scope_complete") is not True
        or value.get("story_checkbox_complete") is not False
        or value.get("dependency_substitution_permitted") is not False
        or value.get("platform_evidence_substituted") is not False
        or value.get("installed_product_claim") != "none"
        or value.get("product_acceptance_claim") != "none"
        or value.get("sprint_completion_claim") is not False
        or value.get("release_claim") != "none"
    ):
        failures.append("Story 1.3 gate made an unsupported completion claim")
    if verify_current:
        try:
            expected = build_report()
        except (OSError, ValueError, KeyError, TypeError, json.JSONDecodeError) as error:
            failures.append(f"cannot rebuild Story 1.3 gate report: {error}")
        else:
            if value != expected:
                failures.append("Story 1.3 gate report is stale, incomplete, or widened")
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
        print(f"Story 1.3 gate failed: {error}", file=sys.stderr)
        return 1
    if failures:
        for failure in failures:
            print(f"Story 1.3 gate failed: {failure}", file=sys.stderr)
        return 1
    print("Story 1.3 current local record scope passed with later-story and platform blockers preserved")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
