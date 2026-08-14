#!/usr/bin/env python3
"""Evaluate Sprint 4 acceptance without substituting for macOS evidence."""

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

from scripts.kernel_architecture_report import check_report as check_architecture
from scripts.kernel_boundary_integration import check_report as check_boundary
from scripts.kernel_contract_fixtures import check_outputs as check_fixtures
from scripts.kernel_dispatch_security import check_report as check_dispatch
from scripts.story_4_1_gate import G_DOD_IDS, check_report as check_story_gate


REPORT_PATH = ROOT / "artifacts/sprints/sprint-4/sprint-gate-report.json"
REVIEWED_COMMIT = "79eb3b35093a35f1dacc69207ceddfbeecc1727a"
REVIEWED_TREE = "8d072d14b9735804ee7a568f677cccc781aefce0"
REVIEWED_PATHS = (
    "artifacts/sprints/sprint-4/story-4.1/story-gate-report.json",
    "scripts/story_4_1_gate.py",
    "tests/test_story_4_1_gate.py",
    "artifacts/sprints/sprint-4/story-4.1/kernel-contract-fixture-report.json",
    "artifacts/sprints/sprint-4/story-4.1/kernel-architecture-dependency-report.json",
    "artifacts/sprints/sprint-4/story-4.1/kernel-dispatch-security-report.json",
    "artifacts/sprints/sprint-4/story-4.1/kernel-boundary-integration-report.json",
    "artifacts/sprints/sprint-4/story-4.1/security-evidence-map.json",
)
REQUIRED_SPRINT_MARKERS = (
    "- [x] **Sprint AC 4.AC1:**",
    "- [x] **Sprint AC 4.AC2:**",
    "- [x] **Sprint AC 4.AC3:**",
    "- [x] **Sprint AC 4.AC4:**",
    "- [x] **Sprint AC 4.AC5:**",
)


def canonical_json(value: Any) -> bytes:
    return (json.dumps(value, indent=2, sort_keys=True) + "\n").encode("utf-8")


def sha256_bytes(value: bytes) -> str:
    return hashlib.sha256(value).hexdigest()


def read_json(path: Path) -> Any:
    return json.loads(path.read_text(encoding="utf-8"))


def write_atomic(path: Path, content: bytes) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    descriptor, temporary_name = tempfile.mkstemp(
        prefix=".agentmage-sprint-4-gate-", dir=path.parent
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
        raise ValueError("Sprint 4 review identity is unavailable or changed")
    records = []
    for path in REVIEWED_PATHS:
        committed = git_output("show", f"{REVIEWED_COMMIT}:{path}", root=root, binary=True)
        current = root / path
        if not current.is_file() or current.read_bytes() != committed:
            raise ValueError(f"reviewed Sprint 4 artifact changed after review: {path}")
        records.append({"path": path, "sha256": sha256_bytes(committed)})
    return records


def sprint_marker_failures(tasks_text: str) -> list[str]:
    failures = [marker for marker in REQUIRED_SPRINT_MARKERS if marker not in tasks_text]
    if "### [ ] Sprint 4 - Kernel Contracts and Typed Boundaries" not in tasks_text:
        failures.append("Sprint 4 checkbox must remain open while macOS is blocked")
    if "#### [ ] Story 4.1 - Kernel Contracts and Typed Boundaries" not in tasks_text:
        failures.append("Story 4.1 checkbox must remain open while macOS is blocked")
    return failures


def validate_inputs(root: Path = ROOT) -> list[str]:
    failures: list[str] = []
    try:
        check_fixtures()
    except (OSError, RuntimeError, ValueError, subprocess.SubprocessError) as error:
        failures.append(f"contract-fixtures: {error}")
    for name, check in (
        ("architecture", check_architecture),
        ("dispatcher", check_dispatch),
        ("boundary-integration", check_boundary),
        ("story-4.1-gate", check_story_gate),
    ):
        failures.extend(f"{name}: {failure}" for failure in check(root))
    try:
        tasks_text = (root / "TASKS.md").read_text(encoding="utf-8")
    except OSError as error:
        failures.append(f"cannot read TASKS.md: {error}")
    else:
        failures.extend(sprint_marker_failures(tasks_text))
    return failures


def story_gate_summary(value: dict[str, Any]) -> dict[str, Any]:
    dod = value["universal_definition_of_done"]
    return {
        "story_id": "4.1",
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


def build_report(root: Path = ROOT) -> dict[str, Any]:
    failures = validate_inputs(root)
    if failures:
        raise ValueError("; ".join(failures))
    fixtures = read_json(
        root / "artifacts/sprints/sprint-4/story-4.1/kernel-contract-fixture-report.json"
    )
    architecture = read_json(
        root
        / "artifacts/sprints/sprint-4/story-4.1/kernel-architecture-dependency-report.json"
    )
    dispatch = read_json(
        root / "artifacts/sprints/sprint-4/story-4.1/kernel-dispatch-security-report.json"
    )
    boundary = read_json(
        root / "artifacts/sprints/sprint-4/story-4.1/kernel-boundary-integration-report.json"
    )
    story = read_json(
        root / "artifacts/sprints/sprint-4/story-4.1/story-gate-report.json"
    )
    return {
        "schema_version": 1,
        "sprint_id": 4,
        "status": "blocked-macos",
        "reviewed_commit": REVIEWED_COMMIT,
        "reviewed_tree": REVIEWED_TREE,
        "acceptance_criteria": [
            {
                "criterion_id": "4.AC1",
                "status": "pass-shared-linux",
                "acceptance_test_id": "AT-ARCH-001",
                "scope": "implemented-contract-layer",
                "declared_edge_count": architecture["graph"]["declared_edge_count"],
                "observed_product_edge_count": architecture["graph"][
                    "observed_product_edge_count"
                ],
                "prohibited_observed_edge_count": len(
                    architecture["graph"]["prohibited_observed_edges"]
                ),
                "runtime_boundary_trace_count": boundary["trace_count"],
                "product_wide_acceptance_claim": "none",
            },
            {
                "criterion_id": "4.AC2",
                "status": "pass-shared-linux",
                "persisted_invalid_fixture_count": fixtures["invalid_fixture_count"],
                "generated_oversized_case_count": fixtures[
                    "generated_oversized_case_count"
                ],
                "compatibility_rejection": fixtures["compatibility_rejection"],
            },
            {
                "criterion_id": "4.AC3",
                "status": "pass-shared-linux",
                "valid_fixture_count": fixtures["valid_fixture_count"],
                "canonical_byte_stability": fixtures["canonical_byte_stability"],
                "public_parser_round_trip": fixtures["public_parser_round_trip"],
            },
            {
                "criterion_id": "4.AC4",
                "status": "pass-shared-linux",
                "typed_boundary_trace_count": boundary["trace_count"],
                "success_contract": boundary["success_contract"],
                "non_success_contract": boundary["non_success_contract"],
                "failure_count": len(boundary["failures"]),
            },
            {
                "criterion_id": "4.AC5",
                "status": "pass-shared-linux",
                "dispatcher_denial_trace_count": dispatch["trace_count"],
                "executor_callback_available": dispatch[
                    "executor_callback_available"
                ],
                "positive_dispatch_path_available": dispatch[
                    "positive_dispatch_path_available"
                ],
                "failure_count": len(dispatch["failures"]),
            },
        ],
        "story_gates": [story_gate_summary(story)],
        "universal_definition_of_done": {
            "control_ids": list(G_DOD_IDS),
            "story_count": 1,
            "all_non_platform_controls_pass_or_not_applicable": True,
            "blocking_controls": ["G-DOD-10"],
            "macos_evidence_substitution": "prohibited",
        },
        "summary": {
            "acceptance_criteria_passed": 5,
            "acceptance_criteria_failed": 0,
            "story_gate_count": 1,
            "shared_linux_sprint_work_complete": True,
            "sprint_checkbox_complete": False,
            "blocking_story_count": 1,
            "blocking_story_ids": ["4.1"],
            "blocking_control_count": 1,
            "blocking_controls": ["G-DOD-10"],
        },
        "independent_review": {
            "reviewer_id": "agentmage-sprint-4-independent-gate-v1",
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
        "positive_authority_path_claim": "none",
        "product_acceptance_claim": "none",
        "release_claim": "none",
    }


def validate_report(
    value: Any, root: Path = ROOT, *, verify_current: bool = True
) -> list[str]:
    if not isinstance(value, dict):
        return ["Sprint 4 gate report must be an object"]
    failures: list[str] = []
    if (
        value.get("schema_version") != 1
        or value.get("sprint_id") != 4
        or value.get("status") != "blocked-macos"
        or value.get("reviewed_commit") != REVIEWED_COMMIT
        or value.get("reviewed_tree") != REVIEWED_TREE
    ):
        failures.append("Sprint 4 gate identity or review boundary is invalid")
    criteria = value.get("acceptance_criteria", [])
    if [item.get("criterion_id") for item in criteria if isinstance(item, dict)] != [
        "4.AC1",
        "4.AC2",
        "4.AC3",
        "4.AC4",
        "4.AC5",
    ] or any(item.get("status") != "pass-shared-linux" for item in criteria):
        failures.append("Sprint 4 acceptance criteria did not close exactly")
    elif (
        criteria[0].get("prohibited_observed_edge_count") != 0
        or criteria[0].get("runtime_boundary_trace_count") != 5
        or criteria[0].get("product_wide_acceptance_claim") != "none"
        or criteria[1].get("persisted_invalid_fixture_count") != 7
        or criteria[1].get("generated_oversized_case_count") != 1
        or criteria[1].get("compatibility_rejection") != "pass"
        or criteria[2].get("valid_fixture_count") != 15
        or criteria[2].get("canonical_byte_stability") != "pass"
        or criteria[2].get("public_parser_round_trip") != "pass"
        or criteria[3].get("typed_boundary_trace_count") != 5
        or criteria[3].get("success_contract") != "ToolResult"
        or criteria[3].get("non_success_contract") != "BoundaryFailure"
        or criteria[3].get("failure_count") != 0
        or criteria[4].get("dispatcher_denial_trace_count") != 7
        or criteria[4].get("executor_callback_available") is not False
        or criteria[4].get("positive_dispatch_path_available") is not False
        or criteria[4].get("failure_count") != 0
    ):
        failures.append("Sprint 4 acceptance evidence is invalid")
    stories = value.get("story_gates", [])
    if len(stories) != 1 or any(
        item.get("story_id") != "4.1"
        or item.get("status") != "blocked-macos"
        or item.get("shared_linux_story_work_complete") is not True
        or item.get("story_checkbox_complete") is not False
        or item.get("only_blocker") != "macos-execution-evidence-unavailable"
        or item.get("dod_control_ids") != list(G_DOD_IDS)
        or item.get("dod_blocking_controls") != ["G-DOD-10"]
        for item in stories
    ):
        failures.append("Sprint 4 story-gate aggregation is invalid")
    if value.get("universal_definition_of_done") != {
        "control_ids": list(G_DOD_IDS),
        "story_count": 1,
        "all_non_platform_controls_pass_or_not_applicable": True,
        "blocking_controls": ["G-DOD-10"],
        "macos_evidence_substitution": "prohibited",
    }:
        failures.append("Sprint 4 Definition-of-Done aggregation is invalid")
    if value.get("summary") != {
        "acceptance_criteria_passed": 5,
        "acceptance_criteria_failed": 0,
        "story_gate_count": 1,
        "shared_linux_sprint_work_complete": True,
        "sprint_checkbox_complete": False,
        "blocking_story_count": 1,
        "blocking_story_ids": ["4.1"],
        "blocking_control_count": 1,
        "blocking_controls": ["G-DOD-10"],
    }:
        failures.append("Sprint 4 gate summary is invalid")
    review = value.get("independent_review", {})
    if (
        review.get("finding_count") != 0
        or review.get("findings") != []
        or review.get("external_human_review_claim") != "none"
        or review.get("disposition")
        != "pass-shared-linux-sprint-blocked-macos"
    ):
        failures.append("Sprint 4 independent review is invalid")
    if value.get("macos") != {
        "status": "blocked-macos",
        "execution_performed": False,
        "evidence_substitution": "prohibited",
        "support_claim": "none",
    } or any(
        value.get(key) != "none"
        for key in (
            "positive_authority_path_claim",
            "product_acceptance_claim",
            "release_claim",
        )
    ):
        failures.append("Sprint 4 gate made an unsupported claim")
    if verify_current:
        try:
            expected = build_report(root)
        except (OSError, ValueError, KeyError, TypeError) as error:
            failures.append(f"cannot rebuild Sprint 4 gate report: {error}")
        else:
            if value != expected:
                failures.append("Sprint 4 gate report is stale or non-deterministic")
    return failures


def check_report(root: Path = ROOT) -> list[str]:
    try:
        value = read_json(root / REPORT_PATH.relative_to(ROOT))
    except (OSError, json.JSONDecodeError) as error:
        return [f"cannot read Sprint 4 gate report: {error}"]
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
        print(f"Sprint 4 gate failed: {error}", file=sys.stderr)
        return 1
    if failures:
        for failure in failures:
            print(f"Sprint 4 gate failed: {failure}", file=sys.stderr)
        return 1
    print("Sprint 4 shared/Linux acceptance passed with macOS blocker preserved")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
