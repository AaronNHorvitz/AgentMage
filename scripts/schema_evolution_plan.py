#!/usr/bin/env python3
"""Validate the closed Task 1.2.4.2 schema evolution and rollback plan."""

from __future__ import annotations

import hashlib
import json
import sys
from pathlib import Path
from typing import Any, Final


ROOT: Final = Path(__file__).resolve().parents[1]
PLAN_PATH: Final = ROOT / "architecture" / "schema-evolution-and-rollback.json"
ROOT_KEYS: Final = {
    "schema_version",
    "record_type",
    "decision_id",
    "task_id",
    "status",
    "published_on",
    "authorities",
    "version_axes",
    "additive_records",
    "event_compatibility",
    "cache_invalidation",
    "unsupported_clients",
    "store_migration",
    "downgrade_behavior",
    "required_test_cases",
    "implementation_truth",
}
VERSION_AXES: Final = [
    "protocol",
    "record-schema",
    "event",
    "store",
    "cache",
    "workflow-definition",
    "capability-manifest",
    "checkpoint",
]
AUTHORITY_PATHS: Final = [
    "ENGINEERING-RUNTIME.md",
    "docs/decisions/0042-universal-artifact-ingestion-and-verified-workflow-execution.md",
    "architecture/rollback-design.json",
]
ADDITIVE_KEYS: Final = {
    "closed_records",
    "unknown_fields_rejected",
    "unknown_versions_rejected",
    "new_optional_field_in_place_allowed",
    "new_required_field_in_place_allowed",
    "semantic_reinterpretation_in_place_allowed",
    "version_increment_required",
    "new_record_identity_required",
    "old_decoder_fixture_retained",
    "migration_is_pure",
    "migration_preserves_source_identity",
    "migration_preserves_unknown_source_bytes",
    "migration_effect_authority",
    "write_policy",
    "read_policy",
    "removal_policy",
}
EVENT_KEYS: Final = {
    "journal_model",
    "event_identity_fields",
    "ordering",
    "duplicate_policy",
    "unknown_event_type",
    "unknown_event_version",
    "unknown_event_field",
    "gap_or_reorder",
    "migration",
    "effect_replay_allowed",
    "terminal_reinterpretation_allowed",
    "checkpoint_rule",
    "writer_rule",
}
CACHE_KEYS: Final = {
    "cache_is_authoritative",
    "key_inputs",
    "invalidation_triggers",
    "stale_read_policy",
    "rebuild_policy",
    "rebuild_failure",
    "publication",
    "old_generation_retention",
    "garbage_collection",
    "cleanup_failure",
}
CLIENT_KEYS: Final = {
    "negotiation",
    "required_exchange",
    "no_overlap_result",
    "result_fields",
    "before_effect_required",
    "before_persistent_session_write_required",
    "field_dropping_allowed",
    "silent_downgrade_allowed",
    "legacy_fallback_allowed",
    "read_only_fallback_allowed",
    "export_path",
    "diagnostics",
}
MIGRATION_KEYS: Final = {
    "owner",
    "second_store_allowed",
    "preconditions",
    "states",
    "strategy",
    "interruption_rule",
    "forward_only_default",
    "reverse_migration_default",
    "source_backup_immutable",
    "later_user_data_overwrite_allowed",
    "uncertain_activation",
    "failure_evidence",
}
DOWNGRADE_KEYS: Final = {
    "binary_and_data_are_separate",
    "target_reverified",
    "target_support_required",
    "target_not_revoked_required",
    "current_store_readability_required",
    "unreadable_store_result",
    "automatic_reverse_migration_allowed",
    "blind_backup_restore_allowed",
    "later_user_data_overwrite_allowed",
    "explicit_reverse_path_requirements",
    "unsupported_target_action",
    "remote_trigger_allowed",
    "silent_rollback_allowed",
}
TRUTH: Final = {
    "plan_published": True,
    "migration_runtime_implemented": False,
    "reverse_migration_implemented": False,
    "compatibility_campaign_executed": False,
    "native_platform_evidence": False,
    "release_readiness": False,
}
REQUIRED_TESTS: Final = [
    "additive-new-record-round-trip",
    "old-record-pure-migration",
    "unknown-field-rejection",
    "unknown-record-version-refusal",
    "unknown-event-type-stop-before-event",
    "unknown-event-version-stop-before-event",
    "event-gap-reorder-and-conflict-refusal",
    "event-replay-starts-no-effect",
    "cache-key-each-input-invalidates",
    "stale-cache-never-served-on-rebuild-failure",
    "unsupported-client-refused-before-write-or-effect",
    "migration-interruption-selects-old-or-new",
    "uncertain-activation-reconciled-before-retry",
    "downgrade-unreadable-store-blocked",
    "reverse-migration-requires-losslessness-and-approval",
    "later-user-data-never-overwritten",
]


def load_plan(path: Path = PLAN_PATH) -> dict[str, Any]:
    return json.loads(path.read_text(encoding="utf-8"))


def _closed(value: Any, keys: set[str], label: str, failures: list[str]) -> bool:
    if not isinstance(value, dict):
        failures.append(f"{label} must be an object")
        return False
    if set(value) != keys:
        failures.append(f"{label} field closure changed")
        return False
    return True


def _exact_true(record: dict[str, Any], keys: set[str], label: str, failures: list[str]) -> None:
    for key in keys:
        if record.get(key) is not True:
            failures.append(f"{label}.{key} must remain true")


def _exact_false(record: dict[str, Any], keys: set[str], label: str, failures: list[str]) -> None:
    for key in keys:
        if record.get(key) is not False:
            failures.append(f"{label}.{key} must remain false")


def _required_list(
    value: Any, expected: list[str], label: str, failures: list[str]
) -> None:
    if value != expected:
        failures.append(f"{label} must retain its complete ordered inventory")


def _validate_authorities(value: Any, root: Path, failures: list[str]) -> None:
    if not isinstance(value, list) or len(value) != len(AUTHORITY_PATHS):
        failures.append("authorities must retain the exact authoritative file set")
        return
    observed: list[str] = []
    for index, authority in enumerate(value):
        if not _closed(authority, {"path", "sha256"}, f"authority[{index}]", failures):
            continue
        path = authority.get("path")
        observed.append(path)
        if not isinstance(path, str) or path.startswith("/") or ".." in Path(path).parts:
            failures.append(f"authority[{index}] path is not repository-relative")
            continue
        try:
            digest = hashlib.sha256((root / path).read_bytes()).hexdigest()
        except OSError as error:
            failures.append(f"authority[{index}] cannot be read: {error}")
            continue
        if authority.get("sha256") != digest:
            failures.append(f"authority[{index}] digest differs from current authority")
    if observed != AUTHORITY_PATHS:
        failures.append("authorities must retain the exact authoritative file set")


def validate_plan(plan: Any, root: Path = ROOT) -> list[str]:
    failures: list[str] = []
    if not _closed(plan, ROOT_KEYS, "plan", failures):
        return failures
    header = {
        "schema_version": 1,
        "record_type": "engineering-runtime-schema-evolution-plan",
        "decision_id": "ADR-0042",
        "task_id": "1.2.4.2",
        "status": "accepted-plan-not-implemented",
        "published_on": "2026-08-29",
    }
    for key, expected in header.items():
        if plan.get(key) != expected:
            failures.append(f"{key} must equal {expected!r}")
    _validate_authorities(plan.get("authorities"), root, failures)
    _required_list(plan.get("version_axes"), VERSION_AXES, "version_axes", failures)
    _required_list(plan.get("required_test_cases"), REQUIRED_TESTS, "required_test_cases", failures)
    if plan.get("implementation_truth") != TRUTH:
        failures.append("implementation truth was widened")

    additive = plan.get("additive_records")
    if _closed(additive, ADDITIVE_KEYS, "additive_records", failures):
        _exact_true(
            additive,
            {
                "closed_records",
                "unknown_fields_rejected",
                "unknown_versions_rejected",
                "version_increment_required",
                "new_record_identity_required",
                "old_decoder_fixture_retained",
                "migration_is_pure",
                "migration_preserves_source_identity",
                "migration_preserves_unknown_source_bytes",
            },
            "additive_records",
            failures,
        )
        _exact_false(
            additive,
            {
                "new_optional_field_in_place_allowed",
                "new_required_field_in_place_allowed",
                "semantic_reinterpretation_in_place_allowed",
            },
            "additive_records",
            failures,
        )
        if additive.get("migration_effect_authority") != "none":
            failures.append("record migration gained effect authority")
        if "unsupported" not in str(additive.get("read_policy")):
            failures.append("record reads lost explicit unsupported behavior")

    events = plan.get("event_compatibility")
    if _closed(events, EVENT_KEYS, "event_compatibility", failures):
        _required_list(
            events.get("event_identity_fields"),
            [
                "event_id",
                "event_version",
                "event_type",
                "aggregate_id",
                "sequence",
                "payload_sha256",
                "prior_event_sha256",
            ],
            "event_identity_fields",
            failures,
        )
        _exact_false(
            events,
            {"effect_replay_allowed", "terminal_reinterpretation_allowed"},
            "event_compatibility",
            failures,
        )
        for key in ("unknown_event_type", "unknown_event_version"):
            if events.get(key) != "stop-before-event-and-return-unsupported":
                failures.append(f"event_compatibility.{key} must stop before the event")
        if events.get("gap_or_reorder") != "corruption-no-replay":
            failures.append("event gaps or reordering no longer fail closed")

    cache = plan.get("cache_invalidation")
    if _closed(cache, CACHE_KEYS, "cache_invalidation", failures):
        _exact_false(cache, {"cache_is_authoritative"}, "cache_invalidation", failures)
        required_key_inputs = {
            "source-content-sha256",
            "extractor-identity",
            "extractor-version",
            "schema-version",
            "policy-digest",
            "tokenizer-identity",
            "index-version",
        }
        if not isinstance(cache.get("key_inputs"), list) or not required_key_inputs <= set(cache["key_inputs"]):
            failures.append("cache key inputs are incomplete")
        required_triggers = {
            "source-content-change",
            "extractor-identity-or-version-change",
            "canonicalization-or-schema-change",
            "policy-or-redaction-change",
            "integrity-or-decryption-failure",
            "component-revocation",
        }
        if not isinstance(cache.get("invalidation_triggers"), list) or not required_triggers <= set(cache["invalidation_triggers"]):
            failures.append("cache invalidation triggers are incomplete")
        if cache.get("stale_read_policy") != "miss-never-serve":
            failures.append("stale cache content could be served")
        if "no-stale-fallback" not in str(cache.get("rebuild_failure")):
            failures.append("cache rebuild failure gained stale fallback")

    clients = plan.get("unsupported_clients")
    if _closed(clients, CLIENT_KEYS, "unsupported_clients", failures):
        _exact_true(
            clients,
            {"before_effect_required", "before_persistent_session_write_required"},
            "unsupported_clients",
            failures,
        )
        _exact_false(
            clients,
            {
                "field_dropping_allowed",
                "silent_downgrade_allowed",
                "legacy_fallback_allowed",
                "read_only_fallback_allowed",
            },
            "unsupported_clients",
            failures,
        )
        if clients.get("no_overlap_result") != "unsupported-client":
            failures.append("unsupported client result was weakened")

    migration = plan.get("store_migration")
    if _closed(migration, MIGRATION_KEYS, "store_migration", failures):
        _exact_true(
            migration,
            {"forward_only_default", "source_backup_immutable"},
            "store_migration",
            failures,
        )
        _exact_false(
            migration,
            {"second_store_allowed", "later_user_data_overwrite_allowed"},
            "store_migration",
            failures,
        )
        if migration.get("owner") != "kernel-engine-operational-store":
            failures.append("store migration owner changed")
        if migration.get("reverse_migration_default") != "unsupported":
            failures.append("reverse migration became implicit")
        states = migration.get("states")
        for required in ("backup-verified", "candidate-verified", "activation-committed", "recovery-required"):
            if not isinstance(states, list) or required not in states:
                failures.append(f"store migration state missing: {required}")

    downgrade = plan.get("downgrade_behavior")
    if _closed(downgrade, DOWNGRADE_KEYS, "downgrade_behavior", failures):
        _exact_true(
            downgrade,
            {
                "binary_and_data_are_separate",
                "target_reverified",
                "target_support_required",
                "target_not_revoked_required",
                "current_store_readability_required",
            },
            "downgrade_behavior",
            failures,
        )
        _exact_false(
            downgrade,
            {
                "automatic_reverse_migration_allowed",
                "blind_backup_restore_allowed",
                "later_user_data_overwrite_allowed",
                "remote_trigger_allowed",
                "silent_rollback_allowed",
            },
            "downgrade_behavior",
            failures,
        )
        if downgrade.get("unreadable_store_result") != "block-target-launch-preserve-current-store":
            failures.append("unreadable downgrade target no longer preserves current store")
        reverse = downgrade.get("explicit_reverse_path_requirements")
        required_reverse = {
            "losslessness-proof",
            "exact-preimage-backup",
            "fresh-local-user-approval",
            "post-rollback-verification",
        }
        if not isinstance(reverse, list) or not required_reverse <= set(reverse):
            failures.append("explicit reverse path requirements are incomplete")
    return failures


def main() -> int:
    try:
        plan = load_plan()
    except (OSError, json.JSONDecodeError) as error:
        print(f"schema evolution plan validation failed: {error}", file=sys.stderr)
        return 1
    failures = validate_plan(plan)
    if failures:
        for failure in failures:
            print(f"schema evolution plan validation failed: {failure}", file=sys.stderr)
        return 1
    print("schema evolution and rollback plan validation passed")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
