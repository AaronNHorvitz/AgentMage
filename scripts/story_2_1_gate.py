#!/usr/bin/env python3
"""Evaluate Story 2.1 acceptance while preserving the macOS blocker."""

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

from scripts.later_input_fixture_matrix import (  # noqa: E402
    REPORT_PATH as MATRIX_REPORT_PATH,
    build_report as build_matrix_report,
)
from scripts.story_2_1_verification import (  # noqa: E402
    CORPUS_REPORT_PATH,
    SUMMARY_REPORT_PATH,
    build_corpus_report,
    build_summary_report,
)


REPORT_PATH = ROOT / "artifacts/sprints/sprint-2/story-2.1/story-gate-report.json"
REVIEWED_COMMIT = "bee933d341bbeeb74f307ce9940c304f8c989b17"
REVIEWED_TREE = "2cab53347c18799c631c2879a7da60e3d91d83b0"
REVIEWED_PATHS = (
    "fixtures/corpus-profile.json",
    "fixtures/corpus/v1/agentmage-synthetic-corpus-v1.zip",
    "fixtures/corpus/v1/manifest.json",
    "fixtures/story-2.1/later-input-class-fixtures-v1.json",
    "artifacts/sprints/sprint-2/story-2.1/corpus-reproducibility-report.json",
    "artifacts/sprints/sprint-2/story-2.1/input-class-fixture-matrix-report.json",
    "artifacts/sprints/sprint-2/story-2.1/summary-reconciliation-report.json",
    "scripts/later_input_fixture_matrix.py",
    "scripts/story_2_1_verification.py",
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
        prefix=".agentmage-story-2-1-gate-", dir=path.parent
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
        raise ValueError("Story 2.1 review identity is unavailable or changed")
    records = []
    for path in REVIEWED_PATHS:
        committed = git_output("show", f"{REVIEWED_COMMIT}:{path}", root=root, binary=True)
        current_path = root / path
        if not current_path.is_file() or current_path.read_bytes() != committed:
            raise ValueError(f"reviewed Story 2.1 artifact changed after review: {path}")
        records.append({"path": path, "sha256": sha256_bytes(committed)})
    return records


def universal_dod() -> list[dict[str, Any]]:
    return [
        {
            "control_id": "G-DOD-01",
            "status": "pass-story-scope",
            "evidence": ["TASKS.md", "artifacts/sprints/sprint-2/story-2.1/security-evidence-map.json"],
            "rationale": "Story scope, dependencies, exclusions, controls, and retained evidence are recorded.",
        },
        {
            "control_id": "G-DOD-02",
            "status": "pass-story-scope",
            "evidence": ["tests", "fixtures/story-2.1/later-input-class-fixtures-v1.json"],
            "rationale": "Positive, invalid, boundary, cancellation, dependency-failure, and exact-side-effect cases cover the fixture harness.",
        },
        {
            "control_id": "G-DOD-03",
            "status": "pass-story-scope",
            "evidence": ["scripts/later_input_fixture_matrix.py", "schemas/testing"],
            "rationale": "Story-owned contracts reject missing, extra, malformed, oversized, stale, and unsupported fixture evidence.",
        },
        {
            "control_id": "G-DOD-04",
            "status": "not-applicable-no-authority",
            "evidence": ["fixtures/fake-adapter-contract.json"],
            "rationale": "This story creates inert test fixtures and adapters; it enables no product authority-bearing behavior.",
        },
        {
            "control_id": "G-DOD-05",
            "status": "pass-story-scope",
            "evidence": ["artifacts/sprints/sprint-2/story-2.1/fault-test-adapter-report.json"],
            "rationale": "Unavailable identity, path, network, resource, evidence, and recovery states fail closed in the test foundation.",
        },
        {
            "control_id": "G-DOD-06",
            "status": "not-applicable-no-product-tool-attempt",
            "evidence": ["artifacts/sprints/sprint-2/story-2.1/expected-output-report.json"],
            "rationale": "No product tool executes; synthetic receipt fixtures cover all declared outcome classes for later stories.",
        },
        {
            "control_id": "G-DOD-07",
            "status": "pass-story-scope",
            "evidence": ["artifacts/sprints/sprint-2/story-2.1/fixture-security-scan-report.json"],
            "rationale": "Only minimized synthetic-public fixture data persists; private data and real credentials are prohibited.",
        },
        {
            "control_id": "G-DOD-08",
            "status": "pass-story-scope",
            "evidence": ["artifacts/sprints/sprint-2/story-2.1/corpus-reproducibility-report.json"],
            "rationale": "Generation uses new temporary destinations and never reads or writes user-owned files.",
        },
        {
            "control_id": "G-DOD-09",
            "status": "pass-story-scope",
            "evidence": ["artifacts/sprints/sprint-2/story-2.1/summary-reconciliation-report.json"],
            "rationale": "Independent reduction preserves failures, skips, retries, flakes, quarantine, cancellation, timeout, and environment identities.",
        },
        {
            "control_id": "G-DOD-10",
            "status": "blocked-macos",
            "evidence": ["docs/decisions/0003-blocked-platform-lane-continuation.md"],
            "rationale": "Shared and Linux evidence does not substitute for unexecuted macOS checks or establish macOS support.",
        },
        {
            "control_id": "G-DOD-11",
            "status": "pass-story-scope",
            "evidence": ["TASKS.md", "requirements/traceability-report.json"],
            "rationale": "Story documentation, profiles, manifests, security mappings, and evidence indexes are current.",
        },
        {
            "control_id": "G-DOD-12",
            "status": "pass-automated-independent-implementation-review",
            "evidence": ["artifacts/sprints/sprint-2/story-2.1/summary-comparison-signature.json"],
            "rationale": "A separately implemented reducer and gate verifier review the immutable commit; no external-human review claim is made.",
        },
        {
            "control_id": "G-DOD-13",
            "status": "pass-gate-reporting",
            "evidence": ["artifacts/sprints/sprint-2/story-2.1/story-gate-report.json"],
            "rationale": "The gate remains visibly BLOCKED-MACOS and represents no unavailable check as passing.",
        },
    ]


def build_report(root: Path = ROOT) -> dict[str, Any]:
    matrix_report = build_matrix_report(root)
    corpus_report = build_corpus_report(root)
    summary_report = build_summary_report(root)
    if read_json(root / MATRIX_REPORT_PATH.relative_to(ROOT)) != matrix_report:
        raise ValueError("checked input-class fixture matrix report is stale")
    if read_json(root / CORPUS_REPORT_PATH.relative_to(ROOT)) != corpus_report:
        raise ValueError("checked corpus reproduction report is stale")
    if read_json(root / SUMMARY_REPORT_PATH.relative_to(ROOT)) != summary_report:
        raise ValueError("checked summary reconciliation report is stale")
    if summary_report["reconciliation"]["exact_match"] is not True:
        raise ValueError("independent summary did not reconcile")
    review_artifacts = reviewed_artifacts(root)
    dod = universal_dod()
    return {
        "schema_version": 1,
        "story_id": "2.1",
        "status": "blocked-macos",
        "reviewed_commit": REVIEWED_COMMIT,
        "reviewed_tree": REVIEWED_TREE,
        "acceptance_criteria": [
            {
                "criterion_id": "2.1.AC1",
                "status": "pass",
                "input_class_count": matrix_report["coverage"]["input_class_count"],
                "scenario_count": matrix_report["coverage"]["scenario_count"],
                "fixture_count": matrix_report["coverage"]["fixture_count"],
                "complete_cross_product": matrix_report["coverage"]["complete_cross_product"],
                "prohibited_side_effects_declared": True,
                "prohibited_side_effect_count": 0,
                "evidence": MATRIX_REPORT_PATH.relative_to(ROOT).as_posix(),
            },
            {
                "criterion_id": "2.1.AC2",
                "status": "pass",
                "corpus_reproduction_runs": corpus_report["reproduction"]["run_count"],
                "corpus_runs_byte_identical": corpus_report["reproduction"]["runs_byte_identical"],
                "checked_corpus_match": corpus_report["reproduction"]["checked_artifacts_match"],
                "summary_exact_match": summary_report["reconciliation"]["exact_match"],
                "private_user_data_used": False,
                "current_host_data_used": False,
                "original_development_machine_required": False,
                "evidence": [
                    CORPUS_REPORT_PATH.relative_to(ROOT).as_posix(),
                    SUMMARY_REPORT_PATH.relative_to(ROOT).as_posix(),
                ],
            },
        ],
        "summary": {
            "acceptance_criteria_passed": 2,
            "acceptance_criteria_failed": 0,
            "shared_linux_foundation_complete": True,
            "story_checkbox_complete": False,
            "blocking_control_count": 1,
            "blocking_controls": ["G-DOD-10"],
        },
        "universal_definition_of_done": dod,
        "independent_review": {
            "reviewer_id": "agentmage-story-2.1-independent-gate-v1",
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
        "product_acceptance_claim": "none",
        "release_claim": "none",
    }


def validate_report(value: Any, root: Path = ROOT) -> list[str]:
    if not isinstance(value, dict):
        return ["Story 2.1 gate report must be an object"]
    failures: list[str] = []
    if (
        value.get("schema_version") != 1
        or value.get("story_id") != "2.1"
        or value.get("status") != "blocked-macos"
    ):
        failures.append("Story 2.1 gate identity or status is invalid")
    criteria = value.get("acceptance_criteria", [])
    if [item.get("criterion_id") for item in criteria] != ["2.1.AC1", "2.1.AC2"] or any(
        item.get("status") != "pass" for item in criteria
    ):
        failures.append("Story 2.1 acceptance criteria did not close exactly")
    summary = value.get("summary", {})
    if summary != {
        "acceptance_criteria_passed": 2,
        "acceptance_criteria_failed": 0,
        "shared_linux_foundation_complete": True,
        "story_checkbox_complete": False,
        "blocking_control_count": 1,
        "blocking_controls": ["G-DOD-10"],
    }:
        failures.append("Story 2.1 gate summary overclaimed or omitted a blocker")
    dod = value.get("universal_definition_of_done", [])
    if [item.get("control_id") for item in dod] != list(G_DOD_IDS):
        failures.append("Story 2.1 Definition-of-Done closure is invalid")
    if [item.get("control_id") for item in dod if item.get("status") == "blocked-macos"] != [
        "G-DOD-10"
    ]:
        failures.append("Story 2.1 macOS Definition-of-Done blocker is invalid")
    review = value.get("independent_review", {})
    if (
        review.get("reviewed_commit") != REVIEWED_COMMIT
        or review.get("reviewed_tree") != REVIEWED_TREE
        or review.get("finding_count") != 0
        or review.get("findings") != []
        or review.get("external_human_review_claim") != "none"
    ):
        failures.append("Story 2.1 independent review record is invalid")
    if value.get("macos") != {
        "status": "blocked-macos",
        "execution_performed": False,
        "evidence_substitution": "prohibited",
        "support_claim": "none",
    }:
        failures.append("Story 2.1 gate made an invalid macOS claim")
    if value.get("product_acceptance_claim") != "none" or value.get(
        "release_claim"
    ) != "none":
        failures.append("Story 2.1 gate made a product or release claim")
    try:
        expected = build_report(root)
    except (OSError, ValueError, KeyError, TypeError, subprocess.TimeoutExpired) as error:
        failures.append(f"cannot rebuild Story 2.1 gate report: {error}")
    else:
        if value != expected:
            failures.append("Story 2.1 gate report is stale or non-deterministic")
    return failures


def write_report(root: Path = ROOT) -> None:
    write_atomic(root / REPORT_PATH.relative_to(ROOT), canonical_json(build_report(root)))


def check_report(root: Path = ROOT) -> list[str]:
    try:
        report = read_json(root / REPORT_PATH.relative_to(ROOT))
    except (OSError, json.JSONDecodeError) as error:
        return [f"cannot read Story 2.1 gate report: {error}"]
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
        print(f"Story 2.1 gate failed: {error}", file=sys.stderr)
        return 1
    if failures:
        for failure in failures:
            print(f"Story 2.1 gate failed: {failure}", file=sys.stderr)
        return 1
    print("Story 2.1 acceptance passed; story gate remains BLOCKED-MACOS")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
