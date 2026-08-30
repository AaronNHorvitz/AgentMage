#!/usr/bin/env python3
"""Validate the closed Task 1.2.4.3 parser/OCR placement decision."""

from __future__ import annotations

import hashlib
import json
import sys
from pathlib import Path
from typing import Any, Final


ROOT: Final = Path(__file__).resolve().parents[1]
RECORD_PATH: Final = ROOT / "architecture" / "parser-ocr-platform-placement.json"
PLATFORM_IDS: Final = [
    "fedora-x86_64",
    "ubuntu-x86_64",
    "windows-11-x86_64",
    "macos-apple-silicon",
    "wsl",
    "remote-ssh",
    "dev-containers",
]
AUTHORITY_PATHS: Final = [
    "docs/decisions/0042-universal-artifact-ingestion-and-verified-workflow-execution.md",
    "RUNTIME-BOUNDARIES.md",
    "architecture/runtime-ownership.json",
    "architecture/dependency-dispositions.json",
]
ROOT_KEYS: Final = {
    "schema_version",
    "record_type",
    "decision_id",
    "task_id",
    "status",
    "reviewed_on",
    "authorities",
    "global_placement",
    "package_features",
    "byte_crossing",
    "cancellation",
    "fallback",
    "platforms",
    "required_evidence",
    "product_truth",
}
GLOBAL_KEYS: Final = {
    "typescript_role",
    "rust_host_role",
    "parser_owner",
    "parser_execution",
    "ocr_execution",
    "process_launcher_owner",
    "policy_owner",
    "store_owner",
    "typescript_parser_allowed",
    "in_process_complex_parser_allowed",
    "ocr_in_host_allowed",
    "worker_network_allowed",
    "ambient_workspace_allowed",
    "credential_access_allowed",
    "second_store_allowed",
}
FEATURE_KEYS: Final = {
    "default_enabled",
    "planned",
    "admission_rule",
    "disabled_rule",
    "ocr_status",
    "native_helper_rule",
    "dynamic_plugin_discovery_allowed",
    "environment_feature_override_allowed",
}
BYTE_KEYS: Final = {
    "extension_to_host",
    "host_to_worker",
    "worker_to_host",
    "source_in_arguments_or_environment_allowed",
    "plaintext_shared_temp_allowed",
    "path_as_authority_allowed",
    "worker_output_trusted_before_validation",
    "raw_source_return_default",
    "crossing_receipt_fields",
}
CANCEL_KEYS: Final = {
    "token_chain",
    "admission_checkpoints",
    "cooperative_grace_then_forced_termination",
    "stdout_and_stderr_drained_concurrently",
    "terminal_before_worker_reaped_allowed",
    "partial_derivative_publication_allowed",
    "residue_recorded_and_cleaned",
    "cancelled_result",
}
FALLBACK_KEYS: Final = {
    "parser_unavailable",
    "parser_failure",
    "ocr_unavailable",
    "sandbox_unavailable",
    "remote_locus_unavailable",
    "cloud_ocr_allowed",
    "typescript_fallback_allowed",
    "in_host_complex_parser_fallback_allowed",
    "silent_format_downgrade_allowed",
    "local_path_reinterpretation_allowed",
}
PLATFORM_KEYS: Final = {
    "id",
    "session_kind",
    "ui_locus",
    "host_locus",
    "parser_locus",
    "ocr_locus",
    "sandbox",
    "ipc",
    "source_rule",
    "remote_rule",
    "cache_locus",
    "parser_status",
    "ocr_status",
    "native_evidence",
}
PLANNED_FEATURES: Final = [
    "artifact-ingress",
    "parser-text-log",
    "parser-pdf",
    "parser-docx",
    "parser-xlsx",
    "ocr-native",
]
RECEIPT_FIELDS: Final = [
    "source-id",
    "source-sha256",
    "byte-count",
    "origin-locus",
    "destination-locus",
    "transport-identity",
    "sequence-range",
    "worker-identity",
    "result-sha256",
]
REQUIRED_EVIDENCE: Final = [
    "placement-and-process-inventory",
    "package-feature-disabled-and-enabled-diffs",
    "authenticated-ipc-and-byte-crossing-capture",
    "sandbox-network-and-ambient-authority-denial",
    "resource-limit-and-cancellation-campaign",
    "crash-timeout-and-residue-campaign",
    "malformed-parser-and-ocr-corpus",
    "remote-origin-and-path-confusion-campaign",
    "install-upgrade-rollback-uninstall-package-campaign",
]
PRODUCT_TRUTH: Final = {
    "placement_decided": True,
    "features_enabled": False,
    "parser_worker_implemented": False,
    "ocr_worker_implemented": False,
    "remote_placement_implemented": False,
    "native_evidence_complete": False,
    "platform_support_claimed": False,
    "release_readiness": False,
}


def load_record(path: Path = RECORD_PATH) -> dict[str, Any]:
    return json.loads(path.read_text(encoding="utf-8"))


def _closed(value: Any, keys: set[str], label: str, failures: list[str]) -> bool:
    if not isinstance(value, dict):
        failures.append(f"{label} must be an object")
        return False
    if set(value) != keys:
        failures.append(f"{label} field closure changed")
        return False
    return True


def _true(record: dict[str, Any], keys: set[str], label: str, failures: list[str]) -> None:
    for key in keys:
        if record.get(key) is not True:
            failures.append(f"{label}.{key} must remain true")


def _false(record: dict[str, Any], keys: set[str], label: str, failures: list[str]) -> None:
    for key in keys:
        if record.get(key) is not False:
            failures.append(f"{label}.{key} must remain false")


def _authorities(value: Any, root: Path, failures: list[str]) -> None:
    if not isinstance(value, list) or len(value) != len(AUTHORITY_PATHS):
        failures.append("authority inventory changed")
        return
    paths: list[Any] = []
    for index, item in enumerate(value):
        if not _closed(item, {"path", "sha256"}, f"authority[{index}]", failures):
            continue
        path = item.get("path")
        paths.append(path)
        if not isinstance(path, str) or path.startswith("/") or ".." in Path(path).parts:
            failures.append(f"authority[{index}] path is not repository-relative")
            continue
        try:
            actual = hashlib.sha256((root / path).read_bytes()).hexdigest()
        except OSError as error:
            failures.append(f"authority[{index}] cannot be read: {error}")
            continue
        if item.get("sha256") != actual:
            failures.append(f"authority[{index}] digest differs from current authority")
    if paths != AUTHORITY_PATHS:
        failures.append("authority inventory changed")


def validate_record(record: Any, root: Path = ROOT) -> list[str]:
    failures: list[str] = []
    if not _closed(record, ROOT_KEYS, "record", failures):
        return failures
    header = {
        "schema_version": 1,
        "record_type": "parser-ocr-platform-placement",
        "decision_id": "ADR-0042",
        "task_id": "1.2.4.3",
        "status": "accepted-placement-not-implemented",
        "reviewed_on": "2026-08-29",
    }
    for key, expected in header.items():
        if record.get(key) != expected:
            failures.append(f"{key} must equal {expected!r}")
    _authorities(record.get("authorities"), root, failures)
    if record.get("required_evidence") != REQUIRED_EVIDENCE:
        failures.append("required evidence inventory changed")
    if record.get("product_truth") != PRODUCT_TRUTH:
        failures.append("product truth was widened")

    placement = record.get("global_placement")
    if _closed(placement, GLOBAL_KEYS, "global_placement", failures):
        if placement.get("parser_owner") != "capability-knowledge-rust":
            failures.append("parser ownership changed")
        if placement.get("process_launcher_owner") != "platform-adapter":
            failures.append("worker launch escaped the platform adapter")
        if placement.get("policy_owner") != "kernel-engine":
            failures.append("policy ownership changed")
        if placement.get("store_owner") != "kernel-engine-operational-store":
            failures.append("store ownership changed")
        _false(
            placement,
            {
                "typescript_parser_allowed",
                "in_process_complex_parser_allowed",
                "ocr_in_host_allowed",
                "worker_network_allowed",
                "ambient_workspace_allowed",
                "credential_access_allowed",
                "second_store_allowed",
            },
            "global_placement",
            failures,
        )

    features = record.get("package_features")
    if _closed(features, FEATURE_KEYS, "package_features", failures):
        if features.get("default_enabled") != []:
            failures.append("parser or OCR feature became enabled by default")
        if features.get("planned") != PLANNED_FEATURES:
            failures.append("planned feature inventory changed")
        if features.get("ocr_status") != "deferred-unavailable":
            failures.append("OCR disposition was promoted")
        _false(
            features,
            {"dynamic_plugin_discovery_allowed", "environment_feature_override_allowed"},
            "package_features",
            failures,
        )
        disabled = str(features.get("disabled_rule"))
        for token in ("no-registration", "no-worker", "no-socket", "no-cache", "no-support-claim"):
            if token not in disabled:
                failures.append(f"disabled feature rule missing: {token}")

    crossing = record.get("byte_crossing")
    if _closed(crossing, BYTE_KEYS, "byte_crossing", failures):
        _false(
            crossing,
            {
                "source_in_arguments_or_environment_allowed",
                "plaintext_shared_temp_allowed",
                "path_as_authority_allowed",
                "worker_output_trusted_before_validation",
                "raw_source_return_default",
            },
            "byte_crossing",
            failures,
        )
        if crossing.get("crossing_receipt_fields") != RECEIPT_FIELDS:
            failures.append("byte-crossing receipt fields changed")
        if "authenticated" not in str(crossing.get("extension_to_host")):
            failures.append("extension-to-host byte transport lost authentication")
        if "digest" not in str(crossing.get("host_to_worker")):
            failures.append("host-to-worker bytes lost source binding")

    cancellation = record.get("cancellation")
    if _closed(cancellation, CANCEL_KEYS, "cancellation", failures):
        _true(
            cancellation,
            {
                "cooperative_grace_then_forced_termination",
                "stdout_and_stderr_drained_concurrently",
                "residue_recorded_and_cleaned",
            },
            "cancellation",
            failures,
        )
        _false(
            cancellation,
            {"terminal_before_worker_reaped_allowed", "partial_derivative_publication_allowed"},
            "cancellation",
            failures,
        )
        checkpoints = cancellation.get("admission_checkpoints")
        required = {"before-byte-transfer", "before-worker-launch", "during-parse-or-ocr", "before-result-publication"}
        if not isinstance(checkpoints, list) or not required <= set(checkpoints):
            failures.append("cancellation checkpoints are incomplete")

    fallback = record.get("fallback")
    if _closed(fallback, FALLBACK_KEYS, "fallback", failures):
        _false(
            fallback,
            {
                "cloud_ocr_allowed",
                "typescript_fallback_allowed",
                "in_host_complex_parser_fallback_allowed",
                "silent_format_downgrade_allowed",
                "local_path_reinterpretation_allowed",
            },
            "fallback",
            failures,
        )
        if not str(fallback.get("sandbox_unavailable")).startswith("block-feature"):
            failures.append("missing sandbox no longer blocks the feature")
        if not str(fallback.get("remote_locus_unavailable")).startswith("block-request"):
            failures.append("missing remote locus no longer blocks the request")

    platforms = record.get("platforms")
    if not isinstance(platforms, list):
        failures.append("platforms must be an array")
        return failures
    ids = [item.get("id") for item in platforms if isinstance(item, dict)]
    if ids != PLATFORM_IDS:
        failures.append("platform inventory must remain complete and ordered")
    for index, platform in enumerate(platforms):
        label = f"platform[{index}]"
        if not _closed(platform, PLATFORM_KEYS, label, failures):
            continue
        platform_id = platform.get("id")
        for key in PLATFORM_KEYS - {"id"}:
            if not isinstance(platform.get(key), str) or not platform[key]:
                failures.append(f"{platform_id}.{key} must be non-empty")
        if platform_id == "macos-apple-silicon":
            if platform.get("parser_status") != "blocked-post-ga" or platform.get("ocr_status") != "blocked-post-ga":
                failures.append("macOS parser and OCR must remain blocked-post-ga")
            if platform.get("native_evidence") != "blocked-macos-not-run":
                failures.append("macOS evidence status was overclaimed")
        else:
            if platform.get("parser_status") != "planned-disabled-evidence-pending":
                failures.append(f"{platform_id} parser status was overclaimed")
            if platform.get("ocr_status") != "deferred-unavailable":
                failures.append(f"{platform_id} OCR disposition was promoted")
            if platform.get("native_evidence") != "not-run":
                failures.append(f"{platform_id} native evidence was overclaimed")
        if platform_id in {"wsl", "remote-ssh", "dev-containers"}:
            if "workspace-side" not in platform.get("host_locus", ""):
                failures.append(f"{platform_id} host must remain workspace-side")
            source = platform.get("source_rule", "")
            if "stays" not in source or "may-cross-once" not in source:
                failures.append(f"{platform_id} source crossing rule was widened")
            if platform.get("remote_rule") == "not-applicable":
                failures.append(f"{platform_id} remote rule must be explicit")
        elif platform.get("remote_rule") != "not-applicable":
            failures.append(f"{platform_id} local session gained a remote placement rule")
    return failures


def main() -> int:
    try:
        record = load_record()
    except (OSError, json.JSONDecodeError) as error:
        print(f"parser/OCR placement validation failed: {error}", file=sys.stderr)
        return 1
    failures = validate_record(record)
    if failures:
        for failure in failures:
            print(f"parser/OCR placement validation failed: {failure}", file=sys.stderr)
        return 1
    print("parser/OCR platform placement validation passed")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
