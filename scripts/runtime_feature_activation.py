#!/usr/bin/env python3
"""Validate the closed foundational-runtime feature activation inventory."""

from __future__ import annotations

import json
import sys
from pathlib import Path
from typing import Any, Final

ROOT: Final = Path(__file__).resolve().parents[1]
MANIFEST: Final = ROOT / "architecture/runtime-feature-activation.json"
FEATURE_IDS: Final = (
    "artifact_ingress", "plain_text_extractor", "log_extractor", "docx_extractor",
    "pdf_extractor", "xlsx_extractor", "ocr", "retrieval", "workflow_supervision",
    "model_assisted_repair", "native_participant", "native_provider_compatibility", "mcp",
)
UNAVAILABLE: Final = {
    "docx_extractor", "pdf_extractor", "xlsx_extractor", "ocr", "model_assisted_repair", "mcp"
}


def validate_manifest(value: Any) -> list[str]:
    if not isinstance(value, dict) or set(value) != {
        "schema_version", "record_type", "features", "disabled_invariants"
    }:
        return ["feature activation manifest shape is invalid"]
    failures: list[str] = []
    if value["schema_version"] != 1 or value["record_type"] != "agentmage-runtime-feature-activation":
        failures.append("feature activation identity is invalid")
    features = value.get("features", [])
    if not isinstance(features, list) or [item.get("id") for item in features if isinstance(item, dict)] != list(FEATURE_IDS):
        failures.append("feature inventory is incomplete or reordered")
        return failures
    for item in features:
        if set(item) != {"id", "available", "default_enabled", "owner", "registration"}:
            failures.append(f"feature fields are not closed: {item.get('id')}")
            continue
        if not isinstance(item["available"], bool) or not isinstance(item["default_enabled"], bool):
            failures.append(f"feature booleans are invalid: {item['id']}")
        if item["id"] in UNAVAILABLE and (
            item["available"] is not False or item["default_enabled"] is not False
            or item["registration"] != "none"
        ):
            failures.append(f"unavailable feature was activated: {item['id']}")
        if item["default_enabled"] and not item["available"]:
            failures.append(f"default-enabled feature is unavailable: {item['id']}")
    expected_invariants = [
        "no_tool_definition", "no_client_registration", "no_process_or_socket",
        "no_cache_or_schema_authority", "no_support_claim",
    ]
    if value.get("disabled_invariants") != expected_invariants:
        failures.append("disabled feature invariants changed")
    return failures


def validate_sources() -> list[str]:
    rust = (ROOT / "shells/host/src/feature_activation.rs").read_text(encoding="utf-8")
    tools = (ROOT / "shells/host/src/runtime_tools.rs").read_text(encoding="utf-8")
    extension = (ROOT / "shells/vscode/src/extension.ts").read_text(encoding="utf-8")
    package = json.loads((ROOT / "shells/vscode/package.json").read_text(encoding="utf-8"))
    failures: list[str] = []
    for marker in ("docx_extractor: false", "pdf_extractor: false", "xlsx_extractor: false", "model_assisted_repair: false", "mcp: false"):
        if marker not in rust:
            failures.append(f"Rust activation lacks marker: {marker}")
    for marker in ("features.artifact_ingress", "features.retrieval", "ArtifactToolKind::Search"):
        if marker not in tools:
            failures.append(f"native registry lacks feature guard: {marker}")
    for marker in ("if (features.nativeParticipant)", "if (features.nativeProviderCompatibility)"):
        if marker not in extension:
            failures.append(f"VS Code registration lacks guard: {marker}")
    properties = package.get("contributes", {}).get("configuration", {}).get("properties", {})
    for key in ("agentmage.features.nativeParticipant", "agentmage.features.nativeProviderCompatibility"):
        if properties.get(key, {}).get("type") != "boolean" or properties.get(key, {}).get("default") is not True:
            failures.append(f"VS Code feature configuration is invalid: {key}")
    return failures


def check() -> list[str]:
    try:
        value = json.loads(MANIFEST.read_text(encoding="utf-8"))
    except (OSError, json.JSONDecodeError) as error:
        return [f"cannot read feature activation manifest: {error}"]
    return validate_manifest(value) + validate_sources()


def main() -> int:
    failures = check()
    if failures:
        print("\n".join(failures), file=sys.stderr)
        return 1
    print(f"Validated {len(FEATURE_IDS)} independent runtime feature activations")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
