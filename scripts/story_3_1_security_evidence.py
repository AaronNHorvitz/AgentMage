#!/usr/bin/env python3
"""Build and validate the Story 3.1 product-security evidence map."""

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

from scripts.component_inventory import check_artifact as check_components
from scripts.configuration_authority_mutation_evidence import (
    check_artifact as check_authority_mutations,
)
from scripts.configuration_loader_evidence import check_artifact as check_loader
from scripts.configuration_migration_recovery_evidence import (
    check_artifact as check_migration_recovery,
)
from scripts.configuration_profiles import check_artifacts as check_profiles
from scripts.configuration_result_evidence import check_artifact as check_results
from scripts.configuration_schema_failure_evidence import (
    check_artifact as check_schema_failures,
)
from scripts.configuration_startup_evidence import check_artifact as check_startup
from scripts.supply_chain import check_outputs as check_supply_chain

REPORT_PATH = ROOT / "artifacts/sprints/sprint-3/story-3.1/security-evidence-map.json"
EXPECTED_REQUIREMENTS = (
    "SR-GOV-008",
    "SR-GOV-009",
    "SR-GOV-010",
    "SR-ACC-001",
    "SR-DAT-007",
    "SR-SUP-003",
    "SR-SUP-013",
    "SR-OPS-002",
    "SR-TST-005",
)
EVIDENCE_PATHS = (
    "SECURITY-REVIEW.md",
    "schemas/configuration/agent-configuration.schema.json",
    "schemas/configuration/profile-catalog.schema.json",
    "configuration/profiles/catalog.json",
    "configuration/profiles/capability-deltas.json",
    "configuration/permission-bearing-values.json",
    "supply-chain/dependency-provenance.json",
    "supply-chain/dependency-hashes.sha256",
    "artifacts/sprints/sprint-1/story-1.1/clean-build-report.json",
    "artifacts/sprints/sprint-3/story-3.1/configuration-schema-report.json",
    "artifacts/sprints/sprint-3/story-3.1/profile-catalog-report.json",
    "artifacts/sprints/sprint-3/story-3.1/configuration-loader-report.json",
    "artifacts/sprints/sprint-3/story-3.1/configuration-authority-mutation-report.json",
    "artifacts/sprints/sprint-3/story-3.1/configuration-migration-recovery-report.json",
    "artifacts/sprints/sprint-3/story-3.1/configuration-startup-report.json",
    "artifacts/sprints/sprint-3/story-3.1/configuration-result-report.json",
    "artifacts/sprints/sprint-3/story-3.1/configuration-schema-failure-report.json",
    "artifacts/sprints/sprint-3/story-3.1/configuration-review-artifacts-report.json",
    "artifacts/sprints/sprint-3/story-3.1/component-inventory-report.json",
    "artifacts/sprints/sprint-3/story-3.1/update-rollback-design-report.json",
    "scripts/story_3_1_security_evidence.py",
    "tests/test_story_3_1_security_evidence.py",
)
MAPPINGS = {
    "SR-GOV-008": {
        "story_contribution": "demonstrated-story-scope",
        "evidence": [
            "configuration/permission-bearing-values.json",
            "artifacts/sprints/sprint-3/story-3.1/configuration-authority-mutation-report.json",
            "artifacts/sprints/sprint-3/story-3.1/configuration-startup-report.json",
        ],
        "demonstrated": "All 97 permission-bearing configuration leaves fail closed across five untrusted channels, and every unregistered catalog profile reaches startup with zero registered capabilities.",
        "remaining": "Each later implemented capability, runtime, extension, and package startup path must repeat the same dependency-removal and registration refusal checks.",
    },
    "SR-GOV-009": {
        "story_contribution": "partial-story-evidence",
        "evidence": [
            "artifacts/sprints/sprint-3/story-3.1/configuration-startup-report.json",
            "artifacts/sprints/sprint-3/story-3.1/configuration-result-report.json",
        ],
        "demonstrated": "Startup and configuration-bound results retain exact configuration, dependency, executable, and policy identities.",
        "remaining": "The release manifest must record the approved review-baseline and standard versions when an actual release candidate exists.",
    },
    "SR-GOV-010": {
        "story_contribution": "partial-story-evidence",
        "evidence": [
            "artifacts/sprints/sprint-3/story-3.1/configuration-review-artifacts-report.json",
            "artifacts/sprints/sprint-3/story-3.1/configuration-authority-mutation-report.json",
        ],
        "demonstrated": "Configuration differences classify authority and resource changes, redact values, and reject every tested authority broadening before application.",
        "remaining": "Integrated CI must escalate security review for every changed authority, storage, network, model, runtime, extension, platform, installer, and package behavior.",
    },
    "SR-ACC-001": {
        "story_contribution": "partial-story-evidence",
        "evidence": [
            "schemas/configuration/agent-configuration.schema.json",
            "artifacts/sprints/sprint-3/story-3.1/configuration-authority-mutation-report.json",
            "artifacts/sprints/sprint-3/story-3.1/configuration-startup-report.json",
        ],
        "demonstrated": "Environment, repository, child-profile, file, and model-output configuration cannot create operation authority, and no catalog capability currently registers.",
        "remaining": "The typed CapabilityGrant boundary and every concrete operation executor must still prove that only a valid grant authorizes an effect.",
    },
    "SR-DAT-007": {
        "story_contribution": "partial-story-evidence",
        "evidence": [
            "artifacts/sprints/sprint-3/story-3.1/configuration-schema-report.json",
            "artifacts/sprints/sprint-3/story-3.1/configuration-schema-failure-report.json",
            "artifacts/sprints/sprint-3/story-3.1/configuration-result-report.json",
        ],
        "demonstrated": "The closed configuration accepts only a secret-provider identifier; reports retain hashes rather than keys, raw configuration values, or private paths.",
        "remaining": "Platform key-service handles, non-exportability, access instrumentation, backup/export exclusion, and product artifact scans must be implemented and exercised.",
    },
    "SR-SUP-003": {
        "story_contribution": "demonstrated-story-scope",
        "evidence": [
            "supply-chain/dependency-provenance.json",
            "supply-chain/dependency-hashes.sha256",
            "artifacts/sprints/sprint-1/story-1.1/clean-build-report.json",
            "artifacts/sprints/sprint-3/story-3.1/component-inventory-report.json",
        ],
        "demonstrated": "All current source dependencies, build tools, and clean Linux environments are locked, versioned, cryptographically hashed, inventoried, and rejected on closure drift.",
        "remaining": "Future model runtimes, parsers, conversion tools, platform SDKs, optional components, and release packages require separate admission before use.",
    },
    "SR-SUP-013": {
        "story_contribution": "partial-story-evidence",
        "evidence": [
            "artifacts/sprints/sprint-3/story-3.1/component-inventory-report.json",
            "artifacts/sprints/sprint-3/story-3.1/configuration-startup-report.json",
            "artifacts/sprints/sprint-3/story-3.1/update-rollback-design-report.json",
        ],
        "demonstrated": "Candidate runtimes and optional components remain unapproved, unregistered profiles remain blocked, and the signed-update design requires revocation and support checks.",
        "remaining": "The implemented package gate must block actual revoked, unmaintained, unverified, unsupported, and policy-excluded components at build and startup.",
    },
    "SR-OPS-002": {
        "story_contribution": "partial-story-evidence",
        "evidence": [
            "artifacts/sprints/sprint-3/story-3.1/configuration-result-report.json",
            "artifacts/sprints/sprint-3/story-3.1/configuration-startup-report.json",
            "artifacts/sprints/sprint-3/story-3.1/configuration-review-artifacts-report.json",
        ],
        "demonstrated": "Minimized configuration and startup records retain exact identities, typed outcomes, authority/resource classifications, and no raw values or private paths.",
        "remaining": "The product audit schema and complete event-coverage matrix must include access, grants, privilege, runtime, installer, integrity, and alert events.",
    },
    "SR-TST-005": {
        "story_contribution": "partial-story-evidence",
        "evidence": [
            "artifacts/sprints/sprint-3/story-3.1/configuration-migration-recovery-report.json",
            "artifacts/sprints/sprint-3/story-3.1/configuration-review-artifacts-report.json",
        ],
        "demonstrated": "Migration interruption is injected before and after all three durable transitions; every scenario selects the exact old or complete new state and rollback is idempotent.",
        "remaining": "Product-wide recovery must execute at least 100 crash resumes across all durable state transitions and prove no repeated completed operation.",
    },
}
Validator = Callable[[Path], list[str]]


def canonical_json(value: Any) -> bytes:
    return (json.dumps(value, indent=2, sort_keys=True) + "\n").encode("utf-8")


def sha256_file(path: Path) -> str:
    return hashlib.sha256(path.read_bytes()).hexdigest()


def read_json(path: Path) -> Any:
    return json.loads(path.read_text(encoding="utf-8"))


def safe_relative_path(value: Any) -> bool:
    if not isinstance(value, str) or not value:
        return False
    path = PurePosixPath(value)
    return not path.is_absolute() and ".." not in path.parts and str(path) == value


def write_atomic(path: Path, content: bytes) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    descriptor, temporary_name = tempfile.mkstemp(
        prefix=".agentmage-story-3-1-security-", dir=path.parent
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
        ("profiles", check_profiles),
        ("loader", check_loader),
        ("authority-mutations", check_authority_mutations),
        ("migration-recovery", check_migration_recovery),
        ("startup", check_startup),
        ("configuration-results", check_results),
        ("schema-failures", check_schema_failures),
        ("components", check_components),
        ("supply-chain", check_supply_chain),
    )
    failures = []
    for name, validator in validators:
        failures.extend(f"{name}: {failure}" for failure in validator(root))
    security_review = (root / "SECURITY-REVIEW.md").read_text(encoding="utf-8")
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


def build_map(root: Path = ROOT) -> dict[str, Any]:
    failures = validate_inputs(root)
    if failures:
        raise ValueError("; ".join(failures))
    return {
        "schema_version": 1,
        "story_id": "3.1",
        "task_id": "3.1.3.5",
        "status": "pass-shared-linux-security-mapping",
        "requirements": [
            {"requirement_id": requirement_id, **MAPPINGS[requirement_id]}
            for requirement_id in EXPECTED_REQUIREMENTS
        ],
        "artifacts": evidence_records(root),
        "summary": {
            "mapped_requirement_count": len(EXPECTED_REQUIREMENTS),
            "demonstrated_story_scope_count": 2,
            "partial_story_evidence_count": 7,
            "product_requirements_complete": 0,
            "retained_artifact_count": len(EVIDENCE_PATHS),
            "story_gate_complete": False,
        },
        "private_user_data_used": False,
        "raw_configuration_persisted": False,
        "network_used": False,
        "product_startup_activation_claim": "none",
        "release_claim": "none",
        "macos_execution_status": "blocked-macos",
        "macos_evidence_substituted": False,
        "macos_support_claim": "none",
    }


def validate_map(value: Any, root: Path = ROOT) -> list[str]:
    if not isinstance(value, dict):
        return ["Story 3.1 security evidence map must be an object"]
    failures = []
    if (
        value.get("schema_version") != 1
        or value.get("story_id") != "3.1"
        or value.get("task_id") != "3.1.3.5"
        or value.get("status") != "pass-shared-linux-security-mapping"
    ):
        failures.append("Story 3.1 security evidence identity is invalid")
    requirements = value.get("requirements", [])
    if not isinstance(requirements, list) or [
        item.get("requirement_id") for item in requirements if isinstance(item, dict)
    ] != list(EXPECTED_REQUIREMENTS):
        failures.append("Story 3.1 security requirement closure is invalid")
    else:
        for item in requirements:
            requirement_id = item["requirement_id"]
            if item != {"requirement_id": requirement_id, **MAPPINGS[requirement_id]}:
                failures.append(f"Story 3.1 security mapping changed: {requirement_id}")
            for path in item.get("evidence", []):
                if not safe_relative_path(path) or path not in EVIDENCE_PATHS:
                    failures.append(
                        f"Story 3.1 security evidence path is invalid: {requirement_id}"
                    )
    if value.get("summary") != {
        "mapped_requirement_count": 9,
        "demonstrated_story_scope_count": 2,
        "partial_story_evidence_count": 7,
        "product_requirements_complete": 0,
        "retained_artifact_count": len(EVIDENCE_PATHS),
        "story_gate_complete": False,
    }:
        failures.append("Story 3.1 security evidence summary is invalid")
    try:
        expected_artifacts = evidence_records(root)
    except OSError:
        failures.append("Story 3.1 retained evidence closure is unavailable")
    else:
        if value.get("artifacts") != expected_artifacts:
            failures.append("Story 3.1 retained evidence closure is stale")
    if (
        value.get("private_user_data_used") is not False
        or value.get("raw_configuration_persisted") is not False
        or value.get("network_used") is not False
        or value.get("product_startup_activation_claim") != "none"
        or value.get("release_claim") != "none"
        or value.get("macos_execution_status") != "blocked-macos"
        or value.get("macos_evidence_substituted") is not False
        or value.get("macos_support_claim") != "none"
    ):
        failures.append("Story 3.1 security evidence made an unsupported claim")
    return failures


def check_map(root: Path = ROOT) -> list[str]:
    try:
        value = read_json(root / REPORT_PATH.relative_to(ROOT))
    except (OSError, json.JSONDecodeError) as error:
        return [f"cannot read Story 3.1 security evidence map: {error}"]
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
        print(f"Story 3.1 security evidence failed: {error}")
        return 1
    if failures:
        for failure in failures:
            print(f"Story 3.1 security evidence failed: {failure}")
        return 1
    print("Story 3.1 security requirements mapped without product or macOS promotion")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
