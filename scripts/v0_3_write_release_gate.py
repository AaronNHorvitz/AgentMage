#!/usr/bin/env python3
"""Validate the blocked, hash-bound AgentMage v0.3 write-pack candidate."""

from __future__ import annotations

import argparse
import hashlib
import json
from pathlib import Path
from typing import Any, Final


ROOT: Final = Path(__file__).resolve().parents[1]
MANIFEST_PATH: Final = ROOT / "release/v0.3-write-pack-manifest.json"
REPORT_PATH: Final = ROOT / "artifacts/sprints/sprint-40/write-release-readiness.json"
PRIMITIVES: Final = [
    "create_new_file",
    "exact_preimage_patch",
    "copy_without_overwrite",
    "move_without_collision",
    "trash_first_delete",
    "structured_markdown_update",
    "knowledge_record_create",
]
STAGES: Final = [
    "observe_exact_preimage",
    "build_shadow_change_set",
    "validate",
    "preview_complete_diff",
    "approve_exact_change",
    "consume_single_use_grant",
    "revalidate_fresh_preimage",
    "apply_atomically_where_supported",
    "verify_postimage",
    "publish_receipt",
    "reconcile_or_restore",
]
EXCLUSIONS: Final = [
    "connector_access",
    "generic_shell",
    "git_commit",
    "git_push",
    "network_publication",
    "scheduled_execution",
    "unattended_write",
    "wildcard_approval",
]
BLOCKERS: Final = [
    "upstream-sprints-35-through-39-blocked",
    "write-profile-not-product-registered",
    "native-cross-platform-write-acceptance-absent",
    "complete-native-crash-race-evidence-absent",
    "signed-v0.3-packages-absent",
    "independent-release-decision-absent",
    "manual-fuzzing-deferred",
]


def sha256(path: Path) -> str:
    return hashlib.sha256(path.read_bytes()).hexdigest()


def read_json(path: Path) -> Any:
    return json.loads(path.read_text(encoding="utf-8"))


def validate_manifest(manifest: Any) -> list[str]:
    failures: list[str] = []
    if not isinstance(manifest, dict):
        return ["write manifest must be an object"]
    expected_fields = {
        "schema_version", "record_type", "version", "status", "gate_id",
        "gate_closed", "product_registration", "package_artifacts_published",
        "signed_release", "capability_delta", "controlled_primitives",
        "required_transaction_stages", "excluded_capabilities", "source_files",
        "blockers", "network_access", "release_claim",
    }
    if set(manifest) != expected_fields:
        failures.append("write manifest field closure drifted")
    expected_identity = {
        "schema_version": 1,
        "record_type": "agentmage-v0.3-write-pack-manifest",
        "version": "0.3.0",
        "status": "blocked-local-candidate",
        "gate_id": "G-V0.3",
        "gate_closed": False,
        "product_registration": False,
        "package_artifacts_published": False,
        "signed_release": False,
        "network_access": False,
        "release_claim": "none",
    }
    for field, expected in expected_identity.items():
        if manifest.get(field) != expected:
            failures.append(f"write manifest identity drifted: {field}")
    if manifest.get("controlled_primitives") != PRIMITIVES:
        failures.append("controlled primitive closure drifted")
    if manifest.get("required_transaction_stages") != STAGES:
        failures.append("write transaction stage closure drifted")
    if manifest.get("excluded_capabilities") != EXCLUSIONS:
        failures.append("excluded capability closure drifted")
    if manifest.get("blockers") != BLOCKERS:
        failures.append("write release blocker closure drifted")
    if manifest.get("capability_delta") != {
        "baseline": ["workspace.read"],
        "planned_additions": ["workspace.write.controlled"],
        "effective_additions": [],
        "effective_authority_broadening": False,
    }:
        failures.append("write capability delta drifted")
    sources = manifest.get("source_files", [])
    paths = [item.get("path") for item in sources if isinstance(item, dict)]
    if len(sources) != 10 or paths != sorted(set(paths)):
        failures.append("write source inventory is not closed and sorted")
    else:
        for item in sources:
            path = ROOT / item["path"]
            if (
                not path.is_file()
                or path.is_symlink()
                or item.get("sha256") != sha256(path)
            ):
                failures.append(f"write source hash drifted: {item['path']}")
    return failures


def validate_profile() -> list[str]:
    failures: list[str] = []
    catalog = read_json(ROOT / "configuration/profiles/catalog.json")
    matches = [item for item in catalog.get("profiles", []) if item.get("profile_id") == "write"]
    if len(matches) != 1:
        return ["write profile catalog identity drifted"]
    profile = matches[0]
    if (
        profile.get("activation_status") != "future-disabled"
        or profile.get("product_registration") is not False
        or profile.get("network_effective") is not False
        or profile.get("effective_capabilities") != ["workspace.read"]
        or profile.get("planned_capabilities") != ["workspace.read", "workspace.write"]
        or profile.get("blocked_by")
        != ["gate-v0.2", "gate-v0.3", "product-configuration-loader"]
    ):
        failures.append("write profile activation boundary drifted")
    configuration = read_json(ROOT / "configuration/profiles/write.json")
    if (
        configuration.get("permission", {}).get("allowed_capabilities")
        != ["workspace.read"]
        or configuration.get("permission", {}).get("network", {}).get("mode")
        != "deny-all"
        or configuration.get("shell", {}).get("direct_command_execution") is not False
        or configuration.get("shell", {}).get("network_access") is not False
        or configuration.get("budget", {}).get("maximum_processes") != 0
        or configuration.get("tool", {}).get("tools") != []
        or configuration.get("workspace", {}).get("roots", [{}])[0].get("access")
        != "read-only"
    ):
        failures.append("write profile safe-disabled configuration drifted")
    return failures


def build_report() -> dict[str, Any]:
    manifest = read_json(MANIFEST_PATH)
    failures = validate_manifest(manifest) + validate_profile()
    return {
        "schema_version": 1,
        "record_type": "agentmage-v0.3-write-release-readiness",
        "manifest_sha256": sha256(MANIFEST_PATH),
        "local_manifest_valid": not failures,
        "write_profile_active": False,
        "unsigned_linux_candidate_exercised": False,
        "fedora_write_acceptance": False,
        "ubuntu_write_acceptance": False,
        "macos_write_acceptance": False,
        "windows_write_acceptance": False,
        "upgrade_downgrade_restore_uninstall_complete": False,
        "independent_review_complete": False,
        "manual_fuzzing_complete": False,
        "package_signing_allowed": False,
        "gate_closed": False,
        "release_allowed": False,
        "failures": failures,
        "blockers": BLOCKERS,
    }


def validate_report(report: Any) -> list[str]:
    failures: list[str] = []
    expected = build_report()
    if report != expected:
        failures.append("write release readiness report drifted")
    if expected["failures"]:
        failures.extend(expected["failures"])
    for field in (
        "write_profile_active",
        "unsigned_linux_candidate_exercised",
        "fedora_write_acceptance",
        "ubuntu_write_acceptance",
        "macos_write_acceptance",
        "windows_write_acceptance",
        "upgrade_downgrade_restore_uninstall_complete",
        "independent_review_complete",
        "manual_fuzzing_complete",
        "package_signing_allowed",
        "gate_closed",
        "release_allowed",
    ):
        if report.get(field) is not False:
            failures.append(f"write release overclaim: {field}")
    return failures


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--write", action="store_true")
    arguments = parser.parse_args()
    report = build_report() if arguments.write else read_json(REPORT_PATH)
    failures = validate_report(report)
    if failures:
        print("v0.3 write release gate failed:")
        for failure in failures:
            print(f"- {failure}")
        return 1
    if arguments.write:
        REPORT_PATH.parent.mkdir(parents=True, exist_ok=True)
        REPORT_PATH.write_text(
            json.dumps(report, indent=2, sort_keys=True) + "\n", encoding="utf-8"
        )
    print("v0.3 write release gate remains blocked as declared")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
