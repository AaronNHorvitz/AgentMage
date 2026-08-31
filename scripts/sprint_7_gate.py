#!/usr/bin/env python3
"""Aggregate Sprint 7 without promoting shared contracts to native platform support."""

from __future__ import annotations

import argparse
import hashlib
import json
import os
import subprocess
import sys
import tempfile
from pathlib import Path
from typing import Any


ROOT = Path(__file__).resolve().parents[1]
if str(ROOT) not in sys.path:
    sys.path.insert(0, str(ROOT))

from scripts.story_7_1_gate import BLOCKERS, G_DOD_IDS, check_report as check_story


REPORT_PATH = ROOT / "artifacts/sprints/sprint-7/sprint-gate-report.json"
REVIEWED_COMMIT = "e69d5ae225d4a73f3467dcd7859110bd7bc23333"
REVIEWED_TREE = "5ff918c349d2524d857887123fd45b03c8fca32f"
REVIEWED_PATHS = (
    "artifacts/sprints/sprint-7/story-7.1/story-gate-report.json",
    "scripts/story_7_1_gate.py",
    "artifacts/sprints/sprint-7/story-7.1/platform-contract-report.json",
    "artifacts/sprints/sprint-7/story-7.1/security-evidence-map.json",
    "artifacts/sprints/sprint-2/story-2.1/platform-result-recorder-report.json",
    "artifacts/sprints/sprint-7/story-7.1/macos-release-manifest-contract.json",
)


def canonical_json(value: Any) -> bytes:
    return (json.dumps(value, indent=2, sort_keys=True) + "\n").encode()


def read_json(path: Path) -> Any:
    return json.loads(path.read_text(encoding="utf-8"))


def write_atomic(path: Path, content: bytes) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    descriptor, name = tempfile.mkstemp(prefix=".agentmage-sprint-7-gate-", dir=path.parent)
    temporary = Path(name)
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


def git_output(*args: str, binary: bool = False) -> bytes | str:
    result = subprocess.run(
        ["git", *args], cwd=ROOT, stdin=subprocess.DEVNULL, stdout=subprocess.PIPE,
        stderr=subprocess.PIPE, check=False, timeout=15,
    )
    if result.returncode != 0:
        raise ValueError(f"git Sprint 7 review operation failed: {' '.join(args)}")
    return result.stdout if binary else result.stdout.decode().strip()


def reviewed_artifacts() -> list[dict[str, Any]]:
    if git_output("rev-parse", REVIEWED_COMMIT) != REVIEWED_COMMIT:
        raise ValueError("Sprint 7 reviewed commit is unavailable")
    if git_output("show", "-s", "--format=%T", REVIEWED_COMMIT) != REVIEWED_TREE:
        raise ValueError("Sprint 7 reviewed tree changed")
    records = []
    for path in REVIEWED_PATHS:
        committed = git_output("show", f"{REVIEWED_COMMIT}:{path}", binary=True)
        if (ROOT / path).read_bytes() != committed:
            raise ValueError(f"reviewed Sprint 7 artifact changed: {path}")
        records.append({"path": path, "sha256": hashlib.sha256(committed).hexdigest()})
    return records


def checklist_failures(text: str) -> list[str]:
    required = (
        "### [ ] Sprint 7 - Platform Adapter Contract and Release Manifests",
        "#### [ ] Story 7.1 - Platform Adapter Contract and Release Manifests",
        "- [ ] **Sprint AC 7.AC1:**", "- [ ] **Sprint AC 7.AC2:**",
        "- [x] **Sprint AC 7.AC3:**", "- [ ] **Sprint AC 7.AC4:**",
        "- [ ] **Sprint AC 7.AC5:**",
    )
    return [marker for marker in required if marker not in text]


def build_report() -> dict[str, Any]:
    failures = check_story()
    if failures:
        raise ValueError("; ".join(failures))
    checklist = checklist_failures((ROOT / "TASKS.md").read_text(encoding="utf-8"))
    if checklist:
        raise ValueError(f"Sprint 7 checklist state changed: {checklist[0]}")
    story = read_json(ROOT / "artifacts/sprints/sprint-7/story-7.1/story-gate-report.json")
    platform = read_json(ROOT / "artifacts/sprints/sprint-7/story-7.1/platform-contract-report.json")
    recorder = read_json(ROOT / "artifacts/sprints/sprint-2/story-2.1/platform-result-recorder-report.json")
    macos_manifest = read_json(ROOT / "artifacts/sprints/sprint-7/story-7.1/macos-release-manifest-contract.json")
    return {
        "schema_version": 1,
        "sprint_id": 7,
        "status": "blocked-native-platforms-and-external-review",
        "reviewed_commit": REVIEWED_COMMIT,
        "reviewed_tree": REVIEWED_TREE,
        "acceptance_criteria": [
            {
                "criterion_id": "7.AC1",
                "status": "blocked-native-platforms",
                "available_reference_contract_result_count": 2,
                "required_reference_platform_count": 3,
                "deterministic_fake_result_count": 1,
                "frozen_macos_manifest_contract_field_count": macos_manifest["field_closure"]["top_level_field_count"],
            },
            {
                "criterion_id": "7.AC2",
                "status": "partial-shared-failure-matrix-blocked-native-platforms",
                "unavailable_or_invalid_capability_case_count": platform["api"]["required_capability_count"] * 2,
                "native_primitive_corruption_case_count": 0,
            },
            {
                "criterion_id": "7.AC3",
                "status": "pass-current-shared-boundary",
                "required_capability_count": platform["api"]["required_capability_count"],
                "operating_system_branches_in_kernel_selector": platform["api"]["operating_system_branches_in_kernel_selector"],
            },
            {
                "criterion_id": "7.AC4",
                "status": "partial-non-macos-recorder-blocked-macos",
                "synthetic_environment_record_count": recorder["synthetic_record_set"]["record_count"],
                "forbidden_environment_field_count": len(recorder["allowlist"]["forbidden_environment_fields"]),
                "ambient_environment_values_recorded": recorder["ambient_environment_values_recorded"],
                "macos_execution_status": recorder["macos_execution_status"],
            },
            {
                "criterion_id": "7.AC5",
                "status": "blocked-native-platforms-and-packaging",
                "available_contract_scope_passed": True,
                "release_package_acceptance_result_count": 0,
                "macos_platform_acceptance_result_count": 0,
            },
        ],
        "story_gates": [{
            "story_id": "7.1", "status": story["status"],
            "current_shared_linux_contract_scope_complete": story["current_shared_linux_contract_scope_complete"],
            "story_checkbox_complete": story["story_checkbox_complete"],
            "blockers": story["blockers"],
        }],
        "universal_definition_of_done": {
            "control_ids": list(G_DOD_IDS),
            "blocking_controls": ["G-DOD-10", "G-DOD-12"],
            "native_or_macos_evidence_substitution": "prohibited",
        },
        "summary": {
            "acceptance_criteria_passed": 1,
            "acceptance_criteria_partial": 2,
            "acceptance_criteria_blocked": 2,
            "current_shared_linux_contract_scope_complete": True,
            "sprint_checkbox_complete": False,
            "blocking_story_ids": ["7.1"],
            "blockers": list(BLOCKERS),
        },
        "automated_aggregate_review": {
            "reviewer_id": "agentmage-sprint-7-automated-aggregate-v1",
            "reviewed_commit": REVIEWED_COMMIT,
            "reviewed_tree": REVIEWED_TREE,
            "reviewed_artifacts": reviewed_artifacts(),
            "finding_count": 0,
            "external_human_review_status": "not-performed",
        },
        "macos_evidence_substituted": False,
        "native_mechanism_evidence_substituted": False,
        "installed_product_claim": "none",
        "product_acceptance_claim": "none",
        "release_claim": "none",
    }


def validate_report(value: Any, *, verify_current: bool = True) -> list[str]:
    if not isinstance(value, dict):
        return ["Sprint 7 gate report must be an object"]
    failures = []
    if (
        value.get("schema_version") != 1 or value.get("sprint_id") != 7
        or value.get("status") != "blocked-native-platforms-and-external-review"
        or value.get("reviewed_commit") != REVIEWED_COMMIT
        or value.get("reviewed_tree") != REVIEWED_TREE
    ):
        failures.append("Sprint 7 gate identity is invalid")
    criteria = value.get("acceptance_criteria", [])
    if [item.get("criterion_id") for item in criteria if isinstance(item, dict)] != [f"7.AC{i}" for i in range(1, 6)]:
        failures.append("Sprint 7 criterion closure is invalid")
    elif (
        criteria[0].get("available_reference_contract_result_count") != 2
        or criteria[0].get("required_reference_platform_count") != 3
        or criteria[0].get("deterministic_fake_result_count") != 1
        or criteria[0].get("frozen_macos_manifest_contract_field_count") != 16
        or criteria[1].get("unavailable_or_invalid_capability_case_count") != 20
        or criteria[1].get("native_primitive_corruption_case_count") != 0
        or criteria[2].get("required_capability_count") != 10
        or criteria[2].get("operating_system_branches_in_kernel_selector") != 0
        or criteria[3].get("synthetic_environment_record_count") != 2
        or criteria[3].get("forbidden_environment_field_count") != 13
        or criteria[3].get("ambient_environment_values_recorded") is not False
        or criteria[3].get("macos_execution_status") != "blocked-macos"
        or criteria[4].get("release_package_acceptance_result_count") != 0
        or criteria[4].get("macos_platform_acceptance_result_count") != 0
    ):
        failures.append("Sprint 7 criterion evidence is invalid")
    if value.get("summary") != {
        "acceptance_criteria_passed": 1,
        "acceptance_criteria_partial": 2,
        "acceptance_criteria_blocked": 2,
        "current_shared_linux_contract_scope_complete": True,
        "sprint_checkbox_complete": False,
        "blocking_story_ids": ["7.1"],
        "blockers": list(BLOCKERS),
    }:
        failures.append("Sprint 7 summary or blockers are invalid")
    review = value.get("automated_aggregate_review", {})
    if (
        review.get("reviewer_id") != "agentmage-sprint-7-automated-aggregate-v1"
        or len(review.get("reviewed_artifacts", [])) != len(REVIEWED_PATHS)
        or review.get("finding_count") != 0
        or review.get("external_human_review_status") != "not-performed"
    ):
        failures.append("Sprint 7 aggregate review is invalid")
    if (
        value.get("macos_evidence_substituted") is not False
        or value.get("native_mechanism_evidence_substituted") is not False
        or value.get("installed_product_claim") != "none"
        or value.get("product_acceptance_claim") != "none"
        or value.get("release_claim") != "none"
    ):
        failures.append("Sprint 7 gate made an unsupported claim")
    if verify_current:
        try:
            expected = build_report()
        except (OSError, ValueError, KeyError, TypeError, json.JSONDecodeError) as error:
            failures.append(f"cannot rebuild Sprint 7 gate report: {error}")
        else:
            if value != expected:
                failures.append("Sprint 7 gate report is stale or widened")
    return failures


def check_report() -> list[str]:
    try:
        return validate_report(read_json(REPORT_PATH))
    except (OSError, json.JSONDecodeError) as error:
        return [f"cannot read Sprint 7 gate report: {error}"]


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--write", action="store_true")
    args = parser.parse_args()
    try:
        if args.write:
            write_atomic(REPORT_PATH, canonical_json(build_report()))
        failures = check_report()
    except (OSError, ValueError, KeyError, TypeError, json.JSONDecodeError) as error:
        print(f"Sprint 7 gate failed: {error}", file=sys.stderr)
        return 1
    if failures:
        for failure in failures:
            print(f"Sprint 7 gate failed: {failure}", file=sys.stderr)
        return 1
    print("Sprint 7 current shared/Linux contract scope passed with blockers preserved")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
