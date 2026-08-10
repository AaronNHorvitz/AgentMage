#!/usr/bin/env python3
"""Validate and exercise the local emergency-disable policy contract."""

from __future__ import annotations

import argparse
import hashlib
import json
import os
import subprocess
import tempfile
from datetime import datetime
from pathlib import Path
from typing import Any


ROOT = Path(__file__).resolve().parents[1]
FIXTURE_PATH = ROOT / "schemas/support/examples/emergency-disable-policy.valid.json"
REPORT_PATH = ROOT / "artifacts/sprints/sprint-3/story-3.2/emergency-disable-policy-report.json"
SUBJECT_KINDS = (
    "model-artifact",
    "runtime",
    "component",
    "capability",
    "release-version",
)
SOURCE_PATHS = (
    "docs/decisions/0007-local-emergency-disablement.md",
    "schemas/support/emergency-disable-policy.schema.json",
    "schemas/support/examples/emergency-disable-policy.valid.json",
    "scripts/validate_emergency_disable_schema.mjs",
    "scripts/emergency_disable_policy.py",
    "tests/test_emergency_disable_schema.mjs",
    "tests/test_emergency_disable_policy.py",
)
TOP_LEVEL_FIELDS = {
    "schema_version",
    "record_type",
    "fixture_status",
    "policy",
    "signing",
    "installation",
    "entries",
    "evaluation",
    "receipt_policy",
    "recovery",
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
        prefix=".agentmage-emergency-disable-", dir=path.parent
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
        ("node", "scripts/validate_emergency_disable_schema.mjs"),
        ("node", "--test", "tests/test_emergency_disable_schema.mjs"),
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
            failures.append(f"emergency-disable schema check failed: {' '.join(command)}")
    return failures


def validate_policy(value: Any) -> list[str]:
    if not isinstance(value, dict):
        return ["emergency-disable policy must be an object"]
    failures = []
    if set(value) != TOP_LEVEL_FIELDS:
        failures.append("emergency-disable policy field closure is invalid")
    if (
        value.get("schema_version") != 1
        or value.get("record_type") != "local-emergency-disable-policy"
        or value.get("fixture_status")
        != "synthetic-contract-fixture-not-activatable"
    ):
        failures.append("emergency-disable policy identity is invalid")
    policy = value.get("policy", {})
    try:
        issued = datetime.fromisoformat(policy["issued_at"].replace("Z", "+00:00"))
        not_before = datetime.fromisoformat(
            policy["not_before"].replace("Z", "+00:00")
        )
        expires = datetime.fromisoformat(policy["expires_at"].replace("Z", "+00:00"))
    except (KeyError, TypeError, ValueError):
        failures.append("emergency-disable policy validity window is malformed")
    else:
        if not issued <= not_before < expires:
            failures.append("emergency-disable policy validity window is invalid")
    signing = value.get("signing", {})
    authorized = signing.get("authorized_signer_key_ids", [])
    signatures = signing.get("detached_signatures", [])
    signer_ids = [item.get("signer_key_id") for item in signatures if isinstance(item, dict)]
    if (
        signing.get("algorithm") != "ed25519"
        or signing.get("verify_before_activation") is not True
        or not isinstance(signing.get("threshold"), int)
        or signing.get("threshold", 0) < 1
        or signing.get("threshold", 0) > len(set(signer_ids))
        or len(authorized) != len(set(authorized))
        or any(signer_id not in authorized for signer_id in signer_ids)
    ):
        failures.append("emergency-disable signing contract is invalid")
    if value.get("installation") != {
        "mode": "user-selected-local-bundle",
        "acquisition_authority": "outside-agentmage-runtime",
        "local_trust_root_required": True,
        "sequence_must_increase": True,
        "atomic_activation_required": True,
        "durable_staging_required": True,
        "prior_valid_policy_retained_until_postcheck": True,
        "automatic_checks": False,
        "background_downloads": False,
        "remote_trigger": False,
        "remote_control": False,
        "network_allowed": False,
        "workstation_data_transmitted": False,
    }:
        failures.append("emergency-disable installation introduced hidden authority")
    entries = value.get("entries", [])
    if (
        tuple(item.get("subject_kind") for item in entries if isinstance(item, dict))
        != SUBJECT_KINDS
        or len({item.get("entry_id") for item in entries if isinstance(item, dict)})
        != len(SUBJECT_KINDS)
        or len(
            {
                (item.get("subject_kind"), item.get("subject_id"))
                for item in entries
                if isinstance(item, dict)
            }
        )
        != len(SUBJECT_KINDS)
        or any(item.get("effect") != "block" for item in entries if isinstance(item, dict))
        or any(
            item.get("advisory_id") != policy.get("advisory_id")
            for item in entries
            if isinstance(item, dict)
        )
    ):
        failures.append("emergency-disable subject closure is invalid")
    if value.get("evaluation") != {
        "checkpoints": [
            "product-startup-admission",
            "model-load",
            "runtime-load",
            "component-load",
            "capability-registration",
            "work-acceptance",
        ],
        "exact_kind_and_id_match_required": True,
        "declared_hash_must_match": True,
        "block_precedence": "before-ordinary-authority-evaluation",
        "no_match_effect": "continue-without-creating-authority",
        "invalid_candidate_effect": "reject-and-preserve-active-policy",
        "unreadable_active_policy_effect": "local-safe-mode-block-model-and-capability-registration",
        "may_create_authority": False,
    }:
        failures.append("emergency-disable evaluation contract is invalid")
    receipt = value.get("receipt_policy", {})
    if (
        tuple(receipt.get("required_fields", []))
        != (
            "policy-id",
            "policy-sequence",
            "policy-payload-sha256",
            "subject-kind",
            "subject-id",
            "subject-sha256-if-declared",
            "matched-entry-id-if-any",
            "outcome",
            "reason-code",
            "advisory-id",
        )
        or tuple(receipt.get("prohibited_fields", []))
        != (
            "prompt",
            "user-file-content",
            "credential",
            "private-key",
            "environment-value",
            "unrelated-path",
        )
        or receipt.get("raw_values_retained") is not False
    ):
        failures.append("emergency-disable receipt minimization is invalid")
    if value.get("recovery") != {
        "newer_signed_policy_required": True,
        "explicit_subject_unblock_required": True,
        "expiration_silently_unblocks": False,
        "deletion_silently_unblocks": False,
        "downgrade_allowed": False,
        "remote_unblock_allowed": False,
    }:
        failures.append("emergency-disable recovery contract is invalid")
    if value.get("claims") != {
        "production_signer": False,
        "production_trust_root": False,
        "implemented_installer": False,
        "implemented_startup_hook": False,
        "product_activation": False,
        "macos_support": False,
    }:
        failures.append("emergency-disable policy made an implementation overclaim")
    return failures


def evaluate_subject(
    policy: dict[str, Any],
    subject_kind: str,
    subject_id: str,
    subject_sha256: str | None,
) -> dict[str, Any]:
    matched = None
    for entry in policy["entries"]:
        if entry["subject_kind"] != subject_kind or entry["subject_id"] != subject_id:
            continue
        if entry["subject_sha256"] is not None and entry["subject_sha256"] != subject_sha256:
            continue
        matched = entry
        break
    identity = policy["policy"]
    return {
        "policy_id": identity["policy_id"],
        "policy_sequence": identity["policy_sequence"],
        "policy_payload_sha256": identity["payload_sha256"],
        "subject_kind": subject_kind,
        "subject_id": subject_id,
        "subject_sha256": subject_sha256,
        "matched_entry_id": matched["entry_id"] if matched else None,
        "outcome": "blocked" if matched else "continue-without-authority",
        "reason_code": matched["reason_code"] if matched else "no-match",
        "advisory_id": matched["advisory_id"] if matched else None,
    }


def validate_inputs(root: Path = ROOT) -> list[str]:
    try:
        fixture = read_json(root / FIXTURE_PATH.relative_to(ROOT))
    except (OSError, json.JSONDecodeError) as error:
        return [f"cannot read emergency-disable fixture: {error}"]
    failures = validate_policy(fixture)
    failures.extend(validate_formal_schema(root))
    for relative in SOURCE_PATHS:
        if not (root / relative).is_file():
            failures.append(f"emergency-disable source is missing: {relative}")
    return failures


def build_report(root: Path = ROOT) -> dict[str, Any]:
    failures = validate_inputs(root)
    if failures:
        raise ValueError("; ".join(failures))
    fixture = read_json(root / FIXTURE_PATH.relative_to(ROOT))
    blocked_results = [
        evaluate_subject(
            fixture,
            entry["subject_kind"],
            entry["subject_id"],
            entry["subject_sha256"],
        )
        for entry in fixture["entries"]
    ]
    mismatch = evaluate_subject(
        fixture,
        "model-artifact",
        fixture["entries"][0]["subject_id"],
        "f" * 64,
    )
    unknown = evaluate_subject(fixture, "component", "unknown-component", None)
    return {
        "schema_version": 1,
        "story_id": "3.2",
        "task_id": "3.2.1.3",
        "status": "pass-contract-definition-product-integration-pending",
        "policy_identity": {
            "fixture_status": fixture["fixture_status"],
            "sha256": sha256_file(root / FIXTURE_PATH.relative_to(ROOT)),
        },
        "subject_results": blocked_results,
        "negative_results": [mismatch, unknown],
        "summary": {
            "subject_class_count": len(SUBJECT_KINDS),
            "blocked_subject_count": sum(
                result["outcome"] == "blocked" for result in blocked_results
            ),
            "hash_mismatch_blocked_count": int(mismatch["outcome"] == "blocked"),
            "unknown_subject_blocked_count": int(unknown["outcome"] == "blocked"),
            "network_authority_count": 0,
            "workstation_data_field_count": 0,
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
        "implemented_installer_claim": "none",
        "implemented_startup_hook_claim": "none",
        "product_activation_claim": "none",
        "release_claim": "none",
        "macos_execution_status": "blocked-macos",
        "macos_evidence_substituted": False,
        "macos_support_claim": "none",
    }


def validate_report(value: Any, root: Path = ROOT) -> list[str]:
    if not isinstance(value, dict):
        return ["emergency-disable report must be an object"]
    failures = []
    try:
        expected = build_report(root)
    except (OSError, ValueError, KeyError, TypeError) as error:
        return [f"cannot rebuild emergency-disable report: {error}"]
    if value != expected:
        failures.append("emergency-disable report is stale or non-deterministic")
    if (
        value.get("summary", {}).get("blocked_subject_count") != 5
        or value.get("summary", {}).get("network_authority_count") != 0
        or value.get("summary", {}).get("workstation_data_field_count") != 0
        or value.get("private_user_data_used") is not False
        or value.get("network_used") is not False
        or value.get("production_signer_claim") != "none"
        or value.get("implemented_installer_claim") != "none"
        or value.get("implemented_startup_hook_claim") != "none"
        or value.get("product_activation_claim") != "none"
        or value.get("release_claim") != "none"
        or value.get("macos_execution_status") != "blocked-macos"
        or value.get("macos_evidence_substituted") is not False
        or value.get("macos_support_claim") != "none"
    ):
        failures.append("emergency-disable report made an unsupported claim")
    return failures


def check_artifact(root: Path = ROOT) -> list[str]:
    try:
        report = read_json(root / REPORT_PATH.relative_to(ROOT))
    except (OSError, json.JSONDecodeError) as error:
        return [f"cannot read emergency-disable report: {error}"]
    return validate_report(report, root)


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--write", action="store_true")
    args = parser.parse_args()
    try:
        if args.write:
            write_atomic(REPORT_PATH, canonical_json(build_report()))
        failures = check_artifact()
    except (OSError, ValueError, KeyError, TypeError) as error:
        print(f"Emergency-disable policy failed: {error}")
        return 1
    if failures:
        for failure in failures:
            print(f"Emergency-disable policy failed: {failure}")
        return 1
    print("Story 3.2 local emergency-disable policy validated")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
