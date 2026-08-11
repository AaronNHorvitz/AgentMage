#!/usr/bin/env python3
"""Build and validate the Story 7.1 product-security evidence map."""

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

from scripts.platform_manifest_artifact import check_report as check_platform_report


REPORT_PATH = ROOT / "artifacts/sprints/sprint-7/story-7.1/security-evidence-map.json"
EXPECTED_REQUIREMENTS = (
    "SR-GOV-006",
    "SR-PLT-009",
    "SR-PLT-010",
    "SR-PLT-011",
    "SR-PLT-012",
    "SR-SUP-002",
    "SR-SUP-003",
    "SR-SUP-004",
    "SR-SUP-005",
    "SR-TST-007",
)
EVIDENCE_PATHS = (
    "SECURITY-REVIEW.md",
    "docs/architecture/platform-adapter-contract.md",
    "kernel/contracts/src/platform.rs",
    "kernel/engine/src/platform_startup.rs",
    "kernel/engine/tests/platform_adapter_conformance.rs",
    "release/platform-manifests/README.md",
    "release/platform-manifests/v1/fedora-x86_64.json",
    "release/platform-manifests/v1/ubuntu-x86_64.json",
    "artifacts/sprints/sprint-2/story-2.1/platform-result-recorder-report.json",
    "artifacts/sprints/sprint-6/story-6.1/path-boundary-review.json",
    "artifacts/sprints/sprint-7/story-7.1/platform-contract-report.json",
    "supply-chain/dependency-provenance.json",
    "supply-chain/sbom.cdx.json",
    "scripts/platform_manifest_artifact.py",
    "scripts/platform_result_recorder.py",
    "scripts/story_7_1_security_evidence.py",
    "tests/test_platform_manifest_artifact.py",
    "tests/test_platform_result_recorder.py",
    "tests/test_story_7_1_security_evidence.py",
)
MAPPINGS = {
    "SR-GOV-006": {
        "story_contribution": "partial-boundary-model",
        "evidence": [
            "docs/architecture/platform-adapter-contract.md",
            "kernel/contracts/src/platform.rs",
            "release/platform-manifests/v1/fedora-x86_64.json",
            "release/platform-manifests/v1/ubuntu-x86_64.json",
        ],
        "demonstrated": "The shared startup boundary, runtime identity flow, ten capability domains, failure route, and Linux manifest mechanism inventories are explicit and machine checked.",
        "remaining": "A release-derived component, process, socket, store, and classified data-flow inventory must be generated from actual packages.",
    },
    "SR-PLT-009": {
        "story_contribution": "blocked-managed-platform-assessment",
        "evidence": ["docs/architecture/platform-adapter-contract.md"],
        "demonstrated": "The adapter contract does not request weakening endpoint controls and has a typed unavailable or invalid result for missing primitives.",
        "remaining": "The exact managed-like macOS baseline assessment and before/after comparison remain blocked and cannot be inferred from Linux.",
    },
    "SR-PLT-010": {
        "story_contribution": "partial-linux-manifest-contract",
        "evidence": [
            "release/platform-manifests/v1/fedora-x86_64.json",
            "release/platform-manifests/v1/ubuntu-x86_64.json",
            "artifacts/sprints/sprint-7/story-7.1/platform-contract-report.json",
        ],
        "demonstrated": "Fedora and Ubuntu fixtures freeze architecture, OS build, toolchain, Visual Studio Code, package, and mechanism identity fields; every shared runtime identity mismatch has a typed refusal.",
        "remaining": "Fixture package digests must be replaced by built packages, native Linux probes must bind them, and the complete macOS manifest and mutation matrix remain open.",
    },
    "SR-PLT-011": {
        "story_contribution": "partial-closed-contract",
        "evidence": [
            "kernel/contracts/src/platform.rs",
            "kernel/engine/src/platform_startup.rs",
            "supply-chain/sbom.cdx.json",
        ],
        "demonstrated": "Startup accepts only the closed adapter API, exact manifest identity, and ten declared capabilities; no arbitrary capability or fallback member exists.",
        "remaining": "Package loaders must prove refusal of unmanifested executable code, plugins, dynamic agents, and libraries against shipped binaries.",
    },
    "SR-PLT-012": {
        "story_contribution": "demonstrated-contract-evidence-separation",
        "evidence": [
            "kernel/engine/tests/platform_adapter_conformance.rs",
            "artifacts/sprints/sprint-7/story-7.1/platform-contract-report.json",
            "artifacts/sprints/sprint-2/story-2.1/platform-result-recorder-report.json",
        ],
        "demonstrated": "Manifest and result records distinguish fake, Fedora-local, Ubuntu-container, and blocked-macOS status; platform and manifest evidence cannot be replayed across adapters.",
        "remaining": "Every later native runtime, package, model, hardware, and clean-install result must continue using the same non-substitution rule.",
    },
    "SR-SUP-002": {
        "story_contribution": "partial-dependency-and-credential-boundary",
        "evidence": [
            "docs/architecture/platform-adapter-contract.md",
            "release/platform-manifests/v1/fedora-x86_64.json",
            "release/platform-manifests/v1/ubuntu-x86_64.json",
        ],
        "demonstrated": "The manifests separate maintainer-release, end-user-runtime, and excluded-end-user dependencies and explicitly retain no credential values.",
        "remaining": "Ephemeral release runners, protected branch controls, access records, MFA, and isolated signing authority require external release-environment evidence.",
    },
    "SR-SUP-003": {
        "story_contribution": "partial-pinned-input-contract",
        "evidence": [
            "release/platform-manifests/v1/fedora-x86_64.json",
            "release/platform-manifests/v1/ubuntu-x86_64.json",
            "supply-chain/dependency-provenance.json",
        ],
        "demonstrated": "Platform fixtures pin toolchain, Visual Studio Code, package, and mechanism identities while the repository records locked dependency provenance.",
        "remaining": "Actual package builders, SDKs, runtime binaries, conversion tools, and all shipped transitive artifacts must be bound and substitution-tested.",
    },
    "SR-SUP-004": {
        "story_contribution": "partial-source-dependency-sbom",
        "evidence": [
            "supply-chain/sbom.cdx.json",
            "supply-chain/dependency-provenance.json",
        ],
        "demonstrated": "The current accepted source dependency graph has a checked CycloneDX SBOM and dependency provenance ledger.",
        "remaining": "A complete release-build and shipped-binary SBOM with binary reconciliation and exact generation context does not yet exist.",
    },
    "SR-SUP-005": {
        "story_contribution": "partial-source-and-test-provenance",
        "evidence": [
            "artifacts/sprints/sprint-7/story-7.1/platform-contract-report.json",
            "supply-chain/dependency-provenance.json",
        ],
        "demonstrated": "The immutable evidence binds an exact source revision, every contract input hash, Fedora execution, and a pinned no-network Ubuntu image identity.",
        "remaining": "Signed release provenance must link an actual package to its builder, commands, dependencies, complete tests, artifacts, and signer.",
    },
    "SR-TST-007": {
        "story_contribution": "contract-execution-only",
        "evidence": ["artifacts/sprints/sprint-7/story-7.1/platform-contract-report.json"],
        "demonstrated": "The identical adapter contract runs locally on Fedora and in a constrained no-network Ubuntu 26.04 container from the committed source revision.",
        "remaining": "No installation, update, migration, rollback, uninstall, or recovery run is claimed; three clean systems per supported platform and all macOS runs remain open.",
    },
}
REVIEWER_PROTOCOLS = (
    ("RV-01", "partial-contract-no-release-package"),
    ("RV-02", "not-executed-no-installation"),
    ("RV-03", "partial-contract-no-native-sandbox"),
    ("RV-04", "partial-inherited-story-6-path-evidence"),
    ("RV-05", "not-implemented-no-ipc"),
)


def pretty_json(value: Any) -> bytes:
    return (json.dumps(value, indent=2, sort_keys=True) + "\n").encode()


def sha256_file(path: Path) -> str:
    return hashlib.sha256(path.read_bytes()).hexdigest()


def git_revision(candidate: str = "HEAD", root: Path = ROOT) -> str:
    completed = subprocess.run(
        ["git", "rev-parse", "--verify", f"{candidate}^{{commit}}"],
        cwd=root,
        check=False,
        stdout=subprocess.PIPE,
        stderr=subprocess.DEVNULL,
        timeout=10,
    )
    revision = completed.stdout.decode("ascii", "strict").strip()
    if completed.returncode != 0 or re.fullmatch(r"[0-9a-f]{40}", revision) is None:
        raise ValueError("Story 7.1 source revision is unavailable")
    return revision


def safe_relative_path(value: Any) -> bool:
    if not isinstance(value, str) or not value:
        return False
    path = PurePosixPath(value)
    return not path.is_absolute() and ".." not in path.parts and str(path) == value


def write_atomic(path: Path, content: bytes) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    descriptor, name = tempfile.mkstemp(prefix=".agentmage-story-7-1-security-", dir=path.parent)
    temporary = Path(name)
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
        check_platform_report(root)
    except (OSError, RuntimeError, ValueError, subprocess.SubprocessError) as error:
        failures.append(f"platform contract evidence: {error}")
    security = (root / "SECURITY-REVIEW.md").read_text(encoding="utf-8")
    for identifier in (*EXPECTED_REQUIREMENTS, *(item[0] for item in REVIEWER_PROTOCOLS)):
        if f"`{identifier}`" not in security:
            failures.append(f"security identifier is missing: {identifier}")
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
        "story_id": "7.1",
        "task_id": "7.1.3.5",
        "source_revision": source_revision,
        "status": "pass-non-macos-security-mapping",
        "requirements": [
            {
                "requirement_id": identifier,
                "product_requirement_status": "not-complete",
                **MAPPINGS[identifier],
            }
            for identifier in EXPECTED_REQUIREMENTS
        ],
        "reviewer_protocols": [
            {"protocol_id": identifier, "status": status}
            for identifier, status in REVIEWER_PROTOCOLS
        ],
        "artifacts": evidence_records(root),
        "summary": {
            "mapped_requirement_count": len(EXPECTED_REQUIREMENTS),
            "demonstrated_contract_scope_count": 1,
            "partial_or_blocked_scope_count": len(EXPECTED_REQUIREMENTS) - 1,
            "product_requirements_complete": 0,
            "retained_artifact_count": len(EVIDENCE_PATHS),
            "story_gate_complete": False,
        },
        "private_user_data_used": False,
        "network_used": False,
        "containerized_ubuntu_contract_execution": True,
        "external_human_review_status": "not-performed",
        "product_requirement_completion_claim": "none",
        "release_claim": "none",
        "macos_execution_status": "blocked-macos",
        "macos_evidence_substituted": False,
    }


def validate_map(value: Any, root: Path = ROOT) -> list[str]:
    failures: list[str] = []
    if not isinstance(value, dict):
        return ["Story 7.1 security evidence map must be an object"]
    if (
        value.get("schema_version") != 1
        or value.get("story_id") != "7.1"
        or value.get("task_id") != "7.1.3.5"
        or value.get("status") != "pass-non-macos-security-mapping"
        or re.fullmatch(r"[0-9a-f]{40}", str(value.get("source_revision"))) is None
    ):
        failures.append("Story 7.1 security evidence identity is invalid")
    requirements = value.get("requirements")
    identifiers = (
        [item.get("requirement_id") for item in requirements]
        if isinstance(requirements, list)
        else []
    )
    if identifiers != list(EXPECTED_REQUIREMENTS) or len(identifiers) != len(set(identifiers)):
        failures.append("Story 7.1 security requirement closure is invalid")
    else:
        for item in requirements:
            identifier = item["requirement_id"]
            expected = {
                "requirement_id": identifier,
                "product_requirement_status": "not-complete",
                **MAPPINGS[identifier],
            }
            if item != expected:
                failures.append(f"Story 7.1 security mapping changed: {identifier}")
            if any(
                not safe_relative_path(path) or path not in EVIDENCE_PATHS
                for path in item.get("evidence", [])
            ):
                failures.append(f"Story 7.1 security evidence path is invalid: {identifier}")
    expected_protocols = [
        {"protocol_id": identifier, "status": status}
        for identifier, status in REVIEWER_PROTOCOLS
    ]
    if value.get("reviewer_protocols") != expected_protocols:
        failures.append("Story 7.1 reviewer protocol states changed")
    expected_summary = {
        "mapped_requirement_count": len(EXPECTED_REQUIREMENTS),
        "demonstrated_contract_scope_count": 1,
        "partial_or_blocked_scope_count": len(EXPECTED_REQUIREMENTS) - 1,
        "product_requirements_complete": 0,
        "retained_artifact_count": len(EVIDENCE_PATHS),
        "story_gate_complete": False,
    }
    if value.get("summary") != expected_summary:
        failures.append("Story 7.1 security summary is invalid")
    try:
        expected_artifacts = evidence_records(root)
    except OSError:
        failures.append("Story 7.1 retained evidence is unavailable")
    else:
        if value.get("artifacts") != expected_artifacts:
            failures.append("Story 7.1 retained evidence closure is stale")
    if (
        value.get("private_user_data_used") is not False
        or value.get("network_used") is not False
        or value.get("containerized_ubuntu_contract_execution") is not True
        or value.get("external_human_review_status") != "not-performed"
        or value.get("product_requirement_completion_claim") != "none"
        or value.get("release_claim") != "none"
        or value.get("macos_execution_status") != "blocked-macos"
        or value.get("macos_evidence_substituted") is not False
    ):
        failures.append("Story 7.1 security evidence made an unsupported claim")
    return failures


def check_map(root: Path = ROOT) -> list[str]:
    try:
        value = json.loads((root / REPORT_PATH.relative_to(ROOT)).read_text(encoding="utf-8"))
    except (OSError, json.JSONDecodeError) as error:
        return [f"cannot read Story 7.1 security evidence map: {error}"]
    return [*validate_inputs(root), *validate_map(value, root)]


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--write", action="store_true")
    parser.add_argument("--source-revision", default="HEAD")
    args = parser.parse_args()
    try:
        if args.write:
            write_atomic(REPORT_PATH, pretty_json(build_map(git_revision(args.source_revision))))
        failures = check_map()
    except (OSError, ValueError, KeyError, TypeError, subprocess.SubprocessError) as error:
        print(f"Story 7.1 security evidence failed: {error}", file=sys.stderr)
        return 1
    if failures:
        for failure in failures:
            print(f"Story 7.1 security evidence failed: {failure}", file=sys.stderr)
        return 1
    print("Story 7.1 security requirements mapped without product or macOS promotion")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
