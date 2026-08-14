#!/usr/bin/env python3
"""Validate the disabled Muse Glimmer source and runtime admission records."""

from __future__ import annotations

import hashlib
import json
import re
from pathlib import Path
from typing import Final
from urllib.parse import urlparse


ROOT: Final = Path(__file__).resolve().parents[1]
CANDIDATE: Final = ROOT / "model-profiles/candidates/muse-glimmer-30b-text-8k"
SOURCE: Final = CANDIDATE / "source-admission.json"
RUNTIME: Final = CANDIDATE / "runtime-support.json"
POLICY: Final = ROOT / "MODEL-PROVENANCE-POLICY.md"
SHA256: Final = re.compile(r"^[0-9a-f]{64}$")
COMMIT: Final = re.compile(r"^[0-9a-f]{40}$")
ALLOWED_HOSTS: Final = {"research.meta.ai", "huggingface.co", "github.com"}


def load(path: Path) -> dict[str, object]:
    value = json.loads(path.read_text(encoding="utf-8"))
    if not isinstance(value, dict):
        raise ValueError(f"{path.relative_to(ROOT)} must contain one object")
    return value


def _sha256(path: Path) -> str:
    return hashlib.sha256(path.read_bytes()).hexdigest()


def validate_source(record: dict[str, object]) -> list[str]:
    failures: list[str] = []
    expected_keys = {
        "schema_version", "record_type", "profile_id", "evaluated_at", "policy",
        "identity", "ownership_and_origin", "license_and_use_policy", "source_artifacts",
        "codec_contract", "context_contract", "known_limitations", "source_evidence", "decision",
    }
    if set(record) != expected_keys:
        failures.append("source record fields are not closed")
        return failures
    if record["schema_version"] != 1 or record["record_type"] != "model_source_admission":
        failures.append("source record schema identity changed")
    if record["profile_id"] != "muse-glimmer-30b-q4-k-m-text-8k-fedora":
        failures.append("source profile identity changed")

    policy = record["policy"]
    if not isinstance(policy, dict) or policy.get("path") != "MODEL-PROVENANCE-POLICY.md":
        failures.append("source policy binding is absent")
    elif policy.get("sha256") != _sha256(POLICY):
        failures.append("source policy binding is stale")

    identity = record["identity"]
    if not isinstance(identity, dict):
        failures.append("source identity is not an object")
    else:
        expected = {
            "canonical_model": "meta-models/Muse-Glimmer-30B",
            "upstream_revision": "a4e59da52a7bc87ae7251dd5545c0dd437c44b68",
            "gguf_repository": "meta-models/Muse-Glimmer-30B-GGUF",
            "gguf_revision": "43c7eadd41352a299ea8e0a36b3157978dd63596",
            "architecture": "MuseGlimmerForConditionalGeneration",
            "model_type": "muse_glimmer",
        }
        for field, value in expected.items():
            if identity.get(field) != value:
                failures.append(f"source identity changed: {field}")
        for field in ("upstream_revision", "gguf_revision"):
            if not COMMIT.fullmatch(str(identity.get(field, ""))):
                failures.append(f"source identity is not immutable: {field}")

    origin = record["ownership_and_origin"]
    if not isinstance(origin, dict) or origin.get("developer") != "Meta":
        failures.append("first-party developer identity changed")
    elif origin.get("control_disposition") != "permitted_non_chinese_supplier":
        failures.append("supplier control disposition changed")

    terms = record["license_and_use_policy"]
    if not isinstance(terms, dict) or terms.get("spdx") != "Apache-2.0":
        failures.append("Apache-2.0 identity is absent")
    elif not all(SHA256.fullmatch(str(terms.get(field, ""))) for field in ("license_sha256", "usage_policy_sha256")):
        failures.append("license or use policy is not hash-pinned")

    artifacts = record["source_artifacts"]
    if not isinstance(artifacts, dict):
        failures.append("source artifacts are absent")
    else:
        expected_hashes = {
            "chat_template": "cfc67e5f349f37690dfd31ed1f18bc4442a9dd32fe39a648f993cb4eb3cae678",
            "tokenizer": "c9dbee66967b58f31a7c27f723c3760da3526ccd0427578e8905b0abb0031c4d",
            "gguf": "4cc57c0f51040a226e5a72cc47b7613f7772950e460a665f7083de89f183f60e",
        }
        for name, expected_hash in expected_hashes.items():
            artifact = artifacts.get(name)
            if not isinstance(artifact, dict) or artifact.get("sha256") != expected_hash:
                failures.append(f"exact source artifact changed: {name}")
        gguf = artifacts.get("gguf") if isinstance(artifacts.get("gguf"), dict) else {}
        if gguf.get("size") != 16_756_683_904 or gguf.get("downloaded") is not False:
            failures.append("GGUF pre-admission state changed")

    codec = record["codec_contract"]
    if not isinstance(codec, dict) or codec.get("protocol") != "ATEM":
        failures.append("Muse ATEM codec identity is absent")
    elif codec.get("kernel_family_branch_allowed") is not False:
        failures.append("kernel family branching must remain prohibited")

    context = record["context_contract"]
    expected_context = {
        "declared_tokens": 131072,
        "first_evaluation_tokens": 8192,
        "vision_enabled": False,
        "audio_supported": False,
        "speculative_draft_enabled": False,
        "parallel_slots": 1,
    }
    if not isinstance(context, dict) or any(context.get(key) != value for key, value in expected_context.items()):
        failures.append("first evaluation context tuple changed")

    sources = record["source_evidence"]
    if not isinstance(sources, list) or len(sources) != 4:
        failures.append("first-party source set changed")
    else:
        for source in sources:
            if not isinstance(source, dict):
                failures.append("source evidence entry is malformed")
                continue
            parsed = urlparse(str(source.get("url", "")))
            if parsed.scheme != "https" or parsed.hostname not in ALLOWED_HOSTS:
                failures.append("source evidence host is not admitted")

    decision = record["decision"]
    if not isinstance(decision, dict) or decision.get("status") != "BLOCKED":
        failures.append("source candidate must remain BLOCKED")
    else:
        codes = {item.get("code") for item in decision.get("blockers", []) if isinstance(item, dict)}
        required = {
            "ARTIFACT-NOT-LOCALLY-VERIFIED", "MUSE-RUNTIME-PACKAGE-NOT-ADMITTED",
            "MUSE-CODEC-NOT-CONFORMANCE-VERIFIED", "QUALITY-AND-REPEATABILITY-NOT-MEASURED",
        }
        if codes != required:
            failures.append("source blocker set changed")
        if decision.get("release_approval") is not False:
            failures.append("source record overstates release approval")
        prohibited = decision.get("prohibited_actions", [])
        if "automatic fallback" not in prohibited or "AgentMage profile activation" not in prohibited:
            failures.append("source prohibitions are incomplete")
    return failures


def validate_runtime(record: dict[str, object]) -> list[str]:
    failures: list[str] = []
    if set(record) != {
        "schema_version", "record_type", "profile_id", "preserved_runtime",
        "first_upstream_support", "candidate_runtime", "execution_tuple", "decision",
    }:
        return ["runtime record fields are not closed"]
    preserved = record["preserved_runtime"]
    if not isinstance(preserved, dict) or preserved.get("source_commit") != "08659901c43b51de735740f1cf61bb82fbe0c4e4":
        failures.append("preserved b10333 identity changed")
    elif preserved.get("muse_reference_count") != 0 or preserved.get("compatible") is not False:
        failures.append("b10333 compatibility is overstated")
    support = record["first_upstream_support"]
    if not isinstance(support, dict) or support.get("source_commit") != "62bf73d25c53b8161f8a22894d4f90c4aebbd7d0":
        failures.append("first Muse support commit changed")
    candidate = record["candidate_runtime"]
    if not isinstance(candidate, dict):
        failures.append("candidate runtime is absent")
    else:
        exact = {
            "release": "b10423",
            "source_commit": "a94d563ed801d1da1b8c2432946de07d0231bb3d",
            "asset_sha256": "3b1194ef38f4b02b6329d698e29532435a5a7c3567c84b8bb822459ca0893286",
            "asset_size": 32_989_764,
            "package_profile_path": "model-profiles/runtimes/llama-cpp-b10423-muse-linux-x86_64.json",
            "package_profile_sha256": "3255d01ab010cf958da13874402018195beeeaf387c3c2a519dbb5c11fc09e39",
            "package_sha256": "83d08dd70d46d55c9f576368c0a7ad6861e74b9bc93fe510d1ad965bcf43d31c",
            "package_size": 25_449_031,
            "package_status": "LOCALLY_BUILT_HASH_VERIFIED_NOT_ADMITTED",
            "unprivileged_extraction_verified": True,
            "version_self_check_verified": True,
            "adapter_status": "CONTRACT_IMPLEMENTED_NOT_LIVE_PROVEN",
            "enabled": False,
        }
        for field, value in exact.items():
            if candidate.get(field) != value:
                failures.append(f"candidate runtime changed: {field}")
    execution = record["execution_tuple"]
    if not isinstance(execution, dict) or execution.get("context_tokens") != 8192:
        failures.append("runtime execution tuple changed")
    elif any(execution.get(field) is not False for field in ("network_egress", "vision", "speculative_draft")):
        failures.append("runtime isolation tuple widened")
    decision = record["decision"]
    if not isinstance(decision, dict) or decision.get("status") != "BLOCKED":
        failures.append("runtime candidate must remain BLOCKED")
    elif decision.get("activation") is not False or decision.get("fallback") is not False:
        failures.append("runtime candidate gained activation or fallback")
    return failures


def validate() -> list[str]:
    return validate_source(load(SOURCE)) + validate_runtime(load(RUNTIME))


def main() -> int:
    failures = validate()
    if failures:
        print("Muse candidate admission: invalid")
        for failure in failures:
            print(f"- {failure}")
        return 1
    print("Muse candidate admission: BLOCKED (records valid; nothing activated)")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
