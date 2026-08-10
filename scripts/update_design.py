#!/usr/bin/env python3
"""Validate signed-manual-update and rollback design decisions."""

from __future__ import annotations

import argparse
import hashlib
import json
import os
import tempfile
from pathlib import Path
from typing import Any


ROOT = Path(__file__).resolve().parents[1]
UPDATE_PATH = ROOT / "architecture/signed-update-design.json"
ROLLBACK_PATH = ROOT / "architecture/rollback-design.json"
REPORT_PATH = ROOT / "artifacts/sprints/sprint-3/story-3.1/update-rollback-design-report.json"
UPDATE_IDENTITIES = (
    "release-id",
    "semantic-version",
    "release-sequence",
    "source-commit",
    "package-sha256",
    "configuration-sha256",
    "component-inventory-sha256",
    "sbom-sha256",
    "cryptographic-bom-sha256",
    "model-bom-sha256",
    "platform-and-architecture",
    "signer-key-ids",
)
UPDATE_STATES = (
    "idle",
    "local-bundle-selected",
    "metadata-verified",
    "package-verified",
    "compatibility-verified",
    "staged",
    "user-approved",
    "activated",
    "post-activation-verified",
    "complete",
)
ROLLBACK_STATES = (
    "idle",
    "rollback-requested",
    "preconditions-verified",
    "target-reverified",
    "recovery-plan-rendered",
    "user-approved",
    "staged",
    "activated",
    "post-rollback-verified",
    "complete",
)
UPDATE_TOP_LEVEL = {
    "schema_version",
    "record_type",
    "decision_id",
    "status",
    "delivery",
    "trusted_metadata",
    "required_identities",
    "required_change_records",
    "version_policy",
    "transaction",
    "implementation_status",
    "platform_status",
}
ROLLBACK_TOP_LEVEL = {
    "schema_version",
    "record_type",
    "decision_id",
    "status",
    "initiation",
    "eligibility",
    "transaction",
    "data_policy",
    "failure_policy",
    "implementation_status",
    "platform_status",
}
PLATFORM_STATUS = {
    "shared_design": "accepted",
    "fedora_execution": "not-run",
    "ubuntu_execution": "not-run",
    "macos_execution": "blocked-macos",
}
SOURCE_PATHS = (
    "SECURITY.md",
    "SECURITY-REVIEW.md",
    "architecture/signed-update-design.json",
    "architecture/rollback-design.json",
    "docs/decisions/0005-signed-manual-update-design.md",
    "docs/decisions/0006-update-rollback-design.md",
    "scripts/update_design.py",
    "tests/test_update_design.py",
)


def read_json(path: Path) -> Any:
    return json.loads(path.read_text(encoding="utf-8"))


def canonical_json(value: Any) -> bytes:
    return (json.dumps(value, indent=2, sort_keys=True) + "\n").encode("utf-8")


def sha256_file(path: Path) -> str:
    return hashlib.sha256(path.read_bytes()).hexdigest()


def write_atomic(path: Path, content: bytes) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    descriptor, temporary_name = tempfile.mkstemp(
        prefix=".agentmage-update-design-", dir=path.parent
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


def _closed(record: Any, keys: set[str], label: str) -> list[str]:
    if not isinstance(record, dict):
        return [f"{label} must be an object"]
    return [] if set(record) == keys else [f"{label} field closure is invalid"]


def _all_true(record: Any, keys: set[str], label: str) -> list[str]:
    failures = _closed(record, keys, label)
    if isinstance(record, dict) and any(record.get(key) is not True for key in keys):
        failures.append(f"{label} contains a weakened control")
    return failures


def validate_update(record: Any) -> list[str]:
    failures = _closed(record, UPDATE_TOP_LEVEL, "signed update design")
    if not isinstance(record, dict):
        return failures
    if (
        record.get("schema_version") != 1
        or record.get("record_type") != "signed-manual-update-design"
        or record.get("decision_id") != "ADR-0005"
        or record.get("status") != "accepted-design-not-implemented"
    ):
        failures.append("signed update design identity is invalid")
    delivery = record.get("delivery", {})
    if delivery != {
        "mode": "user-initiated-local-import",
        "acquisition_authority": "outside-agentmage-runtime",
        "automatic_update_checks": False,
        "background_downloads": False,
        "remote_triggers": False,
        "remote_control": False,
    }:
        failures.append("signed update delivery introduced hidden authority")
    trusted_metadata = record.get("trusted_metadata", {})
    trusted_boolean_fields = {
        "detached_signatures_required",
        "signature_algorithm_identifier_required",
        "signer_key_ids_required",
        "signature_threshold_required",
        "local_trust_root_required",
        "verify_before_extraction",
        "expiration_and_support_state_required",
        "revocation_state_required",
    }
    failures.extend(
        _closed(
            trusted_metadata,
            trusted_boolean_fields | {"format_version"},
            "trusted update metadata",
        )
    )
    if isinstance(trusted_metadata, dict) and any(
        trusted_metadata.get(key) is not True for key in trusted_boolean_fields
    ):
        failures.append("trusted update metadata contains a weakened control")
    if trusted_metadata.get("format_version") != 1:
        failures.append("trusted update metadata version is invalid")
    if tuple(record.get("required_identities", [])) != UPDATE_IDENTITIES:
        failures.append("signed update identity closure is invalid")
    if len(record.get("required_change_records", [])) != 10 or len(
        set(record.get("required_change_records", []))
    ) != 10:
        failures.append("signed update change-record closure is invalid")
    failures.extend(
        _all_true(
            record.get("version_policy"),
            {
                "upgrade_only",
                "release_sequence_must_increase",
                "downgrade_rejected",
                "current_release_precondition_required",
                "platform_compatibility_required",
                "unknown_or_revoked_signer_rejected",
                "unknown_component_rejected",
            },
            "signed update version policy",
        )
    )
    transaction = record.get("transaction", {})
    if set(transaction) != {
        "states",
        "atomic_activation_required",
        "previous_release_preserved_until_verified",
        "durable_receipt_each_transition",
        "interruption_selects_prior-or-complete-new_release",
        "network_allowed",
    }:
        failures.append("signed update transaction field closure is invalid")
    if tuple(transaction.get("states", [])) != UPDATE_STATES:
        failures.append("signed update transaction state closure is invalid")
    for key in (
        "atomic_activation_required",
        "previous_release_preserved_until_verified",
        "durable_receipt_each_transition",
        "interruption_selects_prior-or-complete-new_release",
    ):
        if transaction.get(key) is not True:
            failures.append(f"signed update transaction control was weakened: {key}")
    if transaction.get("network_allowed") is not False:
        failures.append("signed update transaction gained network authority")
    if record.get("implementation_status") != {
        "metadata_verifier": "not-implemented",
        "package_stager": "not-implemented",
        "activation_transaction": "not-implemented",
        "automatic_update_client": "prohibited",
    }:
        failures.append("signed update design made an implementation overclaim")
    if record.get("platform_status") != PLATFORM_STATUS:
        failures.append("signed update design made a platform overclaim")
    return failures


def validate_rollback(record: Any) -> list[str]:
    failures = _closed(record, ROLLBACK_TOP_LEVEL, "rollback design")
    if not isinstance(record, dict):
        return failures
    if (
        record.get("schema_version") != 1
        or record.get("record_type") != "signed-update-rollback-design"
        or record.get("decision_id") != "ADR-0006"
        or record.get("status") != "accepted-design-not-implemented"
    ):
        failures.append("rollback design identity is invalid")
    initiation = record.get("initiation", {})
    if initiation != {
        "mode": "local-user-approved-transaction",
        "remote_trigger": False,
        "automatic_remote_policy": False,
        "silent_rollback": False,
    }:
        failures.append("rollback initiation introduced hidden authority")
    eligibility = record.get("eligibility", {})
    expected_eligibility = {
        "exact_active_release_precondition",
        "exact_active_configuration_precondition",
        "target_was_previously_verified",
        "target_signature_and_hashes_reverified",
        "target_support_active",
        "target_not_revoked",
        "schema_compatibility_verified",
        "backup_identity_verified",
    }
    failures.extend(
        _closed(
            eligibility,
            expected_eligibility | {"downgrade_exception_allowed"},
            "rollback eligibility",
        )
    )
    if isinstance(eligibility, dict) and any(
        eligibility.get(key) is not True for key in expected_eligibility
    ):
        failures.append("rollback eligibility contains a weakened control")
    if eligibility.get("downgrade_exception_allowed") is not False:
        failures.append("rollback permits an unsupported downgrade exception")
    transaction = record.get("transaction", {})
    if set(transaction) != {
        "states",
        "atomic_activation_required",
        "durable_receipt_each_transition",
        "interruption_selects_current-or-complete-prior_release",
        "network_allowed",
    }:
        failures.append("rollback transaction field closure is invalid")
    if tuple(transaction.get("states", [])) != ROLLBACK_STATES:
        failures.append("rollback transaction state closure is invalid")
    for key in (
        "atomic_activation_required",
        "durable_receipt_each_transition",
        "interruption_selects_current-or-complete-prior_release",
    ):
        if transaction.get(key) is not True:
            failures.append(f"rollback transaction control was weakened: {key}")
    if transaction.get("network_allowed") is not False:
        failures.append("rollback transaction gained network authority")
    data_policy = record.get("data_policy", {})
    data_true_fields = {
        "binary_and_configuration_rollback_separate",
        "migration_reverse_plan_required",
        "unrecoverable_schema_change_blocks_rollback",
        "preserve_bounded_failure_evidence",
    }
    data_false_fields = {
        "later_user_data_overwrite_allowed",
        "blind_database_restore_allowed",
    }
    failures.extend(
        _closed(
            data_policy,
            data_true_fields | data_false_fields,
            "rollback data policy",
        )
    )
    if isinstance(data_policy, dict) and any(
        data_policy.get(key) is not True for key in data_true_fields
    ):
        failures.append("rollback data policy contains a weakened control")
    for prohibited in ("later_user_data_overwrite_allowed", "blind_database_restore_allowed"):
        if data_policy.get(prohibited) is not False:
            failures.append(f"rollback data policy permits unsafe behavior: {prohibited}")
    failure_policy = record.get("failure_policy", {})
    failure_true_fields = {
        "stale_precondition_blocks_before_write",
        "revoked_or_unsupported_target_blocks",
        "signature_or_hash_mismatch_blocks",
        "uncertain_result_requires_reconciliation",
        "failed_postcheck_enters_local_safe_mode",
    }
    failures.extend(
        _closed(
            failure_policy,
            failure_true_fields | {"remote_kill_switch"},
            "rollback failure policy",
        )
    )
    if isinstance(failure_policy, dict) and any(
        failure_policy.get(key) is not True for key in failure_true_fields
    ):
        failures.append("rollback failure policy contains a weakened control")
    if failure_policy.get("remote_kill_switch") is not False:
        failures.append("rollback design permits a remote kill switch")
    if record.get("implementation_status") != {
        "rollback_verifier": "not-implemented",
        "rollback_transaction": "not-implemented",
        "migration_reversal": "not-implemented",
    }:
        failures.append("rollback design made an implementation overclaim")
    if record.get("platform_status") != PLATFORM_STATUS:
        failures.append("rollback design made a platform overclaim")
    return failures


def build_report(root: Path = ROOT) -> dict[str, Any]:
    update = read_json(root / UPDATE_PATH.relative_to(ROOT))
    rollback = read_json(root / ROLLBACK_PATH.relative_to(ROOT))
    failures = [*validate_update(update), *validate_rollback(rollback)]
    if failures:
        raise ValueError("; ".join(failures))
    return {
        "schema_version": 1,
        "task_id": "3.1.1.7",
        "status": "pass-shared-signed-update-and-rollback-design",
        "source_artifacts": [
            {"path": path, "sha256": sha256_file(root / path)} for path in SOURCE_PATHS
        ],
        "decisions": ["ADR-0005", "ADR-0006"],
        "delivery_mode": update["delivery"]["mode"],
        "automatic_update_checks": False,
        "background_downloads": False,
        "remote_triggers": False,
        "update_identity_count": len(UPDATE_IDENTITIES),
        "update_states": list(UPDATE_STATES),
        "rollback_states": list(ROLLBACK_STATES),
        "rollback_requires_local_user_approval": True,
        "rollback_to_revoked_or_unsupported_target": False,
        "later_user_data_overwrite_allowed": False,
        "implementation_claim": "none",
        "platform_execution_claim": "none",
        "macos_execution_status": "blocked-macos",
        "macos_support_claim": "none",
    }


def validate_report(value: Any, root: Path = ROOT) -> list[str]:
    if not isinstance(value, dict):
        return ["update and rollback design report must be an object"]
    failures = []
    if (
        value.get("schema_version") != 1
        or value.get("task_id") != "3.1.1.7"
        or value.get("status") != "pass-shared-signed-update-and-rollback-design"
    ):
        failures.append("update and rollback design report identity is invalid")
    if (
        value.get("automatic_update_checks") is not False
        or value.get("background_downloads") is not False
        or value.get("remote_triggers") is not False
        or value.get("rollback_requires_local_user_approval") is not True
        or value.get("rollback_to_revoked_or_unsupported_target") is not False
        or value.get("later_user_data_overwrite_allowed") is not False
        or value.get("implementation_claim") != "none"
        or value.get("platform_execution_claim") != "none"
        or value.get("macos_execution_status") != "blocked-macos"
        or value.get("macos_support_claim") != "none"
    ):
        failures.append("update and rollback design report made an unsupported claim")
    try:
        expected = build_report(root)
    except (KeyError, OSError, TypeError, ValueError) as error:
        failures.append(f"cannot rebuild update and rollback design report: {error}")
    else:
        if value != expected:
            failures.append("update and rollback design report is stale or non-deterministic")
    return failures


def check_artifact(root: Path = ROOT) -> list[str]:
    try:
        report = read_json(root / REPORT_PATH.relative_to(ROOT))
    except (OSError, json.JSONDecodeError) as error:
        return [f"cannot read update and rollback design report: {error}"]
    return validate_report(report, root)


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--write", action="store_true")
    args = parser.parse_args()
    try:
        if args.write:
            write_atomic(REPORT_PATH, canonical_json(build_report()))
        failures = check_artifact()
    except (KeyError, OSError, TypeError, ValueError) as error:
        print(f"update and rollback design failed: {error}")
        return 1
    if failures:
        for failure in failures:
            print(f"update and rollback design failed: {failure}")
        return 1
    print("Story 3.1 signed-update and rollback design validated")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
