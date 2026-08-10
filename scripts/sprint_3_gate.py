#!/usr/bin/env python3
"""Evaluate Sprint 3 acceptance without substituting for macOS evidence."""

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

from scripts.configuration_authority_mutation_evidence import (
    check_artifact as check_authority_mutations,
)
from scripts.configuration_migration_recovery_evidence import (
    check_artifact as check_migration,
)
from scripts.configuration_result_evidence import check_artifact as check_results
from scripts.configuration_schema_failure_evidence import (
    check_artifact as check_schema_failures,
)
from scripts.configuration_startup_evidence import check_artifact as check_startup
from scripts.story_3_1_gate import check_report as check_story_3_1_gate
from scripts.story_3_2_gate import check_report as check_story_3_2_gate
from scripts.vulnerability_report_evidence import check_artifact as check_diagnostics


REPORT_PATH = ROOT / "artifacts/sprints/sprint-3/sprint-gate-report.json"
REVIEWED_COMMIT = "aac2a33e549c18cf6784e0552a2ebfaa13f2c66e"
REVIEWED_TREE = "fbe3ff4f87eaa0c0bea6a6479c2cc010b8d8d929"
REVIEWED_PATHS = (
    "artifacts/sprints/sprint-3/story-3.1/story-gate-report.json",
    "artifacts/sprints/sprint-3/story-3.2/story-gate-report.json",
    "scripts/story_3_1_gate.py",
    "scripts/story_3_2_gate.py",
    "tests/test_story_3_1_gate.py",
    "tests/test_story_3_2_gate.py",
    "artifacts/sprints/sprint-3/story-3.1/configuration-schema-failure-report.json",
    "artifacts/sprints/sprint-3/story-3.1/configuration-authority-mutation-report.json",
    "artifacts/sprints/sprint-3/story-3.1/configuration-startup-report.json",
    "artifacts/sprints/sprint-3/story-3.1/configuration-migration-recovery-report.json",
    "artifacts/sprints/sprint-3/story-3.1/configuration-result-report.json",
    "artifacts/sprints/sprint-3/story-3.2/vulnerability-report-evidence-report.json",
    "artifacts/sprints/sprint-3/story-3.2/vulnerability-workflow-report.json",
    "artifacts/sprints/sprint-3/story-3.2/security-evidence-map.json",
)
G_DOD_IDS = tuple(f"G-DOD-{index:02d}" for index in range(1, 14))


def canonical_json(value: Any) -> bytes:
    return (json.dumps(value, indent=2, sort_keys=True) + "\n").encode("utf-8")


def sha256_bytes(value: bytes) -> str:
    return hashlib.sha256(value).hexdigest()


def read_json(path: Path) -> Any:
    return json.loads(path.read_text(encoding="utf-8"))


def write_atomic(path: Path, content: bytes) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    descriptor, temporary_name = tempfile.mkstemp(
        prefix=".agentmage-sprint-3-gate-", dir=path.parent
    )
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


def git_output(*arguments: str, root: Path = ROOT, binary: bool = False) -> bytes | str:
    result = subprocess.run(
        ["git", *arguments],
        cwd=root,
        stdin=subprocess.DEVNULL,
        stdout=subprocess.PIPE,
        stderr=subprocess.PIPE,
        check=False,
        timeout=10,
    )
    if result.returncode != 0:
        raise ValueError(f"git sprint review operation failed: {' '.join(arguments)}")
    return result.stdout if binary else result.stdout.decode("utf-8").strip()


def reviewed_artifacts(root: Path = ROOT) -> list[dict[str, str]]:
    commit = git_output("rev-parse", REVIEWED_COMMIT, root=root)
    tree = git_output("show", "-s", "--format=%T", REVIEWED_COMMIT, root=root)
    if commit != REVIEWED_COMMIT or tree != REVIEWED_TREE:
        raise ValueError("Sprint 3 review identity is unavailable or changed")
    records = []
    for path in REVIEWED_PATHS:
        committed = git_output("show", f"{REVIEWED_COMMIT}:{path}", root=root, binary=True)
        current = root / path
        if not current.is_file() or current.read_bytes() != committed:
            raise ValueError(f"reviewed Sprint 3 artifact changed after review: {path}")
        records.append({"path": path, "sha256": sha256_bytes(committed)})
    return records


def story_gate_summary(story_id: str, value: dict[str, Any]) -> dict[str, Any]:
    dod = value["universal_definition_of_done"]
    return {
        "story_id": story_id,
        "status": value["status"],
        "acceptance_criteria_passed": len(value["acceptance_criteria"]),
        "acceptance_criteria_failed": 0,
        "shared_linux_story_work_complete": value[
            "shared_linux_story_work_complete"
        ],
        "story_checkbox_complete": False,
        "only_blocker": value["only_blocker"],
        "dod_control_ids": [item["control_id"] for item in dod],
        "dod_blocking_controls": [
            item["control_id"] for item in dod if item["status"] == "blocked-macos"
        ],
    }


def validate_inputs(root: Path = ROOT) -> list[str]:
    validators = (
        ("schema-failures", check_schema_failures),
        ("authority-mutations", check_authority_mutations),
        ("startup", check_startup),
        ("migration", check_migration),
        ("configuration-results", check_results),
        ("bounded-diagnostics", check_diagnostics),
        ("story-3.1-gate", check_story_3_1_gate),
        ("story-3.2-gate", check_story_3_2_gate),
    )
    failures = []
    for name, validator in validators:
        failures.extend(f"{name}: {failure}" for failure in validator(root))
    return failures


def build_report(root: Path = ROOT) -> dict[str, Any]:
    failures = validate_inputs(root)
    if failures:
        raise ValueError("; ".join(failures))
    schema = read_json(
        root
        / "artifacts/sprints/sprint-3/story-3.1/configuration-schema-failure-report.json"
    )
    authority = read_json(
        root
        / "artifacts/sprints/sprint-3/story-3.1/configuration-authority-mutation-report.json"
    )
    startup = read_json(
        root / "artifacts/sprints/sprint-3/story-3.1/configuration-startup-report.json"
    )
    migration = read_json(
        root
        / "artifacts/sprints/sprint-3/story-3.1/configuration-migration-recovery-report.json"
    )
    results = read_json(
        root / "artifacts/sprints/sprint-3/story-3.1/configuration-result-report.json"
    )
    diagnostics = read_json(
        root
        / "artifacts/sprints/sprint-3/story-3.2/vulnerability-report-evidence-report.json"
    )
    story_3_1 = read_json(
        root / "artifacts/sprints/sprint-3/story-3.1/story-gate-report.json"
    )
    story_3_2 = read_json(
        root / "artifacts/sprints/sprint-3/story-3.2/story-gate-report.json"
    )
    stories = [
        story_gate_summary("3.1", story_3_1),
        story_gate_summary("3.2", story_3_2),
    ]
    return {
        "schema_version": 1,
        "sprint_id": 3,
        "status": "blocked-macos",
        "reviewed_commit": REVIEWED_COMMIT,
        "reviewed_tree": REVIEWED_TREE,
        "acceptance_criteria": [
            {
                "criterion_id": "3.AC1",
                "status": "pass-shared-linux",
                "acceptance_test_id": "AT-CFG-001",
                "required_mutation_rejections": 100,
                "executed_mutation_attempts": authority["summary"][
                    "cross_channel_mutation_attempt_count"
                ],
                "accepted_broadening_count": authority["summary"][
                    "accepted_broadening_count"
                ],
                "stable_schema_diagnostic_count": schema["summary"][
                    "stable_diagnostic_case_count"
                ],
            },
            {
                "criterion_id": "3.AC2",
                "status": "pass-shared-linux",
                "schema_failure_case_count": schema["summary"]["matrix_case_count"],
                "schema_rejected_case_count": schema["summary"]["rejected_case_count"],
                "partial_startup_case_count": schema["summary"][
                    "partial_startup_case_count"
                ],
                "authority_mutation_attempt_count": authority["summary"][
                    "cross_channel_mutation_attempt_count"
                ],
                "accepted_broadening_count": authority["summary"][
                    "accepted_broadening_count"
                ],
            },
            {
                "criterion_id": "3.AC3",
                "status": "pass-shared-linux",
                "profile_count": startup["summary"]["profile_count"],
                "future_disabled_profile_count": startup["summary"][
                    "future_disabled_profile_count"
                ],
                "registered_capability_count": startup["summary"][
                    "registered_capability_count"
                ],
                "early_product_registration_count": startup["summary"][
                    "early_product_registration_count"
                ],
            },
            {
                "criterion_id": "3.AC4",
                "status": "pass-shared-linux",
                "durable_transition_count": migration["summary"][
                    "durable_transition_count"
                ],
                "injected_interruption_count": migration["summary"][
                    "injected_interruption_count"
                ],
                "invalid_selected_state_count": migration["summary"][
                    "invalid_selected_state_count"
                ],
                "repeatable_rollback": migration["summary"]["repeatable_rollback"],
            },
            {
                "criterion_id": "3.AC5",
                "status": "pass-shared-linux",
                "configuration_identity_fields": results[
                    "configuration_identity_fields"
                ],
                "raw_configuration_persisted": results[
                    "raw_configuration_persisted"
                ],
                "private_paths_persisted": results["private_paths_persisted"],
                "diagnostic_scan_finding_count": diagnostics["summary"][
                    "scan_finding_count"
                ],
                "diagnostic_private_user_data_record_count": diagnostics["summary"][
                    "private_user_data_record_count"
                ],
            },
        ],
        "story_gates": stories,
        "universal_definition_of_done": {
            "control_ids": list(G_DOD_IDS),
            "story_count": 2,
            "all_non_platform_controls_pass_or_not_applicable": True,
            "blocking_controls": ["G-DOD-10"],
            "macos_evidence_substitution": "prohibited",
        },
        "summary": {
            "acceptance_criteria_passed": 5,
            "acceptance_criteria_failed": 0,
            "story_gate_count": 2,
            "shared_linux_sprint_work_complete": True,
            "sprint_checkbox_complete": False,
            "blocking_story_count": 2,
            "blocking_story_ids": ["3.1", "3.2"],
            "blocking_control_count": 1,
            "blocking_controls": ["G-DOD-10"],
        },
        "independent_review": {
            "reviewer_id": "agentmage-sprint-3-independent-gate-v1",
            "review_type": "automated-independent-aggregate-review",
            "reviewed_commit": REVIEWED_COMMIT,
            "reviewed_tree": REVIEWED_TREE,
            "reviewed_artifacts": reviewed_artifacts(root),
            "finding_count": 0,
            "findings": [],
            "disposition": "pass-shared-linux-sprint-blocked-macos",
            "external_human_review_claim": "none",
        },
        "macos": {
            "status": "blocked-macos",
            "execution_performed": False,
            "evidence_substitution": "prohibited",
            "support_claim": "none",
        },
        "product_startup_claim": "none",
        "product_acceptance_claim": "none",
        "release_claim": "none",
    }


def validate_report(value: Any, root: Path = ROOT) -> list[str]:
    if not isinstance(value, dict):
        return ["Sprint 3 gate report must be an object"]
    failures = []
    if (
        value.get("schema_version") != 1
        or value.get("sprint_id") != 3
        or value.get("status") != "blocked-macos"
        or value.get("reviewed_commit") != REVIEWED_COMMIT
        or value.get("reviewed_tree") != REVIEWED_TREE
    ):
        failures.append("Sprint 3 gate identity or review boundary is invalid")
    criteria = value.get("acceptance_criteria", [])
    if [item.get("criterion_id") for item in criteria] != [
        "3.AC1",
        "3.AC2",
        "3.AC3",
        "3.AC4",
        "3.AC5",
    ] or any(item.get("status") != "pass-shared-linux" for item in criteria):
        failures.append("Sprint 3 acceptance criteria did not close exactly")
    elif (
        criteria[0].get("executed_mutation_attempts", 0)
        < criteria[0].get("required_mutation_rejections", 100)
        or criteria[0].get("accepted_broadening_count") != 0
        or criteria[1].get("partial_startup_case_count") != 0
        or criteria[1].get("accepted_broadening_count") != 0
        or criteria[2].get("registered_capability_count") != 0
        or criteria[2].get("early_product_registration_count") != 0
        or criteria[3].get("invalid_selected_state_count") != 0
        or criteria[4].get("configuration_identity_fields") != [
            "profile_id",
            "sha256",
        ]
        or criteria[4].get("raw_configuration_persisted") is not False
        or criteria[4].get("private_paths_persisted") is not False
        or criteria[4].get("diagnostic_scan_finding_count") != 0
        or criteria[4].get("diagnostic_private_user_data_record_count") != 0
    ):
        failures.append("Sprint 3 acceptance evidence is invalid")
    stories = value.get("story_gates", [])
    if [item.get("story_id") for item in stories] != ["3.1", "3.2"] or any(
        item.get("status") != "blocked-macos"
        or item.get("shared_linux_story_work_complete") is not True
        or item.get("story_checkbox_complete") is not False
        or item.get("only_blocker") != "macos-execution-evidence-unavailable"
        or item.get("dod_control_ids") != list(G_DOD_IDS)
        or item.get("dod_blocking_controls") != ["G-DOD-10"]
        for item in stories
    ):
        failures.append("Sprint 3 story-gate aggregation is invalid")
    if value.get("universal_definition_of_done") != {
        "control_ids": list(G_DOD_IDS),
        "story_count": 2,
        "all_non_platform_controls_pass_or_not_applicable": True,
        "blocking_controls": ["G-DOD-10"],
        "macos_evidence_substitution": "prohibited",
    }:
        failures.append("Sprint 3 Definition-of-Done aggregation is invalid")
    if value.get("summary") != {
        "acceptance_criteria_passed": 5,
        "acceptance_criteria_failed": 0,
        "story_gate_count": 2,
        "shared_linux_sprint_work_complete": True,
        "sprint_checkbox_complete": False,
        "blocking_story_count": 2,
        "blocking_story_ids": ["3.1", "3.2"],
        "blocking_control_count": 1,
        "blocking_controls": ["G-DOD-10"],
    }:
        failures.append("Sprint 3 gate summary overclaimed or omitted a blocker")
    review = value.get("independent_review", {})
    if (
        review.get("reviewed_commit") != REVIEWED_COMMIT
        or review.get("reviewed_tree") != REVIEWED_TREE
        or review.get("finding_count") != 0
        or review.get("findings") != []
        or review.get("external_human_review_claim") != "none"
    ):
        failures.append("Sprint 3 independent review record is invalid")
    if value.get("macos") != {
        "status": "blocked-macos",
        "execution_performed": False,
        "evidence_substitution": "prohibited",
        "support_claim": "none",
    }:
        failures.append("Sprint 3 gate made an invalid macOS claim")
    if (
        value.get("product_startup_claim") != "none"
        or value.get("product_acceptance_claim") != "none"
        or value.get("release_claim") != "none"
    ):
        failures.append("Sprint 3 gate made a product or release claim")
    try:
        expected = build_report(root)
    except (OSError, ValueError, KeyError, TypeError, subprocess.TimeoutExpired) as error:
        failures.append(f"cannot rebuild Sprint 3 gate report: {error}")
    else:
        if value != expected:
            failures.append("Sprint 3 gate report is stale or non-deterministic")
    return failures


def check_report(root: Path = ROOT) -> list[str]:
    try:
        report = read_json(root / REPORT_PATH.relative_to(ROOT))
    except (OSError, json.JSONDecodeError) as error:
        return [f"cannot read Sprint 3 gate report: {error}"]
    return validate_report(report, root)


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--write", action="store_true")
    args = parser.parse_args()
    try:
        if args.write:
            write_atomic(REPORT_PATH, canonical_json(build_report()))
        failures = check_report()
    except (OSError, ValueError, KeyError, TypeError, subprocess.TimeoutExpired) as error:
        print(f"Sprint 3 gate failed: {error}", file=sys.stderr)
        return 1
    if failures:
        for failure in failures:
            print(f"Sprint 3 gate failed: {failure}", file=sys.stderr)
        return 1
    print("Sprint 3 shared/Linux acceptance passed with macOS blocker preserved")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
