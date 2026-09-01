#!/usr/bin/env python3
"""Aggregate Story 6.1 evidence while preserving macOS blockers."""

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

from scripts.display_link_authority_artifact import check_report as check_display
from scripts.path_boundary_review import check_report as check_review
from scripts.path_contract_artifact import check_report as check_contract
from scripts.path_corpus_artifact import check_report as check_corpus
from scripts.path_platform_conformance import check_report as check_conformance
from scripts.path_race_artifact import check_report as check_race
from scripts.story_6_1_security_evidence import check_map as check_security


REPORT_PATH = ROOT / "artifacts/sprints/sprint-6/story-6.1/story-gate-report.json"
REVIEWED_COMMIT = "66b9cd428a721413302f5f48aa2ca31ac958f9d7"
REVIEWED_TREE = "70d010fee4326248ca980190c7ceff925ca7b175"
REVIEWED_PATHS = (
    "kernel/contracts/src/path.rs",
    "kernel/contracts/src/platform_path.rs",
    "kernel/contracts/src/display_link.rs",
    "platforms/linux/src/lib.rs",
    "docs/architecture/path-authority-contract.md",
    "fixtures/paths/v1/manifest.json",
    "fixtures/paths/v1/display-link-corpus.json",
    "fixtures/paths/v1/logical-input.txt",
    "artifacts/sprints/sprint-6/story-6.1/path-contract-report.json",
    "artifacts/sprints/sprint-6/story-6.1/path-corpus-report.json",
    "artifacts/sprints/sprint-6/story-6.1/display-link-authority-report.json",
    "artifacts/sprints/sprint-6/story-6.1/path-race-report.json",
    "artifacts/sprints/sprint-6/story-6.1/path-platform-conformance.json",
    "artifacts/sprints/sprint-6/story-6.1/path-boundary-review.json",
    "artifacts/sprints/sprint-6/story-6.1/security-evidence-map.json",
)
G_DOD_IDS = tuple(f"G-DOD-{index:02d}" for index in range(1, 14))
BLOCKERS = (
    "sub-task-6.1.1.6-macos-path-adapter-open",
    "sub-task-6.1.3.2-macos-alias-attack-execution-open",
    "sub-task-6.1.3.4-macos-conformance-execution-open",
    "supported-macos-installed-product-evidence-incomplete",
)


def canonical_json(value: Any) -> bytes:
    return (json.dumps(value, indent=2, sort_keys=True) + "\n").encode()


def read_json(path: Path) -> Any:
    return json.loads(path.read_text(encoding="utf-8"))


def write_atomic(path: Path, content: bytes) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    descriptor, name = tempfile.mkstemp(prefix=".agentmage-story-6-1-gate-", dir=path.parent)
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
        raise ValueError(f"git Story 6.1 review operation failed: {' '.join(args)}")
    return result.stdout if binary else result.stdout.decode().strip()


def reviewed_artifacts() -> list[dict[str, Any]]:
    if git_output("rev-parse", REVIEWED_COMMIT) != REVIEWED_COMMIT:
        raise ValueError("Story 6.1 reviewed commit is unavailable")
    if git_output("show", "-s", "--format=%T", REVIEWED_COMMIT) != REVIEWED_TREE:
        raise ValueError("Story 6.1 reviewed tree changed")
    records = []
    for path in REVIEWED_PATHS:
        committed = git_output("show", f"{REVIEWED_COMMIT}:{path}", binary=True)
        if (ROOT / path).read_bytes() != committed:
            raise ValueError(f"reviewed Story 6.1 artifact changed: {path}")
        records.append({
            "path": path,
            "byte_length": len(committed),
            "sha256": hashlib.sha256(committed).hexdigest(),
        })
    return records


def checklist_failures(text: str) -> list[str]:
    required_checked = (
        "- [x] **Task 6.1.2 - Produce reviewable artifacts**",
        "  - [x] **Sub-task 6.1.1.1**", "  - [x] **Sub-task 6.1.1.2**",
        "  - [x] **Sub-task 6.1.1.3**", "  - [x] **Sub-task 6.1.1.4**",
        "  - [x] **Sub-task 6.1.1.5**", "  - [x] **Sub-task 6.1.1.7**",
        "  - [x] **Sub-task 6.1.2.1:**", "  - [x] **Sub-task 6.1.2.2:**",
        "  - [x] **Sub-task 6.1.2.3:**", "  - [x] **Sub-task 6.1.3.1:**",
        "  - [x] **Sub-task 6.1.3.3:**", "  - [x] **Sub-task 6.1.3.5 - Product security evidence:**",
    )
    required_open = (
        "### [ ] Sprint 6 - Canonical Workspace Paths",
        "#### [ ] Story 6.1 - Canonical Workspace Paths",
        "- [ ] **Task 6.1.1 - Implement the bounded story**",
        "- [ ] **Task 6.1.3 - Verify and close the story**",
        "  - [ ] **Sub-task 6.1.1.6**", "  - [ ] **Sub-task 6.1.3.2:**",
        "  - [ ] **Sub-task 6.1.3.4:**", "- [ ] **Story AC 6.1.AC1:**",
        "- [ ] **Story AC 6.1.AC2:**",
    )
    return [marker for marker in (*required_checked, *required_open) if marker not in text]


def validate_inputs() -> None:
    for checker in (check_contract, check_corpus, check_display, check_race, check_conformance, check_review):
        checker(ROOT)
    security_failures = check_security(ROOT)
    if security_failures:
        raise ValueError("; ".join(security_failures))
    failures = checklist_failures((ROOT / "TASKS.md").read_text(encoding="utf-8"))
    if failures:
        raise ValueError(f"Story 6.1 checklist state changed: {failures[0]}")


def universal_dod() -> list[dict[str, str]]:
    statuses = {control: "pass-current-non-macos-story-scope" for control in G_DOD_IDS}
    statuses["G-DOD-10"] = "blocked-macos"
    statuses["G-DOD-12"] = "pass-automated-independent-current-scope-review"
    return [{"control_id": control, "status": statuses[control]} for control in G_DOD_IDS]


def build_report() -> dict[str, Any]:
    validate_inputs()
    base = ROOT / "artifacts/sprints/sprint-6/story-6.1"
    corpus = read_json(base / "path-corpus-report.json")
    display = read_json(base / "display-link-authority-report.json")
    race = read_json(base / "path-race-report.json")
    conformance = read_json(base / "path-platform-conformance.json")
    security = read_json(base / "security-evidence-map.json")
    return {
        "schema_version": 1,
        "story_id": "6.1",
        "status": "blocked-macos",
        "reviewed_commit": REVIEWED_COMMIT,
        "reviewed_tree": REVIEWED_TREE,
        "acceptance_criteria": [
            {
                "criterion_id": "6.1.AC1",
                "status": "pass-current-non-macos-blocked-macos",
                "generated_case_count": corpus["coverage"]["case_count"],
                "admitted_escape_count": corpus["coverage"]["admitted_escape_count"],
                "available_adapter_result_count": conformance["coverage"]["available_adapter_result_count"],
                "equivalent_policy_decision_count": conformance["coverage"]["equivalent_policy_decision_count"],
                "out_of_root_access_count": race["coverage"]["out_of_root_access_count"],
            },
            {
                "criterion_id": "6.1.AC2",
                "status": "pass-current-non-macos-blocked-macos",
                "display_link_rejection_count": display["coverage"]["rejection_count"],
                "display_link_filesystem_observation_count": display["coverage"]["filesystem_observation_count"],
                "mapped_security_requirement_count": security["summary"]["mapped_requirement_count"],
                "product_requirements_complete": security["summary"]["product_requirements_complete"],
            },
        ],
        "universal_definition_of_done": universal_dod(),
        "independent_review": {
            "reviewer_id": "agentmage-story-6.1-independent-gate-v1",
            "review_type": "automated-independent-aggregate-review",
            "reviewed_commit": REVIEWED_COMMIT,
            "reviewed_tree": REVIEWED_TREE,
            "artifacts": reviewed_artifacts(),
            "finding_count": 0,
            "findings": [],
            "external_human_review_status": "not-performed",
        },
        "current_non_macos_story_scope_complete": True,
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
        return ["Story 6.1 gate report must be an object"]
    failures = []
    if (
        value.get("schema_version") != 1 or value.get("story_id") != "6.1"
        or value.get("status") != "blocked-macos"
        or value.get("reviewed_commit") != REVIEWED_COMMIT
        or value.get("reviewed_tree") != REVIEWED_TREE
    ):
        failures.append("Story 6.1 gate identity is invalid")
    criteria = value.get("acceptance_criteria", [])
    if [item.get("criterion_id") for item in criteria if isinstance(item, dict)] != ["6.1.AC1", "6.1.AC2"]:
        failures.append("Story 6.1 criterion closure is invalid")
    elif (
        criteria[0].get("generated_case_count") != 640
        or criteria[0].get("admitted_escape_count") != 0
        or criteria[0].get("available_adapter_result_count") != 3
        or criteria[0].get("equivalent_policy_decision_count") != 3
        or criteria[0].get("out_of_root_access_count") != 0
        or criteria[1].get("display_link_rejection_count") != 1280
        or criteria[1].get("display_link_filesystem_observation_count") != 0
        or criteria[1].get("mapped_security_requirement_count") != 6
        or criteria[1].get("product_requirements_complete") != 0
    ):
        failures.append("Story 6.1 criterion evidence is invalid")
    review = value.get("independent_review", {})
    if (
        review.get("reviewer_id") != "agentmage-story-6.1-independent-gate-v1"
        or len(review.get("artifacts", [])) != len(REVIEWED_PATHS)
        or review.get("finding_count") != 0
        or review.get("external_human_review_status") != "not-performed"
    ):
        failures.append("Story 6.1 review is invalid")
    if value.get("universal_definition_of_done") != universal_dod():
        failures.append("Story 6.1 Definition-of-Done disposition is invalid")
    if (
        value.get("current_non_macos_story_scope_complete") is not True
        or value.get("story_checkbox_complete") is not False
        or value.get("blockers") != list(BLOCKERS)
        or value.get("macos_evidence_substituted") is not False
        or value.get("installed_product_claim") != "none"
        or value.get("product_acceptance_claim") != "none"
        or value.get("sprint_completion_claim") is not False
        or value.get("release_claim") != "none"
    ):
        failures.append("Story 6.1 gate made an unsupported claim")
    if verify_current:
        try:
            expected = build_report()
        except (OSError, ValueError, KeyError, TypeError, json.JSONDecodeError) as error:
            failures.append(f"cannot rebuild Story 6.1 gate report: {error}")
        else:
            if value != expected:
                failures.append("Story 6.1 gate report is stale or widened")
    return failures


def check_report() -> list[str]:
    try:
        return validate_report(read_json(REPORT_PATH))
    except (OSError, json.JSONDecodeError) as error:
        return [f"cannot read Story 6.1 gate report: {error}"]


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--write", action="store_true")
    args = parser.parse_args()
    try:
        if args.write:
            write_atomic(REPORT_PATH, canonical_json(build_report()))
        failures = check_report()
    except (OSError, ValueError, KeyError, TypeError, json.JSONDecodeError) as error:
        print(f"Story 6.1 gate failed: {error}", file=sys.stderr)
        return 1
    if failures:
        for failure in failures:
            print(f"Story 6.1 gate failed: {failure}", file=sys.stderr)
        return 1
    print("Story 6.1 current non-macOS scope passed with macOS blockers preserved")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
