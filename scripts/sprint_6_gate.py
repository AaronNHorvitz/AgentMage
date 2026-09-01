#!/usr/bin/env python3
"""Aggregate Sprint 6 without substituting non-macOS path evidence."""

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

from scripts.story_6_1_gate import BLOCKERS, G_DOD_IDS, check_report as check_story


REPORT_PATH = ROOT / "artifacts/sprints/sprint-6/sprint-gate-report.json"
REVIEWED_COMMIT = "96172b22798c3d100462131a40434dc5c3af43b6"
REVIEWED_TREE = "de09759b690225c9e7aded7829310876817c5429"
REVIEWED_PATHS = (
    "artifacts/sprints/sprint-6/story-6.1/story-gate-report.json",
    "scripts/story_6_1_gate.py",
    "artifacts/sprints/sprint-6/story-6.1/path-contract-report.json",
    "artifacts/sprints/sprint-6/story-6.1/path-corpus-report.json",
    "artifacts/sprints/sprint-6/story-6.1/display-link-authority-report.json",
    "artifacts/sprints/sprint-6/story-6.1/path-race-report.json",
    "artifacts/sprints/sprint-6/story-6.1/path-platform-conformance.json",
    "artifacts/sprints/sprint-6/story-6.1/path-boundary-review.json",
    "artifacts/sprints/sprint-6/story-6.1/security-evidence-map.json",
)


def canonical_json(value: Any) -> bytes:
    return (json.dumps(value, indent=2, sort_keys=True) + "\n").encode()


def read_json(path: Path) -> Any:
    return json.loads(path.read_text(encoding="utf-8"))


def write_atomic(path: Path, content: bytes) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    descriptor, name = tempfile.mkstemp(prefix=".agentmage-sprint-6-gate-", dir=path.parent)
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
        raise ValueError(f"git Sprint 6 review operation failed: {' '.join(args)}")
    return result.stdout if binary else result.stdout.decode().strip()


def reviewed_artifacts() -> list[dict[str, Any]]:
    if git_output("rev-parse", REVIEWED_COMMIT) != REVIEWED_COMMIT:
        raise ValueError("Sprint 6 reviewed commit is unavailable")
    if git_output("show", "-s", "--format=%T", REVIEWED_COMMIT) != REVIEWED_TREE:
        raise ValueError("Sprint 6 reviewed tree changed")
    records = []
    for path in REVIEWED_PATHS:
        committed = git_output("show", f"{REVIEWED_COMMIT}:{path}", binary=True)
        if (ROOT / path).read_bytes() != committed:
            raise ValueError(f"reviewed Sprint 6 artifact changed: {path}")
        records.append({"path": path, "sha256": hashlib.sha256(committed).hexdigest()})
    return records


def checklist_failures(text: str) -> list[str]:
    required = (
        "### [ ] Sprint 6 - Canonical Workspace Paths",
        "#### [ ] Story 6.1 - Canonical Workspace Paths",
        "- [ ] **Sprint AC 6.AC1:**", "- [ ] **Sprint AC 6.AC2:**",
        "- [ ] **Sprint AC 6.AC3:**", "- [x] **Sprint AC 6.AC4:**",
        "- [ ] **Sprint AC 6.AC5:**",
    )
    return [marker for marker in required if marker not in text]


def build_report() -> dict[str, Any]:
    failures = check_story()
    if failures:
        raise ValueError("; ".join(failures))
    checklist = checklist_failures((ROOT / "TASKS.md").read_text(encoding="utf-8"))
    if checklist:
        raise ValueError(f"Sprint 6 checklist state changed: {checklist[0]}")
    base = ROOT / "artifacts/sprints/sprint-6/story-6.1"
    story = read_json(base / "story-gate-report.json")
    corpus = read_json(base / "path-corpus-report.json")
    display = read_json(base / "display-link-authority-report.json")
    race = read_json(base / "path-race-report.json")
    conformance = read_json(base / "path-platform-conformance.json")
    return {
        "schema_version": 1,
        "sprint_id": 6,
        "status": "blocked-macos",
        "reviewed_commit": REVIEWED_COMMIT,
        "reviewed_tree": REVIEWED_TREE,
        "acceptance_criteria": [
            {"criterion_id": "6.AC1", "status": "blocked-macos", "available_adapter_result_count": conformance["coverage"]["available_adapter_result_count"], "required_reference_platform_count": 4},
            {"criterion_id": "6.AC2", "status": "pass-current-non-macos-blocked-macos", "generated_case_count": corpus["coverage"]["case_count"], "admitted_escape_count": corpus["coverage"]["admitted_escape_count"]},
            {"criterion_id": "6.AC3", "status": "pass-current-non-macos-blocked-macos", "executed_race_scenario_count": race["coverage"]["executed_scenario_count"], "out_of_root_access_count": race["coverage"]["out_of_root_access_count"]},
            {"criterion_id": "6.AC4", "status": "pass-current-shared-kernel", "display_link_rejection_count": display["coverage"]["rejection_count"], "filesystem_observation_count": display["coverage"]["filesystem_observation_count"]},
            {"criterion_id": "6.AC5", "status": "pass-current-non-macos-blocked-macos", "mapped_security_requirement_count": read_json(base / "security-evidence-map.json")["summary"]["mapped_requirement_count"], "product_requirements_complete": 0},
        ],
        "story_gates": [{
            "story_id": "6.1", "status": story["status"],
            "current_non_macos_story_scope_complete": story["current_non_macos_story_scope_complete"],
            "story_checkbox_complete": story["story_checkbox_complete"],
            "blockers": story["blockers"],
        }],
        "universal_definition_of_done": {
            "control_ids": list(G_DOD_IDS), "blocking_controls": ["G-DOD-10"],
            "macos_evidence_substitution": "prohibited",
        },
        "summary": {
            "acceptance_criteria_passed": 1,
            "acceptance_criteria_blocked_macos": 4,
            "current_non_macos_sprint_scope_complete": True,
            "sprint_checkbox_complete": False,
            "blocking_story_ids": ["6.1"],
            "blockers": list(BLOCKERS),
        },
        "independent_review": {
            "reviewer_id": "agentmage-sprint-6-independent-gate-v1",
            "reviewed_commit": REVIEWED_COMMIT,
            "reviewed_tree": REVIEWED_TREE,
            "reviewed_artifacts": reviewed_artifacts(),
            "finding_count": 0,
            "external_human_review_status": "not-performed",
        },
        "macos_evidence_substituted": False,
        "installed_product_claim": "none",
        "product_acceptance_claim": "none",
        "release_claim": "none",
    }


def validate_report(value: Any, *, verify_current: bool = True) -> list[str]:
    if not isinstance(value, dict):
        return ["Sprint 6 gate report must be an object"]
    failures = []
    if (
        value.get("schema_version") != 1 or value.get("sprint_id") != 6
        or value.get("status") != "blocked-macos"
        or value.get("reviewed_commit") != REVIEWED_COMMIT
        or value.get("reviewed_tree") != REVIEWED_TREE
    ):
        failures.append("Sprint 6 gate identity is invalid")
    criteria = value.get("acceptance_criteria", [])
    if [item.get("criterion_id") for item in criteria if isinstance(item, dict)] != [f"6.AC{i}" for i in range(1, 6)]:
        failures.append("Sprint 6 criterion closure is invalid")
    elif (
        criteria[0].get("available_adapter_result_count") != 3
        or criteria[0].get("required_reference_platform_count") != 4
        or criteria[1].get("generated_case_count") != 640
        or criteria[1].get("admitted_escape_count") != 0
        or criteria[2].get("out_of_root_access_count") != 0
        or criteria[3].get("display_link_rejection_count") != 1280
        or criteria[3].get("filesystem_observation_count") != 0
        or criteria[4].get("mapped_security_requirement_count") != 6
        or criteria[4].get("product_requirements_complete") != 0
    ):
        failures.append("Sprint 6 criterion evidence is invalid")
    if value.get("summary") != {
        "acceptance_criteria_passed": 1, "acceptance_criteria_blocked_macos": 4,
        "current_non_macos_sprint_scope_complete": True, "sprint_checkbox_complete": False,
        "blocking_story_ids": ["6.1"], "blockers": list(BLOCKERS),
    }:
        failures.append("Sprint 6 summary or blockers are invalid")
    review = value.get("independent_review", {})
    if (
        review.get("reviewer_id") != "agentmage-sprint-6-independent-gate-v1"
        or len(review.get("reviewed_artifacts", [])) != len(REVIEWED_PATHS)
        or review.get("finding_count") != 0
        or review.get("external_human_review_status") != "not-performed"
    ):
        failures.append("Sprint 6 review is invalid")
    if (
        value.get("macos_evidence_substituted") is not False
        or value.get("installed_product_claim") != "none"
        or value.get("product_acceptance_claim") != "none"
        or value.get("release_claim") != "none"
    ):
        failures.append("Sprint 6 gate made an unsupported claim")
    if verify_current:
        try:
            expected = build_report()
        except (OSError, ValueError, KeyError, TypeError, json.JSONDecodeError) as error:
            failures.append(f"cannot rebuild Sprint 6 gate report: {error}")
        else:
            if value != expected:
                failures.append("Sprint 6 gate report is stale or widened")
    return failures


def check_report() -> list[str]:
    try:
        return validate_report(read_json(REPORT_PATH))
    except (OSError, json.JSONDecodeError) as error:
        return [f"cannot read Sprint 6 gate report: {error}"]


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--write", action="store_true")
    args = parser.parse_args()
    try:
        if args.write:
            report = build_report()
            write_atomic(REPORT_PATH, canonical_json(report))
            failures = validate_report(report, verify_current=False)
        else:
            failures = check_report()
    except (OSError, ValueError, KeyError, TypeError, json.JSONDecodeError) as error:
        print(f"Sprint 6 gate failed: {error}", file=sys.stderr)
        return 1
    if failures:
        for failure in failures:
            print(f"Sprint 6 gate failed: {failure}", file=sys.stderr)
        return 1
    print("Sprint 6 current non-macOS scope passed with macOS blockers preserved")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
