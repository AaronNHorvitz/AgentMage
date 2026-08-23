#!/usr/bin/env python3
"""Validate zero-model baseline truth and future hash-bound activation."""

from __future__ import annotations

import argparse
import sys
from pathlib import Path
from typing import Any, Final

try:
    from scripts.evidence_core import (
        EvidenceError,
        atomic_write,
        bounded_read,
        canonical_json_bytes,
        read_json_object,
        sha256_file,
        valid_sha256,
    )
except ModuleNotFoundError:
    from evidence_core import (  # type: ignore[no-redef]
        EvidenceError,
        atomic_write,
        bounded_read,
        canonical_json_bytes,
        read_json_object,
        sha256_file,
        valid_sha256,
    )


ROOT: Final = Path(__file__).resolve().parents[1]
POLICY_PATH: Final = ROOT / "architecture" / "model-activation-policy.json"
STATUS_PATH: Final = ROOT / "architecture" / "status-model.json"
PACKAGE_PATH: Final = ROOT / "shells" / "vscode" / "package.json"
PROVIDER_PATH: Final = ROOT / "shells" / "vscode" / "src" / "provider.ts"
REPORT_PATH: Final = ROOT / "evidence" / "current" / "model-activation-report.json"


class ModelActivationError(ValueError):
    """Raised when model activation truth is malformed or unsafe."""


def _denial(code: str, profile_id: str | None) -> dict[str, Any]:
    return {
        "status": "denied",
        "code": code,
        "profile_id": profile_id,
        "fallback_profile_id": None,
        "inference_started": False,
    }


def evaluate_activation(
    policy: dict[str, Any],
    profile: dict[str, Any] | None,
    observed_bindings: dict[str, str],
    platform_id: str,
    offline_runtime_available: bool,
) -> dict[str, Any]:
    """Evaluate one profile without loading a model or selecting a fallback."""

    if profile is None:
        return _denial("model.profile.missing", None)
    profile_id = profile.get("profile_id")
    if not isinstance(profile_id, str) or not profile_id:
        return _denial("model.profile.invalid", None)
    disposition = profile.get("disposition")
    if disposition != "admitted":
        return _denial(f"model.profile.{disposition or 'invalid'}", profile_id)
    if profile.get("enabled") is not True:
        return _denial("model.profile.disabled", profile_id)
    if profile.get("automatic_fallback") is not False:
        return _denial("model.fallback.prohibited", profile_id)
    platforms = profile.get("supported_platforms")
    if (
        not isinstance(platforms, list)
        or platforms != sorted(set(platforms))
        or platform_id not in platforms
        or platform_id not in policy["allowed_platforms"]
    ):
        return _denial("model.platform.unsupported", profile_id)
    bindings = profile.get("bindings")
    required = policy["required_binding_ids"]
    if not isinstance(bindings, dict) or sorted(bindings) != required:
        return _denial("model.bindings.incomplete", profile_id)
    if any(not valid_sha256(bindings[name]) for name in required):
        return _denial("model.bindings.invalid", profile_id)
    if sorted(observed_bindings) != required or any(
        observed_bindings[name] != bindings[name] for name in required
    ):
        return _denial("model.bindings.mismatch", profile_id)
    if not offline_runtime_available:
        return _denial("model.runtime.offline_unavailable", profile_id)
    return {
        "status": "enabled",
        "code": "model.activation.admitted",
        "profile_id": profile_id,
        "fallback_profile_id": None,
        "inference_started": False,
    }


def validate_policy(policy: Any) -> list[str]:
    expected = {
        "schema_version",
        "policy_id",
        "decision_ids",
        "allowed_platforms",
        "automatic_fallback",
        "baseline_enabled_profiles",
        "baseline_enabled_endpoint_profiles",
        "baseline_enabled_routes",
        "strict_local_complete_target",
        "remote_inference_optional",
        "profile_classes",
        "candidate_profile_ids",
        "deterministic_provider",
        "required_binding_ids",
    }
    if not isinstance(policy, dict) or set(policy) != expected:
        return ["model.policy.field_closure"]
    failures: list[str] = []
    if policy.get("schema_version") != 1 or policy.get("policy_id") != (
        "agentmage-model-activation-policy-v1"
    ):
        failures.append("model.policy.identity")
    for field in (
        "allowed_platforms",
        "baseline_enabled_profiles",
        "baseline_enabled_endpoint_profiles",
        "baseline_enabled_routes",
        "candidate_profile_ids",
        "decision_ids",
        "profile_classes",
        "required_binding_ids",
    ):
        value = policy.get(field)
        if (
            not isinstance(value, list)
            or not all(isinstance(item, str) and item for item in value)
            or value != sorted(set(value))
        ):
            failures.append(f"model.policy.{field}")
    if policy.get("automatic_fallback") is not False:
        failures.append("model.policy.fallback")
    if policy.get("decision_ids") != ["ADR-0027", "ADR-0044"]:
        failures.append("model.policy.decisions")
    if policy.get("profile_classes") != [
        "local_network_private",
        "remote_managed",
        "remote_private",
        "strict_local",
    ]:
        failures.append("model.policy.profile_classes")
    if policy.get("strict_local_complete_target") is not True:
        failures.append("model.policy.strict_local")
    if policy.get("remote_inference_optional") is not True:
        failures.append("model.policy.remote_optional")
    if policy.get("baseline_enabled_endpoint_profiles") != []:
        failures.append("model.baseline.enabled_endpoint_drift")
    if policy.get("baseline_enabled_routes") != []:
        failures.append("model.baseline.enabled_route_drift")
    provider = policy.get("deterministic_provider")
    if provider != {
        "model_id": None,
        "model_inference": False,
        "profile_discovery": "signed-exact-admitted-only",
        "vendor": "agentmage",
    }:
        failures.append("model.policy.provider")
    return sorted(set(failures))


def build_report(root: Path = ROOT) -> dict[str, Any]:
    policy = read_json_object(root, POLICY_PATH.relative_to(ROOT).as_posix())
    status = read_json_object(root, STATUS_PATH.relative_to(ROOT).as_posix())
    package = read_json_object(root, PACKAGE_PATH.relative_to(ROOT).as_posix())
    provider_source = bounded_read(
        root, PROVIDER_PATH.relative_to(ROOT).as_posix(), maximum_bytes=256 * 1024
    ).decode("utf-8")
    failures = validate_policy(policy)
    product = status.get("current_product")
    if not isinstance(product, dict) or product.get("enabled_models") != (
        policy.get("baseline_enabled_profiles")
    ):
        failures.append("model.baseline.enabled_profile_drift")
    models = status.get("models")
    model_by_id = (
        {item.get("id"): item for item in models if isinstance(item, dict)}
        if isinstance(models, list)
        else {}
    )
    candidates: list[dict[str, Any]] = []
    for profile_id in policy.get("candidate_profile_ids", []):
        model = model_by_id.get(profile_id)
        if not isinstance(model, dict):
            failures.append("model.candidate.missing")
            continue
        if (
            model.get("disposition_status") != "rejected"
            or model.get("enabled") is not False
            or model.get("automatic_fallback") is not False
        ):
            failures.append("model.candidate.overclaim")
        candidates.append(
            {
                "profile_id": profile_id,
                "disposition": model.get("disposition_status"),
                "enabled": model.get("enabled"),
                "picker_visible": False,
                "automatic_fallback": model.get("automatic_fallback"),
            }
        )
    providers = package.get("contributes", {}).get("languageModelChatProviders", [])
    if providers != [{"vendor": "agentmage", "displayName": "AgentMage"}]:
        failures.append("model.provider.registration")
    if "PROVIDER_MODEL_ID" in provider_source or "PROVIDER_FAMILY" in provider_source:
        failures.append("model.provider.identity")
    if "discoverModels" not in provider_source or "ModelPickerSnapshot" not in provider_source:
        failures.append("model.provider.discovery_missing")
    lowered = provider_source.lower()
    if "gemma" in lowered or any(profile_id in provider_source for profile_id in model_by_id):
        failures.append("model.provider.rejected_identity_exposed")
    if failures:
        raise ModelActivationError("; ".join(sorted(set(failures))))
    inputs = [POLICY_PATH, STATUS_PATH, PACKAGE_PATH, PROVIDER_PATH]
    return {
        "schema_version": 1,
        "report_id": "agentmage-current-model-activation-v1",
        "inputs": [
            {
                "path": path.relative_to(root).as_posix(),
                "sha256": sha256_file(root, path.relative_to(root).as_posix()),
            }
            for path in inputs
        ],
        "enabled_profiles": [],
        "enabled_profile_count": 0,
        "enabled_endpoint_profiles": [],
        "enabled_routes": [],
        "candidates": candidates,
        "deterministic_provider": {
            **policy["deterministic_provider"],
            "classification": "zero-profile-exact-discovery-boundary",
        },
        "automatic_fallback": False,
        "strict_local_complete_target": True,
        "remote_inference_optional": True,
        "activation_status": "disabled-until-new-hash-bound-admission",
        "support_claim": "none-pre-release",
    }


def check_report(root: Path = ROOT) -> list[str]:
    try:
        expected = build_report(root)
        actual = read_json_object(root, REPORT_PATH.relative_to(ROOT).as_posix())
    except (EvidenceError, ModelActivationError, UnicodeDecodeError) as error:
        return [str(error)]
    return [] if actual == expected else ["model.activation_report_stale"]


def main(argv: list[str] | None = None) -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--write", action="store_true")
    args = parser.parse_args(argv)
    if args.write:
        try:
            atomic_write(REPORT_PATH, canonical_json_bytes(build_report()))
        except (EvidenceError, ModelActivationError, OSError, UnicodeDecodeError) as error:
            print(f"model activation failed: {error}", file=sys.stderr)
            return 1
    failures = check_report()
    if failures:
        for failure in failures:
            print(f"model activation failed: {failure}", file=sys.stderr)
        return 1
    print("zero-model baseline and hash-bound activation refusal validated")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
