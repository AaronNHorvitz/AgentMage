#!/usr/bin/env python3
"""Build the gate-owned Sprint 49 product-routing composition review."""

from __future__ import annotations

import argparse
import hashlib
import json
import subprocess
import sys
from pathlib import Path
from typing import Any, Final

ROOT: Final = Path(__file__).resolve().parents[1]
REPORT: Final = ROOT / "artifacts/sprints/sprint-49/product-routing-review.json"
SOURCES: Final = (
    "kernel/engine/src/model_routing.rs",
    "shells/host/src/cli_runtime.rs",
    "docs/architecture/measured-local-model-routing.md",
    "model-profiles/routing/measured-routing-decision-table-v1.json",
    "model-profiles/routing/historical-later-candidates.json",
)
REQUIRED_RUST: Final = (
    "pub struct AuthenticatedRoutingEnvelope",
    "pub trait RoutingAuthenticationVerifier",
    "pub struct RoutingProfileIdentityAudit",
    "pub struct NativeRoutingAuditView",
    "pub struct MeasuredRoutingService",
    "model.routing.authentication-failed",
    "deterministic-measured-local-v1",
    "model.routing.no-eligible-profile",
    "frontier_transfer",
    "model_confidence_used",
    "authenticated_product_service_owns_catalog_and_exposes_exact_native_audit",
    "product_service_fails_authentication_without_decision_or_audit",
)


def git_bytes(revision: str, path: str) -> bytes:
    result = subprocess.run(
        ["git", "show", f"{revision}:{path}"], cwd=ROOT, capture_output=True,
        check=False, timeout=30,
    )
    if result.returncode:
        raise ValueError(f"review source unavailable: {path}")
    return result.stdout


def expected(revision: str) -> dict[str, Any]:
    sources = {path: git_bytes(revision, path) for path in SOURCES}
    rust = sources["kernel/engine/src/model_routing.rs"].decode("utf-8")
    host = sources["shells/host/src/cli_runtime.rs"].decode("utf-8")
    table = json.loads(sources["model-profiles/routing/measured-routing-decision-table-v1.json"])
    historical = json.loads(sources["model-profiles/routing/historical-later-candidates.json"])
    checks = {
        "authenticated_product_envelope_is_bound": all(token in rust for token in REQUIRED_RUST[:2]),
        "kernel_owned_catalog_service_is_bound": all(token in rust for token in REQUIRED_RUST[2:5]),
        "authentication_and_empty_catalog_fail_closed": all(token in rust for token in REQUIRED_RUST[5:8]),
        "native_exact_identity_audit_is_bound": all(token in rust for token in REQUIRED_RUST[8:]),
        "installed_interface_adapter_is_bound": all(
            token in host for token in (
                "pub trait InteractiveCliRoutingSink",
                "pub fn drive_interactive_cli_routing",
                "MeasuredRoutingService<V>",
                "installed_interface_presents_only_the_kernel_owned_zero_profile_audit",
            )
        ),
        "disabled_remote_authority_is_bound": (
            table.get("frontier_transfer") is False
            and table.get("invisible_fallback") is False
            and table.get("model_confidence_used") is False
        ),
        "zero_enabled_profiles_are_bound": (
            historical.get("enabled_profile_count") == 0
            and historical.get("automatic_routing_enabled") is False
        ),
    }
    return {
        "schema_version": 1,
        "record_type": "agentmage-sprint-49-product-routing-review",
        "source_revision": revision,
        "reviewer_identity": "scripts/sprint_49_product_routing_review.py",
        "review_class": "gate-owned-automated-product-routing-composition-review",
        "independent_human_review_performed": False,
        "source_sha256": {path: hashlib.sha256(data).hexdigest() for path, data in sources.items()},
        "checks": checks,
        "enabled_profile_count": 0,
        "automatic_routing_enabled": False,
        "status": "PASS_LOCAL_COMPOSITION_REVIEW" if all(checks.values()) else "FAIL",
        "limitations": [
            "This is gate-owned automated review, not a human-review claim.",
            "The source adapter is not trusted installed-package execution evidence.",
            "No exact profile, model, route, adapter, platform, or release is enabled.",
            "Live benchmarks and supported-platform product campaigns remain absent.",
        ],
    }


def validate(value: Any) -> list[str]:
    if not isinstance(value, dict):
        return ["review is not an object"]
    revision = value.get("source_revision")
    if not isinstance(revision, str) or len(revision) != 40 or any(char not in "0123456789abcdef" for char in revision):
        return ["source revision invalid"]
    try:
        return [] if value == expected(revision) else ["review is stale, incomplete, or widened"]
    except (ValueError, json.JSONDecodeError) as error:
        return [str(error)]


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--write", action="store_true")
    parser.add_argument("--source-revision", default="HEAD")
    arguments = parser.parse_args()
    revision = subprocess.check_output(["git", "rev-parse", arguments.source_revision], cwd=ROOT, text=True).strip()
    if arguments.write:
        REPORT.parent.mkdir(parents=True, exist_ok=True)
        REPORT.write_text(json.dumps(expected(revision), indent=2, sort_keys=True) + "\n")
    try:
        value = json.loads(REPORT.read_text(encoding="utf-8"))
    except (OSError, json.JSONDecodeError) as error:
        print(error, file=sys.stderr)
        return 1
    failures = validate(value)
    if failures:
        print("\n".join(failures), file=sys.stderr)
        return 1
    print("Sprint 49 authenticated product-routing composition passed")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
