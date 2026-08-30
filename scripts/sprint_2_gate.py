#!/usr/bin/env python3
"""Evaluate Sprint 2 acceptance without substituting for macOS evidence."""

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

from scripts.fixture_security_scan import check_report as check_fixture_scan  # noqa: E402
from scripts.platform_result_recorder import check_report as check_platform_report  # noqa: E402
from scripts.story_2_1_gate import check_report as check_story_2_1_gate  # noqa: E402
from scripts.story_2_1_verification import (  # noqa: E402
    check_adapter_report,
    check_corpus_report,
)
from scripts.story_2_2_gate import check_report as check_story_2_2_gate  # noqa: E402
from scripts.story_2_3_gate import validate_report as validate_story_2_3_gate  # noqa: E402
from scripts.story_2_4_gate import validate_report as validate_story_2_4_gate  # noqa: E402


REPORT_PATH = ROOT / "artifacts/sprints/sprint-2/sprint-gate-report.json"
REVIEWED_COMMIT = "817414bc9e1086887249dae79b4d9ed6acfc5b3d"
REVIEWED_TREE = "791385820ce8c9cbde6258f4f4fb06cb30b5212e"
REVIEWED_PATHS = (
    "fixtures/corpus/v1/manifest.json",
    "fixtures/story-2.1/later-input-class-fixtures-v1.json",
    "fuzzing/target-registry.json",
    "fuzzing/story-gate-policy.json",
    "fuzzing/toolchain-policy.json",
    "fuzzing/seeds/security-failures-v1.json",
    "fixtures/artifact-evaluation/v1/golden-manifest-v1.json",
    "fixtures/artifact-evaluation/v1/reproducibility-report.json",
    "fixtures/artifact-evaluation/v1/artifact-golden-metrics.json",
    "fixtures/artifact-evaluation/v1/workflow-golden-metrics.json",
    "fixtures/artifact-admission/v1/manifest.json",
    "fixtures/artifact-admission/v1/adversarial-manifest.json",
    "fixtures/artifact-admission/v1/context-accounting-manifests.json",
    "fixtures/artifact-admission/v1/context-delivery-receipts.json",
    "fixtures/artifact-admission/v1/resource-gate-observations.json",
    "fixtures/engineering-runtime/v2/manifest.json",
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
        prefix=".agentmage-sprint-2-gate-", dir=path.parent
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
        raise ValueError("Sprint 2 review identity is unavailable or changed")
    records = []
    for path in REVIEWED_PATHS:
        committed = git_output("show", f"{REVIEWED_COMMIT}:{path}", root=root, binary=True)
        current = root / path
        if not current.is_file() or current.read_bytes() != committed:
            raise ValueError(f"reviewed Sprint 2 artifact changed after review: {path}")
        records.append({"path": path, "sha256": sha256_bytes(committed)})
    return records


def story_gate_summary(story_id: str, value: dict[str, Any]) -> dict[str, Any]:
    dod = value["universal_definition_of_done"]
    if "summary" in value:
        dod_control_ids = [item["control_id"] for item in dod]
        dod_blocking_controls = [
            item["control_id"] for item in dod if item["status"] == "blocked-macos"
        ]
        summary = value["summary"]
        criteria_passed = summary["acceptance_criteria_passed"]
        criteria_failed = summary["acceptance_criteria_failed"]
        local_scope_complete = summary["shared_linux_foundation_complete"]
        checkbox_complete = summary["story_checkbox_complete"]
        blocking_controls = summary["blocking_controls"]
    else:
        dod_control_ids = [item["control_id"] for item in dod]
        dod_blocking_controls = [
            item["control_id"]
            for item in dod
            if item["status"].startswith("blocked-")
        ]
        criteria = value["acceptance_criteria"]
        criteria_passed = sum(item["status"].startswith("pass-") for item in criteria)
        criteria_failed = len(criteria) - criteria_passed
        local_scope_complete = value.get(
            "current_public_synthetic_corpus_scope_complete",
            value.get("current_public_synthetic_fixture_scope_complete"),
        )
        checkbox_complete = value["story_checkbox_complete"]
        blocking_controls = value["blocking_controls"]
    return {
        "story_id": story_id,
        "status": value["status"],
        "acceptance_criteria_passed": criteria_passed,
        "acceptance_criteria_failed": criteria_failed,
        "current_local_scope_complete": local_scope_complete,
        "story_checkbox_complete": checkbox_complete,
        "blocking_controls": blocking_controls,
        "dod_control_ids": dod_control_ids,
        "dod_blocking_controls": dod_blocking_controls,
    }


def build_report(root: Path = ROOT) -> dict[str, Any]:
    validation_failures = [
        *check_corpus_report(root),
        *check_adapter_report(root),
        *check_platform_report(root),
        *check_fixture_scan(root),
        *check_story_2_1_gate(root),
        *check_story_2_2_gate(root),
        *validate_story_2_3_gate(read_json(
            root / "artifacts/sprints/sprint-2/story-2.3/story-gate-report.json"
        )),
        *validate_story_2_4_gate(read_json(
            root / "artifacts/sprints/sprint-2/story-2.4/story-gate-report.json"
        )),
    ]
    if validation_failures:
        raise ValueError("; ".join(validation_failures))
    corpus = read_json(
        root
        / "artifacts/sprints/sprint-2/story-2.1/corpus-reproducibility-report.json"
    )
    adapters = read_json(
        root
        / "artifacts/sprints/sprint-2/story-2.1/adapter-mode-verification-report.json"
    )
    platform = read_json(
        root
        / "artifacts/sprints/sprint-2/story-2.1/platform-result-recorder-report.json"
    )
    scan = read_json(
        root
        / "artifacts/sprints/sprint-2/story-2.1/fixture-security-scan-report.json"
    )
    baseline = read_json(
        root
        / "artifacts/sprints/sprint-2/story-2.2/baseline-reconciliation-report.json"
    )
    story_2_1 = read_json(
        root / "artifacts/sprints/sprint-2/story-2.1/story-gate-report.json"
    )
    story_2_2 = read_json(
        root / "artifacts/sprints/sprint-2/story-2.2/story-gate-report.json"
    )
    story_2_3 = read_json(
        root / "artifacts/sprints/sprint-2/story-2.3/story-gate-report.json"
    )
    story_2_4 = read_json(
        root / "artifacts/sprints/sprint-2/story-2.4/story-gate-report.json"
    )
    reproducibility = read_json(
        root / "fixtures/artifact-evaluation/v1/reproducibility-report.json"
    )
    golden = read_json(
        root / "fixtures/artifact-evaluation/v1/golden-manifest-v1.json"
    )
    workflow_metrics = read_json(
        root / "fixtures/artifact-evaluation/v1/workflow-golden-metrics.json"
    )
    fuzz_extension = read_json(
        root / "fuzzing/artifact-workflow-target-registry-v1.json"
    )
    stories = [
        story_gate_summary("2.1", story_2_1),
        story_gate_summary("2.2", story_2_2),
        story_gate_summary("2.3", story_2_3),
        story_gate_summary("2.4", story_2_4),
    ]
    platform_records = platform["synthetic_record_set"]["records"]
    result_identities = [
        {
            "record_id": item["record_id"],
            "record_sha256": item["record_sha256"],
            "fixture_id": item["result"]["fixture_set"]["id"],
            "fixture_sha256": item["result"]["fixture_set"]["sha256"],
            "platform_distribution": item["environment"]["distribution_id"],
            "platform_version": item["environment"]["distribution_version"],
            "platform_architecture": item["environment"]["architecture"],
            "build_id": item["result"]["build"]["id"],
            "build_sha256": item["result"]["build"]["sha256"],
            "model_id": item["result"]["model"]["id"],
            "model_sha256": item["result"]["model"]["sha256"],
            "runtime_id": item["result"]["runtime"]["id"],
            "runtime_sha256": item["result"]["runtime"]["sha256"],
            "policy_id": item["result"]["policy"]["id"],
            "policy_sha256": item["result"]["policy"]["sha256"],
        }
        for item in platform_records
    ]
    metrics_by_id = {item["metric_id"]: item for item in workflow_metrics["metrics"]}
    duplicate_effects = metrics_by_id["duplicate_effect_count"]["golden"][
        "duplicate_effect_count"
    ]
    approval_bypasses = metrics_by_id["approval_bypass"]["golden"][
        "approval_bypass_count"
    ]
    oracle_summary = fuzz_extension["deterministic_oracle_summary"]
    reproduction_hashes = {item["output_set_sha256"] for item in reproducibility["runs"]}
    if (
        reproducibility.get("run_count") != 2
        or reproducibility.get("output_count") != 11
        or reproducibility.get("byte_identical") is not True
        or len(reproduction_hashes) != 1
        or golden.get("outcome_count") != 77
        or golden.get("non_success_terminal_count") != 60
        or golden.get("effect_executed") is not False
        or duplicate_effects != 0
        or approval_bypasses != 0
        or fuzz_extension.get("registration_count") != 8
        or oracle_summary.get("seed_case_observation_count") != 153
        or oracle_summary.get("passing_oracle_count") != 8
        or oracle_summary.get("fixed_seed_oracles_are_real_fuzzing") is not False
    ):
        raise ValueError("Sprint 2 Story 2.3 aggregate criterion is incomplete or widened")
    return {
        "schema_version": 1,
        "sprint_id": 2,
        "status": "blocked-open-dependencies-full-protocol-and-platform",
        "reviewed_commit": REVIEWED_COMMIT,
        "reviewed_tree": REVIEWED_TREE,
        "acceptance_criteria": [
            {
                "criterion_id": "2.AC1",
                "status": "pass",
                "corpus_reproduction_run_count": corpus["reproduction"]["run_count"],
                "corpus_runs_byte_identical": corpus["reproduction"][
                    "runs_byte_identical"
                ],
                "checked_corpus_match": corpus["reproduction"][
                    "checked_artifacts_match"
                ],
                "fake_boundary_clean_run_count": baseline["summary"][
                    "clean_run_count"
                ],
                "fake_boundary_comparisons_exact": all(
                    baseline["comparison"].values()
                ),
                "evidence": [
                    "artifacts/sprints/sprint-2/story-2.1/corpus-reproducibility-report.json",
                    "artifacts/sprints/sprint-2/story-2.2/baseline-reconciliation-report.json",
                ],
            },
            {
                "criterion_id": "2.AC2",
                "status": "pass",
                "archive_mutation_detected_before_use": corpus["corruption"][
                    "archive_mutation_detected_before_use"
                ],
                "manifest_mutation_detected_before_use": corpus["corruption"][
                    "manifest_mutation_detected_before_use"
                ],
                "corrupt_artifact_persisted": corpus["corruption"][
                    "corrupt_artifact_persisted"
                ],
                "evidence": "artifacts/sprints/sprint-2/story-2.1/corpus-reproducibility-report.json",
            },
            {
                "criterion_id": "2.AC3",
                "status": "pass",
                "adapter_count": adapters["summary"]["adapter_count"],
                "mode_count": adapters["summary"]["mode_count"],
                "matrix_case_count": adapters["summary"]["matrix_case_count"],
                "all_typed_outcomes_matched": adapters["summary"][
                    "all_typed_outcomes_matched"
                ],
                "all_cleanup_passed": adapters["summary"]["all_cleanup_passed"],
                "evidence": "artifacts/sprints/sprint-2/story-2.1/adapter-mode-verification-report.json",
            },
            {
                "criterion_id": "2.AC4",
                "status": "pass",
                "result_identity_count": len(result_identities),
                "result_identities": result_identities,
                "ambient_environment_values_recorded": platform[
                    "ambient_environment_values_recorded"
                ],
                "evidence": "artifacts/sprints/sprint-2/story-2.1/platform-result-recorder-report.json",
            },
            {
                "criterion_id": "2.AC5",
                "status": "pass",
                "blocking_finding_count": scan["summary"][
                    "blocking_finding_count"
                ],
                "seeded_category_count": scan["summary"]["seeded_category_count"],
                "seeded_categories_detected": scan["summary"][
                    "seeded_categories_detected"
                ],
                "raw_sensitive_values_retained": scan[
                    "raw_sensitive_values_retained"
                ],
                "network_calls_performed": scan["network_calls_performed"],
                "evidence": "artifacts/sprints/sprint-2/story-2.1/fixture-security-scan-report.json",
            },
            {
                "criterion_id": "2.AC6",
                "status": "pass",
                "clean_reproduction_run_count": reproducibility["run_count"],
                "reproduced_output_count": reproducibility["output_count"],
                "reproduced_output_set_sha256": next(iter(reproduction_hashes)),
                "golden_outcome_count": golden["outcome_count"],
                "golden_non_success_terminal_count": golden[
                    "non_success_terminal_count"
                ],
                "duplicate_effect_count": duplicate_effects,
                "approval_bypass_count": approval_bypasses,
                "fixed_seed_registration_count": fuzz_extension[
                    "registration_count"
                ],
                "fixed_seed_observation_count": oracle_summary[
                    "seed_case_observation_count"
                ],
                "manual_fuzz_campaign_status": fuzz_extension["manual_campaign"][
                    "execution_status"
                ],
                "evidence": [
                    "fixtures/artifact-evaluation/v1/reproducibility-report.json",
                    "fixtures/artifact-evaluation/v1/golden-manifest-v1.json",
                    "fixtures/artifact-evaluation/v1/workflow-golden-metrics.json",
                    "fuzzing/artifact-workflow-target-registry-v1.json",
                ],
            },
        ],
        "story_gates": stories,
        "universal_definition_of_done": {
            "control_ids": list(G_DOD_IDS),
            "story_count": 4,
            "all_non_platform_controls_pass_or_not_applicable": True,
            "blocking_controls": ["G-DOD-10"],
            "macos_evidence_substitution": "prohibited",
            "dependency_or_protocol_evidence_substitution": "prohibited",
        },
        "blockers": [
            "story-1.3-aggregate-gate-blocked",
            "story-2.3-aggregate-gate-blocked",
            "rv-51-product-parser-crash-resume-installed-client-and-native-platform-evidence-incomplete",
            "supported-platform-installed-product-matrix-incomplete",
        ],
        "summary": {
            "acceptance_criteria_passed": 6,
            "acceptance_criteria_failed": 0,
            "story_gate_count": 4,
            "current_public_synthetic_scope_complete": True,
            "sprint_checkbox_complete": False,
            "blocking_story_count": 4,
            "blocking_story_ids": ["2.1", "2.2", "2.3", "2.4"],
            "blocking_control_count": 1,
            "blocking_controls": ["G-DOD-10"],
        },
        "independent_review": {
            "reviewer_id": "agentmage-sprint-2-independent-gate-v1",
            "review_type": "automated-independent-aggregate-review",
            "reviewed_commit": REVIEWED_COMMIT,
            "reviewed_tree": REVIEWED_TREE,
            "reviewed_artifacts": reviewed_artifacts(root),
            "finding_count": 0,
            "findings": [],
            "disposition": "pass-current-public-synthetic-scope-blocked-dependencies-full-protocol-and-platform",
            "external_human_review_claim": "none",
        },
        "macos": {
            "status": "blocked-macos",
            "execution_performed": False,
            "evidence_substitution": "prohibited",
            "support_claim": "none",
        },
        "dependency_substitution_permitted": False,
        "full_protocol_substitution_permitted": False,
        "product_runtime_claim": "none",
        "installed_product_claim": "none",
        "product_acceptance_claim": "none",
        "release_claim": "none",
    }


def validate_report(value: Any, root: Path = ROOT) -> list[str]:
    if not isinstance(value, dict):
        return ["Sprint 2 gate report must be an object"]
    failures: list[str] = []
    if (
        value.get("schema_version") != 1
        or value.get("sprint_id") != 2
        or value.get("status") != "blocked-open-dependencies-full-protocol-and-platform"
        or value.get("reviewed_commit") != REVIEWED_COMMIT
        or value.get("reviewed_tree") != REVIEWED_TREE
    ):
        failures.append("Sprint 2 gate identity or review boundary is invalid")
    criteria = value.get("acceptance_criteria", [])
    if [item.get("criterion_id") for item in criteria] != [
        "2.AC1",
        "2.AC2",
        "2.AC3",
        "2.AC4",
        "2.AC5",
        "2.AC6",
    ] or any(item.get("status") != "pass" for item in criteria):
        failures.append("Sprint 2 acceptance criteria did not close exactly")
    stories = value.get("story_gates", [])
    if [item.get("story_id") for item in stories] != ["2.1", "2.2", "2.3", "2.4"] or any(
        not str(item.get("status", "")).startswith("blocked-")
        or item.get("acceptance_criteria_failed") != 0
        or item.get("current_local_scope_complete") is not True
        or item.get("story_checkbox_complete") is not False
        or item.get("blocking_controls") != ["G-DOD-10"]
        or item.get("dod_control_ids") != list(G_DOD_IDS)
        or item.get("dod_blocking_controls") != ["G-DOD-10"]
        for item in stories
    ):
        failures.append("Sprint 2 story-gate aggregation is invalid")
    if value.get("universal_definition_of_done") != {
        "control_ids": list(G_DOD_IDS),
        "story_count": 4,
        "all_non_platform_controls_pass_or_not_applicable": True,
        "blocking_controls": ["G-DOD-10"],
        "macos_evidence_substitution": "prohibited",
        "dependency_or_protocol_evidence_substitution": "prohibited",
    }:
        failures.append("Sprint 2 Definition-of-Done aggregation is invalid")
    if value.get("blockers") != [
        "story-1.3-aggregate-gate-blocked",
        "story-2.3-aggregate-gate-blocked",
        "rv-51-product-parser-crash-resume-installed-client-and-native-platform-evidence-incomplete",
        "supported-platform-installed-product-matrix-incomplete",
    ]:
        failures.append("Sprint 2 dependency, protocol, or platform blocker set is invalid")
    if value.get("summary") != {
        "acceptance_criteria_passed": 6,
        "acceptance_criteria_failed": 0,
        "story_gate_count": 4,
        "current_public_synthetic_scope_complete": True,
        "sprint_checkbox_complete": False,
        "blocking_story_count": 4,
        "blocking_story_ids": ["2.1", "2.2", "2.3", "2.4"],
        "blocking_control_count": 1,
        "blocking_controls": ["G-DOD-10"],
    }:
        failures.append("Sprint 2 gate summary overclaimed or omitted a blocker")
    review = value.get("independent_review", {})
    if (
        review.get("reviewed_commit") != REVIEWED_COMMIT
        or review.get("reviewed_tree") != REVIEWED_TREE
        or review.get("finding_count") != 0
        or review.get("findings") != []
        or review.get("external_human_review_claim") != "none"
    ):
        failures.append("Sprint 2 independent review record is invalid")
    if value.get("macos") != {
        "status": "blocked-macos",
        "execution_performed": False,
        "evidence_substitution": "prohibited",
        "support_claim": "none",
    }:
        failures.append("Sprint 2 gate made an invalid macOS claim")
    if (
        value.get("dependency_substitution_permitted") is not False
        or value.get("full_protocol_substitution_permitted") is not False
        or value.get("product_runtime_claim") != "none"
        or value.get("installed_product_claim") != "none"
        or value.get("product_acceptance_claim") != "none"
        or value.get("release_claim") != "none"
    ):
        failures.append("Sprint 2 gate made a product or release claim")
    try:
        expected = build_report(root)
    except (OSError, ValueError, KeyError, TypeError, subprocess.TimeoutExpired) as error:
        failures.append(f"cannot rebuild Sprint 2 gate report: {error}")
    else:
        if value != expected:
            failures.append("Sprint 2 gate report is stale or non-deterministic")
    return failures


def write_report(root: Path = ROOT) -> None:
    write_atomic(root / REPORT_PATH.relative_to(ROOT), canonical_json(build_report(root)))


def check_report(root: Path = ROOT) -> list[str]:
    try:
        report = read_json(root / REPORT_PATH.relative_to(ROOT))
    except (OSError, json.JSONDecodeError) as error:
        return [f"cannot read Sprint 2 gate report: {error}"]
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
        print(f"Sprint 2 gate failed: {error}", file=sys.stderr)
        return 1
    if failures:
        for failure in failures:
            print(f"Sprint 2 gate failed: {failure}", file=sys.stderr)
        return 1
    print("Sprint 2 current public-synthetic scope passed with dependency, full-protocol, and platform blockers preserved")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
