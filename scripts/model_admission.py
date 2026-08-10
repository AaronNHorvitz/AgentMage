#!/usr/bin/env python3
"""Validate the static source-admission record for the Gemma 4 E4B candidate."""

from __future__ import annotations

import hashlib
import json
import re
import sys
from pathlib import Path
from typing import Final
from urllib.parse import urlparse


ROOT: Final = Path(__file__).resolve().parents[1]
DEFAULT_RECORD: Final = (
    ROOT / "model-profiles" / "candidates" / "gemma-4-e4b" / "source-admission.json"
)
POLICY: Final = ROOT / "MODEL-PROVENANCE-POLICY.md"
SHA256: Final = re.compile(r"^[0-9a-f]{64}$")
COMMIT: Final = re.compile(r"^[0-9a-f]{40}$")
ALLOWED_SOURCE_HOSTS: Final = {"ai.google.dev", "huggingface.co"}
EXPECTED_TOKENIZER_HASHES: Final = {
    "tokenizer_json_sha256": "cc8d3a0ce36466ccc1278bf987df5f71db1719b9ca6b4118264f45cb627bfe0f",
    "tokenizer_config_sha256": "9f4fec4b1dc6ecddf8f4a92e9caea5971c0e67d81309f3f9066a2bee8c362633",
}


def load_record(path: Path = DEFAULT_RECORD) -> dict[str, object]:
    value = json.loads(path.read_text(encoding="utf-8"))
    if not isinstance(value, dict):
        raise ValueError("model source-admission record must be an object")
    return value


def validate_record(record: dict[str, object]) -> list[str]:
    failures: list[str] = []
    required = {
        "schema_version",
        "record_type",
        "profile_id",
        "evaluated_at",
        "policy",
        "identity",
        "ownership_and_origin",
        "license",
        "source_artifacts",
        "tokenizer",
        "context_contract",
        "known_limitations",
        "source_evidence",
        "decision",
    }
    missing = sorted(required - set(record))
    extra = sorted(set(record) - required)
    if missing:
        failures.append("missing top-level fields: " + ", ".join(missing))
    if extra:
        failures.append("unknown top-level fields: " + ", ".join(extra))
    if missing:
        return failures

    if record["schema_version"] != 1 or record["record_type"] != "model_source_admission":
        failures.append("unsupported source-admission schema identity")
    if record["profile_id"] != "gemma-4-e4b-it":
        failures.append("unexpected candidate profile identity")

    policy = record["policy"]
    if not isinstance(policy, dict) or policy.get("path") != "MODEL-PROVENANCE-POLICY.md":
        failures.append("model policy binding is missing")
    else:
        expected_policy_hash = hashlib.sha256(POLICY.read_bytes()).hexdigest()
        if policy.get("sha256") != expected_policy_hash:
            failures.append("model policy binding is stale")

    identity = record["identity"]
    if not isinstance(identity, dict):
        failures.append("model identity must be an object")
    else:
        if identity.get("canonical_model") != "google/gemma-4-E4B-it":
            failures.append("canonical model is not the first-party Google profile")
        for field in ("upstream_revision", "base_revision"):
            if not COMMIT.fullmatch(str(identity.get(field, ""))):
                failures.append(f"{field} is not an immutable revision")
        if identity.get("base_model") != "google/gemma-4-E4B":
            failures.append("declared base-model lineage is missing")

    origin = record["ownership_and_origin"]
    if not isinstance(origin, dict):
        failures.append("ownership and origin must be an object")
    else:
        if origin.get("developer") != "Google DeepMind" or origin.get("publisher") != "Google":
            failures.append("developer or publisher identity is unexpected")
        if origin.get("control_disposition") != "permitted_non_chinese_supplier":
            failures.append("supplier control disposition is not permitted")
        if origin.get("lineage_complete_under_policy") is not False:
            failures.append("incomplete synthetic lineage must remain visible")

    license_record = record["license"]
    if not isinstance(license_record, dict) or license_record.get("spdx") != "Apache-2.0":
        failures.append("Apache-2.0 source-license disposition is missing")

    artifacts = record["source_artifacts"]
    if not isinstance(artifacts, dict):
        failures.append("source artifacts must be an object")
    else:
        for name, artifact in artifacts.items():
            if not isinstance(artifact, dict):
                failures.append(f"source artifact {name} must be an object")
                continue
            if not SHA256.fullmatch(str(artifact.get("sha256", ""))):
                failures.append(f"source artifact {name} lacks a SHA-256 identity")
            if not isinstance(artifact.get("size"), int) or artifact["size"] <= 0:
                failures.append(f"source artifact {name} lacks a positive size")

    tokenizer = record["tokenizer"]
    if not isinstance(tokenizer, dict):
        failures.append("tokenizer contract must be an object")
    else:
        for field in ("tokenizer_json_sha256", "tokenizer_config_sha256"):
            if not SHA256.fullmatch(str(tokenizer.get(field, ""))):
                failures.append(f"tokenizer field {field} is not hash-pinned")
            elif tokenizer.get(field) != EXPECTED_TOKENIZER_HASHES[field]:
                failures.append(f"tokenizer field {field} does not match the admitted identity")
        if tokenizer.get("vocabulary_size") != 262144:
            failures.append("tokenizer vocabulary contract changed")

    context = record["context_contract"]
    if not isinstance(context, dict) or context.get("declared_tokens") != 131072:
        failures.append("declared 128K context contract is missing")
    elif context.get("operational_limit") is not None:
        failures.append("operational context cannot be approved before runtime evaluation")

    limitations = record["known_limitations"]
    if not isinstance(limitations, list) or len(limitations) < 6:
        failures.append("known model limitations are incomplete")

    sources = record["source_evidence"]
    if not isinstance(sources, list) or len(sources) < 3:
        failures.append("first-party source evidence is incomplete")
    else:
        for source in sources:
            if not isinstance(source, dict):
                failures.append("source evidence entry must be an object")
                continue
            parsed = urlparse(str(source.get("url", "")))
            if parsed.scheme != "https" or parsed.hostname not in ALLOWED_SOURCE_HOSTS:
                failures.append(f"source evidence {source.get('id')} is not first-party")

    decision = record["decision"]
    if not isinstance(decision, dict):
        failures.append("model decision must be an object")
    else:
        if decision.get("status") != "BLOCKED":
            failures.append("incomplete lineage must produce a BLOCKED decision")
        blocker_codes = {
            item.get("code") for item in decision.get("blockers", []) if isinstance(item, dict)
        }
        if "LINEAGE-SYNTHETIC-SOURCES-UNVERIFIED" not in blocker_codes:
            failures.append("synthetic-training lineage blocker is missing")
        if decision.get("independent_review_performed") is not False:
            failures.append("source admission overstates independent review")
        if decision.get("release_approval") is not False:
            failures.append("source admission overstates release approval")
        if "AgentMage profile activation" not in decision.get("prohibited_actions", []):
            failures.append("blocked source admission does not prohibit activation")
    return failures


def main() -> int:
    try:
        record = load_record()
        failures = validate_record(record)
    except (OSError, json.JSONDecodeError, ValueError) as error:
        print(f"Model source-admission validation failed: {error}", file=sys.stderr)
        return 1
    if failures:
        print("Model source-admission validation failed:", file=sys.stderr)
        for failure in failures:
            print(f"- {failure}", file=sys.stderr)
        return 1
    print("Validated blocked Gemma 4 E4B source-admission record.")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
