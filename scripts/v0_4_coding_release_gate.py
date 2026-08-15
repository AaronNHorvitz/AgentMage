#!/usr/bin/env python3
"""Build and validate the fail-closed AgentMage v0.4 coding release candidate."""

from __future__ import annotations

import argparse
import hashlib
import json
from pathlib import Path
from typing import Any, Final


ROOT: Final = Path(__file__).resolve().parents[1]
MANIFEST_PATH: Final = ROOT / "release/v0.4-coding-pack-manifest.json"
REPORT_PATH: Final = ROOT / "artifacts/sprints/sprint-50/v0.4-release-readiness.json"
SOURCE_PATHS: Final = sorted([
    "artifacts/sprints/sprint-50/coding-skill-pack.json",
    "capabilities/knowledge/examples/coding_skill_pack.rs",
    "capabilities/knowledge/src/coding_skills.rs",
    "capabilities/knowledge/tests/sprint_50_coding_corpus.rs",
    "docs/architecture/coding-skills-and-release-boundary.md",
    "docs/guides/bounded-coding-workflows.md",
    "docs/verification/sprint-45-coding-corpus.json",
    "docs/verification/sprint-50-coding-corpus.json",
    "kernel/engine/src/filesystem_control.rs",
    "kernel/engine/src/instruction_provenance.rs",
    "kernel/engine/src/local_commit.rs",
    "kernel/engine/src/repository_safety.rs",
    "kernel/engine/src/review_packet.rs",
    "kernel/engine/src/validation_result.rs",
    "kernel/engine/src/write_transaction.rs",
    "shells/host/src/cli.rs",
    "shells/host/src/headless.rs",
])
SKILLS: Final = [
    "repository_cartographer",
    "feature_trace",
    "change_impact",
    "debugging",
    "test_and_verification",
    "repository_documentation",
    "bug_reproduction",
    "git_history_analysis",
    "bounded_review",
]
EXCLUSIONS: Final = [
    "arbitrary-shell",
    "automatic-commit",
    "automatic-dependency-upgrade",
    "automatic-deploy",
    "automatic-merge",
    "automatic-migration",
    "automatic-pr-publication",
    "automatic-push",
    "automatic-refactor",
    "automatic-release",
    "frontier-handoff",
    "network-publication",
    "unattended-write",
]
BLOCKERS: Final = [
    "upstream-sprints-41-through-49-blocked",
    "coding-product-coordinator-absent",
    "authenticated-chat-cli-workflow-absent",
    "native-cross-interface-parity-absent",
    "write-command-and-commit-profiles-unregistered",
    "admitted-live-model-profile-absent",
    "native-cross-platform-acceptance-absent",
    "lifecycle-accessibility-recovery-campaign-absent",
    "signed-v0.4-packages-absent",
    "independent-release-decision-absent",
    "manual-fuzzing-deferred",
]


def sha256(path: Path) -> str:
    return hashlib.sha256(path.read_bytes()).hexdigest()


def build_manifest() -> dict[str, Any]:
    return {
        "schema_version": 1,
        "record_type": "agentmage-v0.4-coding-pack-manifest",
        "version": "0.4.0",
        "status": "blocked-local-candidate",
        "gate_id": "G-V0.4",
        "gate_closed": False,
        "product_registration": False,
        "coding_coordinator_integrated": False,
        "authenticated_cli_transport": False,
        "package_artifacts_published": False,
        "signed_release": False,
        "coding_skills": SKILLS,
        "excluded_capabilities": EXCLUSIONS,
        "source_files": [
            {"path": path, "sha256": sha256(ROOT / path)} for path in SOURCE_PATHS
        ],
        "blockers": BLOCKERS,
        "network_access": False,
        "automatic_publication": False,
        "release_claim": "none",
    }


def validate_manifest(manifest: Any) -> list[str]:
    expected = build_manifest()
    failures = []
    if manifest != expected:
        failures.append("v0.4 coding manifest drifted")
    for field in (
        "gate_closed",
        "product_registration",
        "coding_coordinator_integrated",
        "authenticated_cli_transport",
        "package_artifacts_published",
        "signed_release",
        "network_access",
        "automatic_publication",
    ):
        if not isinstance(manifest, dict) or manifest.get(field) is not False:
            failures.append(f"v0.4 coding manifest overclaim: {field}")
    return failures


def build_report() -> dict[str, Any]:
    manifest = json.loads(MANIFEST_PATH.read_text(encoding="utf-8"))
    failures = validate_manifest(manifest)
    return {
        "schema_version": 1,
        "record_type": "agentmage-v0.4-coding-release-readiness",
        "manifest_sha256": sha256(MANIFEST_PATH),
        "local_manifest_valid": not failures,
        "coding_skill_contracts_passed": True,
        "coding_coordinator_integrated": False,
        "native_chat_workflow_complete": False,
        "native_cli_workflow_complete": False,
        "cross_interface_parity_complete": False,
        "admitted_live_model_available": False,
        "fedora_acceptance": False,
        "ubuntu_acceptance": False,
        "windows_acceptance": False,
        "lifecycle_accessibility_recovery_complete": False,
        "independent_review_complete": False,
        "manual_fuzzing_complete": False,
        "package_signing_allowed": False,
        "gate_closed": False,
        "release_allowed": False,
        "failures": failures,
        "blockers": BLOCKERS,
    }


def validate_report(report: Any) -> list[str]:
    expected = build_report()
    failures = []
    if report != expected:
        failures.append("v0.4 coding release readiness report drifted")
    if expected["failures"]:
        failures.extend(expected["failures"])
    for field in (
        "coding_coordinator_integrated",
        "native_chat_workflow_complete",
        "native_cli_workflow_complete",
        "cross_interface_parity_complete",
        "admitted_live_model_available",
        "fedora_acceptance",
        "ubuntu_acceptance",
        "windows_acceptance",
        "lifecycle_accessibility_recovery_complete",
        "independent_review_complete",
        "manual_fuzzing_complete",
        "package_signing_allowed",
        "gate_closed",
        "release_allowed",
    ):
        if not isinstance(report, dict) or report.get(field) is not False:
            failures.append(f"v0.4 coding release overclaim: {field}")
    return failures


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--write", action="store_true")
    arguments = parser.parse_args()
    if arguments.write:
        MANIFEST_PATH.parent.mkdir(parents=True, exist_ok=True)
        MANIFEST_PATH.write_text(
            json.dumps(build_manifest(), indent=2, sort_keys=True) + "\n",
            encoding="utf-8",
        )
    if not MANIFEST_PATH.is_file():
        print("v0.4 coding release gate failed: manifest absent")
        return 1
    if arguments.write:
        REPORT_PATH.parent.mkdir(parents=True, exist_ok=True)
        REPORT_PATH.write_text(
            json.dumps(build_report(), indent=2, sort_keys=True) + "\n",
            encoding="utf-8",
        )
    if not REPORT_PATH.is_file():
        print("v0.4 coding release gate failed: report absent")
        return 1
    manifest = json.loads(MANIFEST_PATH.read_text(encoding="utf-8"))
    report = json.loads(REPORT_PATH.read_text(encoding="utf-8"))
    failures = validate_manifest(manifest) + validate_report(report)
    if failures:
        print("v0.4 coding release gate failed:")
        for failure in failures:
            print(f"- {failure}")
        return 1
    print("v0.4 coding release gate remains blocked as declared")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
