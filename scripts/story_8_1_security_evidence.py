#!/usr/bin/env python3
"""Build the blocked-external Story 8.1 product-security evidence map."""

from __future__ import annotations

import argparse
import hashlib
import json
import os
import re
import subprocess
import tempfile
from pathlib import Path
from typing import Any

ROOT = Path(__file__).resolve().parents[1]
REPORT_PATH = ROOT / "artifacts/sprints/sprint-8/story-8.1/security-evidence-map.json"
REQUIREMENTS = (
    "SR-PLT-001",
    "SR-PLT-002",
    "SR-PLT-003",
    "SR-PLT-004",
    "SR-PLT-005",
    "SR-PLT-006",
    "SR-PLT-007",
    "SR-PLT-008",
    "SR-SUP-002",
    "SR-TST-007",
    "SR-TST-008",
    "SR-TST-009",
)
PROTOCOLS = ("RV-01", "RV-02", "RV-03", "RV-04", "RV-05")
SOURCE_REPORTS = (
    "macos-kernel-host-source-contract.json",
    "macos-bridge-source-contract.json",
    "macos-peer-verification-source-contract.json",
    "macos-workspace-bookmark-source-contract.json",
    "macos-tool-helper-source-contract.json",
    "macos-metal-inference-source-contract.json",
    "macos-release-runner-source-contract.json",
    "macos-signed-reference-package-source-contract.json",
    "macos-release-reports-source-contract.json",
    "macos-ipc-bookmark-receipts-source-contract.json",
    "macos-sandbox-attack-results-source-contract.json",
    "macos-ipc-handshake-refusal-results-source-contract.json",
    "macos-sandbox-attack-matrix-source-contract.json",
    "macos-lifecycle-recovery-results-source-contract.json",
    "macos-standard-user-acceptance-source-contract.json",
)
REPORT_ROOT = "artifacts/sprints/sprint-8/story-8.1"
EVIDENCE_PATHS = (
    "SECURITY-REVIEW.md",
    "docs/security/story-8.1-product-security-evidence.md",
    "scripts/story_8_1_security_evidence.py",
    "tests/test_story_8_1_security_evidence.py",
    *(f"{REPORT_ROOT}/{name}" for name in SOURCE_REPORTS),
)
NATIVE_CLASSES = (
    "signature-notarization-output",
    "entitlement-output",
    "ipc-traces",
    "sandbox-results",
    "install-log-video",
    "independent-reviewer-record",
)
MAPPING_PATHS = {
    "SR-PLT-001": ("macos-standard-user-acceptance-source-contract.json",),
    "SR-PLT-002": ("macos-release-reports-source-contract.json",),
    "SR-PLT-003": ("macos-sandbox-attack-results-source-contract.json",),
    "SR-PLT-004": ("macos-workspace-bookmark-source-contract.json",),
    "SR-PLT-005": ("macos-bridge-source-contract.json",),
    "SR-PLT-006": ("macos-ipc-handshake-refusal-results-source-contract.json",),
    "SR-PLT-007": ("macos-metal-inference-source-contract.json",),
    "SR-PLT-008": ("macos-release-runner-source-contract.json",),
    "SR-SUP-002": ("macos-release-runner-source-contract.json",),
    "SR-TST-007": (
        "macos-lifecycle-recovery-results-source-contract.json",
        "macos-standard-user-acceptance-source-contract.json",
    ),
    "SR-TST-008": ("macos-sandbox-attack-matrix-source-contract.json",),
    "SR-TST-009": (
        "macos-signed-reference-package-source-contract.json",
        "macos-release-reports-source-contract.json",
    ),
}
PROTOCOL_PATHS = {
    "RV-01": ("macos-release-reports-source-contract.json",),
    "RV-02": ("macos-standard-user-acceptance-source-contract.json",),
    "RV-03": ("macos-sandbox-attack-matrix-source-contract.json",),
    "RV-04": ("macos-workspace-bookmark-source-contract.json",),
    "RV-05": ("macos-ipc-handshake-refusal-results-source-contract.json",),
}


def sha256_file(path: Path) -> str:
    return hashlib.sha256(path.read_bytes()).hexdigest()


def canonical(value: Any) -> bytes:
    return (json.dumps(value, indent=2, sort_keys=True) + "\n").encode()


def write_atomic(path: Path, content: bytes) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    descriptor, name = tempfile.mkstemp(prefix=".agentmage-story-8-1-security-", dir=path.parent)
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


def revision(candidate: str, root: Path = ROOT) -> str:
    value = subprocess.run(
        ["git", "rev-parse", "--verify", f"{candidate}^{{commit}}"],
        cwd=root,
        check=True,
        capture_output=True,
    ).stdout.decode().strip()
    if re.fullmatch(r"[0-9a-f]{40}", value) is None:
        raise ValueError("Story 8.1 source revision is invalid")
    return value


def validate_inputs(root: Path = ROOT) -> list[str]:
    failures = [path for path in EVIDENCE_PATHS if not (root / path).is_file()]
    if failures:
        return [f"missing Story 8.1 security input: {path}" for path in failures]
    security = (root / "SECURITY-REVIEW.md").read_text(encoding="utf-8")
    tasks = (root / "TASKS.md").read_text(encoding="utf-8")
    for identifier in REQUIREMENTS:
        if f"`{identifier}`" not in security:
            failures.append(f"missing security requirement: {identifier}")
    for identifier in PROTOCOLS:
        if f"`{identifier}`" not in tasks:
            failures.append(f"missing reviewer protocol: {identifier}")
    for name in SOURCE_REPORTS:
        value = json.loads((root / REPORT_ROOT / name).read_text(encoding="utf-8"))
        if value.get("status") not in {
            "partial-source-only-blocked-macos",
            "prepared-source-only-blocked-macos",
        }:
            failures.append(f"macOS source report status changed: {name}")
        if any(value.get("claims", {}).values()):
            failures.append(f"macOS source report overclaims: {name}")
    return failures


def report_path(name: str) -> str:
    return f"{REPORT_ROOT}/{name}"


def build_map(source_revision: str, root: Path = ROOT) -> dict[str, Any]:
    failures = validate_inputs(root)
    if failures:
        raise ValueError("; ".join(failures))
    if re.fullmatch(r"[0-9a-f]{40}", source_revision) is None:
        raise ValueError("Story 8.1 source revision is invalid")
    return {
        "schema_version": 1,
        "story_id": "8.1",
        "task_id": "8.1.3.5",
        "source_revision": source_revision,
        "status": "pass-source-map-blocked-external",
        "requirements": [
            {
                "requirement_id": identifier,
                "status": "partial-source-prepared-native-evidence-missing",
                "evidence": [report_path(name) for name in MAPPING_PATHS[identifier]],
            }
            for identifier in REQUIREMENTS
        ],
        "reviewer_protocols": [
            {
                "protocol_id": identifier,
                "status": "prepared-not-executed",
                "evidence": [report_path(name) for name in PROTOCOL_PATHS[identifier]],
            }
            for identifier in PROTOCOLS
        ],
        "native_evidence_inventory": [
            {"evidence_class": name, "status": "missing-external", "retained": False}
            for name in NATIVE_CLASSES
        ],
        "artifacts": [
            {
                "path": path,
                "sha256": sha256_file(root / path),
                "bytes": (root / path).stat().st_size,
            }
            for path in EVIDENCE_PATHS
        ],
        "summary": {
            "mapped_requirement_count": len(REQUIREMENTS),
            "mapped_reviewer_protocol_count": len(PROTOCOLS),
            "source_contract_count": len(SOURCE_REPORTS),
            "required_native_evidence_class_count": len(NATIVE_CLASSES),
            "retained_native_evidence_class_count": 0,
            "product_requirements_complete": 0,
            "story_gate_complete": False,
        },
        "private_user_data_used": False,
        "network_used": False,
        "macos_execution_performed": False,
        "independent_review_performed": False,
        "macos_evidence_substituted": False,
        "release_claim": "none",
        "macos_support_claim": False,
    }


def check_map(root: Path = ROOT) -> list[str]:
    try:
        retained = json.loads((root / REPORT_PATH.relative_to(ROOT)).read_text(encoding="utf-8"))
        source_revision = retained.get("source_revision")
        if not isinstance(source_revision, str):
            return ["Story 8.1 security map has no source revision"]
        if canonical(retained) != canonical(build_map(source_revision, root)):
            return ["Story 8.1 security map is stale or mutated"]
        return []
    except (OSError, ValueError, json.JSONDecodeError, subprocess.SubprocessError) as error:
        return [str(error)]


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--write", action="store_true")
    parser.add_argument("--source-revision", default="HEAD")
    args = parser.parse_args()
    try:
        if args.write:
            write_atomic(REPORT_PATH, canonical(build_map(revision(args.source_revision))))
        failures = check_map()
        if failures:
            raise ValueError("; ".join(failures))
    except (OSError, ValueError, json.JSONDecodeError, subprocess.SubprocessError) as error:
        print(f"Story 8.1 security evidence failed: {error}")
        return 1
    print("Story 8.1 security evidence mapped with native blockers preserved")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
