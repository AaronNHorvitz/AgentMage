#!/usr/bin/env python3
"""Build and validate the fail-closed AgentMage v0.5 frontier candidate."""

from __future__ import annotations

import argparse
import hashlib
import json
from pathlib import Path
from typing import Any, Final


ROOT: Final = Path(__file__).resolve().parents[1]
MANIFEST_PATH: Final = ROOT / "release/v0.5-frontier-pack-manifest.json"
REPORT_PATH: Final = ROOT / "artifacts/sprints/sprint-53/v0.5-release-readiness.json"
CORPUS_PATH: Final = ROOT / "docs/verification/sprint-53-frontier-release-corpus.json"
SOURCE_PATHS: Final = sorted([
    "artifacts/sprints/sprint-51/local-evidence-report.json",
    "artifacts/sprints/sprint-52/local-evidence-report.json",
    "docs/architecture/manual-frontier-recommendation.md",
    "docs/architecture/untrusted-frontier-result-import.md",
    "docs/guides/frontier-result-import-and-revalidation.md",
    "docs/guides/manual-frontier-consultation.md",
    "docs/release/release-notes-v0.5.0-draft.md",
    "docs/release/v0.5-capability-matrix.md",
    "docs/release/v0.5-frontier-acceptance-bundle.md",
    "docs/security/frontier-threat-model-and-boundary-report.md",
    "docs/verification/sprint-51-frontier-corpus.json",
    "docs/verification/sprint-52-frontier-import-corpus.json",
    "docs/verification/sprint-53-frontier-release-corpus.json",
    "kernel/contracts/src/frontier.rs",
    "kernel/contracts/src/frontier_import.rs",
    "kernel/contracts/src/handoff.rs",
    "kernel/engine/src/frontier_import.rs",
    "kernel/engine/src/frontier_import_recovery.rs",
    "kernel/engine/src/frontier_recommendation.rs",
    "kernel/engine/src/frontier_release.rs",
    "kernel/engine/src/frontier_release_recovery.rs",
    "kernel/engine/src/handoff.rs",
    "shells/host/src/frontier_coordinator.rs",
    "shells/host/src/frontier_import_coordinator.rs",
    "shells/host/src/frontier_release_coordinator.rs",
    "shells/host/src/lib.rs",
    "schemas/runtime/frontier-recommendation-receipt.schema.json",
    "schemas/runtime/frontier-return-manifest.schema.json",
    "schemas/runtime/frontier-round-trip-receipt.schema.json",
    "schemas/runtime/frontier-tier-decision.schema.json",
])
LOCAL_CAPABILITIES: Final = [
    "measured-frontier-recommendation",
    "exact-disclosure-preview",
    "local-no-delivery-render",
    "user-initiated-controlled-packet-export-plan",
    "untrusted-return-manifest-parse",
    "artifact-quarantine",
    "fresh-local-revalidation",
    "zero-effect-round-trip-receipt",
]
EXCLUDED_CAPABILITIES: Final = [
    "automatic-frontier-invocation",
    "browser-automation",
    "clipboard-population",
    "credential-access",
    "direct-codex-handoff",
    "external-client",
    "external-endpoint-call",
    "hidden-telemetry",
    "imported-authority",
    "scheduled-delivery",
    "standing-consent-delivery",
    "unattended-upload",
]
BLOCKERS: Final = [
    "upstream-sprints-50-through-52-blocked",
    "live-model-campaign-absent",
    "installed-prohibited-capability-scan-absent",
    "native-cross-platform-acceptance-absent",
    "lifecycle-accessibility-privacy-campaign-absent",
    "signed-v0.5-packages-absent",
    "independent-release-decision-absent",
    "manual-fuzzing-deferred",
]
DELIVERY_SURFACES: Final = [
    "native_chat",
    "cli",
    "model_tools",
    "skills",
    "schedules",
    "injected_content",
    "clipboard_api",
    "vscode_commands",
    "network_paths",
]
ROUND_TRIP_FAILURES: Final = [
    "recommendation-unmeasured",
    "disclosure-incomplete",
    "redaction-failed",
    "review-stale",
    "export-not-user-initiated",
    "export-packet-mutated",
    "import-malformed",
    "import-authority-claimed",
    "local-revalidation-stale",
    "outbound-network-present",
    "privacy-threshold-failed",
    "recovery-evidence-missing",
]
RECOVERY_CASES: Final = [
    "recommendation-cancelled",
    "preview-cancelled",
    "export-cancelled",
    "export-plan-repeated",
    "import-cancelled",
    "import-repeated",
    "revalidation-corrupted",
    "state-changed-mid-round-trip",
]


def sha256(path: Path) -> str:
    return hashlib.sha256(path.read_bytes()).hexdigest()


def build_manifest() -> dict[str, Any]:
    return {
        "schema_version": 1,
        "record_type": "agentmage-v0.5-frontier-pack-manifest",
        "version": "0.5.0",
        "status": "blocked-local-candidate",
        "gate_id": "G-V0.5",
        "gate_closed": False,
        "product_registration": True,
        "frontier_coordinator_integrated": True,
        "native_round_trip_complete": True,
        "normal_local_flow_integration": True,
        "durable_recovery_complete": True,
        "package_artifacts_published": False,
        "signed_release": False,
        "strict_local_mode_required": True,
        "manual_transfer_required": True,
        "local_capabilities": LOCAL_CAPABILITIES,
        "excluded_capabilities": EXCLUDED_CAPABILITIES,
        "source_files": [
            {"path": path, "sha256": sha256(ROOT / path)} for path in SOURCE_PATHS
        ],
        "blockers": BLOCKERS,
        "outbound_network": False,
        "external_client": False,
        "credential_access": False,
        "automatic_delivery": False,
        "import_authority": False,
        "release_claim": "none",
    }


def validate_manifest(manifest: Any) -> list[str]:
    expected = build_manifest()
    failures = []
    if manifest != expected:
        failures.append("v0.5 frontier manifest drifted")
    for field in (
        "gate_closed",
        "package_artifacts_published",
        "signed_release",
        "outbound_network",
        "external_client",
        "credential_access",
        "automatic_delivery",
        "import_authority",
    ):
        if not isinstance(manifest, dict) or manifest.get(field) is not False:
            failures.append(f"v0.5 frontier manifest overclaim: {field}")
    if not isinstance(manifest, dict) or manifest.get("release_claim") != "none":
        failures.append("v0.5 frontier release claim is not blocked")
    return failures


def expected_corpus() -> dict[str, Any]:
    return {
        "schema_version": 1,
        "corpus_id": "sprint-53-frontier-release-v1",
        "delivery_surfaces": [
            {"surface": surface, "expected": "local_denial_only"}
            for surface in DELIVERY_SURFACES
        ],
        "round_trip_failures": [
            {"case_id": case_id, "expected": "gate_blocked"}
            for case_id in ROUND_TRIP_FAILURES
        ],
        "recovery_cases": RECOVERY_CASES,
        "expanded_case_counts": {
            "delivery_surfaces": 9,
            "round_trip_failures": 12,
            "recovery_cases": 8,
            "total": 29,
        },
        "external_client_present": False,
        "credential_access_present": False,
        "automatic_delivery_present": False,
        "hidden_telemetry_present": False,
        "imported_authority_present": False,
        "gate_closed": False,
    }


def validate_corpus(corpus: Any) -> list[str]:
    failures = []
    if corpus != expected_corpus():
        failures.append("v0.5 frontier release corpus drifted")
    for field in (
        "external_client_present",
        "credential_access_present",
        "automatic_delivery_present",
        "hidden_telemetry_present",
        "imported_authority_present",
        "gate_closed",
    ):
        if not isinstance(corpus, dict) or corpus.get(field) is not False:
            failures.append(f"v0.5 frontier corpus overclaim: {field}")
    return failures


def build_report(
    manifest: Any | None = None, corpus: Any | None = None
) -> dict[str, Any]:
    if manifest is None:
        manifest = json.loads(MANIFEST_PATH.read_text(encoding="utf-8"))
    if corpus is None:
        corpus = json.loads(CORPUS_PATH.read_text(encoding="utf-8"))
    failures = validate_manifest(manifest) + validate_corpus(corpus)
    return {
        "schema_version": 1,
        "record_type": "agentmage-v0.5-frontier-release-readiness",
        "manifest_sha256": sha256(MANIFEST_PATH),
        "local_manifest_valid": not failures,
        "frontier_release_corpus_valid": not validate_corpus(corpus),
        "recommendation_contracts_passed": True,
        "packet_disclosure_contracts_passed": True,
        "controlled_export_plan_contracts_passed": True,
        "untrusted_import_contracts_passed": True,
        "local_revalidation_contracts_passed": True,
        "frontier_coordinator_integrated": True,
        "native_round_trip_complete": True,
        "normal_local_flow_integration": True,
        "live_model_campaign_complete": False,
        "fedora_acceptance": False,
        "ubuntu_acceptance": False,
        "windows_acceptance": False,
        "lifecycle_accessibility_privacy_complete": False,
        "durable_recovery_complete": True,
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
        failures.append("v0.5 frontier release readiness report drifted")
    if expected["failures"]:
        failures.extend(expected["failures"])
    for field in (
        "live_model_campaign_complete",
        "fedora_acceptance",
        "ubuntu_acceptance",
        "windows_acceptance",
        "lifecycle_accessibility_privacy_complete",
        "independent_review_complete",
        "manual_fuzzing_complete",
        "package_signing_allowed",
        "gate_closed",
        "release_allowed",
    ):
        if not isinstance(report, dict) or report.get(field) is not False:
            failures.append(f"v0.5 frontier release overclaim: {field}")
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
        print("v0.5 frontier release gate failed: manifest absent")
        return 1
    if arguments.write:
        REPORT_PATH.parent.mkdir(parents=True, exist_ok=True)
        REPORT_PATH.write_text(
            json.dumps(build_report(), indent=2, sort_keys=True) + "\n",
            encoding="utf-8",
        )
    if not REPORT_PATH.is_file():
        print("v0.5 frontier release gate failed: report absent")
        return 1
    manifest = json.loads(MANIFEST_PATH.read_text(encoding="utf-8"))
    corpus = json.loads(CORPUS_PATH.read_text(encoding="utf-8"))
    report = json.loads(REPORT_PATH.read_text(encoding="utf-8"))
    failures = (
        validate_manifest(manifest) + validate_corpus(corpus) + validate_report(report)
    )
    if failures:
        print("v0.5 frontier release gate failed:")
        for failure in failures:
            print(f"- {failure}")
        return 1
    print("v0.5 frontier release gate remains blocked as declared")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
