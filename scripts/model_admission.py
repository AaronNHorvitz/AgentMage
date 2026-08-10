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
DEFAULT_ARTIFACT_RECORD: Final = (
    ROOT / "model-profiles" / "candidates" / "gemma-4-e4b" / "artifact-admission.json"
)
POLICY: Final = ROOT / "MODEL-PROVENANCE-POLICY.md"
SHA256: Final = re.compile(r"^[0-9a-f]{64}$")
COMMIT: Final = re.compile(r"^[0-9a-f]{40}$")
ALLOWED_SOURCE_HOSTS: Final = {"ai.google.dev", "huggingface.co"}
OCI_DIGEST: Final = re.compile(r"^sha256:[0-9a-f]{64}$")
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


def validate_artifact_record(record: dict[str, object]) -> list[str]:
    failures: list[str] = []
    required = {
        "schema_version",
        "record_type",
        "profile_id",
        "evaluated_at",
        "source_admission",
        "source_identity",
        "gguf_identity",
        "native_runtime",
        "docker_engine",
        "docker_model",
        "evaluation_host",
        "source_evidence",
        "decision",
    }
    if set(record) != required:
        failures.append("artifact-admission top-level fields do not match the schema")
        return failures
    if record["schema_version"] != 1 or record["record_type"] != "model_artifact_admission":
        failures.append("unsupported artifact-admission schema identity")
    if record["profile_id"] != "gemma-4-e4b-it":
        failures.append("artifact profile identity does not match source admission")

    source = record["source_identity"]
    if not isinstance(source, dict):
        failures.append("source artifact identity must be an object")
    else:
        expected_source = {
            "revision": "ee0ef6023621cff504d758262d4e04895a5af4a2",
            "weights_sha256": "cfbd3d2f1cd71bd471c37fe2bf8546d5028d41e5736f64e1ca6c6b8893125503",
        }
        for field, expected in expected_source.items():
            if source.get(field) != expected:
                failures.append(f"source identity mismatch: {field}")

    gguf = record["gguf_identity"]
    if not isinstance(gguf, dict):
        failures.append("GGUF identity must be an object")
    else:
        expected_gguf = {
            "revision": "bfc15c382204943c3a8fff0c750b94ae2364d7a3",
            "sha256": "85a896a047553e842f25297ee5b031d64ff30147d9c4af17b1e4b394cd1fab87",
            "size": 4977171584,
        }
        for field, expected in expected_gguf.items():
            if gguf.get(field) != expected:
                failures.append(f"GGUF identity mismatch: {field}")
        projector = gguf.get("multimodal_projector")
        if not isinstance(projector, dict) or projector.get("sha256") != (
            "ddf46c21d7078e95338cfc22306b19b276a29a5ad089023449dd54d4b6170a51"
        ):
            failures.append("multimodal projector identity mismatch")
        if gguf.get("conversion_tool") != "not_reproducibly_disclosed":
            failures.append("unknown GGUF conversion provenance must remain visible")
        if gguf.get("downloaded") is not True:
            failures.append("GGUF local verification state is missing")
        if isinstance(projector, dict) and projector.get("downloaded") is not True:
            failures.append("projector local verification state is missing")

    native = record["native_runtime"]
    expected_native = {
        "source_commit": "08659901c43b51de735740f1cf61bb82fbe0c4e4",
        "asset_sha256": "f14e312fbee33ce60d2eed7036de5debe31c1d7f4d8f0e37920eb0a2de0854a5",
        "llama_cli_sha256": "988611a4c80c615052627544c52a6c78fedbba70ab9359132663e043b508bf74",
        "llama_server_sha256": "1d374fdb717832ec01d4829eff9feb46dfc83b7ccbb9d867c15315dbd8aa4bbe",
    }
    if not isinstance(native, dict):
        failures.append("native runtime identity must be an object")
    else:
        for field, expected in expected_native.items():
            if native.get(field) != expected:
                failures.append(f"native runtime identity mismatch: {field}")

    docker_engine = record["docker_engine"]
    docker_model = record["docker_model"]
    expected_digests = {
        "Docker engine": (
            docker_engine,
            "sha256:bd94095bbc1ddc4266c3a88f582a92562c6b63eceb175572c9a60045663727c9",
        ),
        "Docker model": (
            docker_model,
            "sha256:08fa7b1d44f255be48cfc12359211725bfd659742612ed4b221cd5be90d14444",
        ),
    }
    for label, (value, expected) in expected_digests.items():
        if not isinstance(value, dict) or not OCI_DIGEST.fullmatch(str(value.get("digest", ""))):
            failures.append(f"{label} digest is not immutable")
        elif value.get("digest") != expected:
            failures.append(f"{label} digest substitution detected")
    if isinstance(docker_model, dict) and docker_model.get("model_layer_digest") != (
        "sha256:" + str(gguf.get("sha256", ""))
    ):
        failures.append("Docker model layer does not match GGUF identity")
    if isinstance(docker_engine, dict):
        if docker_engine.get("local_state") != "exact_image_staged_and_executed_via_rootless_podman":
            failures.append("Docker Model Runner compatibility execution state is missing")
        if docker_engine.get("compatibility_container_engine") != "Podman 5.8.4":
            failures.append("Docker Model Runner compatibility engine is not disclosed")
        if docker_engine.get("runtime_source_revision") != (
            "72874f559c598b8f89fbb24864868337cf5afb4c"
        ):
            failures.append("Docker Model Runner llama.cpp revision mismatch")
    if isinstance(docker_model, dict) and docker_model.get("local_state") != (
        "staged_hash_verified_and_executed_in_dedicated_model_store"
    ):
        failures.append("Docker model local execution state is missing")

    host = record["evaluation_host"]
    if not isinstance(host, dict) or host.get("docker_available") is not False:
        failures.append("local Docker unavailability is not recorded")
    if isinstance(host, dict) and (
        host.get("podman_available") is not True
        or host.get("podman_version") != "5.8.4"
        or host.get("dmr_compatibility_execution_complete") is not True
    ):
        failures.append("rootless Podman compatibility execution is not recorded")
    if isinstance(host, dict) and "RTX 4090" not in str(host.get("gpu", "")):
        failures.append("evaluation GPU identity is missing")

    decision = record["decision"]
    if not isinstance(decision, dict):
        failures.append("artifact admission decision must be an object")
    else:
        if decision.get("status") != "BLOCKED":
            failures.append("incomplete artifact admission must remain BLOCKED")
        blocker_codes = {
            item.get("code") for item in decision.get("blockers", []) if isinstance(item, dict)
        }
        expected_blockers = {
            "SOURCE-ADMISSION-BLOCKED",
            "GGUF-CONVERSION-NOT-REPRODUCIBLE",
            "DOCKER-ENGINE-NOT-VERIFIED",
        }
        if blocker_codes != expected_blockers:
            failures.append("artifact-admission blockers are incomplete")
        if decision.get("independent_review_performed") is not False:
            failures.append("artifact admission overstates independent review")
        if decision.get("release_approval") is not False:
            failures.append("artifact admission overstates release approval")
        if "AgentMage profile activation" not in decision.get("prohibited_actions", []):
            failures.append("blocked artifact admission does not prohibit activation")
    return failures


def main() -> int:
    try:
        record = load_record()
        artifact_record = load_record(DEFAULT_ARTIFACT_RECORD)
        failures = validate_record(record) + validate_artifact_record(artifact_record)
    except (OSError, json.JSONDecodeError, ValueError) as error:
        print(f"Model source-admission validation failed: {error}", file=sys.stderr)
        return 1
    if failures:
        print("Model source-admission validation failed:", file=sys.stderr)
        for failure in failures:
            print(f"- {failure}", file=sys.stderr)
        return 1
    print("Validated blocked Gemma 4 E4B source and artifact admission records.")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
