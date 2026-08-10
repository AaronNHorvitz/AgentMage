#!/usr/bin/env python3
"""Build and validate the Story 1.1 product-security evidence map."""

from __future__ import annotations

import argparse
import hashlib
import json
import re
import sys
from pathlib import Path, PurePosixPath
from typing import Any


ROOT = Path(__file__).resolve().parents[1]
OUTPUT_PATH = (
    ROOT
    / "artifacts"
    / "sprints"
    / "sprint-1"
    / "story-1.1"
    / "security-evidence-map.json"
)
EXPECTED_REQUIREMENTS = (
    "SR-PLT-001",
    "SR-PLT-010",
    "SR-PLT-011",
    "SR-PLT-012",
    "SR-SUP-002",
    "SR-SUP-003",
    "SR-SUP-004",
    "SR-SUP-005",
    "SR-SUP-006",
    "SR-SUP-011",
    "SR-TST-003",
)
EVIDENCE_PATHS = (
    "Cargo.lock",
    "LICENSE",
    "architecture/artifact-scan-policy.json",
    "architecture/build-contract.json",
    "architecture/clean-build-policy.json",
    "architecture/dependency-classes.json",
    "architecture/dependency-rules.json",
    "architecture/language-build-matrix.json",
    "architecture/module-inventory.json",
    "artifacts/sprints/sprint-1/story-1.1/artifact-index.json",
    "artifacts/sprints/sprint-1/story-1.1/artifact-scan-report.json",
    "artifacts/sprints/sprint-1/story-1.1/clean-build-report.json",
    "artifacts/sprints/sprint-1/story-1.1/dependency-injection-report.json",
    "artifacts/sprints/sprint-1/story-1.1/locked-resolution-report.json",
    "docs/decisions/0003-blocked-platform-lane-continuation.md",
    "docs/decisions/0004-language-and-build-system-architecture.md",
    "package-lock.json",
    "platforms/macos/Package.resolved",
    "release/clean-build/Containerfile.linux",
    "scripts/artifact_scanner.py",
    "scripts/clean_build_evidence.py",
    "scripts/dependency_injection.py",
    "scripts/locked_resolution.py",
    "scripts/story_1_1_security_evidence.py",
    "scripts/supply_chain.py",
    "supply-chain/dependency-hashes.sha256",
    "supply-chain/dependency-provenance.json",
    "supply-chain/sbom.cdx.json",
)
MAPPINGS = {
    "SR-PLT-001": {
        "story_contribution": "partial-story-evidence",
        "review_protocols": ["RV-02"],
        "evidence": [
            "architecture/clean-build-policy.json",
            "artifacts/sprints/sprint-1/story-1.1/clean-build-report.json",
        ],
        "demonstrated": (
            "The empty shared/Linux product and all declared development checks run as "
            "non-root UID/GID 10001 on clean Fedora and Ubuntu environments."
        ),
        "remaining": (
            "Installation and normal product operation still require clean-account tests on "
            "all release platforms; macOS execution remains blocked."
        ),
    },
    "SR-PLT-010": {
        "story_contribution": "partial-story-evidence",
        "review_protocols": ["RV-01", "RV-02"],
        "evidence": [
            "architecture/build-contract.json",
            "architecture/language-build-matrix.json",
            "artifacts/sprints/sprint-1/story-1.1/clean-build-report.json",
        ],
        "demonstrated": (
            "Build toolchains, Linux OS versions, architectures, source inputs, and container "
            "image identities are frozen for Story 1.1 evidence."
        ),
        "remaining": (
            "The release manifest must still bind VS Code, SDK, package, entitlement, helper, "
            "hardware, and exact macOS identities and reject every substituted release field."
        ),
    },
    "SR-PLT-011": {
        "story_contribution": "partial-story-evidence",
        "review_protocols": ["RV-01", "RV-19"],
        "evidence": [
            "architecture/artifact-scan-policy.json",
            "architecture/dependency-classes.json",
            "artifacts/sprints/sprint-1/story-1.1/artifact-scan-report.json",
            "supply-chain/sbom.cdx.json",
        ],
        "demonstrated": (
            "The source, synthetic build/package surfaces, and SBOM reject seeded undeclared "
            "binaries, dynamic loaders, privileged assumptions, secrets, and production licenses."
        ),
        "remaining": (
            "Signed packaged-binary inventories and runtime load-refusal tests remain required "
            "for every native release artifact, including macOS."
        ),
    },
    "SR-PLT-012": {
        "story_contribution": "demonstrated-story-scope",
        "review_protocols": ["RV-01", "RV-02", "RV-19"],
        "evidence": [
            "artifacts/sprints/sprint-1/story-1.1/artifact-scan-report.json",
            "artifacts/sprints/sprint-1/story-1.1/clean-build-report.json",
            "docs/decisions/0003-blocked-platform-lane-continuation.md",
        ],
        "demonstrated": (
            "Fedora, Ubuntu, shared, synthetic package, and blocked macOS results have distinct "
            "identities and cannot satisfy one another's gate."
        ),
        "remaining": (
            "Later native package, runtime, hardware, and adapter evidence must preserve the "
            "same separation through release."
        ),
    },
    "SR-SUP-002": {
        "story_contribution": "partial-story-evidence",
        "review_protocols": ["RV-01", "RV-19"],
        "evidence": [
            "architecture/clean-build-policy.json",
            "artifacts/sprints/sprint-1/story-1.1/clean-build-report.json",
            "release/clean-build/Containerfile.linux",
        ],
        "demonstrated": (
            "Linux verification uses rootless, ephemeral, least-authority environments with a "
            "read-only committed source archive and fresh writable state."
        ),
        "remaining": (
            "Protected source controls, independent test/release identities, signing, notarization, "
            "MFA, access logs, and credential separation remain release responsibilities."
        ),
    },
    "SR-SUP-003": {
        "story_contribution": "demonstrated-story-scope",
        "review_protocols": ["RV-01", "RV-13", "RV-19"],
        "evidence": [
            "Cargo.lock",
            "package-lock.json",
            "platforms/macos/Package.resolved",
            "artifacts/sprints/sprint-1/story-1.1/locked-resolution-report.json",
            "supply-chain/dependency-hashes.sha256",
        ],
        "demonstrated": (
            "Story 1.1 dependencies and build tools are exact, locked, hashed, resolved twice, and "
            "fail closed for substituted, missing, revoked, and wrong-platform inputs."
        ),
        "remaining": (
            "Every future parser, platform SDK, model, runtime, converter, and package tool must be "
            "admitted under the same rule before release."
        ),
    },
    "SR-SUP-004": {
        "story_contribution": "partial-story-evidence",
        "review_protocols": ["RV-01", "RV-19"],
        "evidence": [
            "artifacts/sprints/sprint-1/story-1.1/artifact-scan-report.json",
            "supply-chain/dependency-provenance.json",
            "supply-chain/sbom.cdx.json",
        ],
        "demonstrated": (
            "A deterministic CycloneDX 1.6 source/build dependency SBOM records component classes, "
            "versions, hashes, licenses, and dependency edges."
        ),
        "remaining": (
            "Shipped-binary and package SBOM generation, schema review in the release runner, and "
            "source-to-binary comparison remain unavailable before packaging."
        ),
    },
    "SR-SUP-005": {
        "story_contribution": "partial-story-evidence",
        "review_protocols": ["RV-01", "RV-19"],
        "evidence": [
            "artifacts/sprints/sprint-1/story-1.1/artifact-index.json",
            "artifacts/sprints/sprint-1/story-1.1/clean-build-report.json",
            "supply-chain/dependency-hashes.sha256",
        ],
        "demonstrated": (
            "Story artifacts bind source revision, build inputs, commands, toolchains, environment "
            "identities, dependency hashes, and evidence hashes."
        ),
        "remaining": (
            "A release runner must produce signed provenance linking final packages and SBOMs to an "
            "independently verifiable builder and signer."
        ),
    },
    "SR-SUP-006": {
        "story_contribution": "partial-story-evidence",
        "review_protocols": ["RV-13", "RV-19"],
        "evidence": [
            "LICENSE",
            "architecture/dependency-classes.json",
            "artifacts/sprints/sprint-1/story-1.1/artifact-scan-report.json",
            "supply-chain/dependency-provenance.json",
        ],
        "demonstrated": (
            "Dependency origin, integrity, class, version, and license metadata are retained; missing "
            "metadata remains visible rather than being approved."
        ),
        "remaining": (
            "The development-only khroma license metadata review is open, and ownership, control, "
            "maintainer, vulnerability, resilience, support, and alternatives review remains for "
            "every critical product component."
        ),
    },
    "SR-SUP-011": {
        "story_contribution": "partial-story-evidence",
        "review_protocols": ["RV-01", "RV-19"],
        "evidence": [
            "artifacts/sprints/sprint-1/story-1.1/clean-build-report.json",
            "artifacts/sprints/sprint-1/story-1.1/locked-resolution-report.json",
            "supply-chain/dependency-hashes.sha256",
        ],
        "demonstrated": (
            "Two clean offline resolutions produce one graph identity, and independent clean Fedora "
            "and Ubuntu runs reproduce unchanged supply-chain artifacts without ambient dependencies."
        ),
        "remaining": (
            "Final native/package builds must compare reproducible outputs or record bounded signed "
            "differences, including the unavailable macOS build."
        ),
    },
    "SR-TST-003": {
        "story_contribution": "partial-story-evidence",
        "review_protocols": ["RV-19"],
        "evidence": [
            "artifacts/sprints/sprint-1/story-1.1/artifact-scan-report.json",
            "artifacts/sprints/sprint-1/story-1.1/clean-build-report.json",
            "artifacts/sprints/sprint-1/story-1.1/dependency-injection-report.json",
            "artifacts/sprints/sprint-1/story-1.1/locked-resolution-report.json",
        ],
        "demonstrated": (
            "Pinned compiler/linter checks, dependency analysis, architecture mutation tests, secret "
            "patterns, and artifact policy scans rerun from clean Linux environments."
        ),
        "remaining": (
            "Release-grade pinned SAST, SCA, secret, binary, and platform-security analyzers, reviewed "
            "suppressions, signed results, and genuine macOS analysis remain required."
        ),
    },
}


def sha256_file(path: Path) -> str:
    return hashlib.sha256(path.read_bytes()).hexdigest()


def safe_relative_path(value: Any) -> bool:
    if not isinstance(value, str) or not value:
        return False
    path = PurePosixPath(value)
    return not path.is_absolute() and ".." not in path.parts and str(path) == value


def build_map(root: Path = ROOT) -> dict[str, Any]:
    clean_build = json.loads(
        (root / "artifacts/sprints/sprint-1/story-1.1/clean-build-report.json").read_text(
            encoding="utf-8"
        )
    )
    artifact_scan = json.loads(
        (root / "artifacts/sprints/sprint-1/story-1.1/artifact-scan-report.json").read_text(
            encoding="utf-8"
        )
    )
    requirements = []
    for requirement_id in EXPECTED_REQUIREMENTS:
        mapping = MAPPINGS[requirement_id]
        requirements.append(
            {
                "requirement_id": requirement_id,
                "product_requirement_status": "not-complete",
                **mapping,
            }
        )
    return {
        "schema_version": 1,
        "task_id": "1.1.3.5",
        "status": "complete-evidence-map-blocked-macos",
        "source_revision": clean_build["source"]["revision"],
        "requirements": requirements,
        "artifacts": [
            {"path": path, "sha256": sha256_file(root / path)}
            for path in EVIDENCE_PATHS
        ],
        "open_reviews": [
            {
                "id": "license-metadata-khroma-2.1.0",
                "status": "open-review",
                "scope": "excluded-development-dependency",
                "release_approved": False,
                "evidence": (
                    "artifacts/sprints/sprint-1/story-1.1/artifact-scan-report.json"
                ),
            }
        ],
        "summary": {
            "mapped_requirements": len(EXPECTED_REQUIREMENTS),
            "product_requirements_complete": 0,
            "linux_clean_platforms_passed": clean_build["summary"][
                "linux_platforms_passed"
            ],
            "canonical_blocking_artifact_findings": artifact_scan["summary"][
                "blocking_findings"
            ],
            "open_reviews": artifact_scan["summary"]["open_review_findings"],
            "task_mapping_complete": True,
            "story_gate_complete": False,
        },
        "macos": {
            "status": "blocked-macos",
            "evidence_substitution": "prohibited",
            "support_claim": "none",
        },
        "release_claim": "none",
    }


def validate_map(evidence_map: Any, root: Path = ROOT) -> list[str]:
    failures: list[str] = []
    if not isinstance(evidence_map, dict):
        return ["Story 1.1 security evidence map must be an object"]
    if evidence_map.get("schema_version") != 1 or evidence_map.get("task_id") != "1.1.3.5":
        failures.append("Story 1.1 security evidence map identity is invalid")
    if evidence_map.get("status") != "complete-evidence-map-blocked-macos":
        failures.append("Story 1.1 security evidence map status is invalid")
    requirements = evidence_map.get("requirements", [])
    if tuple(item.get("requirement_id") for item in requirements) != EXPECTED_REQUIREMENTS:
        failures.append("Story 1.1 security requirement closure is invalid")
    for requirement in requirements:
        requirement_id = requirement.get("requirement_id", "unknown")
        if requirement.get("product_requirement_status") != "not-complete":
            failures.append(f"product requirement was promoted: {requirement_id}")
        if requirement.get("story_contribution") not in {
            "demonstrated-story-scope",
            "partial-story-evidence",
        }:
            failures.append(f"invalid story contribution: {requirement_id}")
        if not requirement.get("remaining"):
            failures.append(f"missing remaining work: {requirement_id}")
        for path in requirement.get("evidence", []):
            if path not in EVIDENCE_PATHS:
                failures.append(f"unbound requirement evidence: {requirement_id}: {path}")
    artifacts = evidence_map.get("artifacts", [])
    if tuple(item.get("path") for item in artifacts) != EVIDENCE_PATHS:
        failures.append("Story 1.1 security artifact closure is invalid")
    for artifact in artifacts:
        path = artifact.get("path")
        if not safe_relative_path(path):
            failures.append(f"unsafe Story 1.1 security artifact path: {path}")
            continue
        candidate = root / path
        if not candidate.is_file():
            failures.append(f"missing Story 1.1 security artifact: {path}")
        elif artifact.get("sha256") != sha256_file(candidate):
            failures.append(f"Story 1.1 security artifact hash mismatch: {path}")
    if evidence_map.get("open_reviews") != [
        {
            "id": "license-metadata-khroma-2.1.0",
            "status": "open-review",
            "scope": "excluded-development-dependency",
            "release_approved": False,
            "evidence": "artifacts/sprints/sprint-1/story-1.1/artifact-scan-report.json",
        }
    ]:
        failures.append("Story 1.1 open review was omitted or weakened")
    if evidence_map.get("macos") != {
        "status": "blocked-macos",
        "evidence_substitution": "prohibited",
        "support_claim": "none",
    }:
        failures.append("Story 1.1 security evidence promoted macOS")
    if evidence_map.get("release_claim") != "none":
        failures.append("Story 1.1 security evidence made a release claim")
    if evidence_map != build_map(root):
        failures.append("Story 1.1 security evidence map is stale or non-deterministic")
    return failures


def write_map(root: Path = ROOT) -> None:
    output = root / OUTPUT_PATH.relative_to(ROOT)
    output.parent.mkdir(parents=True, exist_ok=True)
    output.write_text(
        json.dumps(build_map(root), indent=2, sort_keys=True) + "\n",
        encoding="utf-8",
    )


def check_map(root: Path = ROOT) -> list[str]:
    try:
        evidence_map = json.loads(
            (root / OUTPUT_PATH.relative_to(ROOT)).read_text(encoding="utf-8")
        )
    except (OSError, json.JSONDecodeError) as error:
        return [f"cannot read Story 1.1 security evidence map: {error}"]
    return validate_map(evidence_map, root)


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--write", action="store_true")
    args = parser.parse_args()
    try:
        if args.write:
            write_map()
        failures = check_map()
    except OSError as error:
        print(f"Story 1.1 security evidence failed: {error}", file=sys.stderr)
        return 1
    if failures:
        for failure in failures:
            print(f"Story 1.1 security evidence failed: {failure}", file=sys.stderr)
        return 1
    print("Story 1.1 security requirements mapped without product or macOS promotion")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
