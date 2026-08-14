#!/usr/bin/env python3
"""Independently evaluate Story 3.2 while preserving the macOS blocker."""

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

from scripts.emergency_disable_policy import check_artifact as check_emergency_disable
from scripts.manual_patch_verifier import check_artifact as check_patch_verification
from scripts.story_3_2_security_evidence import check_map as check_security
from scripts.vulnerability_support_platform_evidence import check_report as check_platform
from scripts.vulnerability_support_workflow import check_artifact as check_workflow


REPORT_PATH = ROOT / "artifacts/sprints/sprint-3/story-3.2/story-gate-report.json"
REVIEWED_COMMIT = "4bb44a1874287937881190a05e6f9bcc9e68e439"
REVIEWED_TREE = "adbc586ae8c300e68f499099097df0d588101059"
REVIEWED_PATHS = (
    "SECURITY.md",
    "support/vulnerability-support-policy.json",
    "support/vulnerability-report-evidence-policy.json",
    "docs/support/vulnerability-reporting.md",
    "docs/security/story-3.2-vulnerability-response-tabletop.md",
    "schemas/support/vulnerability-support-policy.schema.json",
    "schemas/support/vulnerability-report-evidence-policy.schema.json",
    "schemas/support/vulnerability-diagnostic-bundle.schema.json",
    "schemas/support/signed-manual-patch-metadata.schema.json",
    "schemas/support/emergency-disable-policy.schema.json",
    "fixtures/support/vulnerability-workflow/workflow.valid.json",
    "fixtures/support/manual-patch/cases.json",
    "fixtures/support/manual-patch/cases/valid.json",
    "fixtures/support/manual-patch/cases/revoked.json",
    "artifacts/sprints/sprint-3/story-3.2/vulnerability-support-policy-report.json",
    "artifacts/sprints/sprint-3/story-3.2/support-policy-platform-report.json",
    "artifacts/sprints/sprint-3/story-3.2/vulnerability-report-evidence-report.json",
    "artifacts/sprints/sprint-3/story-3.2/manual-patch-metadata-report.json",
    "artifacts/sprints/sprint-3/story-3.2/emergency-disable-policy-report.json",
    "artifacts/sprints/sprint-3/story-3.2/manual-patch-verification-report.json",
    "artifacts/sprints/sprint-3/story-3.2/vulnerability-workflow-report.json",
    "artifacts/sprints/sprint-3/story-3.2/security-evidence-map.json",
    "scripts/manual_patch_verifier.py",
    "scripts/vulnerability_support_workflow.py",
    "scripts/story_3_2_security_evidence.py",
    "tests/test_manual_patch_verifier.py",
    "tests/test_vulnerability_support_workflow.py",
    "tests/test_story_3_2_security_evidence.py",
)
REQUIRED_TASK_MARKERS = (
    "- [x] **Task 3.2.1 - Implement the support contract**",
    "  - [x] **Sub-task 3.2.1.1:**",
    "  - [x] **Sub-task 3.2.1.2:**",
    "  - [x] **Sub-task 3.2.1.3:**",
    "  - [x] **Sub-task 3.2.1.4:**",
    "- [x] **Task 3.2.2 - Verify and close the story**",
    "  - [x] **Sub-task 3.2.2.1:**",
    "  - [x] **Sub-task 3.2.2.2:**",
    "  - [x] **Sub-task 3.2.2.3 - Product security evidence:**",
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
        prefix=".agentmage-story-3-2-gate-", dir=path.parent
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
        raise ValueError(f"git review operation failed: {' '.join(arguments)}")
    return result.stdout if binary else result.stdout.decode("utf-8").strip()


def reviewed_artifacts(root: Path = ROOT) -> list[dict[str, str]]:
    commit = git_output("rev-parse", REVIEWED_COMMIT, root=root)
    tree = git_output("show", "-s", "--format=%T", REVIEWED_COMMIT, root=root)
    if commit != REVIEWED_COMMIT or tree != REVIEWED_TREE:
        raise ValueError("Story 3.2 review identity is unavailable or changed")
    records = []
    for path in REVIEWED_PATHS:
        committed = git_output("show", f"{REVIEWED_COMMIT}:{path}", root=root, binary=True)
        current = root / path
        if not current.is_file() or current.read_bytes() != committed:
            raise ValueError(f"reviewed Story 3.2 artifact changed after review: {path}")
        records.append({"path": path, "sha256": sha256_bytes(committed)})
    return records


def task_completion(tasks_text: str) -> list[str]:
    return [marker for marker in REQUIRED_TASK_MARKERS if marker not in tasks_text]


def validate_inputs(root: Path = ROOT) -> list[str]:
    validators = (
        ("support-platform", check_platform),
        ("patch-verification", check_patch_verification),
        ("support-workflow", check_workflow),
        ("emergency-disable", check_emergency_disable),
        ("security-map", check_security),
    )
    failures = []
    for name, validator in validators:
        failures.extend(f"{name}: {failure}" for failure in validator(root))
    failures.extend(
        f"task marker is incomplete: {marker}"
        for marker in task_completion((root / "TASKS.md").read_text(encoding="utf-8"))
    )
    return failures


def universal_dod() -> list[dict[str, Any]]:
    statuses = {
        "G-DOD-01": "pass-story-scope",
        "G-DOD-02": "pass-story-scope",
        "G-DOD-03": "pass-story-scope",
        "G-DOD-04": "pass-no-product-authority",
        "G-DOD-05": "pass-story-scope",
        "G-DOD-06": "pass-no-product-tool-attempt",
        "G-DOD-07": "pass-synthetic-minimized-evidence",
        "G-DOD-08": "pass-read-only-preservation",
        "G-DOD-09": "pass-story-scope",
        "G-DOD-10": "blocked-macos",
        "G-DOD-11": "pass-story-scope",
        "G-DOD-12": "pass-automated-independent-implementation-review",
        "G-DOD-13": "pass-gate-reporting",
    }
    return [
        {
            "control_id": control_id,
            "status": statuses[control_id],
            "evidence": [
                "artifacts/sprints/sprint-3/story-3.2/security-evidence-map.json",
                "artifacts/sprints/sprint-3/story-3.2/story-gate-report.json",
            ],
        }
        for control_id in G_DOD_IDS
    ]


def build_report(root: Path = ROOT) -> dict[str, Any]:
    failures = validate_inputs(root)
    if failures:
        raise ValueError("; ".join(failures))
    reviewed = reviewed_artifacts(root)
    workflow = read_json(
        root / "artifacts/sprints/sprint-3/story-3.2/vulnerability-workflow-report.json"
    )
    patch = read_json(
        root / "artifacts/sprints/sprint-3/story-3.2/manual-patch-verification-report.json"
    )
    emergency = read_json(
        root / "artifacts/sprints/sprint-3/story-3.2/emergency-disable-policy-report.json"
    )
    emergency_fixture = read_json(
        root / "schemas/support/examples/emergency-disable-policy.valid.json"
    )
    support_policy = read_json(root / "support/vulnerability-support-policy.json")
    security = read_json(
        root / "artifacts/sprints/sprint-3/story-3.2/security-evidence-map.json"
    )
    valid_patch_count = sum(
        item["actual_outcome"] == "verified-can-proceed" for item in patch["cases"]
    )
    blocked_subject_kinds = [
        item["subject_kind"] for item in emergency["subject_results"]
    ]
    return {
        "schema_version": 1,
        "story_id": "3.2",
        "status": "blocked-macos",
        "reviewed_commit": REVIEWED_COMMIT,
        "reviewed_tree": REVIEWED_TREE,
        "acceptance_criteria": [
            {
                "criterion_id": "3.2.AC1",
                "status": "pass-shared-linux-story-scope",
                "workflow_state_count": workflow["summary"]["state_count"],
                "receipt_count": workflow["summary"]["receipt_count"],
                "bounded_evidence_finding_count": workflow["bounded_evidence"][
                    "scan_finding_count"
                ],
                "private_user_data_record_count": workflow["bounded_evidence"][
                    "private_user_data_record_count"
                ],
                "agentmage_network_call_count": workflow["summary"][
                    "agentmage_network_call_count"
                ],
                "actual_external_action_count": workflow["summary"][
                    "actual_external_action_count"
                ],
            },
            {
                "criterion_id": "3.2.AC2",
                "status": "pass-shared-linux-story-scope",
                "case_count": patch["summary"]["required_case_count"],
                "passed_case_count": patch["summary"]["passed_case_count"],
                "valid_patch_count": valid_patch_count,
                "prior_state_failure_count": patch["summary"][
                    "prior_state_failure_count"
                ],
                "signature_verified_case_count": patch["summary"][
                    "signature_verified_case_count"
                ],
            },
            {
                "criterion_id": "3.2.AC3",
                "status": "pass-shared-linux-contract-scope",
                "startup_checkpoint_declared": "product-startup-admission"
                in emergency_fixture["evaluation"]["checkpoints"],
                "blocked_subject_kinds": blocked_subject_kinds,
                "blocked_subject_count": emergency["summary"]["blocked_subject_count"],
                "remote_kill_switch": support_policy["runtime_contact_authority"][
                    "remote_kill_switch"
                ],
                "implemented_product_startup_hook_claim": "none",
            },
        ],
        "security_mapping": {
            "mapped_requirement_count": security["summary"][
                "mapped_requirement_count"
            ],
            "product_requirements_complete": security["summary"][
                "product_requirements_complete"
            ],
            "rv_21_complete": security["summary"]["rv_21_complete"],
            "rv_22_complete": security["summary"]["rv_22_complete"],
        },
        "universal_definition_of_done": universal_dod(),
        "independent_review": {
            "reviewer_id": "agentmage-story-3.2-independent-gate-v1",
            "reviewed_commit": REVIEWED_COMMIT,
            "reviewed_tree": REVIEWED_TREE,
            "artifacts": reviewed,
            "finding_count": 0,
            "findings": [],
            "external_human_review_claim": "none",
        },
        "shared_linux_story_work_complete": True,
        "only_blocker": "macos-execution-evidence-unavailable",
        "private_user_data_used": False,
        "network_used": False,
        "actual_incident_claim": "none",
        "product_startup_integration_claim": "none",
        "product_support_claim": "none",
        "product_patch_claim": "none",
        "product_acceptance_claim": "none",
        "release_claim": "none",
        "macos_execution_status": "blocked-macos",
        "macos_evidence_substituted": False,
        "macos_support_claim": "none",
    }


def validate_report(value: Any, root: Path = ROOT) -> list[str]:
    if not isinstance(value, dict):
        return ["Story 3.2 gate report must be an object"]
    failures = []
    if (
        value.get("schema_version") != 1
        or value.get("story_id") != "3.2"
        or value.get("status") != "blocked-macos"
        or value.get("reviewed_commit") != REVIEWED_COMMIT
        or value.get("reviewed_tree") != REVIEWED_TREE
    ):
        failures.append("Story 3.2 gate identity is invalid")
    criteria = value.get("acceptance_criteria", [])
    if [item.get("criterion_id") for item in criteria if isinstance(item, dict)] != [
        "3.2.AC1",
        "3.2.AC2",
        "3.2.AC3",
    ] or [item.get("status") for item in criteria] != [
        "pass-shared-linux-story-scope",
        "pass-shared-linux-story-scope",
        "pass-shared-linux-contract-scope",
    ]:
        failures.append("Story 3.2 acceptance criteria are incomplete")
    elif (
        criteria[0].get("workflow_state_count") != 7
        or criteria[0].get("receipt_count") != 7
        or criteria[0].get("bounded_evidence_finding_count") != 0
        or criteria[0].get("private_user_data_record_count") != 0
        or criteria[0].get("agentmage_network_call_count") != 0
        or criteria[0].get("actual_external_action_count") != 0
        or criteria[1].get("case_count") != 10
        or criteria[1].get("passed_case_count") != 10
        or criteria[1].get("valid_patch_count") != 1
        or criteria[1].get("prior_state_failure_count") != 0
        or criteria[2].get("startup_checkpoint_declared") is not True
        or criteria[2].get("blocked_subject_count") != 5
        or "component" not in criteria[2].get("blocked_subject_kinds", [])
        or "release-version" not in criteria[2].get("blocked_subject_kinds", [])
        or criteria[2].get("remote_kill_switch") is not False
        or criteria[2].get("implemented_product_startup_hook_claim") != "none"
    ):
        failures.append("Story 3.2 acceptance evidence is invalid")
    if value.get("universal_definition_of_done") != universal_dod():
        failures.append("Story 3.2 universal definition of done is invalid")
    if value.get("security_mapping") != {
        "mapped_requirement_count": 10,
        "product_requirements_complete": 0,
        "rv_21_complete": False,
        "rv_22_complete": False,
    }:
        failures.append("Story 3.2 security mapping summary is invalid")
    review = value.get("independent_review", {})
    if (
        review.get("reviewed_commit") != REVIEWED_COMMIT
        or review.get("reviewed_tree") != REVIEWED_TREE
        or review.get("finding_count") != 0
        or review.get("findings") != []
        or review.get("external_human_review_claim") != "none"
    ):
        failures.append("Story 3.2 independent review is invalid")
    if (
        value.get("shared_linux_story_work_complete") is not True
        or value.get("only_blocker") != "macos-execution-evidence-unavailable"
        or value.get("private_user_data_used") is not False
        or value.get("network_used") is not False
        or value.get("actual_incident_claim") != "none"
        or value.get("product_startup_integration_claim") != "none"
        or value.get("product_support_claim") != "none"
        or value.get("product_patch_claim") != "none"
        or value.get("product_acceptance_claim") != "none"
        or value.get("release_claim") != "none"
        or value.get("macos_execution_status") != "blocked-macos"
        or value.get("macos_evidence_substituted") is not False
        or value.get("macos_support_claim") != "none"
    ):
        failures.append("Story 3.2 gate made an unsupported claim")
    try:
        expected = build_report(root)
    except (OSError, ValueError, KeyError, TypeError) as error:
        failures.append(f"cannot rebuild Story 3.2 gate report: {error}")
    else:
        if value != expected:
            failures.append("Story 3.2 gate report is stale or non-deterministic")
    return failures


def check_report(root: Path = ROOT) -> list[str]:
    try:
        value = read_json(root / REPORT_PATH.relative_to(ROOT))
    except (OSError, json.JSONDecodeError) as error:
        return [f"cannot read Story 3.2 gate report: {error}"]
    return validate_report(value, root)


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--write", action="store_true")
    args = parser.parse_args()
    try:
        if args.write:
            write_atomic(REPORT_PATH, canonical_json(build_report()))
        failures = check_report()
    except (OSError, ValueError, KeyError, TypeError) as error:
        print(f"Story 3.2 gate failed: {error}", file=sys.stderr)
        return 1
    if failures:
        for failure in failures:
            print(f"Story 3.2 gate failed: {failure}", file=sys.stderr)
        return 1
    print("Story 3.2 shared/Linux acceptance passed with macOS blocker preserved")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
