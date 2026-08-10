#!/usr/bin/env python3
"""Build and validate the Story 4.1 product-security evidence map."""

from __future__ import annotations

import argparse
import hashlib
import json
import os
import subprocess
import sys
import tempfile
from pathlib import Path, PurePosixPath
from typing import Any

ROOT = Path(__file__).resolve().parents[1]
if str(ROOT) not in sys.path:
    sys.path.insert(0, str(ROOT))

from scripts.kernel_architecture_report import check_report as check_architecture
from scripts.kernel_boundary_integration import check_report as check_boundary
from scripts.kernel_contract_fixtures import check_outputs as check_fixtures
from scripts.kernel_contract_package import check_artifacts as check_package
from scripts.kernel_contract_reference import check_report as check_reference
from scripts.kernel_dispatch_security import check_report as check_dispatch


REPORT_PATH = ROOT / "artifacts/sprints/sprint-4/story-4.1/security-evidence-map.json"
EXPECTED_REQUIREMENTS = (
    "SR-GOV-006",
    "SR-ACC-001",
    "SR-AI-003",
    "SR-OPS-001",
    "SR-TST-001",
    "SR-TST-002",
    "SR-TST-004",
)
EVIDENCE_PATHS = (
    "SECURITY-REVIEW.md",
    "architecture/dependency-rules.json",
    "docs/architecture/kernel-contract-reference.md",
    "docs/architecture/kernel-dependency-report.md",
    "kernel/engine/src/authority.rs",
    "kernel/engine/src/propagation.rs",
    "kernel/engine/src/tooling.rs",
    "kernel/engine/tests/boundary_workflow.rs",
    "fixtures/contracts/fixture_verifier.rs",
    "fixtures/contracts/compatibility.json",
    "fixtures/contracts/v1/manifest.json",
    "artifacts/sprints/sprint-4/story-4.1/agentmage-kernel-contracts-0.0.0.crate",
    "artifacts/sprints/sprint-4/story-4.1/kernel-contract-package-report.json",
    "artifacts/sprints/sprint-4/story-4.1/kernel-contract-reference-report.json",
    "artifacts/sprints/sprint-4/story-4.1/kernel-contract-fixture-report.json",
    "artifacts/sprints/sprint-4/story-4.1/kernel-architecture-dependency-report.json",
    "artifacts/sprints/sprint-4/story-4.1/kernel-dispatch-security-report.json",
    "artifacts/sprints/sprint-4/story-4.1/kernel-boundary-integration-report.json",
    "scripts/kernel_contract_package.py",
    "scripts/kernel_contract_reference.py",
    "scripts/kernel_contract_fixtures.py",
    "scripts/kernel_architecture_report.py",
    "scripts/kernel_dispatch_security.py",
    "scripts/kernel_boundary_integration.py",
    "scripts/story_4_1_security_evidence.py",
    "tests/test_kernel_contract_package.py",
    "tests/test_kernel_contract_reference.py",
    "tests/test_kernel_contract_fixtures.py",
    "tests/test_kernel_architecture_report.py",
    "tests/test_kernel_dispatch_security.py",
    "tests/test_kernel_boundary_integration.py",
    "tests/test_story_4_1_security_evidence.py",
)
MAPPINGS = {
    "SR-GOV-006": {
        "story_contribution": "partial-story-evidence",
        "evidence": [
            "architecture/dependency-rules.json",
            "docs/architecture/kernel-dependency-report.md",
            "artifacts/sprints/sprint-4/story-4.1/kernel-architecture-dependency-report.json",
        ],
        "demonstrated": "The logical and materialized compile-time component graphs are deterministic, acyclic, and closed against prohibited dependency edges; the documented typed routes distinguish shell, kernel, platform adapter, tool, and model boundaries.",
        "remaining": "The running product must inventory and trace every process, file store, socket, IPC endpoint, and runtime data flow, including generated VS Code protocol and implemented platform adapters.",
    },
    "SR-ACC-001": {
        "story_contribution": "partial-story-evidence",
        "evidence": [
            "kernel/engine/src/authority.rs",
            "kernel/engine/src/tooling.rs",
            "artifacts/sprints/sprint-4/story-4.1/kernel-dispatch-security-report.json",
        ],
        "demonstrated": "Descriptions, tasks, work packets, plans, prompts, tool definitions, models, shells, tools, capability packs, forged descriptions, and unregistered callers have no authority path; every current dispatch terminates in a typed zero-execution receipt.",
        "remaining": "Sprint 5 must define the sole typed CapabilityGrant path and prove that every concrete executor validates and atomically consumes an exact grant while every non-grant object remains denied.",
    },
    "SR-AI-003": {
        "story_contribution": "demonstrated-story-scope",
        "evidence": [
            "docs/architecture/kernel-contract-reference.md",
            "kernel/engine/src/authority.rs",
            "artifacts/sprints/sprint-4/story-4.1/kernel-dispatch-security-report.json",
        ],
        "demonstrated": "Prompt and model-origin records are descriptive, non-authoritative provenance; model-origin dispatch cannot execute, create authority, or bypass deterministic kernel validation.",
        "remaining": "Real model adapters and user interfaces must label model claims as inferred until deterministic evidence supports them and must visibly preserve unsupported or conflicting status.",
    },
    "SR-OPS-001": {
        "story_contribution": "partial-story-evidence",
        "evidence": [
            "artifacts/sprints/sprint-4/story-4.1/kernel-dispatch-security-report.json",
            "artifacts/sprints/sprint-4/story-4.1/kernel-boundary-integration-report.json",
        ],
        "demonstrated": "Structured pre-grant receipts and boundary traces retain operation outcome, stable typed error, task identity, correlation identity, route, and zero-execution state-change disposition.",
        "remaining": "The product audit schema and durable ledger must add release identity, monotonic and wall time, pseudonymous actor/session identity, object identity, policy decision, complete event coverage, retention, and protected export.",
    },
    "SR-TST-001": {
        "story_contribution": "partial-story-evidence",
        "evidence": [
            "tests/test_kernel_contract_fixtures.py",
            "tests/test_kernel_dispatch_security.py",
            "tests/test_kernel_boundary_integration.py",
            "artifacts/sprints/sprint-4/story-4.1/kernel-boundary-integration-report.json",
        ],
        "demonstrated": "Linux unit, public-client integration, compatibility, architecture, and adversarial pre-grant suites run through deterministic report checkers with retained evidence.",
        "remaining": "Later product boundaries must add requirement-tagged property, end-to-end, real-platform, recovery, accessibility, and product-wide adversarial suites with complete skipped/flaky/quarantined accounting.",
    },
    "SR-TST-002": {
        "story_contribution": "partial-story-evidence",
        "evidence": [
            "fixtures/contracts/compatibility.json",
            "fixtures/contracts/v1/manifest.json",
            "artifacts/sprints/sprint-4/story-4.1/kernel-contract-fixture-report.json",
        ],
        "demonstrated": "A fixed, hash-bound contract corpus exercises canonical valid records plus malformed, missing, unknown, duplicate, trailing, old-version, new-version, and generated oversized inputs.",
        "remaining": "This fixed corpus is not fuzzing; registered parser and trust-boundary campaigns must retain corpus identity, duration, coverage, sanitizer configuration, crashes, fixes, and deterministic regression replay.",
    },
    "SR-TST-004": {
        "story_contribution": "demonstrated-story-scope",
        "evidence": [
            "fixtures/contracts/compatibility.json",
            "artifacts/sprints/sprint-4/story-4.1/kernel-contract-fixture-report.json",
            "artifacts/sprints/sprint-4/story-4.1/kernel-dispatch-security-report.json",
        ],
        "demonstrated": "The current contract and pre-grant dispatch boundaries reject malformed, hostile, oversized, partial, stale-version, conflicting, forged, and unregistered inputs with bounded redacted failures, zero privilege, and exact receipts.",
        "remaining": "Every later parser and trust boundary must repeat these classes and prove bounded failure, no data disclosure, no authority expansion, and correct receipts for its own input domain.",
    },
}


def canonical_json(value: Any) -> bytes:
    return (json.dumps(value, indent=2, sort_keys=True) + "\n").encode("utf-8")


def read_json(path: Path) -> Any:
    return json.loads(path.read_text(encoding="utf-8"))


def sha256_file(path: Path) -> str:
    return hashlib.sha256(path.read_bytes()).hexdigest()


def git_revision(revision: str = "HEAD", root: Path = ROOT) -> str:
    return subprocess.run(
        ["git", "rev-parse", f"{revision}^{{commit}}"],
        cwd=root,
        check=True,
        capture_output=True,
        text=True,
    ).stdout.strip()


def safe_relative_path(value: Any) -> bool:
    if not isinstance(value, str) or not value:
        return False
    path = PurePosixPath(value)
    return not path.is_absolute() and ".." not in path.parts and str(path) == value


def write_atomic(path: Path, content: bytes) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    descriptor, temporary_name = tempfile.mkstemp(
        prefix=".agentmage-story-4-1-security-", dir=path.parent
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
    raising_checks = (
        ("contract-package", lambda: check_package(root)),
        ("contract-reference", lambda: check_reference(root)),
        ("contract-fixtures", check_fixtures),
    )
    for name, check in raising_checks:
        try:
            check()
        except (OSError, RuntimeError, ValueError, subprocess.SubprocessError) as error:
            failures.append(f"{name}: {error}")
    returning_checks = (
        ("architecture", check_architecture),
        ("dispatcher", check_dispatch),
        ("boundary-integration", check_boundary),
    )
    for name, check in returning_checks:
        failures.extend(f"{name}: {failure}" for failure in check(root))
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
        "story_id": "4.1",
        "task_id": "4.1.3.5",
        "source_revision": source_revision,
        "status": "pass-linux-security-mapping",
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
            "mapped_requirement_count": 7,
            "demonstrated_story_scope_count": 2,
            "partial_story_evidence_count": 5,
            "product_requirements_complete": 0,
            "retained_artifact_count": len(EVIDENCE_PATHS),
            "story_gate_complete": False,
        },
        "private_user_data_used": False,
        "network_used": False,
        "positive_authority_path_claim": "none",
        "product_requirement_completion_claim": "none",
        "durable_audit_claim": "none",
        "fuzzing_claim": "fixed-corpus-only-not-fuzzing",
        "release_claim": "none",
        "macos_execution_status": "blocked-macos",
        "macos_evidence_substituted": False,
        "macos_support_claim": "none",
    }


def validate_map(value: Any, root: Path = ROOT) -> list[str]:
    if not isinstance(value, dict):
        return ["Story 4.1 security evidence map must be an object"]
    failures: list[str] = []
    revision = value.get("source_revision")
    if (
        value.get("schema_version") != 1
        or value.get("story_id") != "4.1"
        or value.get("task_id") != "4.1.3.5"
        or value.get("status") != "pass-linux-security-mapping"
        or not isinstance(revision, str)
        or len(revision) != 40
    ):
        failures.append("Story 4.1 security evidence identity is invalid")
    requirements = value.get("requirements", [])
    ids = [item.get("requirement_id") for item in requirements if isinstance(item, dict)]
    if ids != list(EXPECTED_REQUIREMENTS) or len(ids) != len(set(ids)):
        failures.append("Story 4.1 security requirement closure is invalid")
    else:
        for item in requirements:
            requirement_id = item["requirement_id"]
            expected = {
                "requirement_id": requirement_id,
                "product_requirement_status": "not-complete",
                **MAPPINGS[requirement_id],
            }
            if item != expected:
                failures.append(f"Story 4.1 security mapping changed: {requirement_id}")
            for path in item.get("evidence", []):
                if not safe_relative_path(path) or path not in EVIDENCE_PATHS:
                    failures.append(
                        f"Story 4.1 security evidence path is invalid: {requirement_id}"
                    )
    expected_summary = {
        "mapped_requirement_count": 7,
        "demonstrated_story_scope_count": 2,
        "partial_story_evidence_count": 5,
        "product_requirements_complete": 0,
        "retained_artifact_count": len(EVIDENCE_PATHS),
        "story_gate_complete": False,
    }
    if value.get("summary") != expected_summary:
        failures.append("Story 4.1 security evidence summary is invalid")
    try:
        expected_artifacts = evidence_records(root)
    except OSError:
        failures.append("Story 4.1 retained evidence closure is unavailable")
    else:
        if value.get("artifacts") != expected_artifacts:
            failures.append("Story 4.1 retained evidence closure is stale")
    if (
        value.get("private_user_data_used") is not False
        or value.get("network_used") is not False
        or value.get("positive_authority_path_claim") != "none"
        or value.get("product_requirement_completion_claim") != "none"
        or value.get("durable_audit_claim") != "none"
        or value.get("fuzzing_claim") != "fixed-corpus-only-not-fuzzing"
        or value.get("release_claim") != "none"
        or value.get("macos_execution_status") != "blocked-macos"
        or value.get("macos_evidence_substituted") is not False
        or value.get("macos_support_claim") != "none"
    ):
        failures.append("Story 4.1 security evidence made an unsupported claim")
    return failures


def check_map(root: Path = ROOT) -> list[str]:
    try:
        value = read_json(root / REPORT_PATH.relative_to(ROOT))
    except (OSError, json.JSONDecodeError) as error:
        return [f"cannot read Story 4.1 security evidence map: {error}"]
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
        print(f"Story 4.1 security evidence failed: {error}", file=sys.stderr)
        return 1
    if failures:
        for failure in failures:
            print(f"Story 4.1 security evidence failed: {failure}", file=sys.stderr)
        return 1
    print("Story 4.1 security requirements mapped without product or macOS promotion")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
