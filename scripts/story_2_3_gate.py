#!/usr/bin/env python3
"""Independently evaluate Story 2.3 without widening synthetic evidence."""

from __future__ import annotations

import argparse
import hashlib
import json
import os
import subprocess
import sys
import tempfile
from pathlib import Path
from typing import Any, Final


ROOT: Final = Path(__file__).resolve().parents[1]
EVIDENCE_DIR: Final = ROOT / "artifacts/sprints/sprint-2/story-2.3"
REPORT_PATH: Final = EVIDENCE_DIR / "story-gate-report.json"
RAW_PATH: Final = EVIDENCE_DIR / "story-gate-results.log"
REVIEWED_COMMIT: Final = "0a8c3150e77a49a30ed24fba5d0b6a4d33370401"
REVIEWED_TREE: Final = "387314bbe04e69da2d4288d76abaa0ae09a2a7d2"
VALIDATOR_COMMANDS: Final = (
    ("python3", "scripts/artifact_evaluation_artifact_metrics.py"),
    ("python3", "scripts/artifact_evaluation_workflow_metrics.py"),
    ("python3", "scripts/artifact_evaluation_golden_gate.py"),
    ("python3", "scripts/artifact_evaluation_reproducibility.py"),
    ("python3", "scripts/fixture_security_scan.py"),
)
VALIDATOR_MARKERS: Final = (
    "Validated 10 artifact golden metrics",
    "Validated 10 workflow golden metrics",
    "Validated versioned golden gate with 77 visible outcomes",
    "Validated 11 byte-identical outputs across two clean roots",
    "Sprint 2 fixtures and generated artifacts passed the security scan",
)
REVIEWED_PATHS: Final = (
    "scripts/artifact_evaluation_text_log_fixtures.py",
    "scripts/artifact_evaluation_document_fixtures.py",
    "scripts/artifact_evaluation_lifecycle_scenarios.py",
    "scripts/artifact_evaluation_plan_fixtures.py",
    "scripts/artifact_evaluation_crash_fixtures.py",
    "scripts/artifact_evaluation_terminal_fixtures.py",
    "scripts/artifact_evaluation_artifact_metrics.py",
    "scripts/artifact_evaluation_workflow_metrics.py",
    "scripts/artifact_evaluation_golden_gate.py",
    "scripts/artifact_evaluation_reproducibility.py",
    "scripts/artifact_evaluation_fuzz_registration.py",
    "scripts/fixture_security_scan.py",
    "fixtures/artifact-evaluation/v1/text-reference-manifest.json",
    "fixtures/artifact-evaluation/v1/text-reference-corpus-v1.zip",
    "fixtures/artifact-evaluation/v1/document-variant-manifest.json",
    "fixtures/artifact-evaluation/v1/document-variant-corpus-v1.zip",
    "fixtures/artifact-evaluation/v1/lifecycle-scenarios.json",
    "fixtures/artifact-evaluation/v1/workflow-plan-fixtures.json",
    "fixtures/artifact-evaluation/v1/workflow-crash-points.json",
    "fixtures/artifact-evaluation/v1/workflow-terminal-outcomes.json",
    "fixtures/artifact-evaluation/v1/artifact-golden-metrics.json",
    "fixtures/artifact-evaluation/v1/workflow-golden-metrics.json",
    "fixtures/artifact-evaluation/v1/golden-manifest-v1.json",
    "fixtures/artifact-evaluation/v1/reproducibility-report.json",
)
REQUIRED_TASK_MARKERS: Final = (
    *(f"- [x] **Task 2.3.{task} -" for task in range(1, 5)),
    *(f"  - [x] **Sub-task 2.3.{task}.{sub}:" for task in range(1, 5) for sub in range(1, 4)),
    *(f"- [x] **Story AC 2.3.AC{criterion}:" for criterion in range(1, 4)),
)
G_DOD_IDS: Final = tuple(f"G-DOD-{index:02d}" for index in range(1, 14))
BLOCKERS: Final = (
    "story-2.1-acceptance-gate-open",
    "story-2.2-acceptance-gate-open",
    "supported-platform-installed-product-evidence-incomplete",
)


def canonical_json(value: Any) -> bytes:
    return (json.dumps(value, indent=2, sort_keys=True) + "\n").encode("utf-8")


def sha256_bytes(value: bytes) -> str:
    return hashlib.sha256(value).hexdigest()


def read_json(path: Path) -> Any:
    return json.loads(path.read_text(encoding="utf-8"))


def write_atomic(path: Path, content: bytes) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    descriptor, temporary_name = tempfile.mkstemp(prefix=".agentmage-story-2-3-gate-", dir=path.parent)
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


def git_output(*arguments: str, binary: bool = False) -> bytes | str:
    result = subprocess.run(
        ["git", *arguments], cwd=ROOT, stdin=subprocess.DEVNULL, stdout=subprocess.PIPE,
        stderr=subprocess.PIPE, check=False, timeout=15,
    )
    if result.returncode != 0:
        raise ValueError(f"git review operation failed: {' '.join(arguments)}")
    return result.stdout if binary else result.stdout.decode("utf-8").strip()


def reviewed_artifacts() -> list[dict[str, Any]]:
    if git_output("rev-parse", REVIEWED_COMMIT) != REVIEWED_COMMIT:
        raise ValueError("Story 2.3 reviewed commit is unavailable")
    if git_output("show", "-s", "--format=%T", REVIEWED_COMMIT) != REVIEWED_TREE:
        raise ValueError("Story 2.3 reviewed tree changed")
    records = []
    for path in REVIEWED_PATHS:
        committed = git_output("show", f"{REVIEWED_COMMIT}:{path}", binary=True)
        current = ROOT / path
        if not current.is_file() or current.read_bytes() != committed:
            raise ValueError(f"reviewed Story 2.3 artifact changed after review: {path}")
        records.append({"path": path, "byte_length": len(committed), "sha256": sha256_bytes(committed)})
    return records


def task_completion(tasks_text: str) -> list[str]:
    return [marker for marker in REQUIRED_TASK_MARKERS if marker not in tasks_text]


def dependency_state(tasks_text: str) -> list[dict[str, str]]:
    expected = (
        ("1.2", "#### [x] Story 1.2 -", "complete"),
        ("2.1", "#### [ ] Story 2.1 -", "blocked-open-acceptance"),
        ("2.2", "#### [ ] Story 2.2 -", "blocked-open-acceptance"),
    )
    states = []
    for story_id, marker, status in expected:
        if marker not in tasks_text:
            raise ValueError(f"Story 2.3 dependency state changed: {story_id}")
        states.append({"story_id": story_id, "status": status})
    return states


def universal_dod() -> list[dict[str, str]]:
    statuses = {control_id: "pass-current-story-scope" for control_id in G_DOD_IDS}
    statuses["G-DOD-04"] = "pass-fixtures-mint-no-authority"
    statuses["G-DOD-06"] = "pass-no-production-tool-attempt"
    statuses["G-DOD-07"] = "pass-public-synthetic-data-only"
    statuses["G-DOD-08"] = "pass-clean-root-generation-no-user-file-effects"
    statuses["G-DOD-10"] = "blocked-supported-platform-installed-product-matrix"
    statuses["G-DOD-12"] = "pass-automated-independent-aggregate-review"
    return [{"control_id": control_id, "status": statuses[control_id]} for control_id in G_DOD_IDS]


def capture_validators() -> tuple[str, int]:
    chunks = []
    for command in VALIDATOR_COMMANDS:
        result = subprocess.run(
            command, cwd=ROOT, stdout=subprocess.PIPE, stderr=subprocess.STDOUT,
            text=True, check=False,
        )
        chunks.append(f"$ {' '.join(command)}\n{result.stdout.rstrip()}\n")
        if result.returncode != 0:
            return "".join(chunks), result.returncode
    return "".join(chunks), 0


def validate_raw(value: str) -> list[str]:
    failures = [f"Story 2.3 gate results missing marker: {marker}" for marker in VALIDATOR_MARKERS if marker not in value]
    for prohibited in ("Traceback", "FAILED", "validation failed"):
        if prohibited in value:
            failures.append(f"Story 2.3 gate results contain prohibited marker: {prohibited}")
    return failures


def acceptance_criteria() -> list[dict[str, Any]]:
    artifact = read_json(ROOT / "fixtures/artifact-evaluation/v1/artifact-golden-metrics.json")
    workflow = read_json(ROOT / "fixtures/artifact-evaluation/v1/workflow-golden-metrics.json")
    golden = read_json(ROOT / "fixtures/artifact-evaluation/v1/golden-manifest-v1.json")
    reproducibility = read_json(ROOT / "fixtures/artifact-evaluation/v1/reproducibility-report.json")
    security = read_json(ROOT / "artifacts/sprints/sprint-2/story-2.1/fixture-security-scan-report.json")
    if (
        artifact.get("metric_count") != 10
        or golden.get("outcome_count") != 77
        or golden.get("non_success_terminal_count") != 60
        or golden.get("effect_executed") is not False
    ):
        raise ValueError("Story 2.3.AC1 artifact accounting is incomplete")
    if (
        workflow.get("metric_count") != 10
        or workflow.get("authority_minted") is not False
        or workflow.get("effect_executed") is not False
    ):
        raise ValueError("Story 2.3.AC2 workflow accounting is incomplete")
    if (
        reproducibility.get("run_count") != 2
        or reproducibility.get("output_count") != 11
        or reproducibility.get("byte_identical") is not True
        or security.get("summary", {}).get("blocking_finding_count") != 0
        or security.get("summary", {}).get("unapproved_raw_canary_count") != 0
    ):
        raise ValueError("Story 2.3.AC3 reproducibility or security truth is incomplete")
    return [
        {
            "criterion_id": "2.3.AC1",
            "status": "pass-local-public-synthetic-artifact-corpus",
            "evidence_sha256": sha256_bytes(canonical_json({"artifact": artifact, "golden": golden})),
        },
        {
            "criterion_id": "2.3.AC2",
            "status": "pass-local-public-synthetic-workflow-corpus",
            "evidence_sha256": sha256_bytes(canonical_json({"workflow": workflow, "golden": golden})),
        },
        {
            "criterion_id": "2.3.AC3",
            "status": "pass-local-clean-root-reproducibility-and-security",
            "evidence_sha256": sha256_bytes(canonical_json({"reproducibility": reproducibility, "security": security})),
        },
    ]


def build_report() -> dict[str, Any]:
    tasks_text = (ROOT / "TASKS.md").read_text(encoding="utf-8")
    incomplete = task_completion(tasks_text)
    if incomplete:
        raise ValueError(f"Story 2.3 task or criterion is incomplete: {incomplete[0]}")
    raw = RAW_PATH.read_text(encoding="utf-8")
    raw_failures = validate_raw(raw)
    if raw_failures:
        raise ValueError("; ".join(raw_failures))
    return {
        "schema_version": 1,
        "story_id": "2.3",
        "status": "blocked-open-dependencies-and-platform",
        "reviewed_commit": REVIEWED_COMMIT,
        "reviewed_tree": REVIEWED_TREE,
        "task_count": 4,
        "sub_task_count": 12,
        "acceptance_criteria": acceptance_criteria(),
        "dependencies": dependency_state(tasks_text),
        "universal_definition_of_done": universal_dod(),
        "blocking_controls": ["G-DOD-10"],
        "blockers": list(BLOCKERS),
        "independent_review": {
            "reviewer_id": "agentmage-story-2.3-independent-gate-v1",
            "review_type": "automated-independent-aggregate-review",
            "reviewed_commit": REVIEWED_COMMIT,
            "reviewed_tree": REVIEWED_TREE,
            "artifacts": reviewed_artifacts(),
            "finding_count": 0,
            "findings": [],
            "external_human_review_status": "not-performed",
        },
        "validator_results_sha256": sha256_bytes(raw.encode("utf-8")),
        "current_public_synthetic_corpus_scope_complete": True,
        "story_checkbox_complete": False,
        "dependency_substitution_permitted": False,
        "platform_evidence_substituted": False,
        "product_runtime_claim": "none",
        "installed_product_claim": "none",
        "product_acceptance_claim": "none",
        "sprint_completion_claim": False,
        "release_claim": "none",
    }


def validate_report(value: Any, *, verify_current: bool = True) -> list[str]:
    if not isinstance(value, dict):
        return ["Story 2.3 gate report must be an object"]
    failures = []
    if (
        value.get("schema_version") != 1
        or value.get("story_id") != "2.3"
        or value.get("status") != "blocked-open-dependencies-and-platform"
        or value.get("reviewed_commit") != REVIEWED_COMMIT
        or value.get("reviewed_tree") != REVIEWED_TREE
    ):
        failures.append("Story 2.3 gate identity or status is invalid")
    criteria = value.get("acceptance_criteria", [])
    if [item.get("criterion_id") for item in criteria if isinstance(item, dict)] != ["2.3.AC1", "2.3.AC2", "2.3.AC3"]:
        failures.append("Story 2.3 acceptance criterion closure is invalid")
    elif not all(item.get("status", "").startswith("pass-local-") for item in criteria):
        failures.append("Story 2.3 acceptance criterion status is invalid")
    if value.get("universal_definition_of_done") != universal_dod() or value.get("blocking_controls") != ["G-DOD-10"]:
        failures.append("Story 2.3 Definition-of-Done disposition is invalid")
    if value.get("blockers") != list(BLOCKERS):
        failures.append("Story 2.3 blocker set is invalid")
    if value.get("dependencies") != [
        {"story_id": "1.2", "status": "complete"},
        {"story_id": "2.1", "status": "blocked-open-acceptance"},
        {"story_id": "2.2", "status": "blocked-open-acceptance"},
    ]:
        failures.append("Story 2.3 dependency disposition is invalid")
    review = value.get("independent_review", {})
    if (
        review.get("reviewer_id") != "agentmage-story-2.3-independent-gate-v1"
        or review.get("review_type") != "automated-independent-aggregate-review"
        or review.get("reviewed_commit") != REVIEWED_COMMIT
        or review.get("reviewed_tree") != REVIEWED_TREE
        or review.get("finding_count") != 0
        or review.get("findings") != []
        or review.get("external_human_review_status") != "not-performed"
        or len(review.get("artifacts", [])) != len(REVIEWED_PATHS)
    ):
        failures.append("Story 2.3 independent review is invalid")
    if (
        value.get("current_public_synthetic_corpus_scope_complete") is not True
        or value.get("story_checkbox_complete") is not False
        or value.get("dependency_substitution_permitted") is not False
        or value.get("platform_evidence_substituted") is not False
        or value.get("product_runtime_claim") != "none"
        or value.get("installed_product_claim") != "none"
        or value.get("product_acceptance_claim") != "none"
        or value.get("sprint_completion_claim") is not False
        or value.get("release_claim") != "none"
    ):
        failures.append("Story 2.3 gate made an unsupported completion claim")
    if verify_current:
        try:
            expected = build_report()
        except (OSError, ValueError, KeyError, TypeError, json.JSONDecodeError) as error:
            failures.append(f"cannot rebuild Story 2.3 gate report: {error}")
        else:
            if value != expected:
                failures.append("Story 2.3 gate report is stale, incomplete, or widened")
    return failures


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--write", action="store_true")
    args = parser.parse_args()
    try:
        if args.write:
            raw, returncode = capture_validators()
            if returncode != 0:
                sys.stderr.write(raw)
                return 1
            raw_failures = validate_raw(raw)
            if raw_failures:
                raise ValueError("; ".join(raw_failures))
            write_atomic(RAW_PATH, raw.encode("utf-8"))
            write_atomic(REPORT_PATH, canonical_json(build_report()))
        report = read_json(REPORT_PATH)
        failures = validate_report(report)
    except (OSError, ValueError, KeyError, TypeError, json.JSONDecodeError) as error:
        print(f"Story 2.3 gate failed: {error}", file=sys.stderr)
        return 1
    if failures:
        for failure in failures:
            print(f"Story 2.3 gate failed: {failure}", file=sys.stderr)
        return 1
    print("Story 2.3 public synthetic scope passed with dependency and platform blockers preserved")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
