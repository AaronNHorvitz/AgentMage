#!/usr/bin/env python3
"""Build and validate the Sprint 11 product-security evidence map."""

from __future__ import annotations

import argparse
import hashlib
import json
import os
import re
import subprocess
from pathlib import Path, PurePosixPath
from typing import Any, Final

try:
    from scripts.evidence_core import atomic_write, canonical_json_bytes
except ModuleNotFoundError:
    from evidence_core import atomic_write, canonical_json_bytes

ROOT: Final = Path(__file__).resolve().parents[1]
REPORT_PATH: Final = ROOT / "artifacts/sprints/sprint-11/story-11.1/security-evidence-map.json"
REQUIREMENT_IDS: Final = tuple(f"SR-DAT-{index:03}" for index in range(1, 13)) + tuple(
    f"SR-OPS-{index:03}" for index in range(1, 6)
) + ("SR-TST-005",)
PROTOCOL_IDS: Final = ("RV-08", "RV-09", "RV-10")
EXPECTED_IDS: Final = REQUIREMENT_IDS + PROTOCOL_IDS
ARTIFACT_ROOT: Final = "artifacts/sprints/sprint-11/story-11.1"
EVIDENCE_PATHS: Final = (
    "SECURITY-REVIEW.md",
    "Cargo.toml",
    "docs/architecture/data-classification-retention-table.md",
    "docs/architecture/durable-authority-store.md",
    "docs/architecture/secret-store-encrypted-backup-format.md",
    "docs/security/sprint-11-cryptographic-inventory.md",
    "docs/verification/s-011-it01-provider-substitution-results.md",
    "docs/verification/s-011-rt01-seeded-crash-recovery-results.md",
    "docs/verification/s-011-st01-secret-canary-results.md",
    "docs/verification/s-011-ut01-store-unit-results.md",
    "kernel/engine/src/authority_transaction.rs",
    "kernel/engine/src/operational_store.rs",
    "kernel/engine/src/persistence.rs",
    "platforms/linux/src/platform.rs",
    "platforms/linux/src/secret_service.rs",
    f"{ARTIFACT_ROOT}/classification-retention-table.json",
    f"{ARTIFACT_ROOT}/crash-canary-results.json",
    f"{ARTIFACT_ROOT}/derived-json-lines.json",
    f"{ARTIFACT_ROOT}/ephemeral-content.json",
    f"{ARTIFACT_ROOT}/operational-schema-v3.json",
    f"{ARTIFACT_ROOT}/persistence-policy.json",
    f"{ARTIFACT_ROOT}/s-011-it01.json",
    f"{ARTIFACT_ROOT}/s-011-rt01.json",
    f"{ARTIFACT_ROOT}/s-011-st01.json",
    f"{ARTIFACT_ROOT}/s-011-ut01.json",
    f"{ARTIFACT_ROOT}/secret-backup-format.json",
    f"{ARTIFACT_ROOT}/secret-store-key-boundary.json",
    f"{ARTIFACT_ROOT}/store-invariants.json",
    f"{ARTIFACT_ROOT}/store-lifecycle.json",
    "artifacts/sprints/sprint-3/story-3.1/component-inventory-report.json",
    "fixtures/support/manual-patch/artifacts/metadata/cryptographic-bom.json",
    "scripts/sprint_11_security_evidence.py",
    "tests/test_sprint_11_security_evidence.py",
)

POLICY = [
    "docs/architecture/data-classification-retention-table.md",
    f"{ARTIFACT_ROOT}/persistence-policy.json",
    f"{ARTIFACT_ROOT}/ephemeral-content.json",
    f"{ARTIFACT_ROOT}/s-011-st01.json",
]
STORE = [
    "docs/architecture/durable-authority-store.md",
    f"{ARTIFACT_ROOT}/operational-schema-v3.json",
    f"{ARTIFACT_ROOT}/store-invariants.json",
    f"{ARTIFACT_ROOT}/s-011-ut01.json",
]
CRYPTO = [
    "docs/security/sprint-11-cryptographic-inventory.md",
    "docs/architecture/secret-store-encrypted-backup-format.md",
    f"{ARTIFACT_ROOT}/secret-store-key-boundary.json",
    f"{ARTIFACT_ROOT}/secret-backup-format.json",
    f"{ARTIFACT_ROOT}/s-011-it01.json",
]
LIFECYCLE = [
    f"{ARTIFACT_ROOT}/store-lifecycle.json",
    f"{ARTIFACT_ROOT}/derived-json-lines.json",
    f"{ARTIFACT_ROOT}/classification-retention-table.json",
]
RECOVERY = [
    "docs/verification/s-011-rt01-seeded-crash-recovery-results.md",
    f"{ARTIFACT_ROOT}/s-011-rt01.json",
    f"{ARTIFACT_ROOT}/crash-canary-results.json",
]
AUDIT = [
    "kernel/engine/src/authority_transaction.rs",
    f"{ARTIFACT_ROOT}/crash-canary-results.json",
    f"{ARTIFACT_ROOT}/derived-json-lines.json",
]


def mapping(status: str, evidence: list[str], demonstrated: str, remaining: str) -> dict[str, Any]:
    return {
        "story_contribution": status,
        "evidence": evidence,
        "demonstrated": demonstrated,
        "remaining": remaining,
    }


MAPPINGS: Final = {
    "SR-DAT-001": mapping("partial-story-evidence", STORE + POLICY, "The current encrypted schema, policy families, retention table, and derived surfaces are inventoried and versioned.", "Typed writers and runtime inventories for later product families must extend the dictionary."),
    "SR-DAT-002": mapping("demonstrated-story-scope", POLICY, "All currently representable persistence candidates pass classification, minimization, secret detection, encryption selection, and retention before prepared bytes exist.", "Every later ingress and durable writer must bind the same gate."),
    "SR-DAT-003": mapping("demonstrated-story-scope", POLICY, "Five raw-content classes are structurally ephemeral and retain neither value nor value digest.", "Later active prompt, model, attachment, and tool adapters must rerun the canary gate."),
    "SR-DAT-004": mapping("demonstrated-story-scope", CRYPTO + POLICY, "Missing, malformed, substituted, or plaintext providers fail closed while explicit ephemeral policy remains storage-free.", "Other platform key stores require equivalent native evidence."),
    "SR-DAT-005": mapping("partial-story-evidence", CRYPTO, "The pinned bundled SQLCipher, OpenSSL boundary, runtime cipher check, key sizes, configuration, and errors are inventoried.", "Signed release runtime extraction, provider self-tests where available, and independent review remain open."),
    "SR-DAT-006": mapping("demonstrated-story-scope", CRYPTO, "Documents and receipts make bounded provider, erasure, remanence, backup, and platform claims with explicit negative claims.", "Release-generated wording must remain bound to signed manifests."),
    "SR-DAT-007": mapping("partial-story-evidence", CRYPTO, "Linux uses one exact Secret Service identity and excludes key values from arguments, environment, configuration, receipts, and artifacts.", "Live retained evidence, backup-key provisioning, rotation, macOS, and Windows remain open."),
    "SR-DAT-008": mapping("partial-story-evidence", CRYPTO + ["artifacts/sprints/sprint-3/story-3.1/component-inventory-report.json", "fixtures/support/manual-patch/artifacts/metadata/cryptographic-bom.json"], "Current Sprint 11 cryptographic components and key uses are inventoried and earlier component/CBOM schemas exist.", "A release-derived product CBOM reconciled with runtime observations remains open."),
    "SR-DAT-009": mapping("partial-story-evidence", CRYPTO, "Provider and format identities are versioned and downgrade-sensitive; schema migrations are additive and exact.", "Runtime provider agility, approved replacement, rotation, and post-quantum migration are later work."),
    "SR-DAT-010": mapping("demonstrated-story-scope", LIFECYCLE + RECOVERY, "Fake-clock expiry, user/legal holds, export, separately keyed backup, fresh restore, and deletion transitions are deterministic and restart-safe.", "Every later store/index and live-store swap must join the lifecycle engine."),
    "SR-DAT-011": mapping("partial-story-evidence", LIFECYCLE + CRYPTO, "Whole-store erasure destroys and verifies the scoped key before ciphertext removal and disclaims overwrite or backup erasure.", "Live key destruction and all separately keyed copies require later end-to-end evidence."),
    "SR-DAT-012": mapping("partial-story-evidence", LIFECYCLE, "Known store, WAL, SHM, backup, and export boundaries and sanitization limits are documented and tested.", "Complete product uninstall, model/cache/log/socket residue scanning, and externally managed remnants remain Sprint 24/25 work."),
    "SR-OPS-001": mapping("partial-story-evidence", AUDIT, "Authority receipts and retention events are structured, sequenced, identity-bound, and content-free.", "The product audit schema must add complete release, actor/session, object, policy, and correlation coverage."),
    "SR-OPS-002": mapping("partial-story-evidence", AUDIT, "Grant, transaction, denial, recovery, retention, and integrity outcomes contribute typed records.", "Complete installer, configuration, model/runtime, alert, and access event coverage remains later work."),
    "SR-OPS-003": mapping("partial-story-evidence", POLICY + AUDIT, "Current receipts, errors, debug surfaces, encrypted artifacts, exports, and absent logger/model paths pass canary checks.", "Active logging and model-context paths must rerun and extend the gate."),
    "SR-OPS-004": mapping("partial-story-evidence", AUDIT, "Receipts and retention events are hash chained and JSON Lines is an explicit non-authoritative export.", "A complete tamper-evident product audit ledger and protected external-collector export remain later work."),
    "SR-OPS-005": mapping("partial-story-evidence", AUDIT, "Receipt and lifecycle revisions preserve monotonic sequence and explicit trusted wall times.", "Monotonic clock capture, anomaly events, backward/forward clock tests, and sleep/resume evidence remain later work."),
    "SR-TST-005": mapping("demonstrated-story-scope", RECOVERY, "A 128-seed subprocess campaign covers every before/after durable boundary and the authority test proves zero recovery/replay launches.", "Power-loss, torn-sector, and later durable boundaries require their own campaigns."),
    "RV-08": mapping("demonstrated-story-scope", POLICY + [f"{ARTIFACT_ROOT}/s-011-st01.json"], "All current input classes and 12 reachable surfaces close with zero raw canary leakage.", "Future active adapters, logging, and model context invalidate this result until extended."),
    "RV-09": mapping("partial-story-evidence", CRYPTO, "Provider, build features, configuration, key channels, substitution failures, and bounded claims are recorded.", "Release runtime/CBOM reconciliation, self-tests where available, rotation, and native cross-platform evidence remain open."),
    "RV-10": mapping("partial-story-evidence", LIFECYCLE + RECOVERY, "Current retention, hold, export, backup, restore, crash, and whole-store erasure paths pass deterministic tests.", "Complete uninstall/reinstall sanitization, every future index/copy, live swap, and clean-device recovery remain open."),
}

COMMAND_SPECS: Final = (
    (("python3", "scripts/secret_canary_acceptance_evidence.py"), "S-011-ST01 evidence: pass"),
    (("python3", "scripts/seeded_crash_recovery_evidence.py"), "S-011-RT01 evidence: pass"),
    (("python3", "scripts/provider_substitution_evidence.py"), "S-011-IT01 evidence: pass"),
    (("python3", "-m", "unittest", "tests.test_sprint_11_security_evidence"), "Ran 5 tests"),
    (("npm", "run", "docs:lint"), "Summary: 0 issues in 0 files"),
)
REVISION = re.compile(r"^[0-9a-f]{40}$")
SHA256 = re.compile(r"^[0-9a-f]{64}$")


class EvidenceError(ValueError):
    pass


def safe_path(path: str) -> bool:
    value = PurePosixPath(path)
    return bool(path) and not value.is_absolute() and ".." not in value.parts and str(value) == path


def git_revision(candidate: str) -> str:
    result = subprocess.run(["git", "rev-parse", "--verify", f"{candidate}^{{commit}}"], cwd=ROOT, capture_output=True, text=True, timeout=30, check=False)
    revision = result.stdout.strip()
    if result.returncode or REVISION.fullmatch(revision) is None:
        raise EvidenceError("source revision unavailable")
    return revision


def git_bytes(revision: str, path: str) -> bytes:
    result = subprocess.run(["git", "show", f"{revision}:{path}"], cwd=ROOT, stdout=subprocess.PIPE, stderr=subprocess.DEVNULL, timeout=60, check=False)
    if result.returncode or not result.stdout:
        raise EvidenceError(f"committed evidence unavailable: {path}")
    return result.stdout


def evidence_records(revision: str) -> list[dict[str, Any]]:
    records = []
    for path in EVIDENCE_PATHS:
        data = git_bytes(revision, path)
        if path.startswith(f"{ARTIFACT_ROOT}/"):
            value = json.loads(data)
            if REVISION.fullmatch(str(value.get("source_revision", ""))) is None:
                raise EvidenceError(f"nested evidence revision invalid: {path}")
        records.append({"bytes": len(data), "path": path, "sha256": hashlib.sha256(data).hexdigest()})
    return records


def command_record(arguments: tuple[str, ...], marker: str) -> dict[str, Any]:
    return {"command_id": hashlib.sha256("\0".join(arguments).encode()).hexdigest(), "expected_marker_sha256": hashlib.sha256(marker.encode()).hexdigest(), "exit_code": 0, "status": "pass"}


def expected_commands() -> list[dict[str, Any]]:
    return [command_record(arguments, marker) for arguments, marker in COMMAND_SPECS]


def run_checked(arguments: tuple[str, ...], marker: str) -> dict[str, Any]:
    result = subprocess.run(list(arguments), cwd=ROOT, capture_output=True, text=True, timeout=300, check=False, env={**os.environ, "LANG": "C", "LC_ALL": "C"})
    if result.returncode or marker not in result.stdout + result.stderr:
        raise EvidenceError(f"verification failed: {arguments[0]}")
    return command_record(arguments, marker)


def build_report(revision: str) -> dict[str, Any]:
    return {
        "artifact_id": "sprint-11-product-security-evidence-map",
        "artifacts": evidence_records(revision),
        "controls": [{"control_id": control_id, "product_status": "not-complete", **MAPPINGS[control_id]} for control_id in EXPECTED_IDS],
        "external_network_used": False,
        "manual_fuzzing_executed": False,
        "private_user_data_used": False,
        "product_requirement_completion_claim": "none",
        "release_claim": "none",
        "schema_version": 1,
        "source_revision": revision,
        "status": "pass-story-security-mapping-with-open-product-controls",
        "summary": {"demonstrated_story_scope_count": 7, "mapped_control_count": 21, "partial_story_evidence_count": 14, "product_controls_complete": 0, "retained_artifact_count": len(EVIDENCE_PATHS)},
        "task_ids": ["11.1.3.5"],
        "verification_commands": [run_checked(arguments, marker) for arguments, marker in COMMAND_SPECS],
    }


def validate_report(report: dict[str, Any], revision_bound: bool = False) -> list[str]:
    failures = []
    controls = report.get("controls")
    if not isinstance(controls, list) or [item.get("control_id") for item in controls] != list(EXPECTED_IDS):
        failures.append("Sprint 11 security control closure changed")
    else:
        for item in controls:
            control_id = item["control_id"]
            if item != {"control_id": control_id, "product_status": "not-complete", **MAPPINGS[control_id]}:
                failures.append(f"Sprint 11 security mapping changed: {control_id}")
            if any(path not in EVIDENCE_PATHS or not safe_path(path) for path in item.get("evidence", [])):
                failures.append(f"Sprint 11 security path invalid: {control_id}")
    exact = {
        "artifact_id": "sprint-11-product-security-evidence-map",
        "external_network_used": False,
        "manual_fuzzing_executed": False,
        "private_user_data_used": False,
        "product_requirement_completion_claim": "none",
        "release_claim": "none",
        "schema_version": 1,
        "status": "pass-story-security-mapping-with-open-product-controls",
        "summary": {"demonstrated_story_scope_count": 7, "mapped_control_count": 21, "partial_story_evidence_count": 14, "product_controls_complete": 0, "retained_artifact_count": len(EVIDENCE_PATHS)},
        "task_ids": ["11.1.3.5"],
        "verification_commands": expected_commands(),
    }
    failures.extend(f"Sprint 11 security evidence {key} changed" for key, value in exact.items() if report.get(key) != value)
    if REVISION.fullmatch(str(report.get("source_revision", ""))) is None:
        failures.append("Sprint 11 security revision invalid")
    artifacts = report.get("artifacts")
    if not isinstance(artifacts, list) or [item.get("path") for item in artifacts] != list(EVIDENCE_PATHS):
        failures.append("Sprint 11 security artifact closure changed")
    elif any(not isinstance(item.get("bytes"), int) or item["bytes"] <= 0 or SHA256.fullmatch(str(item.get("sha256", ""))) is None for item in artifacts):
        failures.append("Sprint 11 security artifact records invalid")
    if revision_bound and not failures:
        for item in artifacts:
            if hashlib.sha256(git_bytes(report["source_revision"], item["path"])).hexdigest() != item["sha256"]:
                failures.append(f"Sprint 11 security artifact binding changed: {item['path']}")
    return failures


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--write", action="store_true")
    parser.add_argument("--source-revision", default="HEAD")
    arguments = parser.parse_args()
    revision = git_revision(arguments.source_revision)
    report = build_report(revision) if arguments.write else json.loads(REPORT_PATH.read_text())
    if arguments.write:
        atomic_write(REPORT_PATH, canonical_json_bytes(report))
    if failures := validate_report(report, revision_bound=True):
        raise EvidenceError("; ".join(failures))
    print("Sprint 11 security evidence mapped with zero product completion overclaim")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
