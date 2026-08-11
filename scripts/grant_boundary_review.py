#!/usr/bin/env python3
"""Build and verify the independent automated Story 5 grant-boundary review."""

from __future__ import annotations

import argparse
import hashlib
import json
import os
import re
import subprocess
import sys
import tempfile
from pathlib import Path
from typing import Any, Callable


ROOT = Path(__file__).resolve().parents[1]
if str(ROOT) not in sys.path:
    sys.path.insert(0, str(ROOT))
REPORT_PATH = ROOT / "artifacts/sprints/sprint-5/story-5.1/grant-boundary-review.json"
SUBJECT_REPORTS = (
    "grant-policy-reference-report.json",
    "grant-state-report.json",
    "grant-review-fixture-report.json",
    "adversarial-grant-corpus-report.json",
    "grant-race-replay-report.json",
    "authority-escalation-report.json",
    "grant-stale-dispatch-report.json",
)
REPORT_DIRECTORY = "artifacts/sprints/sprint-5/story-5.1"
EXPECTED_SUBJECTS: dict[str, dict[str, Any]] = {
    "grant-policy-reference-report.json": {
        "artifact_id": "grant-schema-policy-decision-reference",
        "task_id": "5.1.2.1",
        "coverage": {
            "grant_field_count": 29,
            "grant_operation_count": 15,
            "policy_denial_scope_count": 15,
            "strict_explicit_denial_count": 12,
        },
    },
    "grant-state-report.json": {
        "artifact_id": "grant-state-transition-reference",
        "task_id": "5.1.2.2",
        "coverage": {
            "grant_status_count": 6,
            "implemented_transition_count": 8,
            "operation_transition_count": 5,
            "session_parent_transition_count": 3,
        },
    },
    "grant-review-fixture-report.json": {
        "artifact_id": "grant-approval-denial-review-fixtures",
        "task_id": "5.1.2.3",
        "coverage": {
            "approval_forbidden_authority_field_count": 0,
            "denial_resulting_status": "invalidated",
            "denial_resulting_use_count": 0,
            "fixture_count": 3,
        },
    },
    "adversarial-grant-corpus-report.json": {
        "artifact_id": "adversarial-grant-corpus",
        "task_id": "5.1.2.4",
        "coverage": {
            "admitted_attempt_count": 0,
            "case_count": 560,
            "cases_per_class": 40,
            "mutation_class_count": 14,
        },
    },
    "grant-race-replay-report.json": {
        "artifact_id": "grant-race-replay-verification",
        "task_id": "5.1.3.2",
        "coverage": {
            "maximum_effect_count": 1,
            "maximum_worker_start_count": 1,
            "replay_success_count": 0,
            "scenario_count": 5,
        },
    },
    "authority-escalation-report.json": {
        "artifact_id": "authority-escalation-denial-verification",
        "task_id": "5.1.3.3",
        "coverage": {
            "admitted_authority_count": 0,
            "escalation_kind_count": 4,
            "receipt_count": 28,
            "source_count": 7,
        },
    },
    "grant-stale-dispatch-report.json": {
        "artifact_id": "post-approval-stale-dispatch-verification",
        "task_id": "5.1.3.4",
        "coverage": {
            "atomic_consumption_count": 0,
            "effect_count": 0,
            "mutation_count": 4,
            "worker_start_count": 0,
        },
    },
}
OPEN_BOUNDARIES = (
    "authenticated user decisions and process identities",
    "crash-durable grant and policy transactions",
    "canonical descriptor-relative platform paths",
    "isolated production worker execution",
    "durable keyed receipts and audit chaining",
    "effect reconciliation after interruption",
    "explicit grant revocation",
    "macOS implementation and execution evidence",
)
SOURCE_PATHS = (
    "SECURITY-REVIEW.md",
    "docs/architecture/grant-policy-reference.md",
    "docs/architecture/grant-state-transitions.md",
    "fixtures/grants/v1/manifest.json",
    "fixtures/grants/adversarial/v1/corpus.json",
    "fixtures/grants/adversarial/v1/manifest.json",
    "kernel/contracts/src/approval.rs",
    "kernel/contracts/src/grant.rs",
    "kernel/engine/src/approval.rs",
    "kernel/engine/src/authority.rs",
    "kernel/engine/src/grants.rs",
    "kernel/engine/src/policy.rs",
    *(f"{REPORT_DIRECTORY}/{name}" for name in SUBJECT_REPORTS),
    "scripts/grant_policy_reference.py",
    "scripts/grant_state_transitions.py",
    "scripts/grant_review_fixtures.py",
    "scripts/adversarial_grant_corpus.py",
    "scripts/grant_race_replay.py",
    "scripts/authority_escalation_matrix.py",
    "scripts/grant_stale_dispatch.py",
    "scripts/grant_boundary_review.py",
    "tests/test_grant_boundary_review.py",
)


class GrantBoundaryReviewError(ValueError):
    """Raised when Story 5 boundary evidence is stale, incomplete, or overclaimed."""


def canonical_json(value: Any) -> bytes:
    return (json.dumps(value, indent=2, sort_keys=True) + "\n").encode("utf-8")


def sha256_bytes(value: bytes) -> str:
    return hashlib.sha256(value).hexdigest()


def write_atomic(path: Path, content: bytes) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    descriptor, temporary_name = tempfile.mkstemp(
        prefix=".agentmage-grant-boundary-review-", dir=path.parent
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


def load_subjects(root: Path = ROOT) -> dict[str, Any]:
    subjects = {}
    for name in SUBJECT_REPORTS:
        try:
            subjects[name] = json.loads(
                (root / REPORT_DIRECTORY / name).read_text(encoding="utf-8")
            )
        except (OSError, json.JSONDecodeError) as error:
            raise GrantBoundaryReviewError(f"cannot read subject report: {name}") from error
    return subjects


def validate_subjects(subjects: Any) -> list[str]:
    if not isinstance(subjects, dict) or tuple(subjects) != SUBJECT_REPORTS:
        return ["subject report closure or order changed"]
    failures: list[str] = []
    for name, expected in EXPECTED_SUBJECTS.items():
        subject = subjects.get(name)
        if not isinstance(subject, dict):
            failures.append(f"subject report is not an object: {name}")
            continue
        if (
            subject.get("schema_version") != 1
            or subject.get("status") != "pass-shared-linux-reference"
            or subject.get("artifact_id") != expected["artifact_id"]
            or subject.get("task_id") != expected["task_id"]
            or re.fullmatch(r"[0-9a-f]{40}", str(subject.get("reference_revision"))) is None
        ):
            failures.append(f"subject identity or status changed: {name}")
        coverage = subject.get("coverage")
        if not isinstance(coverage, dict) or any(
            coverage.get(field) != value for field, value in expected["coverage"].items()
        ):
            failures.append(f"subject security coverage changed: {name}")
        verification = subject.get("verification")
        if not isinstance(verification, dict) or not verification or set(verification.values()) != {"pass"}:
            failures.append(f"subject verification is not closed: {name}")
        platform = subject.get("platform_status")
        if not isinstance(platform, dict) or platform.get("macos") != "blocked-macos":
            failures.append(f"subject macOS blocker is absent: {name}")
        limitations = subject.get("limitations")
        if not isinstance(limitations, list) or not limitations:
            failures.append(f"subject limitations are absent: {name}")
    return failures


def run_source_checkers(root: Path = ROOT) -> None:
    if root != ROOT:
        return
    from scripts.adversarial_grant_corpus import check_report as check_adversarial
    from scripts.authority_escalation_matrix import check_report as check_escalation
    from scripts.grant_policy_reference import check_report as check_policy
    from scripts.grant_race_replay import check_report as check_race
    from scripts.grant_review_fixtures import check_report as check_fixtures
    from scripts.grant_stale_dispatch import check_report as check_stale
    from scripts.grant_state_transitions import check_report as check_state

    checks: tuple[Callable[[Path], None], ...] = (
        check_policy,
        check_state,
        check_fixtures,
        check_adversarial,
        check_race,
        check_escalation,
        check_stale,
    )
    for check in checks:
        check(root)


def independent_review(root: Path = ROOT) -> dict[str, Any]:
    run_source_checkers(root)
    subjects = load_subjects(root)
    failures = validate_subjects(subjects)
    security_review = (root / "SECURITY-REVIEW.md").read_text(encoding="utf-8")
    for requirement_id in (
        "SR-ACC-001",
        "SR-ACC-002",
        "SR-ACC-003",
        "SR-ACC-007",
        "SR-AI-004",
        "SR-AI-005",
        "SR-OPS-001",
        "SR-TST-012",
    ):
        if f"`{requirement_id}`" not in security_review:
            failures.append(f"security requirement is absent: {requirement_id}")
    if failures:
        raise GrantBoundaryReviewError("; ".join(failures))
    return {
        "reviewed_subject_count": len(subjects),
        "mutation_seed_count": subjects["adversarial-grant-corpus-report.json"]["coverage"]["case_count"],
        "race_scenario_count": subjects["grant-race-replay-report.json"]["coverage"]["scenario_count"],
        "state_transition_count": subjects["grant-state-report.json"]["coverage"]["implemented_transition_count"],
        "authority_escalation_case_count": subjects["authority-escalation-report.json"]["coverage"]["receipt_count"],
        "stale_dispatch_case_count": subjects["grant-stale-dispatch-report.json"]["coverage"]["mutation_count"],
        "admitted_mutation_count": subjects["adversarial-grant-corpus-report.json"]["coverage"]["admitted_attempt_count"],
        "admitted_escalation_count": subjects["authority-escalation-report.json"]["coverage"]["admitted_authority_count"],
        "replay_success_count": subjects["grant-race-replay-report.json"]["coverage"]["replay_success_count"],
        "open_product_boundary_count": len(OPEN_BOUNDARIES),
    }


def git_revision(root: Path = ROOT) -> str:
    completed = subprocess.run(
        ["git", "rev-parse", "HEAD"],
        cwd=root,
        check=False,
        stdout=subprocess.PIPE,
        stderr=subprocess.DEVNULL,
        timeout=10,
    )
    revision = completed.stdout.decode("ascii", "strict").strip()
    if completed.returncode != 0 or re.fullmatch(r"[0-9a-f]{40}", revision) is None:
        raise GrantBoundaryReviewError("source revision is unavailable")
    return revision


def git_file(revision: str, relative: str, root: Path = ROOT) -> bytes:
    completed = subprocess.run(
        ["git", "show", f"{revision}:{relative}"],
        cwd=root,
        check=False,
        stdout=subprocess.PIPE,
        stderr=subprocess.DEVNULL,
        timeout=10,
    )
    if completed.returncode != 0:
        raise GrantBoundaryReviewError(f"source is absent at revision: {relative}")
    return completed.stdout


def build_report(reference_revision: str, root: Path = ROOT) -> dict[str, Any]:
    coverage = independent_review(root)
    ancestor = subprocess.run(
        ["git", "merge-base", "--is-ancestor", reference_revision, "HEAD"],
        cwd=root,
        check=False,
        stdout=subprocess.DEVNULL,
        stderr=subprocess.DEVNULL,
        timeout=10,
    )
    if ancestor.returncode != 0:
        raise GrantBoundaryReviewError("reference revision is not an ancestor of HEAD")
    sources = []
    for relative in SOURCE_PATHS:
        committed = git_file(reference_revision, relative, root)
        current = (root / relative).read_bytes()
        if committed != current:
            raise GrantBoundaryReviewError(f"source differs from reference revision: {relative}")
        sources.append({"path": relative, "sha256": sha256_bytes(committed)})
    return {
        "schema_version": 1,
        "task_id": "5.1.3.5",
        "artifact_id": "grant-boundary-independent-automated-review",
        "status": "pass-shared-linux-story-scope",
        "reference_revision": reference_revision,
        "review_type": "independent-automated-boundary-review",
        "reviewer_identity": "agentmage-grant-boundary-review-v1",
        "external_human_review_status": "not-performed",
        "subjects": [
            {
                "path": f"{REPORT_DIRECTORY}/{name}",
                "reference_revision": load_subjects(root)[name]["reference_revision"],
                "sha256": sha256_bytes((root / REPORT_DIRECTORY / name).read_bytes()),
            }
            for name in SUBJECT_REPORTS
        ],
        "sources": sources,
        "coverage": coverage,
        "verification": {
            "source_checker_reexecution": "pass",
            "subject_revision_and_hash_binding": "pass",
            "grant_schema_and_policy_closure": "pass",
            "grant_state_machine_closure": "pass",
            "mutation_result_reconciliation": "pass",
            "race_and_replay_reconciliation": "pass",
            "authority_escalation_reconciliation": "pass",
            "stale_dispatch_reconciliation": "pass",
            "limitation_and_platform_disclosure": "pass",
        },
        "findings": [],
        "open_product_boundaries": list(OPEN_BOUNDARIES),
        "product_requirement_completion_claim": "none",
        "release_claim": "none",
        "macos_execution_status": "blocked-macos",
        "macos_evidence_substituted": False,
    }


def validate_report(value: Any) -> list[str]:
    if not isinstance(value, dict):
        return ["grant boundary review must be an object"]
    failures: list[str] = []
    if (
        value.get("schema_version") != 1
        or value.get("task_id") != "5.1.3.5"
        or value.get("artifact_id") != "grant-boundary-independent-automated-review"
        or value.get("status") != "pass-shared-linux-story-scope"
        or value.get("review_type") != "independent-automated-boundary-review"
        or value.get("reviewer_identity") != "agentmage-grant-boundary-review-v1"
        or value.get("external_human_review_status") != "not-performed"
    ):
        failures.append("grant boundary review identity changed")
    coverage = value.get("coverage")
    if not isinstance(coverage, dict) or coverage != {
        "reviewed_subject_count": 7,
        "mutation_seed_count": 560,
        "race_scenario_count": 5,
        "state_transition_count": 8,
        "authority_escalation_case_count": 28,
        "stale_dispatch_case_count": 4,
        "admitted_mutation_count": 0,
        "admitted_escalation_count": 0,
        "replay_success_count": 0,
        "open_product_boundary_count": 8,
    }:
        failures.append("grant boundary review coverage changed")
    verification = value.get("verification")
    if not isinstance(verification, dict) or len(verification) != 9 or set(verification.values()) != {"pass"}:
        failures.append("grant boundary review verification is incomplete")
    if value.get("findings") != [] or value.get("open_product_boundaries") != list(OPEN_BOUNDARIES):
        failures.append("grant boundary findings or limitations changed")
    if (
        value.get("product_requirement_completion_claim") != "none"
        or value.get("release_claim") != "none"
        or value.get("macos_execution_status") != "blocked-macos"
        or value.get("macos_evidence_substituted") is not False
    ):
        failures.append("grant boundary review made an unsupported claim")
    return failures


def write_report(root: Path = ROOT) -> None:
    write_atomic(REPORT_PATH, canonical_json(build_report(git_revision(root), root)))


def check_report(root: Path = ROOT) -> None:
    try:
        actual = json.loads(REPORT_PATH.read_text(encoding="utf-8"))
        revision = actual["reference_revision"]
    except (OSError, KeyError, TypeError, json.JSONDecodeError) as error:
        raise GrantBoundaryReviewError(f"cannot read grant boundary review: {error}") from error
    failures = validate_report(actual)
    if not isinstance(revision, str) or actual != build_report(revision, root):
        failures.append("grant boundary review is stale or malformed")
    if failures:
        raise GrantBoundaryReviewError("; ".join(failures))


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--write", action="store_true")
    args = parser.parse_args()
    try:
        if args.write:
            write_report()
        check_report()
    except (OSError, GrantBoundaryReviewError, subprocess.SubprocessError) as error:
        print(f"Grant boundary review failed: {error}", file=sys.stderr)
        return 1
    print("Independent automated Story 5 grant-boundary review validated")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
