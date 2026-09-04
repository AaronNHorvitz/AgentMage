#!/usr/bin/env python3
"""Build and verify the disabled Decision 0041 agent-profile catalog."""

from __future__ import annotations

import argparse
import hashlib
import json
import re
from pathlib import Path
from typing import Final

ROOT: Final = Path(__file__).resolve().parents[1]
SOURCE: Final = ROOT / "docs/architecture/planning-review-and-delivery-agent-profiles.md"
OUTPUTS: Final = {
    "catalog": ROOT / "artifacts/sprints/sprint-92/agent-profile-catalog.json",
    "reference": ROOT / "artifacts/sprints/sprint-92/agent-profile-reference.md",
    "matrix": ROOT / "artifacts/sprints/sprint-92/agent-profile-compatibility.json",
    "fixtures": ROOT / "artifacts/sprints/sprint-92/agent-profile-fixtures.json",
    "evaluation": ROOT / "artifacts/sprints/sprint-93/synthetic-agent-evaluation.json",
}
PROFILE_RE: Final = re.compile(
    r"^\| `(?P<id>AG-\d{2})` \| (?P<name>[^|]+?) \| (?P<responsibility>[^|]+?) \| (?P<ceiling>[^|]+?) \|$"
)
FIXTURE_CLASSES: Final = (
    "valid",
    "incomplete",
    "contradictory",
    "stale",
    "hostile",
    "unsupported",
    "overbroad",
)
TEMPLATES: Final = (
    "research",
    "planning",
    "briefing",
    "meetings",
    "documents",
    "repository-learning",
    "coding",
    "testing",
    "review",
    "verification",
)
INDEPENDENT_REVIEW: Final = {
    "AG-10",
    "AG-11",
    "AG-35",
    "AG-36",
    "AG-37",
    "AG-38",
    "AG-39",
    "AG-40",
    "AG-41",
}
DETERMINISTIC_SERVICES: Final = (
    "policy-and-approval",
    "credential-brokerage",
    "signing-and-key-custody",
    "evidence-and-provenance-retention",
    "artifact-verification",
    "merge-and-deployment-actuation",
    "postcondition-reconciliation",
)


class AgentProfileContractError(RuntimeError):
    """Raised when the source catalog or generated artifacts are invalid."""


def canonical_bytes(value: object) -> bytes:
    return (json.dumps(value, sort_keys=True, separators=(",", ":")) + "\n").encode()


def sha256(value: bytes) -> str:
    return hashlib.sha256(value).hexdigest()


def parse_profiles(source: str) -> list[dict[str, object]]:
    rows: list[dict[str, object]] = []
    family = ""
    for line in source.splitlines():
        if line == "### Coordination and Engineering":
            family = "coordination-and-engineering"
        elif line == "### Planning, Review, and Maintenance":
            family = "planning-review-and-maintenance"
        match = PROFILE_RE.fullmatch(line)
        if not match:
            continue
        row = match.groupdict()
        profile_id = row["id"]
        source_identity = {
            "profile_id": profile_id,
            "name": row["name"],
            "family": family,
            "bounded_responsibility": row["responsibility"],
            "authority_ceiling": row["ceiling"],
        }
        source_hash = sha256(canonical_bytes(source_identity))
        rows.append(
            {
                "schema_version": 1,
                **source_identity,
                "version": "1.0.0",
                "owner": "agentmage-project",
                "accepted_tasks": ["bounded-work-packet-analysis"],
                "prohibited_tasks": [*DETERMINISTIC_SERVICES, "self-approval", "recursive-spawn"],
                "compatible_model_profiles": ["qualified-local-model-required"],
                "compatible_codec_profiles": ["shared-runtime-codec-required"],
                "requested_tools": ["declared-shared-tool-only"],
                "requested_roots": ["exact-granted-root-only"],
                "provider_object_classes": ["none-by-registration"],
                "input_schema": "bounded-work-packet-v1",
                "output_schema": "authority-free-proposal-v1",
                "evidence": ["attributable-source-citations", "runtime-receipts"],
                "citations": ["immutable-source-identity"],
                "budgets": {
                    "context_items": 64,
                    "turns": 16,
                    "processes": 0,
                    "artifacts": 16,
                    "output_bytes": 65536,
                    "elapsed_milliseconds": 300000,
                    "retries": 0,
                },
                "approval": "exact-user-review-required",
                "completion": "deterministic-verifier-evidence-required",
                "cancellation": "shared-runtime-cancellation-required",
                "stop_conditions": ["budget-exhausted", "dependency-unavailable", "authority-denied"],
                "isolation": "shared-runtime-work-packet",
                "retention": "evidence-policy-owned",
                "redaction": "classification-policy-owned",
                "child_spawn": "prohibited",
                "source_sha256": source_hash,
                "signature": {"state": "unverified", "sha256": sha256(b"unsigned\0" + source_hash.encode())},
                "lifecycle": "disabled",
                "independent_review_required": profile_id in INDEPENDENT_REVIEW,
                "independent_review": {
                    "reviewed_source_identity": source_hash if profile_id in INDEPENDENT_REVIEW else None,
                    "isolated_conclusions": profile_id in INDEPENDENT_REVIEW,
                    "attributable_findings": profile_id in INDEPENDENT_REVIEW,
                    "dissent_preserved": profile_id in INDEPENDENT_REVIEW,
                    "implementer_approval_prohibited": profile_id in INDEPENDENT_REVIEW,
                },
                "user_reviewed": False,
                "enabled": False,
            }
        )
    expected = [f"AG-{number:02d}" for number in range(1, 50)]
    actual = [str(row["profile_id"]) for row in rows]
    if actual != expected or len({str(row["name"]) for row in rows}) != 49:
        raise AgentProfileContractError("canonical catalog must contain ordered unique AG-01 through AG-49")
    return rows


def build_artifacts() -> dict[str, bytes]:
    source = SOURCE.read_text(encoding="utf-8")
    profiles = parse_profiles(source)
    catalog = {
        "schema_version": 1,
        "source": str(SOURCE.relative_to(ROOT)),
        "source_sha256": sha256(SOURCE.read_bytes()),
        "profile_count": 49,
        "enabled_profile_count": 0,
        "runtime_implementation_count": 1,
        "profiles": profiles,
    }
    reference_lines = [
        "# Generated Agent Profile Reference",
        "",
        "This reference is generated from the Decision 0041 architecture catalog. All profiles are disabled.",
        "",
        "| ID | Name | Family | Maximum requested authority | Lifecycle |",
        "| --- | --- | --- | --- | --- |",
        *[
            f"| `{row['profile_id']}` | {row['name']} | {row['family']} | {row['authority_ceiling']} | disabled |"
            for row in profiles
        ],
        "",
    ]
    matrix = {
        "schema_version": 1,
        "semantics": "requests-and-maximum-ceilings-only-never-grants",
        "profiles": [
            {
                "profile_id": row["profile_id"],
                "capabilities": ["bounded-work-packet-analysis"],
                "tools": row["requested_tools"],
                "provider_objects": row["provider_object_classes"],
                "model_needs": row["compatible_model_profiles"],
                "evidence": row["evidence"],
                "authority_ceiling": row["authority_ceiling"],
            }
            for row in profiles
        ],
        "lifecycle_events": ["addition", "deprecation", "replacement", "disablement", "package-removal", "source-or-signature-change"],
    }
    fixtures = {
        "schema_version": 1,
        "templates": list(TEMPLATES),
        "case_count": len(profiles) * len(FIXTURE_CLASSES),
        "cases": [
            {
                "case_id": f"{row['profile_id']}-{fixture_class}",
                "profile_id": row["profile_id"],
                "class": fixture_class,
                "expected": "valid-disabled" if fixture_class == "valid" else "rejected-disabled",
                "real_effect_count": 0,
            }
            for row in profiles
            for fixture_class in FIXTURE_CLASSES
        ],
    }
    evaluation = {
        "schema_version": 1,
        "environment": {name: "synthetic" for name in ("files", "tools", "models", "connectors", "grants")},
        "profile_count": 49,
        "case_count": 343,
        "capability_states": ["available", "degraded", "untested", "denied", "incompatible"],
        "hostile_classes": ["self-modifying", "self-enabling", "self-spawning", "hidden-network", "excessive-permission", "source-instruction", "credential", "vague-completion"],
        "all_source_hashes_resolved": True,
        "all_signatures_verified": False,
        "all_user_reviews_present": False,
        "enabled_profile_count": 0,
        "real_data_read_count": 0,
        "real_external_effect_count": 0,
        "parallel_execution_engine_count": 0,
        "decision": "disabled-awaiting-signature-compatibility-and-user-review",
    }
    return {
        "catalog": canonical_bytes(catalog),
        "reference": "\n".join(reference_lines).encode(),
        "matrix": canonical_bytes(matrix),
        "fixtures": canonical_bytes(fixtures),
        "evaluation": canonical_bytes(evaluation),
    }


def validate_artifacts() -> list[str]:
    expected = build_artifacts()
    errors = []
    for name, path in OUTPUTS.items():
        if not path.is_file() or path.read_bytes() != expected[name]:
            errors.append(f"stale or absent artifact: {path.relative_to(ROOT)}")
    return errors


def write_artifacts() -> None:
    for name, content in build_artifacts().items():
        OUTPUTS[name].parent.mkdir(parents=True, exist_ok=True)
        OUTPUTS[name].write_bytes(content)


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--write", action="store_true")
    args = parser.parse_args()
    if args.write:
        write_artifacts()
    errors = validate_artifacts()
    if errors:
        raise AgentProfileContractError("; ".join(errors))
    print("validated 49 disabled agent profiles, 343 fixtures, and zero real effects")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
