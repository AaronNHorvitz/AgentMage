#!/usr/bin/env python3
"""Validate and report the signed manual patch metadata contract."""

from __future__ import annotations

import argparse
import hashlib
import json
import os
import subprocess
import tempfile
from pathlib import Path
from typing import Any


ROOT = Path(__file__).resolve().parents[1]
SCHEMA_PATH = ROOT / "schemas/support/signed-manual-patch-metadata.schema.json"
FIXTURE_PATH = ROOT / "schemas/support/examples/signed-manual-patch-metadata.valid.json"
REPORT_PATH = ROOT / "artifacts/sprints/sprint-3/story-3.2/manual-patch-metadata-report.json"
ARTIFACT_IDS = (
    "agentmage-package",
    "release-manifest",
    "provenance",
    "sbom",
    "cryptographic-bom",
    "model-bom",
    "component-inventory",
    "configuration",
    "capability-delta",
    "authority-delta",
)
ARTIFACT_KINDS = (
    "package",
    "release-manifest",
    "provenance",
    "sbom",
    "cryptographic-bom",
    "model-bom",
    "component-inventory",
    "configuration",
    "capability-delta",
    "authority-delta",
)
SOURCE_PATHS = (
    "architecture/signed-update-design.json",
    "architecture/rollback-design.json",
    "support/vulnerability-support-policy.json",
    "schemas/support/signed-manual-patch-metadata.schema.json",
    "schemas/support/examples/signed-manual-patch-metadata.valid.json",
    "scripts/validate_manual_patch_schema.mjs",
    "scripts/manual_patch_metadata.py",
    "tests/test_manual_patch_schema.mjs",
    "tests/test_manual_patch_metadata.py",
)
TOP_LEVEL_FIELDS = {
    "schema_version",
    "record_type",
    "metadata_version",
    "fixture_status",
    "release",
    "current_release_precondition",
    "platform",
    "signing",
    "artifacts",
    "provenance",
    "prerequisites",
    "schema_impact",
    "rollback",
    "revocation",
    "support",
    "advisory",
    "delivery",
    "activation",
    "claims",
}


def canonical_json(value: Any) -> bytes:
    return (json.dumps(value, indent=2, sort_keys=True) + "\n").encode("utf-8")


def read_json(path: Path) -> Any:
    return json.loads(path.read_text(encoding="utf-8"))


def sha256_file(path: Path) -> str:
    return hashlib.sha256(path.read_bytes()).hexdigest()


def write_atomic(path: Path, content: bytes) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    descriptor, temporary_name = tempfile.mkstemp(
        prefix=".agentmage-manual-patch-", dir=path.parent
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


def validate_formal_schema(root: Path = ROOT) -> list[str]:
    commands = (
        ("node", "scripts/validate_manual_patch_schema.mjs"),
        ("node", "--test", "tests/test_manual_patch_schema.mjs"),
    )
    failures = []
    for command in commands:
        result = subprocess.run(
            command,
            cwd=root,
            stdin=subprocess.DEVNULL,
            stdout=subprocess.PIPE,
            stderr=subprocess.STDOUT,
            text=True,
            check=False,
            timeout=20,
        )
        if result.returncode != 0:
            failures.append(f"manual patch schema check failed: {' '.join(command)}")
    return failures


def validate_metadata(value: Any) -> list[str]:
    if not isinstance(value, dict):
        return ["manual patch metadata must be an object"]
    failures = []
    if set(value) != TOP_LEVEL_FIELDS:
        failures.append("manual patch metadata field closure is invalid")
    if (
        value.get("schema_version") != 1
        or value.get("record_type") != "signed-manual-patch-metadata"
        or value.get("metadata_version") != 1
        or value.get("fixture_status")
        != "synthetic-contract-fixture-not-releasable"
    ):
        failures.append("manual patch metadata identity is invalid")
    release = value.get("release", {})
    current = value.get("current_release_precondition", {})
    if (
        release.get("release_id") != f"agentmage-{release.get('semantic_version')}"
        or current.get("release_id") != f"agentmage-{current.get('semantic_version')}"
        or not isinstance(release.get("release_sequence"), int)
        or not isinstance(current.get("release_sequence"), int)
        or release.get("release_sequence", 0) <= current.get("release_sequence", 0)
    ):
        failures.append("manual patch release identity or anti-downgrade precondition is invalid")

    signing = value.get("signing", {})
    authorized = signing.get("authorized_signer_key_ids", [])
    signatures = signing.get("detached_signatures", [])
    signer_ids = [item.get("signer_key_id") for item in signatures if isinstance(item, dict)]
    if (
        signing.get("algorithm") != "ed25519"
        or signing.get("verify_before_extraction") is not True
        or not isinstance(signing.get("threshold"), int)
        or signing.get("threshold", 0) < 1
        or signing.get("threshold", 0) > len(set(signer_ids))
        or len(authorized) != len(set(authorized))
        or any(signer_id not in authorized for signer_id in signer_ids)
    ):
        failures.append("manual patch signer threshold or trust policy is invalid")

    artifacts = value.get("artifacts", [])
    if (
        tuple(item.get("artifact_id") for item in artifacts if isinstance(item, dict))
        != ARTIFACT_IDS
        or tuple(item.get("kind") for item in artifacts if isinstance(item, dict))
        != ARTIFACT_KINDS
        or len({item.get("path") for item in artifacts if isinstance(item, dict)})
        != len(ARTIFACT_IDS)
        or len({item.get("sha256") for item in artifacts if isinstance(item, dict)})
        != len(ARTIFACT_IDS)
    ):
        failures.append("manual patch artifact checksum closure is invalid")
    by_id = {
        item.get("artifact_id"): item
        for item in artifacts
        if isinstance(item, dict)
    }
    provenance = value.get("provenance", {})
    if (
        by_id.get("release-manifest", {}).get("sha256")
        != release.get("release_manifest_sha256")
        or by_id.get("provenance", {}).get("sha256")
        != provenance.get("statement_sha256")
        or provenance.get("source_commit") != release.get("source_commit")
        or provenance.get("synthetic_values") is not True
    ):
        failures.append("manual patch provenance binding is invalid")

    if value.get("prerequisites") != {
        "current_release_must_match": True,
        "platform_must_match": True,
        "local_trust_root_required": True,
        "minimum_free_bytes": 2_000_000,
        "administrator_required": False,
        "user_selected_local_bundle": True,
    }:
        failures.append("manual patch prerequisites are invalid")
    impact = value.get("schema_impact", {})
    if (
        impact.get("configuration_from") != 1
        or impact.get("configuration_to") != 1
        or impact.get("data_schema_changed") is not False
        or impact.get("migration_required") is not False
        or impact.get("migration_id") is not None
        or impact.get("reversible") is not True
        or impact.get("later_user_data_overwrite_allowed") is not False
    ):
        failures.append("manual patch schema-impact contract is invalid")

    rollback = value.get("rollback", {})
    if (
        rollback.get("design_id") != "ADR-0006"
        or rollback.get("target_release_id") != current.get("release_id")
        or rollback.get("target_package_sha256") != current.get("package_sha256")
        or rollback.get("target_supported") is not True
        or rollback.get("target_revoked") is not False
        or rollback.get("exact_active_precondition_required") is not True
        or rollback.get("reverse_migration_required") is not False
    ):
        failures.append("manual patch rollback binding is invalid")
    revocation = value.get("revocation", {})
    if (
        revocation.get("local_policy_check_required") is not True
        or any(key_id in revocation.get("revoked_signer_key_ids", []) for key_id in signer_ids)
        or release.get("release_id") in revocation.get("revoked_release_ids", [])
        or any(
            item.get("sha256") in revocation.get("revoked_artifact_sha256", [])
            for item in artifacts
            if isinstance(item, dict)
        )
    ):
        failures.append("manual patch revocation state is invalid")
    if value.get("support") != {
        "target_release_state": "supported",
        "target_support_end": "2027-01-01",
        "current_release_state_after_patch": "security-fixes-only",
        "support_policy_id": "AM-VSP-001",
    }:
        failures.append("manual patch support state is invalid")
    if value.get("delivery") != {
        "mode": "user-initiated-local-import",
        "acquisition_authority": "outside-agentmage-runtime",
        "automatic_update_checks": False,
        "background_downloads": False,
        "remote_trigger": False,
        "remote_control": False,
        "transaction_network_allowed": False,
    }:
        failures.append("manual patch metadata introduced hidden delivery authority")
    if value.get("activation") != {
        "verify_before_staging": True,
        "atomic_activation_required": True,
        "prior_release_preserved_until_postcheck": True,
        "durable_receipt_each_transition": True,
        "explicit_user_approval_required": True,
    }:
        failures.append("manual patch activation controls are invalid")
    if value.get("claims") != {
        "production_signer": False,
        "releasable_package": False,
        "implemented_verifier": False,
        "implemented_activation": False,
        "macos_support": False,
    }:
        failures.append("manual patch fixture made an implementation or support overclaim")
    return failures


def validate_inputs(root: Path = ROOT) -> list[str]:
    try:
        fixture = read_json(root / FIXTURE_PATH.relative_to(ROOT))
    except (OSError, json.JSONDecodeError) as error:
        return [f"cannot read manual patch fixture: {error}"]
    failures = validate_metadata(fixture)
    failures.extend(validate_formal_schema(root))
    for relative in SOURCE_PATHS:
        if not (root / relative).is_file():
            failures.append(f"manual patch metadata source is missing: {relative}")
    return failures


def build_report(root: Path = ROOT) -> dict[str, Any]:
    failures = validate_inputs(root)
    if failures:
        raise ValueError("; ".join(failures))
    fixture = read_json(root / FIXTURE_PATH.relative_to(ROOT))
    return {
        "schema_version": 1,
        "story_id": "3.2",
        "task_id": "3.2.1.2",
        "status": "pass-contract-definition",
        "fixture_identity": {
            "status": fixture["fixture_status"],
            "sha256": sha256_file(root / FIXTURE_PATH.relative_to(ROOT)),
        },
        "summary": {
            "artifact_checksum_count": len(fixture["artifacts"]),
            "authorized_signer_count": len(
                fixture["signing"]["authorized_signer_key_ids"]
            ),
            "detached_signature_count": len(
                fixture["signing"]["detached_signatures"]
            ),
            "release_sequence_increase": (
                fixture["release"]["release_sequence"]
                - fixture["current_release_precondition"]["release_sequence"]
            ),
            "network_authority_count": 0,
        },
        "formal_schema_validation": {
            "engine": "ajv-draft-2020-12",
            "canonical_record_count": 1,
            "node_mutation_test_count": 4,
            "status": "pass",
        },
        "source_artifacts": [
            {
                "path": relative,
                "sha256": sha256_file(root / relative),
                "bytes": (root / relative).stat().st_size,
            }
            for relative in SOURCE_PATHS
        ],
        "private_user_data_used": False,
        "network_used": False,
        "production_signer_claim": "none",
        "releasable_package_claim": "none",
        "implemented_verifier_claim": "none",
        "implemented_activation_claim": "none",
        "release_claim": "none",
        "macos_execution_status": "blocked-macos",
        "macos_evidence_substituted": False,
        "macos_support_claim": "none",
    }


def validate_report(value: Any, root: Path = ROOT) -> list[str]:
    if not isinstance(value, dict):
        return ["manual patch metadata report must be an object"]
    failures = []
    try:
        expected = build_report(root)
    except (OSError, ValueError, KeyError, TypeError) as error:
        return [f"cannot rebuild manual patch metadata report: {error}"]
    if value != expected:
        failures.append("manual patch metadata report is stale or non-deterministic")
    if (
        value.get("private_user_data_used") is not False
        or value.get("network_used") is not False
        or value.get("production_signer_claim") != "none"
        or value.get("releasable_package_claim") != "none"
        or value.get("implemented_verifier_claim") != "none"
        or value.get("implemented_activation_claim") != "none"
        or value.get("release_claim") != "none"
        or value.get("macos_execution_status") != "blocked-macos"
        or value.get("macos_evidence_substituted") is not False
        or value.get("macos_support_claim") != "none"
    ):
        failures.append("manual patch metadata report made an unsupported claim")
    return failures


def check_artifact(root: Path = ROOT) -> list[str]:
    try:
        value = read_json(root / REPORT_PATH.relative_to(ROOT))
    except (OSError, json.JSONDecodeError) as error:
        return [f"cannot read manual patch metadata report: {error}"]
    return validate_report(value, root)


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--write", action="store_true")
    args = parser.parse_args()
    try:
        if args.write:
            write_atomic(REPORT_PATH, canonical_json(build_report()))
        failures = check_artifact()
    except (OSError, ValueError, KeyError, TypeError) as error:
        print(f"Manual patch metadata failed: {error}")
        return 1
    if failures:
        for failure in failures:
            print(f"Manual patch metadata failed: {failure}")
        return 1
    print("Story 3.2 signed manual patch metadata validated")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
