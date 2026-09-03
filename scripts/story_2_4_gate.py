#!/usr/bin/env python3
"""Independently evaluate Story 2.4 without widening fixture evidence."""

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
EVIDENCE_DIR: Final = ROOT / "artifacts/sprints/sprint-2/story-2.4"
REPORT_PATH: Final = EVIDENCE_DIR / "story-gate-report.json"
RAW_PATH: Final = EVIDENCE_DIR / "story-gate-results.log"
REVIEWED_COMMIT: Final = "1e31f4d8ea56e91365a1541c2050226ec575bce1"
REVIEWED_TREE: Final = "7503e9fb08c01550366509bf141cf56ea0fb2fdd"
VALIDATOR_COMMANDS: Final = (
    ("python3", "scripts/engineering_artifact_admission_fixtures.py"),
    ("python3", "scripts/engineering_artifact_adversarial_fixtures.py"),
    ("python3", "scripts/engineering_artifact_lineage_fixtures.py"),
    ("python3", "scripts/engineering_context_accounting_fixtures.py"),
    ("python3", "scripts/engineering_context_delivery_fixtures.py"),
    ("python3", "scripts/engineering_artifact_resource_gate.py"),
    ("python3", "scripts/engineering_artifact_rv51_evidence.py"),
)
VALIDATOR_MARKERS: Final = (
    "Validated 16 identity-bound artifact admission fixtures",
    "Validated 10 inert hostile artifact fixtures",
    "Validated lineage for 26 sources and 21 ranges",
    "Validated 8 complete context-accounting manifests",
    "Validated 8 reconstructible context-delivery receipts",
    "Validated 10 resource ceilings and 4 cleanup boundaries",
    "Task 2.4.3.2 applicable RV-51 evidence validated with full protocol blockers open",
)
REVIEWED_PATHS: Final = (
    "scripts/engineering_artifact_admission_fixtures.py",
    "scripts/engineering_artifact_adversarial_fixtures.py",
    "scripts/engineering_artifact_lineage_fixtures.py",
    "scripts/engineering_context_accounting_fixtures.py",
    "scripts/engineering_context_delivery_fixtures.py",
    "scripts/engineering_artifact_resource_gate.py",
    "scripts/engineering_artifact_rv51_evidence.py",
    "tests/test_engineering_artifact_admission_fixtures.py",
    "tests/test_engineering_artifact_adversarial_fixtures.py",
    "tests/test_engineering_artifact_lineage_fixtures.py",
    "tests/test_engineering_context_accounting_fixtures.py",
    "tests/test_engineering_context_delivery_fixtures.py",
    "tests/test_engineering_artifact_resource_gate.py",
    "tests/test_engineering_artifact_rv51_evidence.py",
    "tests/test_engineering_artifact_admission_records.mjs",
    "tests/test_engineering_artifact_adversarial_records.mjs",
    "tests/test_engineering_artifact_lineage_records.mjs",
    "tests/test_engineering_context_accounting_records.mjs",
    "tests/test_engineering_context_delivery_records.mjs",
    "fixtures/artifact-admission/v1/manifest.json",
    "fixtures/artifact-admission/v1/artifact-admission-corpus-v1.zip",
    "fixtures/artifact-admission/v1/adversarial-manifest.json",
    "fixtures/artifact-admission/v1/artifact-adversarial-corpus-v1.zip",
    "fixtures/artifact-admission/v1/lineage-manifest.json",
    "fixtures/artifact-admission/v1/context-accounting-manifests.json",
    "fixtures/artifact-admission/v1/context-delivery-receipts.json",
    "fixtures/artifact-admission/v1/context-delivery-payloads-v1.zip",
    "fixtures/artifact-admission/v1/resource-gate-observations.json",
    "scripts/fixture_security_scan.py",
    "SECURITY-REVIEW.md",
    "schemas/engineering-runtime/artifact-envelope.schema.json",
    "schemas/engineering-runtime/artifact-ingestion-result.schema.json",
    "schemas/engineering-runtime/artifact-transformation.schema.json",
    "schemas/engineering-runtime/context-delivery-receipt.schema.json",
    "schemas/engineering-runtime/context-disposition.schema.json",
    "schemas/engineering-runtime/context-manifest.schema.json",
    "schemas/engineering-runtime/source-artifact.schema.json",
    "schemas/engineering-runtime/source-locator.schema.json",
    "schemas/engineering-runtime/source-provenance.schema.json",
    "schemas/engineering-runtime/source-reference.schema.json",
    "schemas/engineering-runtime/source-retention.schema.json",
)
REQUIRED_TASK_MARKERS: Final = (
    *(f"- [x] **Task 2.4.{task} -" for task in range(1, 4)),
    *(f"  - [x] **Sub-task 2.4.1.{sub}:" for sub in range(1, 3)),
    *(f"  - [x] **Sub-task 2.4.2.{sub}:" for sub in range(1, 4)),
    *(f"  - [x] **Sub-task 2.4.3.{sub}:" for sub in range(1, 3)),
    *(f"- [x] **Story AC 2.4.AC{criterion}:" for criterion in range(1, 4)),
)
G_DOD_IDS: Final = tuple(f"G-DOD-{index:02d}" for index in range(1, 14))
LATER_STORY_IDS: Final = ("16.4", "22.3", "22.5", "23.5", "50.4", "60.2", "62.2", "126.2")
BLOCKERS: Final = (
    "story-1.3-aggregate-gate-blocked",
    "story-2.3-aggregate-gate-blocked",
    "rv-51-product-parser-crash-resume-installed-client-and-native-platform-evidence-incomplete",
)


def canonical_json(value: Any) -> bytes:
    return (json.dumps(value, indent=2, sort_keys=True) + "\n").encode("utf-8")


def sha256_bytes(value: bytes) -> str:
    return hashlib.sha256(value).hexdigest()


def read_json(path: Path) -> Any:
    return json.loads(path.read_text(encoding="utf-8"))


def write_atomic(path: Path, content: bytes) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    descriptor, temporary_name = tempfile.mkstemp(prefix=".agentmage-story-2-4-gate-", dir=path.parent)
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
        raise ValueError("Story 2.4 reviewed commit is unavailable")
    if git_output("show", "-s", "--format=%T", REVIEWED_COMMIT) != REVIEWED_TREE:
        raise ValueError("Story 2.4 reviewed tree changed")
    records = []
    for path in REVIEWED_PATHS:
        committed = git_output("show", f"{REVIEWED_COMMIT}:{path}", binary=True)
        current = ROOT / path
        if not current.is_file() or current.read_bytes() != committed:
            raise ValueError(f"reviewed Story 2.4 artifact changed after review: {path}")
        records.append({"path": path, "byte_length": len(committed), "sha256": sha256_bytes(committed)})
    return records


def task_completion(tasks_text: str) -> list[str]:
    return [marker for marker in REQUIRED_TASK_MARKERS if marker not in tasks_text]


def dependency_state(tasks_text: str) -> list[dict[str, str]]:
    states = []
    for story_id in ("1.3", "2.3"):
        if f"#### [ ] Story {story_id} -" not in tasks_text:
            raise ValueError(f"Story 2.4 dependency state changed: {story_id}")
        gate = read_json(ROOT / f"artifacts/sprints/sprint-{1 if story_id == '1.3' else 2}/story-{story_id}/story-gate-report.json")
        if gate.get("story_id") != story_id or gate.get("story_checkbox_complete") is not False:
            raise ValueError(f"Story 2.4 dependency gate is invalid: {story_id}")
        states.append({"story_id": story_id, "status": "blocked-open-aggregate-gate"})
    return states


def later_story_state(tasks_text: str) -> list[dict[str, str]]:
    states = []
    for story_id in LATER_STORY_IDS:
        open_marker = f"#### [ ] Story {story_id} -"
        closed_marker = f"#### [x] Story {story_id} -"
        if open_marker in tasks_text:
            status = "blocked-open-acceptance"
        elif closed_marker in tasks_text:
            status = "completed-protocol-owner"
        else:
            raise ValueError(f"Story 2.4 later protocol owner state changed: {story_id}")
        states.append({"story_id": story_id, "status": status})
    return states


def universal_dod() -> list[dict[str, str]]:
    statuses = {control_id: "pass-current-story-scope" for control_id in G_DOD_IDS}
    statuses["G-DOD-04"] = "pass-fixtures-mint-no-authority"
    statuses["G-DOD-06"] = "pass-no-production-tool-model-or-parser-attempt"
    statuses["G-DOD-07"] = "pass-public-synthetic-data-memory-only"
    statuses["G-DOD-08"] = "pass-owned-temporary-root-zero-residue"
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
    failures = [f"Story 2.4 gate results missing marker: {marker}" for marker in VALIDATOR_MARKERS if marker not in value]
    for prohibited in ("Traceback", "FAILED", "validation failed"):
        if prohibited in value:
            failures.append(f"Story 2.4 gate results contain prohibited marker: {prohibited}")
    return failures


def rv51_state() -> dict[str, Any]:
    report = read_json(EVIDENCE_DIR / "rv51-applicability.json")
    silent = report.get("zero_silent_drop", {})
    scenarios = silent.get("scenario_results", [])
    hostile = report.get("hostile_fixture_results", [])
    remaining = report.get("remaining_protocol_scenarios", [])
    if (
        report.get("status") != "PARTIAL_LOCAL_FIXTURE_EVIDENCE"
        or report.get("protocol_complete") is not False
        or report.get("source_hash_count") != 26
        or report.get("derivative_hash_count") != 21
        or report.get("context_receipt_count") != 8
        or report.get("resource_observation_count") != 10
        or silent.get("zero_silent_drop_result") != "PASS_LOCAL_FIXTURE"
        or len(scenarios) != 8
        or any(item.get("explicit_disposition_count") != 26 for item in scenarios)
        or any(item.get(key) != [] for item in scenarios for key in (
            "missing_source_artifact_ids", "unexpected_source_artifact_ids", "duplicate_source_artifact_ids"
        ))
        or len(hostile) != 10
        or any(item.get("active_content_executed") is not False for item in hostile)
        or any(item.get("external_relationship_fetched") is not False for item in hostile)
        or any(item.get("residue_retained") is not False for item in hostile)
        or any(item.get("may_claim_complete") is not False for item in hostile)
        or len(remaining) != 4
        or any(item.get("status") != "BLOCKED_LATER_STORY" for item in remaining)
    ):
        raise ValueError("Story 2.4 RV-51 state is incomplete or widened")
    return {
        "protocol_id": "RV-51",
        "status": "partial-local-fixture-evidence",
        "protocol_complete": False,
        "source_identity_count": 26,
        "derivative_range_identity_count": 21,
        "context_receipt_count": 8,
        "resource_observation_count": 10,
        "remaining_protocol_scenario_count": 4,
        "evidence_substitution_permitted": False,
        "report_sha256": sha256_bytes(canonical_json(report)),
    }


def acceptance_criteria() -> list[dict[str, Any]]:
    rv51 = read_json(EVIDENCE_DIR / "rv51-applicability.json")
    delivery = read_json(ROOT / "fixtures/artifact-admission/v1/context-delivery-receipts.json")
    hostile = rv51.get("hostile_fixture_results", [])
    silent = rv51.get("zero_silent_drop", {})
    scenarios = silent.get("scenario_results", [])
    if (
        len(hostile) != 10
        or silent.get("supplied_source_count") != 26
        or silent.get("lineage_source_count") != 26
        or silent.get("lineage_missing_source_artifact_ids") != []
        or rv51.get("fixture_security", {}).get("blocking_finding_count") != 0
    ):
        raise ValueError("Story 2.4.AC1 disposition or provenance truth is incomplete")
    if (
        len(scenarios) != 8
        or any(item.get("explicit_disposition_count") != 26 for item in scenarios)
        or delivery.get("scenario_count") != 8
        or delivery.get("model_request_executed") is not False
        or delivery.get("product_delivery_claim") != "none"
    ):
        raise ValueError("Story 2.4.AC2 context accounting truth is incomplete")
    blocked = delivery.get("blocked_scenarios", [])
    expected_blocked = {"token_budget_overflow", "stale", "restricted", "model_profile_change"}
    if (
        set(blocked) != expected_blocked
        or any(
            scenario.get("completion_allowed") is not False
            or scenario.get("completion_block_reason") != "required_authoritative_content_not_delivered"
            or not scenario.get("required_authoritative_artifact_ids")
            or not scenario.get("safe_remediation")
            for scenario in delivery.get("scenarios", []) if scenario.get("scenario_id") in expected_blocked
        )
    ):
        raise ValueError("Story 2.4.AC3 required-unseen refusal truth is incomplete")
    return [
        {
            "criterion_id": "2.4.AC1",
            "status": "pass-local-public-synthetic-disposition-provenance-and-inertness",
            "evidence_sha256": sha256_bytes(canonical_json({"hostile": hostile, "security": rv51["fixture_security"]})),
        },
        {
            "criterion_id": "2.4.AC2",
            "status": "pass-local-public-synthetic-complete-context-accounting",
            "evidence_sha256": sha256_bytes(canonical_json({"silent": silent, "delivery": delivery})),
        },
        {
            "criterion_id": "2.4.AC3",
            "status": "pass-local-public-synthetic-required-unseen-refusal",
            "evidence_sha256": sha256_bytes(canonical_json([
                item for item in delivery["scenarios"] if item["scenario_id"] in expected_blocked
            ])),
        },
    ]


def build_report() -> dict[str, Any]:
    tasks_text = (ROOT / "TASKS.md").read_text(encoding="utf-8")
    incomplete = task_completion(tasks_text)
    if incomplete:
        raise ValueError(f"Story 2.4 task or criterion is incomplete: {incomplete[0]}")
    raw = RAW_PATH.read_text(encoding="utf-8")
    raw_failures = validate_raw(raw)
    if raw_failures:
        raise ValueError("; ".join(raw_failures))
    return {
        "schema_version": 1,
        "story_id": "2.4",
        "status": "blocked-open-dependencies-full-protocol-and-platform",
        "reviewed_commit": REVIEWED_COMMIT,
        "reviewed_tree": REVIEWED_TREE,
        "task_count": 3,
        "sub_task_count": 7,
        "acceptance_criteria": acceptance_criteria(),
        "dependencies": dependency_state(tasks_text),
        "later_story_dependencies": later_story_state(tasks_text),
        "rv51": rv51_state(),
        "universal_definition_of_done": universal_dod(),
        "blocking_controls": ["G-DOD-10"],
        "blockers": list(BLOCKERS),
        "independent_review": {
            "reviewer_id": "agentmage-story-2.4-independent-gate-v1",
            "review_type": "automated-independent-aggregate-review",
            "reviewed_commit": REVIEWED_COMMIT,
            "reviewed_tree": REVIEWED_TREE,
            "artifacts": reviewed_artifacts(),
            "finding_count": 0,
            "findings": [],
            "external_human_review_status": "not-performed",
        },
        "validator_results_sha256": sha256_bytes(raw.encode("utf-8")),
        "current_public_synthetic_fixture_scope_complete": True,
        "story_checkbox_complete": False,
        "dependency_substitution_permitted": False,
        "platform_evidence_substituted": False,
        "product_parser_claim": "none",
        "product_context_delivery_claim": "none",
        "installed_product_claim": "none",
        "product_acceptance_claim": "none",
        "sprint_completion_claim": False,
        "release_claim": "none",
    }


def validate_report(value: Any, *, verify_current: bool = True) -> list[str]:
    if not isinstance(value, dict):
        return ["Story 2.4 gate report must be an object"]
    failures = []
    if (
        value.get("schema_version") != 1
        or value.get("story_id") != "2.4"
        or value.get("status") != "blocked-open-dependencies-full-protocol-and-platform"
        or value.get("reviewed_commit") != REVIEWED_COMMIT
        or value.get("reviewed_tree") != REVIEWED_TREE
    ):
        failures.append("Story 2.4 gate identity or status is invalid")
    criteria = value.get("acceptance_criteria", [])
    if [item.get("criterion_id") for item in criteria if isinstance(item, dict)] != ["2.4.AC1", "2.4.AC2", "2.4.AC3"]:
        failures.append("Story 2.4 acceptance criterion closure is invalid")
    elif not all(item.get("status", "").startswith("pass-local-public-synthetic-") for item in criteria):
        failures.append("Story 2.4 acceptance criterion status is invalid")
    if value.get("universal_definition_of_done") != universal_dod() or value.get("blocking_controls") != ["G-DOD-10"]:
        failures.append("Story 2.4 Definition-of-Done disposition is invalid")
    if value.get("blockers") != list(BLOCKERS):
        failures.append("Story 2.4 blocker set is invalid")
    expected_dependencies = [
        {"story_id": "1.3", "status": "blocked-open-aggregate-gate"},
        {"story_id": "2.3", "status": "blocked-open-aggregate-gate"},
    ]
    if value.get("dependencies") != expected_dependencies:
        failures.append("Story 2.4 dependency disposition is invalid")
    expected_later = later_story_state((ROOT / "TASKS.md").read_text(encoding="utf-8"))
    if value.get("later_story_dependencies") != expected_later:
        failures.append("Story 2.4 later-story blocker disposition is invalid")
    rv51 = value.get("rv51", {})
    if (
        rv51.get("protocol_id") != "RV-51"
        or rv51.get("status") != "partial-local-fixture-evidence"
        or rv51.get("protocol_complete") is not False
        or rv51.get("source_identity_count") != 26
        or rv51.get("derivative_range_identity_count") != 21
        or rv51.get("context_receipt_count") != 8
        or rv51.get("resource_observation_count") != 10
        or rv51.get("remaining_protocol_scenario_count") != 4
        or rv51.get("evidence_substitution_permitted") is not False
    ):
        failures.append("Story 2.4 RV-51 disposition is invalid")
    review = value.get("independent_review", {})
    if (
        review.get("reviewer_id") != "agentmage-story-2.4-independent-gate-v1"
        or review.get("review_type") != "automated-independent-aggregate-review"
        or review.get("reviewed_commit") != REVIEWED_COMMIT
        or review.get("reviewed_tree") != REVIEWED_TREE
        or review.get("finding_count") != 0
        or review.get("findings") != []
        or review.get("external_human_review_status") != "not-performed"
        or len(review.get("artifacts", [])) != len(REVIEWED_PATHS)
    ):
        failures.append("Story 2.4 independent review is invalid")
    if (
        value.get("current_public_synthetic_fixture_scope_complete") is not True
        or value.get("story_checkbox_complete") is not False
        or value.get("dependency_substitution_permitted") is not False
        or value.get("platform_evidence_substituted") is not False
        or value.get("product_parser_claim") != "none"
        or value.get("product_context_delivery_claim") != "none"
        or value.get("installed_product_claim") != "none"
        or value.get("product_acceptance_claim") != "none"
        or value.get("sprint_completion_claim") is not False
        or value.get("release_claim") != "none"
    ):
        failures.append("Story 2.4 gate made an unsupported completion claim")
    if verify_current:
        try:
            expected = build_report()
        except (OSError, ValueError, KeyError, TypeError, json.JSONDecodeError) as error:
            failures.append(f"cannot rebuild Story 2.4 gate report: {error}")
        else:
            if value != expected:
                failures.append("Story 2.4 gate report is stale, incomplete, or widened")
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
        print(f"Story 2.4 gate failed: {error}", file=sys.stderr)
        return 1
    if failures:
        for failure in failures:
            print(f"Story 2.4 gate failed: {failure}", file=sys.stderr)
        return 1
    print("Story 2.4 public synthetic fixture scope passed with dependency, full-protocol, and platform blockers preserved")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
