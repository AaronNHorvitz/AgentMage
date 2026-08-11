#!/usr/bin/env python3
"""Evaluate Story 2.2 acceptance while preserving the macOS blocker."""

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

from scripts.fuzz_baseline_reconciliation import check_report as check_baseline  # noqa: E402
from scripts.fuzz_story_gate import check_artifacts as check_gate_policy  # noqa: E402
from scripts.fuzz_target_registry import check_artifacts as check_registry  # noqa: E402
from scripts.seeded_fuzz_failures import check_artifacts as check_seeded  # noqa: E402
from scripts.story_2_2_security_evidence import check_all as check_security  # noqa: E402


REPORT_PATH = ROOT / "artifacts/sprints/sprint-2/story-2.2/story-gate-report.json"
REVIEWED_COMMIT = "941ccfc94777bd93e5bab2336d85437f19246725"
REVIEWED_TREE = "c1527b953cf95a8a34f30b10c6719abcf669360a"
REVIEWED_PATHS = (
    "fuzzing/target-registry.json",
    "fuzzing/toolchain-policy.json",
    "fuzzing/story-gate-policy.json",
    "fuzzing/seeds/security-failures-v1.json",
    "schemas/testing/fuzz-result.schema.json",
    "artifacts/sprints/sprint-2/story-2.2/target-registry-report.json",
    "artifacts/sprints/sprint-2/story-2.2/toolchain-policy-report.json",
    "artifacts/sprints/sprint-2/story-2.2/fuzz-result-schema-report.json",
    "artifacts/sprints/sprint-2/story-2.2/gate-policy-report.json",
    "artifacts/sprints/sprint-2/story-2.2/seeded-failure-report.json",
    "artifacts/sprints/sprint-2/story-2.2/baseline-reconciliation-report.json",
    "artifacts/sprints/sprint-2/story-2.2/rv-15-control-map.json",
    "artifacts/sprints/sprint-2/story-2.2/reviewer-disposition.json",
    "artifacts/sprints/sprint-2/story-2.2/security-evidence-map.json",
    "scripts/fuzz_target_registry.py",
    "scripts/fuzz_toolchain_policy.py",
    "scripts/fuzz_result_contract.py",
    "scripts/fuzz_story_gate.py",
    "scripts/seeded_fuzz_failures.py",
    "scripts/fuzz_baseline_reconciliation.py",
    "scripts/story_2_2_security_evidence.py",
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
        prefix=".agentmage-story-2-2-gate-", dir=path.parent
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
        raise ValueError("Story 2.2 review identity is unavailable or changed")
    records = []
    for path in REVIEWED_PATHS:
        committed = git_output("show", f"{REVIEWED_COMMIT}:{path}", root=root, binary=True)
        current = root / path
        if not current.is_file() or current.read_bytes() != committed:
            raise ValueError(f"reviewed Story 2.2 artifact changed after review: {path}")
        records.append({"path": path, "sha256": sha256_bytes(committed)})
    return records


def universal_dod() -> list[dict[str, Any]]:
    return [
        {
            "control_id": "G-DOD-01",
            "status": "pass-story-scope",
            "evidence": ["TASKS.md", "artifacts/sprints/sprint-2/story-2.2/security-evidence-map.json"],
            "rationale": "Story scope, trust boundaries, exclusions, security mappings, and retained evidence are explicit.",
        },
        {
            "control_id": "G-DOD-02",
            "status": "pass-story-scope",
            "evidence": ["tests/test_seeded_fuzz_failures.py", "tests/test_fuzz_baseline_reconciliation.py"],
            "rationale": "Positive infrastructure, malformed seeds, boundaries, errors, resource events, side effects, and reconciliation mutations are tested.",
        },
        {
            "control_id": "G-DOD-03",
            "status": "pass-story-scope",
            "evidence": ["schemas/testing/fuzz-result.schema.json", "fuzzing/target-registry.json"],
            "rationale": "Closed schemas and registries reject missing, extra, malformed, stale, unsafe, and unsupported evidence.",
        },
        {
            "control_id": "G-DOD-04",
            "status": "not-applicable-no-product-authority",
            "evidence": ["fuzzing/story-gate-policy.json"],
            "rationale": "The bounded fake fuzz foundation grants no product authority and ordinary development credentials cannot waive a target gate.",
        },
        {
            "control_id": "G-DOD-05",
            "status": "pass-story-scope",
            "evidence": ["artifacts/sprints/sprint-2/story-2.2/seeded-failure-report.json"],
            "rationale": "Crash, hang, resource, path, secret, and authorization failures remain typed, owned, minimized, and blocking.",
        },
        {
            "control_id": "G-DOD-06",
            "status": "not-applicable-no-product-tool-attempt",
            "evidence": ["artifacts/sprints/sprint-2/story-2.2/baseline-reconciliation-report.json"],
            "rationale": "No product tool executes; normalized fake-boundary results retain exact outcome and evidence identities.",
        },
        {
            "control_id": "G-DOD-07",
            "status": "pass-story-scope",
            "evidence": ["fuzzing/seeds/security-failures-v1.json", "artifacts/sprints/sprint-2/story-2.2/security-evidence-map.json"],
            "rationale": "Only bounded synthetic-public seeds and redacted hash evidence persist; no private input or real credential is used.",
        },
        {
            "control_id": "G-DOD-08",
            "status": "pass-story-scope",
            "evidence": ["artifacts/sprints/sprint-2/story-2.2/baseline-reconciliation-report.json"],
            "rationale": "Both runs start in new empty temporary roots, copy only allowlisted inputs, and leave those inputs unchanged.",
        },
        {
            "control_id": "G-DOD-09",
            "status": "pass-story-scope",
            "evidence": ["artifacts/sprints/sprint-2/story-2.2/baseline-reconciliation-report.json"],
            "rationale": "Independent clean-run comparison preserves every non-pass and timeout result plus target, coverage, evidence, and regression identity.",
        },
        {
            "control_id": "G-DOD-10",
            "status": "blocked-macos",
            "evidence": ["docs/decisions/0003-blocked-platform-lane-continuation.md"],
            "rationale": "Shared, Linux, and fake-boundary evidence does not substitute for required macOS execution or establish macOS support.",
        },
        {
            "control_id": "G-DOD-11",
            "status": "pass-story-scope",
            "evidence": ["TASKS.md", "requirements/traceability-report.json"],
            "rationale": "Task, traceability, protocol, security, policy, and reviewer-disposition records are current.",
        },
        {
            "control_id": "G-DOD-12",
            "status": "pass-automated-independent-implementation-review",
            "evidence": ["artifacts/sprints/sprint-2/story-2.2/reviewer-disposition.json"],
            "rationale": "An independent gate implementation reviews the immutable evidence commit; no external-human review claim is made.",
        },
        {
            "control_id": "G-DOD-13",
            "status": "pass-gate-reporting",
            "evidence": ["artifacts/sprints/sprint-2/story-2.2/story-gate-report.json"],
            "rationale": "The gate remains visibly BLOCKED-MACOS and represents no unavailable product or platform check as passing.",
        },
    ]


def build_report(root: Path = ROOT) -> dict[str, Any]:
    validation_failures = [
        *check_registry(root),
        *check_gate_policy(root),
        *check_seeded(root),
        *check_baseline(root),
        *check_security(root),
    ]
    if validation_failures:
        raise ValueError("; ".join(validation_failures))
    registry = read_json(root / "fuzzing/target-registry.json")
    policy = read_json(root / "fuzzing/story-gate-policy.json")
    seeded = read_json(
        root / "artifacts/sprints/sprint-2/story-2.2/seeded-failure-report.json"
    )
    baseline = read_json(
        root
        / "artifacts/sprints/sprint-2/story-2.2/baseline-reconciliation-report.json"
    )
    review_artifacts = reviewed_artifacts(root)
    return {
        "schema_version": 1,
        "story_id": "2.2",
        "status": "blocked-macos",
        "reviewed_commit": REVIEWED_COMMIT,
        "reviewed_tree": REVIEWED_TREE,
        "acceptance_criteria": [
            {
                "criterion_id": "2.2.AC1",
                "status": "pass",
                "registered_target_count": len(registry["targets"]),
                "owner_gate_count": len(policy["owner_gates"]),
                "required_evidence_fields": policy["required_evidence_fields"],
                "missing_or_stale_evidence_disposition": policy[
                    "evaluation_contract"
                ]["missing_or_stale_evidence_disposition"],
                "ordinary_development_waiver_permitted": policy[
                    "evaluation_contract"
                ]["normal_development_waiver_permitted"],
                "evidence": [
                    "fuzzing/target-registry.json",
                    "fuzzing/story-gate-policy.json",
                    "artifacts/sprints/sprint-2/story-2.2/gate-policy-report.json",
                ],
            },
            {
                "criterion_id": "2.2.AC2",
                "status": "pass",
                "seeded_failure_count": seeded["summary"]["seed_count"],
                "non_pass_result_count": seeded["summary"][
                    "non_pass_result_count"
                ],
                "minimized_reproducer_count": seeded["summary"][
                    "minimized_reproducer_count"
                ],
                "secret_canary_values_recorded": seeded[
                    "secret_canary_values_recorded"
                ],
                "blocking_gate_result_count": seeded["summary"][
                    "blocking_gate_result_count"
                ],
                "evidence": "artifacts/sprints/sprint-2/story-2.2/seeded-failure-report.json",
            },
            {
                "criterion_id": "2.2.AC3",
                "status": "pass",
                "clean_run_count": baseline["summary"]["clean_run_count"],
                "exact_comparison_count": baseline["summary"][
                    "exact_comparison_count"
                ],
                "comparison_count": baseline["summary"]["comparison_count"],
                "all_comparisons_exact": all(baseline["comparison"].values()),
                "original_development_machine_required": baseline[
                    "original_development_machine_required"
                ],
                "evidence": "artifacts/sprints/sprint-2/story-2.2/baseline-reconciliation-report.json",
            },
        ],
        "summary": {
            "acceptance_criteria_passed": 3,
            "acceptance_criteria_failed": 0,
            "shared_linux_foundation_complete": True,
            "story_checkbox_complete": False,
            "blocking_control_count": 1,
            "blocking_controls": ["G-DOD-10"],
        },
        "universal_definition_of_done": universal_dod(),
        "independent_review": {
            "reviewer_id": "agentmage-story-2.2-independent-gate-v1",
            "review_type": "automated-independent-implementation-review",
            "reviewed_commit": REVIEWED_COMMIT,
            "reviewed_tree": REVIEWED_TREE,
            "reviewed_artifacts": review_artifacts,
            "finding_count": 0,
            "findings": [],
            "disposition": "pass-story-scope-blocked-macos",
            "rereview_required": False,
            "external_human_review_claim": "none",
        },
        "execution": {
            "network_calls": 0,
            "private_user_data_used": False,
            "original_development_machine_data_used": False,
            "external_commands": ["git-read-only"],
        },
        "macos": {
            "status": "blocked-macos",
            "execution_performed": False,
            "evidence_substitution": "prohibited",
            "support_claim": "none",
        },
        "product_boundary_execution_claim": "none",
        "product_acceptance_claim": "none",
        "release_claim": "none",
    }


def validate_report(value: Any, root: Path = ROOT) -> list[str]:
    if not isinstance(value, dict):
        return ["Story 2.2 gate report must be an object"]
    failures: list[str] = []
    if (
        value.get("schema_version") != 1
        or value.get("story_id") != "2.2"
        or value.get("status") != "blocked-macos"
        or value.get("reviewed_commit") != REVIEWED_COMMIT
        or value.get("reviewed_tree") != REVIEWED_TREE
    ):
        failures.append("Story 2.2 gate identity or review boundary is invalid")
    criteria = value.get("acceptance_criteria", [])
    if [item.get("criterion_id") for item in criteria] != [
        "2.2.AC1",
        "2.2.AC2",
        "2.2.AC3",
    ] or any(item.get("status") != "pass" for item in criteria):
        failures.append("Story 2.2 acceptance criteria did not close exactly")
    if value.get("summary") != {
        "acceptance_criteria_passed": 3,
        "acceptance_criteria_failed": 0,
        "shared_linux_foundation_complete": True,
        "story_checkbox_complete": False,
        "blocking_control_count": 1,
        "blocking_controls": ["G-DOD-10"],
    }:
        failures.append("Story 2.2 gate summary overclaimed or omitted a blocker")
    dod = value.get("universal_definition_of_done", [])
    if [item.get("control_id") for item in dod] != list(G_DOD_IDS):
        failures.append("Story 2.2 Definition-of-Done closure is invalid")
    if [item.get("control_id") for item in dod if item.get("status") == "blocked-macos"] != [
        "G-DOD-10"
    ]:
        failures.append("Story 2.2 macOS Definition-of-Done blocker is invalid")
    review = value.get("independent_review", {})
    if (
        review.get("reviewed_commit") != REVIEWED_COMMIT
        or review.get("reviewed_tree") != REVIEWED_TREE
        or review.get("finding_count") != 0
        or review.get("findings") != []
        or review.get("external_human_review_claim") != "none"
    ):
        failures.append("Story 2.2 independent review record is invalid")
    if value.get("macos") != {
        "status": "blocked-macos",
        "execution_performed": False,
        "evidence_substitution": "prohibited",
        "support_claim": "none",
    }:
        failures.append("Story 2.2 gate made an invalid macOS claim")
    if (
        value.get("product_boundary_execution_claim") != "none"
        or value.get("product_acceptance_claim") != "none"
        or value.get("release_claim") != "none"
    ):
        failures.append("Story 2.2 gate made a product or release claim")
    try:
        expected = build_report(root)
    except (OSError, ValueError, KeyError, TypeError, subprocess.TimeoutExpired) as error:
        failures.append(f"cannot rebuild Story 2.2 gate report: {error}")
    else:
        if value != expected:
            failures.append("Story 2.2 gate report is stale or non-deterministic")
    return failures


def write_report(root: Path = ROOT) -> None:
    write_atomic(root / REPORT_PATH.relative_to(ROOT), canonical_json(build_report(root)))


def check_report(root: Path = ROOT) -> list[str]:
    try:
        report = read_json(root / REPORT_PATH.relative_to(ROOT))
    except (OSError, json.JSONDecodeError) as error:
        return [f"cannot read Story 2.2 gate report: {error}"]
    return validate_report(report, root)


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--write", action="store_true")
    args = parser.parse_args()
    try:
        if args.write:
            write_report()
        failures = check_report()
    except (OSError, ValueError, KeyError, TypeError, subprocess.TimeoutExpired) as error:
        print(f"Story 2.2 gate failed: {error}", file=sys.stderr)
        return 1
    if failures:
        for failure in failures:
            print(f"Story 2.2 gate failed: {failure}", file=sys.stderr)
        return 1
    print("Story 2.2 acceptance passed with macOS blocker preserved")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
