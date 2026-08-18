#!/usr/bin/env python3
"""Build and validate the locally executable Sprint 14 evidence closure."""

from __future__ import annotations

import argparse
import hashlib
import json
import re
import shutil
import subprocess
import sys
from pathlib import Path
from typing import Any, Final

ROOT: Final = Path(__file__).resolve().parents[1]
if str(ROOT) not in sys.path:
    sys.path.insert(0, str(ROOT))

from scripts import model_candidate_inventory as candidate_inventory  # noqa: E402


OUTPUT: Final = ROOT / "artifacts/sprints/sprint-14/local-evidence-report.json"
REVISION: Final = re.compile(r"^[0-9a-f]{40}$")
SHA256: Final = re.compile(r"^[0-9a-f]{64}$")
SOURCE_PATHS: Final = (
    "kernel/contracts/src/model_discovery.rs",
    "kernel/engine/src/model_discovery.rs",
    "platforms/linux-inference/src/model_acquisition.rs",
    "platforms/linux-inference/src/model_download.rs",
    "platforms/linux-inference/src/model_install_lifecycle.rs",
    "platforms/linux-inference/src/model_install_verifier.rs",
    "platforms/linux-inference/src/model_installer_main.rs",
    "platforms/linux-inference/src/model_installer_process.rs",
    "platforms/linux-inference/src/lib.rs",
    "platforms/linux-inference/tests/process_boundary.rs",
    "shells/vscode/src/model_discovery.ts",
    "shells/vscode/test/model_discovery.test.ts",
    "scripts/package_candidate.py",
    "scripts/package_lifecycle.py",
    "scripts/model_candidate_inventory.py",
    "tests/test_model_candidate_inventory.py",
    "scripts/sprint_14_evidence.py",
    "tests/test_sprint_14_evidence.py",
)
EVIDENCE_PATHS: Final = (
    "model-profiles/catalogs/2026-08-14/first-party-source-snapshot.json",
    "model-profiles/catalogs/2026-08-14/candidate-inventory.json",
    "model-profiles/catalogs/2026-08-14/candidate-role-suite-matrix.json",
    "artifacts/sprints/sprint-9/story-9.2/linux-docker-production-prerequisites.json",
    "artifacts/sprints/sprint-10/story-10.1/firewall-packet-capture.json",
)
COMMANDS: Final = (
    (
        "acquisition-review-contracts",
        (
            "cargo",
            "test",
            "-p",
            "agentmage-kernel-engine",
            "model_discovery",
            "--locked",
        ),
    ),
    (
        "acquisition-review-vscode",
        ("npm", "run", "test", "--workspace", "@agentmage/vscode-shell"),
    ),
    (
        "installer-runtime-contracts",
        ("cargo", "test", "-p", "agentmage-platform-linux-inference", "--locked"),
    ),
    (
        "installer-package-contracts",
        (
            "python",
            "-m",
            "unittest",
            "tests.test_package_candidate",
            "tests.test_package_lifecycle",
            "tests.test_linux_package_lifecycle_evidence",
            "tests.test_linux_docker_prerequisite_evidence",
        ),
    ),
    (
        "candidate-inventory-contracts",
        ("python", "-m", "unittest", "tests.test_model_candidate_inventory"),
    ),
    (
        "candidate-inventory-validation",
        ("python", "scripts/model_candidate_inventory.py"),
    ),
)
SECURITY_MAP: Final = {
    "14.1": (
        "SR-PLT-008",
        "SR-NET-005",
        "SR-NET-006",
        "SR-NET-007",
        "SR-SUP-003",
        "SR-SUP-006",
        "SR-SUP-007",
        "SR-SUP-008",
        "SR-TST-007",
        "RV-07",
    ),
    "14.2": (
        "SR-MGM-001",
        "SR-MGM-004",
        "SR-MGM-005",
        "SR-AI-015",
        "RV-41",
    ),
}
BLOCKERS: Final = (
    {
        "code": "INSTALLER-EFFECT-PROTOCOL-INACTIVE",
        "owner": "14.1",
        "reason": (
            "The packaged one-shot process supports self-check and a strict, non-acquiring "
            "preflight over standard input; local import, bounded download, activation, "
            "rollback, and cleanup are not yet wired to a closed end-user process protocol."
        ),
    },
    {
        "code": "MODEL-REVIEW-UI-NOT-IMPLEMENTED",
        "owner": "14.1",
        "reason": "Exact review data exists, but no preflight and license-review screen exists.",
    },
    {
        "code": "PRODUCTION-SIGNING-NOT-AVAILABLE",
        "owner": "14.1",
        "reason": "Deterministic signable packages exist; no approved external production signing identity exists.",
    },
    {
        "code": "LIVE-ACQUISITION-CAPTURE-NOT-PERFORMED",
        "owner": "14.1",
        "reason": "The isolated capture harness is proven, but it has not observed a real model acquisition lifecycle.",
    },
    {
        "code": "REFERENCE-PLATFORM-EVIDENCE-INCOMPLETE",
        "owner": "14.1",
        "reason": "Native macOS installer/importer and clean package lifecycle evidence remain unavailable.",
    },
    {
        "code": "EXACT-ARTIFACT-PROFILES-NOT-ADMITTED",
        "owner": "14.2",
        "reason": (
            "The complete repository-revision inventory is frozen, but every candidate "
            "remains blocked until exact artifact bytes, digests, runtime, context, and "
            "working-set facts are separately admitted."
        ),
    },
)


def sha256_bytes(value: bytes) -> str:
    return hashlib.sha256(value).hexdigest()


def sha256_file(path: Path) -> str:
    return sha256_bytes(path.read_bytes())


def git_file(revision: str, relative: str) -> bytes:
    result = subprocess.run(
        ["git", "show", f"{revision}:{relative}"],
        cwd=ROOT,
        check=False,
        capture_output=True,
        timeout=30,
    )
    if result.returncode != 0:
        raise ValueError(f"committed Sprint 14 source is absent: {relative}")
    return result.stdout


def run_commands() -> list[dict[str, Any]]:
    records = []
    for identifier, argv in COMMANDS:
        executable = shutil.which(argv[0])
        if executable is None:
            exit_code = 127
            output = b"executable-unavailable"
        else:
            result = subprocess.run(
                (executable, *argv[1:]),
                cwd=ROOT,
                check=False,
                capture_output=True,
                timeout=900,
            )
            exit_code = result.returncode
            output = result.stdout + result.stderr
        records.append(
            {
                "id": identifier,
                "argv": list(argv),
                "exit_code": exit_code,
                "output_sha256": sha256_bytes(output),
            }
        )
    return records


def evidence_state() -> dict[str, Any]:
    snapshot = candidate_inventory.read_json(candidate_inventory.SOURCE_SNAPSHOT)
    inventory = candidate_inventory.read_json(candidate_inventory.INVENTORY)
    matrix = candidate_inventory.read_json(candidate_inventory.ROLE_MATRIX)
    inventory_failures = candidate_inventory.validate(snapshot, inventory, matrix)
    package = json.loads((ROOT / EVIDENCE_PATHS[3]).read_text(encoding="utf-8"))
    capture = json.loads((ROOT / EVIDENCE_PATHS[4]).read_text(encoding="utf-8"))
    installer_records = [
        record
        for component in package["components"]
        for record in component["payload"]
        if record["path"].endswith("agentmage-model-installer")
    ]
    return {
        "frozen_source_entries": snapshot["counts"]["total"],
        "google_source_entries": snapshot["counts"]["google"],
        "meta_source_entries": snapshot["counts"]["meta"],
        "normalized_entries": inventory["counts"]["total"],
        "inventory_validation_failures": inventory_failures,
        "inventory_dispositions": inventory["counts"]["by_disposition"],
        "role_matrix_entries": len(matrix["entries"]),
        "enabled_models": inventory["product_state"]["enabled_models"],
        "authorized_acquisitions": inventory["product_state"]["acquisitions_authorized"],
        "packaged_installer_records": len(installer_records),
        "packaged_installer_modes": sorted({record["mode"] for record in installer_records}),
        "isolated_capture_harness_passed": capture["status"]
        == "pass-isolated-linux-firewall-and-capture-harness",
        "capture_observed_product_acquisition": capture["claims"]["product_workflow_observed"],
    }


def build_report(source_revision: str, commands: list[dict[str, Any]]) -> dict[str, Any]:
    state = evidence_state()
    local_contract_pass = all(item["exit_code"] == 0 for item in commands)
    local_contract_pass = local_contract_pass and state == {
        "frozen_source_entries": 416,
        "google_source_entries": 412,
        "meta_source_entries": 4,
        "normalized_entries": 416,
        "inventory_validation_failures": [],
        "inventory_dispositions": {
            "BLOCKED": 415,
            "BLOCKED-HARDWARE": 0,
            "CANDIDATE": 0,
            "INELIGIBLE": 1,
            "REJECTED": 0,
        },
        "role_matrix_entries": 416,
        "enabled_models": 0,
        "authorized_acquisitions": 0,
        "packaged_installer_records": 2,
        "packaged_installer_modes": [493],
        "isolated_capture_harness_passed": True,
        "capture_observed_product_acquisition": False,
    }
    return {
        "schema_version": 1,
        "record_type": "sprint_14_local_evidence",
        "source_revision": source_revision,
        "source_sha256": {
            path: sha256_bytes(git_file(source_revision, path)) for path in SOURCE_PATHS
        },
        "evidence_sha256": {path: sha256_file(ROOT / path) for path in EVIDENCE_PATHS},
        "commands": commands,
        "security_requirement_ids": {
            key: list(value) for key, value in SECURITY_MAP.items()
        },
        "evidence_state": state,
        "stories": [
            {
                "story_id": "14.1",
                "status": "PASS-LOCAL-PREFLIGHT-BLOCKED-EFFECTS-AND-PLATFORMS"
                if local_contract_pass
                else "BLOCKED",
                "preflight_contract_implemented": local_contract_pass,
                "local_import_contract_implemented": local_contract_pass,
                "bounded_download_contract_implemented": local_contract_pass,
                "atomic_lifecycle_contract_implemented": local_contract_pass,
                "installer_packaged": local_contract_pass,
                "acquisition_review_facts_displayed": local_contract_pass,
                "preflight_process_protocol_active": local_contract_pass,
                "end_user_effect_process_protocol_active": False,
                "review_ui_implemented": False,
                "production_package_signed": False,
                "live_acquisition_capture": False,
                "macos_evidence": False,
            },
            {
                "story_id": "14.2",
                "status": "PASS-SOURCE-INVENTORY-BLOCKED-EXACT-ARTIFACT-PREFLIGHT"
                if local_contract_pass
                else "BLOCKED",
                "source_population_reconciled": local_contract_pass,
                "role_matrix_reconciled": local_contract_pass,
                "intake_mutations_passed": local_contract_pass,
                "exact_artifact_profiles_admitted": False,
                "reference_machine_preflight_complete": False,
            },
        ],
        "blockers": list(BLOCKERS),
        "summary": {
            "local_contract_passed": local_contract_pass,
            "sprint_status": "BLOCKED",
            "product_activation": False,
            "automatic_fallback": False,
            "release_approval": False,
        },
    }


def validate_report(report: dict[str, Any], *, verify_current: bool = True) -> list[str]:
    failures = []
    if report.get("schema_version") != 1 or report.get("record_type") != "sprint_14_local_evidence":
        failures.append("Sprint 14 report identity changed")
    if REVISION.fullmatch(str(report.get("source_revision", ""))) is None:
        failures.append("Sprint 14 source revision is invalid")
    commands = report.get("commands", [])
    if [item.get("id") for item in commands] != [item[0] for item in COMMANDS]:
        failures.append("Sprint 14 command closure changed")
    elif any(item.get("exit_code") != 0 or SHA256.fullmatch(str(item.get("output_sha256", ""))) is None for item in commands):
        failures.append("Sprint 14 verification command failed")
    if report.get("security_requirement_ids") != {key: list(value) for key, value in SECURITY_MAP.items()}:
        failures.append("Sprint 14 security map changed")
    if report.get("blockers") != list(BLOCKERS):
        failures.append("Sprint 14 blocker ledger changed")
    stories = report.get("stories", [])
    if (
        len(stories) != 2
        or stories[0].get("preflight_process_protocol_active") is not True
        or stories[0].get("end_user_effect_process_protocol_active") is not False
    ):
        failures.append("Sprint 14 installer process state changed or was overstated")
    if len(stories) != 2 or stories[1].get("exact_artifact_profiles_admitted") is not False:
        failures.append("Sprint 14 exact artifact state was overstated")
    summary = report.get("summary", {})
    if summary != {
        "local_contract_passed": True,
        "sprint_status": "BLOCKED",
        "product_activation": False,
        "automatic_fallback": False,
        "release_approval": False,
    }:
        failures.append("Sprint 14 summary changed or overclaimed")
    if verify_current and not failures:
        revision = report["source_revision"]
        expected_sources = {
            path: sha256_bytes(git_file(revision, path)) for path in SOURCE_PATHS
        }
        expected_evidence = {path: sha256_file(ROOT / path) for path in EVIDENCE_PATHS}
        if report.get("source_sha256") != expected_sources:
            failures.append("Sprint 14 committed source binding is stale")
        if report.get("evidence_sha256") != expected_evidence:
            failures.append("Sprint 14 evidence binding is stale")
        if report.get("evidence_state") != evidence_state():
            failures.append("Sprint 14 evidence state is stale")
    return failures


def write_report(path: Path, value: object) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_text(json.dumps(value, indent=2, sort_keys=True) + "\n", encoding="utf-8")


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--write", action="store_true")
    parser.add_argument("--source-revision", default="HEAD")
    args = parser.parse_args()
    if args.write:
        revision = subprocess.run(
            ["git", "rev-parse", f"{args.source_revision}^{{commit}}"],
            cwd=ROOT,
            check=True,
            capture_output=True,
            text=True,
        ).stdout.strip()
        report = build_report(revision, run_commands())
        write_report(OUTPUT, report)
    report = json.loads(OUTPUT.read_text(encoding="utf-8"))
    failures = validate_report(report)
    if failures:
        print("Sprint 14 local evidence: invalid")
        for failure in failures:
            print(f"- {failure}")
        return 1
    print("Sprint 14 local evidence: PASS locally; sprint remains BLOCKED")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
