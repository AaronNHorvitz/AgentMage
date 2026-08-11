#!/usr/bin/env python3
"""Build and validate the Story 5.1 product-security evidence map."""

from __future__ import annotations

import argparse
import hashlib
import json
import os
import re
import subprocess
import sys
import tempfile
from pathlib import Path, PurePosixPath
from typing import Any


ROOT = Path(__file__).resolve().parents[1]
if str(ROOT) not in sys.path:
    sys.path.insert(0, str(ROOT))

from scripts.grant_boundary_review import check_report as check_boundary_review


REPORT_PATH = ROOT / "artifacts/sprints/sprint-5/story-5.1/security-evidence-map.json"
EXPECTED_REQUIREMENTS = (
    "SR-ACC-001",
    "SR-ACC-002",
    "SR-ACC-003",
    "SR-ACC-007",
    "SR-AI-004",
    "SR-AI-005",
    "SR-OPS-001",
    "SR-TST-012",
)
EVIDENCE_PATHS = (
    "SECURITY-REVIEW.md",
    "docs/architecture/grant-policy-reference.md",
    "docs/architecture/grant-state-transitions.md",
    "fixtures/grants/v1/manifest.json",
    "fixtures/grants/v1/approval-preview.workspace-read.json",
    "fixtures/grants/v1/policy-denial.argument-mismatch.json",
    "fixtures/grants/v1/denial-receipt.argument-mismatch.json",
    "fixtures/grants/adversarial/v1/corpus.json",
    "fixtures/grants/adversarial/v1/manifest.json",
    "kernel/contracts/src/approval.rs",
    "kernel/contracts/src/grant.rs",
    "kernel/engine/src/approval.rs",
    "kernel/engine/src/authority.rs",
    "kernel/engine/src/grants.rs",
    "kernel/engine/src/policy.rs",
    "kernel/engine/tests/adversarial_grant_corpus.rs",
    "kernel/engine/tests/authority_escalation_matrix.rs",
    "kernel/engine/tests/grant_race_replay.rs",
    "kernel/engine/tests/grant_stale_dispatch.rs",
    "artifacts/sprints/sprint-5/story-5.1/grant-policy-reference-report.json",
    "artifacts/sprints/sprint-5/story-5.1/grant-state-report.json",
    "artifacts/sprints/sprint-5/story-5.1/grant-review-fixture-report.json",
    "artifacts/sprints/sprint-5/story-5.1/adversarial-grant-corpus-report.json",
    "artifacts/sprints/sprint-5/story-5.1/grant-race-replay-report.json",
    "artifacts/sprints/sprint-5/story-5.1/authority-escalation-report.json",
    "artifacts/sprints/sprint-5/story-5.1/grant-stale-dispatch-report.json",
    "artifacts/sprints/sprint-5/story-5.1/grant-boundary-review.json",
    "scripts/grant_boundary_review.py",
    "scripts/story_5_1_security_evidence.py",
    "tests/test_grant_boundary_review.py",
    "tests/test_story_5_1_security_evidence.py",
)
MAPPINGS = {
    "SR-ACC-001": {
        "story_contribution": "demonstrated-story-scope",
        "evidence": [
            "kernel/contracts/src/grant.rs",
            "kernel/engine/src/authority.rs",
            "artifacts/sprints/sprint-5/story-5.1/grant-policy-reference-report.json",
            "artifacts/sprints/sprint-5/story-5.1/authority-escalation-report.json",
            "artifacts/sprints/sprint-5/story-5.1/grant-boundary-review.json",
        ],
        "demonstrated": "Within the current shared contracts and in-memory Linux reference engine, CapabilityGrant is the sole authority-bearing record. Approval displays, models, prompts, shells, tools, plugins, and simulated children remain descriptive and cannot mint, widen, transfer, combine, or substitute authority.",
        "remaining": "The running product must authenticate callers and grants across process boundaries and prove that every concrete executor accepts only a kernel-issued, current, atomically consumed grant.",
    },
    "SR-ACC-002": {
        "story_contribution": "partial-story-evidence",
        "evidence": [
            "kernel/contracts/src/grant.rs",
            "fixtures/grants/adversarial/v1/manifest.json",
            "artifacts/sprints/sprint-5/story-5.1/adversarial-grant-corpus-report.json",
            "artifacts/sprints/sprint-5/story-5.1/grant-stale-dispatch-report.json",
        ],
        "demonstrated": "The 29-field grant schema binds exact actor, session, task, action, tool, operation, targets, arguments, preimages, effects, parent revision, expiry, nonce, preview, policy, use, and lifecycle values; 560 independently seeded mutations admit zero attempts.",
        "remaining": "Grant serialization must receive kernel-controlled authentication or signatures, canonical platform targets, authenticated identity provenance, durable nonce state, and cross-process verification.",
    },
    "SR-ACC-003": {
        "story_contribution": "partial-story-evidence",
        "evidence": [
            "kernel/engine/src/grants.rs",
            "artifacts/sprints/sprint-5/story-5.1/grant-state-report.json",
            "artifacts/sprints/sprint-5/story-5.1/grant-race-replay-report.json",
        ],
        "demonstrated": "The in-memory issuer validates and consumes an operation grant under one exclusive transaction. Two-consumer races admit at most one attempt, and success, denial, timeout, crash, and uncertain scenarios permit no replay or second modeled effect.",
        "remaining": "The product must place validation, durable consumption, worker admission, effect reconciliation, and terminal receipt persistence in one crash-consistent cross-process protocol immediately before every real operation.",
    },
    "SR-ACC-007": {
        "story_contribution": "partial-story-evidence",
        "evidence": [
            "kernel/contracts/src/approval.rs",
            "artifacts/sprints/sprint-5/story-5.1/grant-review-fixture-report.json",
            "artifacts/sprints/sprint-5/story-5.1/authority-escalation-report.json",
            "artifacts/sprints/sprint-5/story-5.1/grant-stale-dispatch-report.json",
        ],
        "demonstrated": "Approval requests are exact non-authoritative displays, 28 typed authority-escalation attempts are denied, and four post-approval scope changes invalidate authority before a worker or effect can start.",
        "remaining": "The shell must capture an authenticated, accessible user decision and the full transfer/escalation suite must cover at least 200 real interface and process-boundary cases while proving zero external action.",
    },
    "SR-AI-004": {
        "story_contribution": "partial-story-evidence",
        "evidence": [
            "kernel/engine/src/policy.rs",
            "artifacts/sprints/sprint-5/story-5.1/grant-policy-reference-report.json",
            "artifacts/sprints/sprint-5/story-5.1/authority-escalation-report.json",
        ],
        "demonstrated": "The strict-local policy admits only exact workspace reads; twelve hazardous operation classes are explicitly denied, two additional operation classes are denied by absence, and descriptive AI-origin records cannot expand authority.",
        "remaining": "The complete v0.1 product and user interface must prove human control and zero write, send, upload, deploy, execute, transfer, or privilege-change effects through real adapters and workers.",
    },
    "SR-AI-005": {
        "story_contribution": "partial-story-evidence",
        "evidence": [
            "kernel/engine/src/authority.rs",
            "kernel/engine/tests/authority_escalation_matrix.rs",
            "artifacts/sprints/sprint-5/story-5.1/authority-escalation-report.json",
        ],
        "demonstrated": "Hostile authority claims in prompts and other descriptive records remain inert, are excluded from denial receipts, and cannot alter the constant kernel rejection path.",
        "remaining": "This is not the required prompt-injection evaluation. At least 200 labeled direct and indirect injections across filenames, source, documentation, tool results, Git metadata, model output, and real adapters must prove zero unauthorized action and zero secret disclosure.",
    },
    "SR-OPS-001": {
        "story_contribution": "partial-story-evidence",
        "evidence": [
            "fixtures/grants/v1/denial-receipt.argument-mismatch.json",
            "artifacts/sprints/sprint-5/story-5.1/grant-review-fixture-report.json",
            "artifacts/sprints/sprint-5/story-5.1/grant-race-replay-report.json",
            "artifacts/sprints/sprint-5/story-5.1/authority-escalation-report.json",
        ],
        "demonstrated": "Typed review evidence preserves task, action, tool-call, policy-decision, result, correlation, lifecycle, actor, and session attribution without retaining candidate authority content.",
        "remaining": "The product audit schema and durable tamper-evident ledger must add release identity, wall and monotonic time, pseudonymous identity controls, object identity, complete event coverage, redaction, retention, and protected export.",
    },
    "SR-TST-012": {
        "story_contribution": "partial-story-evidence",
        "evidence": [
            "artifacts/sprints/sprint-5/story-5.1/grant-boundary-review.json",
            "scripts/grant_boundary_review.py",
            "scripts/story_5_1_security_evidence.py",
            "tests/test_grant_boundary_review.py",
            "tests/test_story_5_1_security_evidence.py",
        ],
        "demonstrated": "Deterministic Story 5 checkers fail closed on omitted subjects, changed admissions, failed verification, stale hashes, unsupported completion, release, external-review, or macOS claims.",
        "remaining": "Production release creation must be technically blocked by every security threshold, with forced-failure evidence and any exception requiring a signed dated risk decision outside normal development credentials.",
    },
}


def canonical_json(value: Any) -> bytes:
    return (json.dumps(value, indent=2, sort_keys=True) + "\n").encode("utf-8")


def read_json(path: Path) -> Any:
    return json.loads(path.read_text(encoding="utf-8"))


def sha256_file(path: Path) -> str:
    return hashlib.sha256(path.read_bytes()).hexdigest()


def git_revision(revision: str = "HEAD", root: Path = ROOT) -> str:
    completed = subprocess.run(
        ["git", "rev-parse", f"{revision}^{{commit}}"],
        cwd=root,
        check=False,
        capture_output=True,
        text=True,
        timeout=10,
    )
    value = completed.stdout.strip()
    if completed.returncode != 0 or re.fullmatch(r"[0-9a-f]{40}", value) is None:
        raise ValueError("source revision is unavailable")
    return value


def safe_relative_path(value: Any) -> bool:
    if not isinstance(value, str) or not value:
        return False
    path = PurePosixPath(value)
    return not path.is_absolute() and ".." not in path.parts and str(path) == value


def write_atomic(path: Path, content: bytes) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    descriptor, temporary_name = tempfile.mkstemp(
        prefix=".agentmage-story-5-1-security-", dir=path.parent
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


def validate_inputs(root: Path = ROOT) -> list[str]:
    failures: list[str] = []
    try:
        check_boundary_review(root)
    except (OSError, RuntimeError, ValueError, subprocess.SubprocessError) as error:
        failures.append(f"independent-boundary-review: {error}")
    try:
        security_review = (root / "SECURITY-REVIEW.md").read_text(encoding="utf-8")
    except OSError as error:
        failures.append(f"cannot read product-security authority: {error}")
        return failures
    for requirement_id in EXPECTED_REQUIREMENTS:
        if f"`{requirement_id}`" not in security_review:
            failures.append(f"security requirement is missing: {requirement_id}")
    return failures


def evidence_records(root: Path = ROOT) -> list[dict[str, Any]]:
    return [
        {
            "path": path,
            "sha256": sha256_file(root / path),
            "bytes": (root / path).stat().st_size,
        }
        for path in EVIDENCE_PATHS
    ]


def build_map(source_revision: str, root: Path = ROOT) -> dict[str, Any]:
    failures = validate_inputs(root)
    if failures:
        raise ValueError("; ".join(failures))
    return {
        "schema_version": 1,
        "story_id": "5.1",
        "task_id": "5.1.3.5",
        "source_revision": source_revision,
        "status": "pass-shared-linux-security-mapping",
        "requirements": [
            {
                "requirement_id": requirement_id,
                "product_requirement_status": "not-complete",
                **MAPPINGS[requirement_id],
            }
            for requirement_id in EXPECTED_REQUIREMENTS
        ],
        "artifacts": evidence_records(root),
        "summary": {
            "mapped_requirement_count": 8,
            "demonstrated_story_scope_count": 1,
            "partial_story_evidence_count": 7,
            "product_requirements_complete": 0,
            "retained_artifact_count": len(EVIDENCE_PATHS),
            "story_gate_complete": False,
        },
        "private_user_data_used": False,
        "network_used": False,
        "independent_review_type": "independent-automated-boundary-review",
        "external_human_review_status": "not-performed",
        "product_requirement_completion_claim": "none",
        "durable_audit_claim": "none",
        "fuzzing_claim": "seeded-fixed-mutation-corpus-not-fuzzing",
        "release_claim": "none",
        "macos_execution_status": "blocked-macos",
        "macos_evidence_substituted": False,
        "macos_support_claim": "none",
    }


def validate_map(value: Any, root: Path = ROOT) -> list[str]:
    if not isinstance(value, dict):
        return ["Story 5.1 security evidence map must be an object"]
    failures: list[str] = []
    revision = value.get("source_revision")
    if (
        value.get("schema_version") != 1
        or value.get("story_id") != "5.1"
        or value.get("task_id") != "5.1.3.5"
        or value.get("status") != "pass-shared-linux-security-mapping"
        or not isinstance(revision, str)
        or re.fullmatch(r"[0-9a-f]{40}", revision) is None
    ):
        failures.append("Story 5.1 security evidence identity is invalid")
    requirements = value.get("requirements", [])
    ids = [item.get("requirement_id") for item in requirements if isinstance(item, dict)]
    if ids != list(EXPECTED_REQUIREMENTS) or len(ids) != len(set(ids)):
        failures.append("Story 5.1 security requirement closure is invalid")
    else:
        for item in requirements:
            requirement_id = item["requirement_id"]
            expected = {
                "requirement_id": requirement_id,
                "product_requirement_status": "not-complete",
                **MAPPINGS[requirement_id],
            }
            if item != expected:
                failures.append(f"Story 5.1 security mapping changed: {requirement_id}")
            for path in item.get("evidence", []):
                if not safe_relative_path(path) or path not in EVIDENCE_PATHS:
                    failures.append(
                        f"Story 5.1 security evidence path is invalid: {requirement_id}"
                    )
    expected_summary = {
        "mapped_requirement_count": 8,
        "demonstrated_story_scope_count": 1,
        "partial_story_evidence_count": 7,
        "product_requirements_complete": 0,
        "retained_artifact_count": len(EVIDENCE_PATHS),
        "story_gate_complete": False,
    }
    if value.get("summary") != expected_summary:
        failures.append("Story 5.1 security evidence summary is invalid")
    try:
        expected_artifacts = evidence_records(root)
    except OSError:
        failures.append("Story 5.1 retained evidence closure is unavailable")
    else:
        if value.get("artifacts") != expected_artifacts:
            failures.append("Story 5.1 retained evidence closure is stale")
    if (
        value.get("private_user_data_used") is not False
        or value.get("network_used") is not False
        or value.get("independent_review_type")
        != "independent-automated-boundary-review"
        or value.get("external_human_review_status") != "not-performed"
        or value.get("product_requirement_completion_claim") != "none"
        or value.get("durable_audit_claim") != "none"
        or value.get("fuzzing_claim")
        != "seeded-fixed-mutation-corpus-not-fuzzing"
        or value.get("release_claim") != "none"
        or value.get("macos_execution_status") != "blocked-macos"
        or value.get("macos_evidence_substituted") is not False
        or value.get("macos_support_claim") != "none"
    ):
        failures.append("Story 5.1 security evidence made an unsupported claim")
    return failures


def check_map(root: Path = ROOT) -> list[str]:
    try:
        value = read_json(root / REPORT_PATH.relative_to(ROOT))
    except (OSError, json.JSONDecodeError) as error:
        return [f"cannot read Story 5.1 security evidence map: {error}"]
    failures = validate_inputs(root)
    failures.extend(validate_map(value, root))
    return failures


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--write", action="store_true")
    parser.add_argument("--source-revision")
    args = parser.parse_args()
    try:
        if args.write:
            revision = git_revision(args.source_revision or "HEAD")
            write_atomic(REPORT_PATH, canonical_json(build_map(revision)))
        failures = check_map()
    except (OSError, ValueError, KeyError, TypeError, subprocess.SubprocessError) as error:
        print(f"Story 5.1 security evidence failed: {error}", file=sys.stderr)
        return 1
    if failures:
        for failure in failures:
            print(f"Story 5.1 security evidence failed: {failure}", file=sys.stderr)
        return 1
    print("Story 5.1 security requirements mapped without product or macOS promotion")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
