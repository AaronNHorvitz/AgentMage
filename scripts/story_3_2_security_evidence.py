#!/usr/bin/env python3
"""Build and validate the Story 3.2 product-security evidence map."""

from __future__ import annotations

import argparse
import hashlib
import json
import os
import sys
import tempfile
from collections.abc import Callable
from pathlib import Path, PurePosixPath
from typing import Any

ROOT = Path(__file__).resolve().parents[1]
if str(ROOT) not in sys.path:
    sys.path.insert(0, str(ROOT))

from scripts.emergency_disable_policy import check_artifact as check_emergency_disable
from scripts.manual_patch_metadata import check_artifact as check_patch_metadata
from scripts.manual_patch_verifier import check_artifact as check_patch_verification
from scripts.update_design import check_artifact as check_update_design
from scripts.vulnerability_report_evidence import check_artifact as check_report_evidence
from scripts.vulnerability_support_platform_evidence import check_report as check_platform
from scripts.vulnerability_support_policy import check_artifact as check_support_policy
from scripts.vulnerability_support_workflow import check_artifact as check_workflow


REPORT_PATH = ROOT / "artifacts/sprints/sprint-3/story-3.2/security-evidence-map.json"
TABLETOP_PATH = ROOT / "docs/security/story-3.2-vulnerability-response-tabletop.md"
EXPECTED_REQUIREMENTS = (
    "SR-GOV-004",
    "SR-GOV-010",
    "SR-SUP-005",
    "SR-SUP-010",
    "SR-SUP-011",
    "SR-SUP-013",
    "SR-OPS-004",
    "SR-OPS-005",
    "SR-OPS-006",
    "SR-OPS-007",
)
EVIDENCE_PATHS = (
    "SECURITY-REVIEW.md",
    "SECURITY.md",
    "docs/support/vulnerability-reporting.md",
    "docs/security/story-3.2-vulnerability-response-tabletop.md",
    "docs/decisions/0005-signed-manual-update-design.md",
    "docs/decisions/0006-update-rollback-design.md",
    "docs/decisions/0007-local-emergency-disablement.md",
    "architecture/signed-update-design.json",
    "architecture/rollback-design.json",
    "support/vulnerability-support-policy.json",
    "support/vulnerability-report-evidence-policy.json",
    "schemas/support/vulnerability-support-policy.schema.json",
    "schemas/support/vulnerability-report-evidence-policy.schema.json",
    "schemas/support/vulnerability-diagnostic-bundle.schema.json",
    "schemas/support/signed-manual-patch-metadata.schema.json",
    "schemas/support/emergency-disable-policy.schema.json",
    "schemas/support/examples/vulnerability-diagnostic-bundle.valid.json",
    "schemas/support/examples/signed-manual-patch-metadata.valid.json",
    "schemas/support/examples/emergency-disable-policy.valid.json",
    "fixtures/support/vulnerability-workflow/workflow.valid.json",
    "fixtures/support/manual-patch/cases.json",
    "fixtures/support/manual-patch/cases/valid.json",
    "fixtures/support/manual-patch/cases/revoked.json",
    "artifacts/sprints/sprint-3/story-3.2/vulnerability-support-policy-report.json",
    "artifacts/sprints/sprint-3/story-3.2/support-policy-platform-report.json",
    "artifacts/sprints/sprint-3/story-3.2/vulnerability-report-evidence-report.json",
    "artifacts/sprints/sprint-3/story-3.2/manual-patch-metadata-report.json",
    "artifacts/sprints/sprint-3/story-3.2/emergency-disable-policy-report.json",
    "artifacts/sprints/sprint-3/story-3.2/manual-patch-verification-report.json",
    "artifacts/sprints/sprint-3/story-3.2/vulnerability-workflow-report.json",
    "scripts/manual_patch_verifier.py",
    "scripts/vulnerability_support_workflow.py",
    "scripts/story_3_2_security_evidence.py",
    "tests/test_manual_patch_verifier.py",
    "tests/test_emergency_disable_policy.py",
    "tests/test_vulnerability_support_workflow.py",
    "tests/test_story_3_2_security_evidence.py",
)
MAPPINGS = {
    "SR-GOV-004": {
        "story_contribution": "partial-story-evidence",
        "evidence": [
            "SECURITY.md",
            "support/vulnerability-support-policy.json",
            "artifacts/sprints/sprint-3/story-3.2/vulnerability-support-policy-report.json",
        ],
        "demonstrated": "The pre-release support contract identifies accountable, intake, severity, remediation, and disclosure roles; private contact routes; response targets; support states; and end-of-support rules without claiming a supported binary.",
        "remaining": "An actual signed release manifest must carry immutable product version, owner, maintainer, support period, security contact, and support-state identities for a release candidate.",
    },
    "SR-GOV-010": {
        "story_contribution": "partial-story-evidence",
        "evidence": [
            "schemas/support/signed-manual-patch-metadata.schema.json",
            "architecture/signed-update-design.json",
            "artifacts/sprints/sprint-3/story-3.2/manual-patch-metadata-report.json",
        ],
        "demonstrated": "Patch metadata binds configuration-schema impact, migration, platform, runtime, capability, authority, component, provenance, support, and rollback identities and rejects hidden network or authority changes.",
        "remaining": "Integrated CI must classify each real authority, storage, network, model, runtime, extension, platform, installer, and package change and require a new security-impact review before release.",
    },
    "SR-SUP-005": {
        "story_contribution": "partial-story-evidence",
        "evidence": [
            "schemas/support/signed-manual-patch-metadata.schema.json",
            "fixtures/support/manual-patch/cases/valid.json",
            "artifacts/sprints/sprint-3/story-3.2/manual-patch-verification-report.json",
        ],
        "demonstrated": "The synthetic patch binds source, builder, recipe, clean-build report, package, release manifest, provenance, SBOM, cryptographic BOM, Model BOM, component, configuration, capability, authority, signer, and signature identities, then verifies exact bytes and the detached Ed25519 signature.",
        "remaining": "A release runner must generate signed provenance for an actual package, and an independent reviewer must trace that package through real source, commands, tests, build inputs, and production signing authority.",
    },
    "SR-SUP-010": {
        "story_contribution": "demonstrated-story-scope",
        "evidence": [
            "SECURITY.md",
            "docs/support/vulnerability-reporting.md",
            "docs/security/story-3.2-vulnerability-response-tabletop.md",
            "artifacts/sprints/sprint-3/story-3.2/vulnerability-workflow-report.json",
        ],
        "demonstrated": "The public policy and bounded-evidence guidance define disclosure, triage, remediation, signed manual patch delivery, local disablement, embargo, notification, closure, and end of support; the seven-state synthetic tabletop executes that exact process with traceable role and evidence receipts.",
        "remaining": "The first supported package must execute the process with real release evidence and the complete package-level RV-22 matrix on every reference platform.",
    },
    "SR-SUP-011": {
        "story_contribution": "partial-story-evidence",
        "evidence": [
            "artifacts/sprints/sprint-3/story-3.2/support-policy-platform-report.json",
            "artifacts/sprints/sprint-3/story-3.2/vulnerability-workflow-report.json",
        ],
        "demonstrated": "The support contract validates in isolated no-network Fedora and Ubuntu environments, and the synthetic lifecycle and evidence maps rebuild deterministically from hash-bound inputs without ambient timestamps or paths.",
        "remaining": "Actual source and shipped-package builds must run twice in clean release environments and prove byte identity or retain signed explanations for bounded differences on every reference platform.",
    },
    "SR-SUP-013": {
        "story_contribution": "partial-story-evidence",
        "evidence": [
            "schemas/support/emergency-disable-policy.schema.json",
            "fixtures/support/manual-patch/cases/revoked.json",
            "artifacts/sprints/sprint-3/story-3.2/emergency-disable-policy-report.json",
            "artifacts/sprints/sprint-3/story-3.2/manual-patch-verification-report.json",
        ],
        "demonstrated": "A revoked release is rejected by patch verification, and the signed local-disable contract blocks exact model, runtime, component, capability, and release-version subjects before ordinary authority evaluation without a remote kill switch.",
        "remaining": "Real build and startup gates must consume signed revocation/support policy and visibly block unmaintained, revoked, unverified, unsupported, and policy-excluded production components.",
    },
    "SR-OPS-004": {
        "story_contribution": "partial-story-evidence",
        "evidence": [
            "fixtures/support/vulnerability-workflow/workflow.valid.json",
            "artifacts/sprints/sprint-3/story-3.2/vulnerability-workflow-report.json",
            "docs/security/story-3.2-vulnerability-response-tabletop.md",
        ],
        "demonstrated": "Seven minimized workflow receipts are deterministically hash-chained, so event mutation and reordering change the chain while no network client or external action is introduced.",
        "remaining": "The product audit store must detect alteration, removal, reorder, and duplication across all security events and support an explicit approved export or signed checkpoint without adding a v0.1 network client.",
    },
    "SR-OPS-005": {
        "story_contribution": "partial-story-evidence",
        "evidence": [
            "fixtures/support/vulnerability-workflow/workflow.valid.json",
            "artifacts/sprints/sprint-3/story-3.2/vulnerability-workflow-report.json",
        ],
        "demonstrated": "The synthetic workflow requires exact monotonic sequence numbers and increasing fixed wall timestamps across all seven states.",
        "remaining": "Product events must record monotonic time and visibly handle backward/forward wall-clock changes, sleep, restart, and resume while preserving order.",
    },
    "SR-OPS-006": {
        "story_contribution": "partial-story-evidence",
        "evidence": [
            "SECURITY.md",
            "docs/security/story-3.2-vulnerability-response-tabletop.md",
            "artifacts/sprints/sprint-3/story-3.2/vulnerability-workflow-report.json",
        ],
        "demonstrated": "The narrow vulnerability-response exercise assigns intake, triage, remediation, verification, notification, closure, and evidence-retention responsibilities without improvising authority.",
        "remaining": "The product incident runbook and complete RV-21 suspected-egress, compromised-package, prompt-injection-disclosure, and key-store-failure tabletops must cover detection, suspension, containment, recovery, external-environment interfaces, and lessons learned.",
    },
    "SR-OPS-007": {
        "story_contribution": "partial-story-evidence",
        "evidence": [
            "support/vulnerability-report-evidence-policy.json",
            "schemas/support/examples/vulnerability-diagnostic-bundle.valid.json",
            "artifacts/sprints/sprint-3/story-3.2/vulnerability-report-evidence-report.json",
            "artifacts/sprints/sprint-3/story-3.2/vulnerability-workflow-report.json",
        ],
        "demonstrated": "The evidence contract limits fields, values, roles, transfer, closure review, and retention; the synthetic workflow admits three sanitized records with zero findings, a 180-day maximum, and no active hold or collection authority.",
        "remaining": "The product must enforce evidence encryption, role access, expiry/deletion, explicit incident-hold scope and expiration, and bounded approved export against real storage without collecting new material.",
    },
}
Validator = Callable[[Path], list[str]]


def canonical_json(value: Any) -> bytes:
    return (json.dumps(value, indent=2, sort_keys=True) + "\n").encode("utf-8")


def read_json(path: Path) -> Any:
    return json.loads(path.read_text(encoding="utf-8"))


def sha256_file(path: Path) -> str:
    return hashlib.sha256(path.read_bytes()).hexdigest()


def safe_relative_path(value: Any) -> bool:
    if not isinstance(value, str) or not value:
        return False
    path = PurePosixPath(value)
    return not path.is_absolute() and ".." not in path.parts and str(path) == value


def write_atomic(path: Path, content: bytes) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    descriptor, temporary_name = tempfile.mkstemp(
        prefix=".agentmage-story-3-2-security-", dir=path.parent
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
    validators: tuple[tuple[str, Validator], ...] = (
        ("support-policy", check_support_policy),
        ("support-platform", check_platform),
        ("bounded-report-evidence", check_report_evidence),
        ("patch-metadata", check_patch_metadata),
        ("emergency-disable", check_emergency_disable),
        ("patch-verification", check_patch_verification),
        ("support-workflow", check_workflow),
        ("update-design", check_update_design),
    )
    failures = []
    for name, validator in validators:
        failures.extend(f"{name}: {failure}" for failure in validator(root))
    try:
        security_review = (root / "SECURITY-REVIEW.md").read_text(encoding="utf-8")
        tabletop = (root / TABLETOP_PATH.relative_to(ROOT)).read_text(encoding="utf-8")
    except OSError as error:
        failures.append(f"cannot read Story 3.2 security authority: {error}")
        return failures
    for requirement_id in EXPECTED_REQUIREMENTS:
        if f"`{requirement_id}`" not in security_review:
            failures.append(f"security requirement is missing: {requirement_id}")
    for required_text in (
        "not the four-scenario `RV-21`",
        "not the first complete package-level `RV-22`",
        "macOS remains blocked",
        "No report was received",
    ):
        if required_text not in tabletop:
            failures.append("Story 3.2 tabletop boundary is incomplete")
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


def build_map(root: Path = ROOT) -> dict[str, Any]:
    failures = validate_inputs(root)
    if failures:
        raise ValueError("; ".join(failures))
    return {
        "schema_version": 1,
        "story_id": "3.2",
        "task_id": "3.2.2.3",
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
            "mapped_requirement_count": len(EXPECTED_REQUIREMENTS),
            "demonstrated_story_scope_count": 1,
            "partial_story_evidence_count": 9,
            "product_requirements_complete": 0,
            "retained_artifact_count": len(EVIDENCE_PATHS),
            "story_gate_complete": False,
            "rv_21_complete": False,
            "rv_22_complete": False,
        },
        "private_user_data_used": False,
        "network_used": False,
        "actual_incident_claim": "none",
        "product_support_claim": "none",
        "product_patch_claim": "none",
        "release_claim": "none",
        "macos_execution_status": "blocked-macos",
        "macos_evidence_substituted": False,
        "macos_support_claim": "none",
    }


def validate_map(value: Any, root: Path = ROOT) -> list[str]:
    if not isinstance(value, dict):
        return ["Story 3.2 security evidence map must be an object"]
    failures = []
    if (
        value.get("schema_version") != 1
        or value.get("story_id") != "3.2"
        or value.get("task_id") != "3.2.2.3"
        or value.get("status") != "pass-shared-linux-security-mapping"
    ):
        failures.append("Story 3.2 security evidence identity is invalid")
    requirements = value.get("requirements", [])
    ids = [item.get("requirement_id") for item in requirements if isinstance(item, dict)]
    if ids != list(EXPECTED_REQUIREMENTS) or len(ids) != len(set(ids)):
        failures.append("Story 3.2 security requirement closure is invalid")
    else:
        for item in requirements:
            requirement_id = item["requirement_id"]
            expected = {
                "requirement_id": requirement_id,
                "product_requirement_status": "not-complete",
                **MAPPINGS[requirement_id],
            }
            if item != expected:
                failures.append(f"Story 3.2 security mapping changed: {requirement_id}")
            for path in item.get("evidence", []):
                if not safe_relative_path(path) or path not in EVIDENCE_PATHS:
                    failures.append(
                        f"Story 3.2 security evidence path is invalid: {requirement_id}"
                    )
    if value.get("summary") != {
        "mapped_requirement_count": 10,
        "demonstrated_story_scope_count": 1,
        "partial_story_evidence_count": 9,
        "product_requirements_complete": 0,
        "retained_artifact_count": len(EVIDENCE_PATHS),
        "story_gate_complete": False,
        "rv_21_complete": False,
        "rv_22_complete": False,
    }:
        failures.append("Story 3.2 security evidence summary is invalid")
    try:
        expected_artifacts = evidence_records(root)
    except OSError:
        failures.append("Story 3.2 retained evidence closure is unavailable")
    else:
        if value.get("artifacts") != expected_artifacts:
            failures.append("Story 3.2 retained evidence closure is stale")
    if (
        value.get("private_user_data_used") is not False
        or value.get("network_used") is not False
        or value.get("actual_incident_claim") != "none"
        or value.get("product_support_claim") != "none"
        or value.get("product_patch_claim") != "none"
        or value.get("release_claim") != "none"
        or value.get("macos_execution_status") != "blocked-macos"
        or value.get("macos_evidence_substituted") is not False
        or value.get("macos_support_claim") != "none"
    ):
        failures.append("Story 3.2 security evidence made an unsupported claim")
    return failures


def check_map(root: Path = ROOT) -> list[str]:
    try:
        value = read_json(root / REPORT_PATH.relative_to(ROOT))
    except (OSError, json.JSONDecodeError) as error:
        return [f"cannot read Story 3.2 security evidence map: {error}"]
    failures = validate_inputs(root)
    failures.extend(validate_map(value, root))
    return failures


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--write", action="store_true")
    args = parser.parse_args()
    try:
        if args.write:
            write_atomic(REPORT_PATH, canonical_json(build_map()))
        failures = check_map()
    except (OSError, ValueError, KeyError, TypeError) as error:
        print(f"Story 3.2 security evidence failed: {error}")
        return 1
    if failures:
        for failure in failures:
            print(f"Story 3.2 security evidence failed: {failure}")
        return 1
    print("Story 3.2 security requirements mapped without product or macOS promotion")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
