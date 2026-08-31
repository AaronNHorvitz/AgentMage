#!/usr/bin/env python3
"""Aggregate Story 7.1 without promoting contract fixtures to platform releases."""

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

from scripts.platform_manifest_artifact import check_report as check_platform
from scripts.story_7_1_security_evidence import check_map as check_security


REPORT_PATH = ROOT / "artifacts/sprints/sprint-7/story-7.1/story-gate-report.json"
REVIEWED_COMMIT = "8a325ae0c1009b5fe39f823718bad543cadbfc5a"
REVIEWED_TREE = "8185071f266005742918da45f069f0ae0fdf7e11"
REVIEWED_PATHS = (
    "docs/architecture/platform-adapter-contract.md",
    "kernel/contracts/src/platform.rs",
    "kernel/engine/src/platform_startup.rs",
    "kernel/engine/tests/platform_adapter_conformance.rs",
    "release/platform-manifests/README.md",
    "release/platform-manifests/v1/fedora-x86_64.json",
    "release/platform-manifests/v1/ubuntu-x86_64.json",
    "artifacts/sprints/sprint-2/story-2.1/platform-result-recorder-report.json",
    "artifacts/sprints/sprint-7/story-7.1/platform-contract-report.json",
    "artifacts/sprints/sprint-7/story-7.1/security-evidence-map.json",
    "schemas/platform/macos-release-manifest.schema.json",
    "release/platform-manifests/macos/v1/contract-fixture.json",
    "artifacts/sprints/sprint-7/story-7.1/macos-release-manifest-contract.json",
    "scripts/macos_release_manifest_contract.mjs",
    "tests/test_macos_release_manifest_contract.mjs",
    "scripts/platform_manifest_artifact.py",
    "scripts/platform_result_recorder.py",
    "scripts/story_7_1_security_evidence.py",
    "tests/test_platform_manifest_artifact.py",
    "tests/test_platform_result_recorder.py",
    "tests/test_story_7_1_security_evidence.py",
)
G_DOD_IDS = tuple(f"G-DOD-{index:02d}" for index in range(1, 14))
BLOCKERS = (
    "sub-task-7.1.1.1-native-linux-and-macos-adapters-open",
    "sub-task-7.1.1.4-native-startup-probes-open",
    "sub-task-7.1.1.6-macos-result-recorder-open",
    "sub-task-7.1.1.7-macos-conformance-open",
    "sub-task-7.1.2.2-reference-release-manifests-incomplete",
    "sub-task-7.1.3.1-macos-conformance-open",
    "sub-task-7.1.3.2-native-primitive-corruption-open",
    "sub-task-7.1.3.3-macos-helper-entitlement-mutations-open",
    "sub-task-7.1.3.4-release-package-and-macos-manifest-open",
    "g-dod-12-independent-critical-boundary-review-not-performed",
    "supported-platform-installed-product-evidence-incomplete",
)


def canonical_json(value: Any) -> bytes:
    return (json.dumps(value, indent=2, sort_keys=True) + "\n").encode()


def read_json(path: Path) -> Any:
    return json.loads(path.read_text(encoding="utf-8"))


def write_atomic(path: Path, content: bytes) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    descriptor, name = tempfile.mkstemp(prefix=".agentmage-story-7-1-gate-", dir=path.parent)
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
        raise ValueError(f"git Story 7.1 review operation failed: {' '.join(args)}")
    return result.stdout if binary else result.stdout.decode().strip()


def reviewed_artifacts() -> list[dict[str, Any]]:
    if git_output("rev-parse", REVIEWED_COMMIT) != REVIEWED_COMMIT:
        raise ValueError("Story 7.1 reviewed commit is unavailable")
    if git_output("show", "-s", "--format=%T", REVIEWED_COMMIT) != REVIEWED_TREE:
        raise ValueError("Story 7.1 reviewed tree changed")
    records = []
    for path in REVIEWED_PATHS:
        committed = git_output("show", f"{REVIEWED_COMMIT}:{path}", binary=True)
        if (ROOT / path).read_bytes() != committed:
            raise ValueError(f"reviewed Story 7.1 artifact changed: {path}")
        records.append({
            "path": path,
            "byte_length": len(committed),
            "sha256": hashlib.sha256(committed).hexdigest(),
        })
    return records


def checklist_failures(text: str) -> list[str]:
    required_checked = (
        "  - [x] **Sub-task 7.1.1.2**", "  - [x] **Sub-task 7.1.1.3**",
        "  - [x] **Sub-task 7.1.1.5**",
        "  - [x] **Sub-task 7.1.1.8:**", "  - [x] **Sub-task 7.1.2.1:**",
        "  - [x] **Sub-task 7.1.2.3:**", "  - [x] **Sub-task 7.1.2.4:**",
        "  - [x] **Sub-task 7.1.3.5 - Product security evidence:**",
    )
    required_open = (
        "### [ ] Sprint 7 - Platform Adapter Contract and Release Manifests",
        "#### [ ] Story 7.1 - Platform Adapter Contract and Release Manifests",
        "- [ ] **Task 7.1.1 - Implement the bounded story**",
        "  - [ ] **Sub-task 7.1.1.1**",
        "  - [ ] **Sub-task 7.1.1.4**", "  - [ ] **Sub-task 7.1.1.6**",
        "  - [ ] **Sub-task 7.1.1.7**", "- [ ] **Task 7.1.2 - Produce reviewable artifacts**",
        "  - [ ] **Sub-task 7.1.2.2:**", "- [ ] **Task 7.1.3 - Verify and close the story**",
        "  - [ ] **Sub-task 7.1.3.1:**", "  - [ ] **Sub-task 7.1.3.2:**",
        "  - [ ] **Sub-task 7.1.3.3:**", "  - [ ] **Sub-task 7.1.3.4:**",
        "- [ ] **Story AC 7.1.AC1:**", "- [ ] **Story AC 7.1.AC2:**",
    )
    return [marker for marker in (*required_checked, *required_open) if marker not in text]


def validate_inputs() -> None:
    check_platform(ROOT)
    security_failures = check_security(ROOT)
    if security_failures:
        raise ValueError("; ".join(security_failures))
    failures = checklist_failures((ROOT / "TASKS.md").read_text(encoding="utf-8"))
    if failures:
        raise ValueError(f"Story 7.1 checklist state changed: {failures[0]}")


def universal_dod() -> list[dict[str, str]]:
    statuses = {control: "pass-current-shared-linux-contract-scope" for control in G_DOD_IDS}
    statuses["G-DOD-10"] = "blocked-native-platform-and-installed-product-evidence"
    statuses["G-DOD-12"] = "blocked-independent-critical-boundary-review"
    return [{"control_id": control, "status": statuses[control]} for control in G_DOD_IDS]


def build_report() -> dict[str, Any]:
    validate_inputs()
    base = ROOT / "artifacts/sprints/sprint-7/story-7.1"
    platform = read_json(base / "platform-contract-report.json")
    security = read_json(base / "security-evidence-map.json")
    macos_manifest = read_json(base / "macos-release-manifest-contract.json")
    return {
        "schema_version": 1,
        "story_id": "7.1",
        "status": "blocked-native-platforms-and-external-review",
        "reviewed_commit": REVIEWED_COMMIT,
        "reviewed_tree": REVIEWED_TREE,
        "acceptance_criteria": [
            {
                "criterion_id": "7.1.AC1",
                "status": "partial-shared-boundary-blocked-native-platforms",
                "adapter_api_version": platform["api"]["version"],
                "required_capability_count": platform["api"]["required_capability_count"],
                "startup_failure_class_count": platform["api"]["startup_failure_class_count"],
                "operating_system_branches_in_kernel_selector": platform["api"]["operating_system_branches_in_kernel_selector"],
                "declared_linux_manifest_count": len(platform["manifests"]),
                "frozen_macos_manifest_contract_field_count": macos_manifest["field_closure"]["top_level_field_count"],
                "installed_runtime_component_manifest_count": 0,
            },
            {
                "criterion_id": "7.1.AC2",
                "status": "pass-available-platform-evidence-blocked-macos",
                "equivalent_available_contract_result_count": platform["conformance"]["equivalent_available_contract_result_count"],
                "mapped_security_requirement_count": security["summary"]["mapped_requirement_count"],
                "maintainer_credentials_recorded": platform["maintainer_credentials_recorded"],
                "private_environment_values_recorded": platform["private_environment_values_recorded"],
                "macos_execution_status": security["macos_execution_status"],
                "macos_evidence_substituted": security["macos_evidence_substituted"],
            },
        ],
        "universal_definition_of_done": universal_dod(),
        "independent_review": {
            "reviewer_id": "agentmage-story-7.1-automated-aggregate-v1",
            "review_type": "automated-aggregate-review-not-external-human-review",
            "reviewed_commit": REVIEWED_COMMIT,
            "reviewed_tree": REVIEWED_TREE,
            "artifacts": reviewed_artifacts(),
            "finding_count": 0,
            "findings": [],
            "external_human_review_status": "not-performed",
        },
        "current_shared_linux_contract_scope_complete": True,
        "story_checkbox_complete": False,
        "blockers": list(BLOCKERS),
        "macos_evidence_substituted": False,
        "installed_product_claim": "none",
        "product_acceptance_claim": "none",
        "sprint_completion_claim": False,
        "release_claim": "none",
    }


def validate_report(value: Any, *, verify_current: bool = True) -> list[str]:
    if not isinstance(value, dict):
        return ["Story 7.1 gate report must be an object"]
    failures = []
    if (
        value.get("schema_version") != 1 or value.get("story_id") != "7.1"
        or value.get("status") != "blocked-native-platforms-and-external-review"
        or value.get("reviewed_commit") != REVIEWED_COMMIT
        or value.get("reviewed_tree") != REVIEWED_TREE
    ):
        failures.append("Story 7.1 gate identity is invalid")
    criteria = value.get("acceptance_criteria", [])
    if [item.get("criterion_id") for item in criteria if isinstance(item, dict)] != ["7.1.AC1", "7.1.AC2"]:
        failures.append("Story 7.1 criterion closure is invalid")
    elif (
        criteria[0].get("adapter_api_version") != 1
        or criteria[0].get("required_capability_count") != 10
        or criteria[0].get("startup_failure_class_count") != 11
        or criteria[0].get("operating_system_branches_in_kernel_selector") != 0
        or criteria[0].get("declared_linux_manifest_count") != 2
        or criteria[0].get("frozen_macos_manifest_contract_field_count") != 16
        or criteria[0].get("installed_runtime_component_manifest_count") != 0
        or criteria[1].get("equivalent_available_contract_result_count") != 3
        or criteria[1].get("mapped_security_requirement_count") != 10
        or criteria[1].get("maintainer_credentials_recorded") is not False
        or criteria[1].get("private_environment_values_recorded") is not False
        or criteria[1].get("macos_execution_status") != "blocked-macos"
        or criteria[1].get("macos_evidence_substituted") is not False
    ):
        failures.append("Story 7.1 criterion evidence is invalid")
    review = value.get("independent_review", {})
    if (
        review.get("reviewer_id") != "agentmage-story-7.1-automated-aggregate-v1"
        or len(review.get("artifacts", [])) != len(REVIEWED_PATHS)
        or review.get("finding_count") != 0
        or review.get("external_human_review_status") != "not-performed"
    ):
        failures.append("Story 7.1 review is invalid")
    if value.get("universal_definition_of_done") != universal_dod():
        failures.append("Story 7.1 Definition-of-Done disposition is invalid")
    if (
        value.get("current_shared_linux_contract_scope_complete") is not True
        or value.get("story_checkbox_complete") is not False
        or value.get("blockers") != list(BLOCKERS)
        or value.get("macos_evidence_substituted") is not False
        or value.get("installed_product_claim") != "none"
        or value.get("product_acceptance_claim") != "none"
        or value.get("sprint_completion_claim") is not False
        or value.get("release_claim") != "none"
    ):
        failures.append("Story 7.1 gate made an unsupported claim")
    if verify_current:
        try:
            expected = build_report()
        except (OSError, ValueError, KeyError, TypeError, json.JSONDecodeError) as error:
            failures.append(f"cannot rebuild Story 7.1 gate report: {error}")
        else:
            if value != expected:
                failures.append("Story 7.1 gate report is stale or widened")
    return failures


def check_report() -> list[str]:
    try:
        return validate_report(read_json(REPORT_PATH))
    except (OSError, json.JSONDecodeError) as error:
        return [f"cannot read Story 7.1 gate report: {error}"]


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--write", action="store_true")
    args = parser.parse_args()
    try:
        if args.write:
            write_atomic(REPORT_PATH, canonical_json(build_report()))
        failures = check_report()
    except (OSError, ValueError, KeyError, TypeError, json.JSONDecodeError) as error:
        print(f"Story 7.1 gate failed: {error}", file=sys.stderr)
        return 1
    if failures:
        for failure in failures:
            print(f"Story 7.1 gate failed: {failure}", file=sys.stderr)
        return 1
    print("Story 7.1 current shared/Linux contract scope passed with blockers preserved")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
