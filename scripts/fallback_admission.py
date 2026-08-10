#!/usr/bin/env python3
"""Validate the disabled Gemma 4 12B Unified fallback admission plan."""

from __future__ import annotations

import hashlib
import json
import re
import sys
from pathlib import Path
from typing import Final
from urllib.parse import urlparse


ROOT: Final = Path(__file__).resolve().parents[1]
PROFILE_ROOT: Final = ROOT / "model-profiles" / "candidates" / "gemma-4-12b-unified"
SOURCE_RECORD: Final = PROFILE_ROOT / "source-admission.json"
ARTIFACT_RECORD: Final = PROFILE_ROOT / "artifact-admission.json"
EVALUATION_PLAN: Final = PROFILE_ROOT / "evaluation-plan.json"
POLICY: Final = ROOT / "MODEL-PROVENANCE-POLICY.md"
CORPUS: Final = ROOT / "model-profiles" / "evaluation" / "corpus-v1.json"
TRIGGER: Final = (
    ROOT
    / "model-profiles"
    / "candidates"
    / "gemma-4-e4b"
    / "feasibility-disposition.json"
)
SHA256: Final = re.compile(r"^[0-9a-f]{64}$")
COMMIT: Final = re.compile(r"^[0-9a-f]{40}$")
OCI_DIGEST: Final = re.compile(r"^sha256:[0-9a-f]{64}$")
ALLOWED_SOURCE_HOSTS: Final = {"ai.google.dev", "huggingface.co"}


def read_json(path: Path) -> dict[str, object]:
    value = json.loads(path.read_text(encoding="utf-8"))
    if not isinstance(value, dict):
        raise ValueError(f"expected a JSON object: {path}")
    return value


def sha256_file(path: Path) -> str:
    digest = hashlib.sha256()
    with path.open("rb") as handle:
        for block in iter(lambda: handle.read(1024 * 1024), b""):
            digest.update(block)
    return digest.hexdigest()


def validate_source(record: dict[str, object]) -> list[str]:
    failures: list[str] = []
    expected_fields = {
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
    if set(record) != expected_fields:
        return ["fallback source-admission top-level fields do not match the schema"]
    if record["schema_version"] != 1 or record["record_type"] != "model_source_admission":
        failures.append("unsupported fallback source-admission schema identity")
    if record["profile_id"] != "gemma-4-12b-unified-it":
        failures.append("unexpected fallback profile identity")

    policy = record["policy"]
    if not isinstance(policy, dict) or policy.get("path") != "MODEL-PROVENANCE-POLICY.md":
        failures.append("fallback policy binding is missing")
    elif policy.get("sha256") != sha256_file(POLICY):
        failures.append("fallback policy binding is stale")

    identity = record["identity"]
    expected_identity = {
        "canonical_model": "google/gemma-4-12B-it",
        "upstream_revision": "707f0a3b8a3c7ad586ed01e27eafbad8a27dd0f7",
        "base_model": "google/gemma-4-12B",
        "base_revision": "023679ed352de9bb66cc873c9009ce3482585c08",
        "variant": "instruction-tuned 12B Unified",
        "architecture": "Gemma4UnifiedForConditionalGeneration",
        "model_type": "gemma4_unified",
    }
    if not isinstance(identity, dict) or identity != expected_identity:
        failures.append("fallback source identity changed")
    elif not all(
        COMMIT.fullmatch(str(identity[field]))
        for field in ("upstream_revision", "base_revision")
    ):
        failures.append("fallback source revisions are not immutable")

    origin = record["ownership_and_origin"]
    if not isinstance(origin, dict):
        failures.append("fallback ownership and origin must be an object")
    else:
        if origin.get("developer") != "Google DeepMind" or origin.get("publisher") != "Google":
            failures.append("fallback developer or publisher identity changed")
        if origin.get("control_disposition") != "permitted_non_chinese_supplier":
            failures.append("fallback supplier control disposition is not permitted")
        if origin.get("lineage_complete_under_policy") is not False:
            failures.append("fallback incomplete synthetic lineage must remain visible")

    license_record = record["license"]
    if not isinstance(license_record, dict) or license_record.get("spdx") != "Apache-2.0":
        failures.append("fallback Apache-2.0 source-license disposition is missing")
    elif not SHA256.fullmatch(str(license_record.get("page_sha256_at_evaluation", ""))):
        failures.append("fallback license evidence is not hash-bound")

    expected_artifacts = {
        "model_card": ("04a06469e3d83b941eaa776a3647b8690b5a7cc5c833af2381e5345bb1bb45c0", 28484),
        "config": ("478c46e8d2c52d5c2d85bf67e3b3e8c90e7c9d91086cee27e3c267907e936bd9", 4423),
        "source_weights": ("5a84cb313260ac447237b890387116dfa8682e49a6b44bc585ae8353abbff18d", 23919549408),
        "chat_template": ("ae53464bf3be25802b3a5b37def7fd89667067d7577049b3b2d74c4d8de4c6d4", 18683),
        "generation_config": ("a8349d9bd64cc5841297fcb5002f0fdc4749c473c8f1b10ea337f9ce4ee7014e", 260),
        "processor_config": ("6b938e76555b3e9946890770e1abcd442a4718f34041a58e8139dc8ad34545c9", 1382),
    }
    artifacts = record["source_artifacts"]
    if not isinstance(artifacts, dict) or set(artifacts) != set(expected_artifacts):
        failures.append("fallback source-artifact inventory changed")
    else:
        for name, (expected_hash, expected_size) in expected_artifacts.items():
            value = artifacts.get(name)
            if not isinstance(value, dict) or value.get("sha256") != expected_hash:
                failures.append(f"fallback source artifact hash changed: {name}")
            elif value.get("size") != expected_size:
                failures.append(f"fallback source artifact size changed: {name}")

    tokenizer = record["tokenizer"]
    if not isinstance(tokenizer, dict):
        failures.append("fallback tokenizer contract must be an object")
    else:
        expected_tokenizer = {
            "tokenizer_json_sha256": "cc8d3a0ce36466ccc1278bf987df5f71db1719b9ca6b4118264f45cb627bfe0f",
            "tokenizer_config_sha256": "a62f4e85a47c0c136edaaa3a4f591fd6783717299a9def47e5ad03a49f6a5eb9",
            "vocabulary_size": 262144,
        }
        for field, expected in expected_tokenizer.items():
            if tokenizer.get(field) != expected:
                failures.append(f"fallback tokenizer identity changed: {field}")

    context = record["context_contract"]
    if not isinstance(context, dict):
        failures.append("fallback context contract must be an object")
    elif (
        context.get("declared_tokens") != 262144
        or context.get("sliding_window_tokens") != 1024
        or context.get("operational_limit") is not None
    ):
        failures.append("fallback context contract changed or overstates evaluation")

    if not isinstance(record["known_limitations"], list) or len(record["known_limitations"]) < 8:
        failures.append("fallback known limitations are incomplete")
    sources = record["source_evidence"]
    if not isinstance(sources, list) or len(sources) < 4:
        failures.append("fallback first-party source evidence is incomplete")
    else:
        for source in sources:
            if not isinstance(source, dict):
                failures.append("fallback source evidence entry must be an object")
                continue
            parsed = urlparse(str(source.get("url", "")))
            if parsed.scheme != "https" or parsed.hostname not in ALLOWED_SOURCE_HOSTS:
                failures.append(f"fallback source evidence {source.get('id')} is not first-party")

    decision = record["decision"]
    if not isinstance(decision, dict):
        failures.append("fallback source decision must be an object")
    else:
        if decision.get("status") != "BLOCKED":
            failures.append("incomplete fallback lineage must produce BLOCKED")
        blockers = {
            item.get("code") for item in decision.get("blockers", []) if isinstance(item, dict)
        }
        if blockers != {"LINEAGE-SYNTHETIC-SOURCES-UNVERIFIED"}:
            failures.append("fallback source blockers changed")
        if "AgentMage profile activation" not in decision.get("prohibited_actions", []):
            failures.append("blocked fallback source admission does not prohibit activation")
        if decision.get("independent_review_performed") is not False or decision.get("release_approval") is not False:
            failures.append("fallback source admission overstates review or approval")
    return failures


def validate_artifact(record: dict[str, object]) -> list[str]:
    failures: list[str] = []
    expected_fields = {
        "schema_version",
        "record_type",
        "profile_id",
        "evaluated_at",
        "source_admission",
        "source_identity",
        "official_qat_gguf",
        "selected_gguf_identity",
        "native_runtime",
        "docker_engine",
        "docker_model",
        "evaluation_host",
        "source_evidence",
        "decision",
    }
    if set(record) != expected_fields:
        return ["fallback artifact-admission top-level fields do not match the schema"]
    if record["schema_version"] != 1 or record["record_type"] != "model_artifact_admission":
        failures.append("unsupported fallback artifact-admission schema identity")
    if record["profile_id"] != "gemma-4-12b-unified-it":
        failures.append("fallback artifact profile identity changed")

    source = record["source_identity"]
    expected_source = {
        "repository": "google/gemma-4-12B-it",
        "revision": "707f0a3b8a3c7ad586ed01e27eafbad8a27dd0f7",
        "weights_path": "model.safetensors",
        "weights_sha256": "5a84cb313260ac447237b890387116dfa8682e49a6b44bc585ae8353abbff18d",
        "weights_size": 23919549408,
        "downloaded": False,
    }
    if source != expected_source:
        failures.append("fallback source artifact identity changed")

    official = record["official_qat_gguf"]
    if not isinstance(official, dict) or (
        official.get("revision") != "29d097773436b69ff9feafd636ab4cf873786537"
        or official.get("sha256") != "93567e57a8fe10b23569b9d9ec38cd005deedf71e29477c421a4b83f418a538b"
        or official.get("projector_sha256") != "cb018338a7538a9814d994bfe54644c71eb7ed54e31eae2f721e45fd3c260da7"
        or official.get("selected_for_cross_adapter_evaluation") is not False
    ):
        failures.append("official fallback QAT GGUF distinction changed")

    selected = record["selected_gguf_identity"]
    if not isinstance(selected, dict):
        failures.append("selected fallback GGUF must be an object")
    else:
        if selected.get("revision") != "fc034cfff751157913579611efad8462ac1be606":
            failures.append("selected fallback GGUF revision changed")
        if selected.get("sha256") != "90fd944d227e9d9b68e7e2c7d5b57b79d4c66ed521b0919fbbd932cf834f6f8e":
            failures.append("selected fallback GGUF hash changed")
        projector = selected.get("multimodal_projector")
        if not isinstance(projector, dict) or projector.get("sha256") != "91f086971e56d7a7d8d39e271873fccdb49541bd259d6e02c401a4f1cb7a219e":
            failures.append("selected fallback projector hash changed")
        if selected.get("conversion_tool") != "not_reproducibly_disclosed":
            failures.append("selected fallback conversion uncertainty was removed")
        if selected.get("downloaded") is not False or (
            isinstance(projector, dict) and projector.get("downloaded") is not False
        ):
            failures.append("initial fallback admission overstates local acquisition")

    native = record["native_runtime"]
    if not isinstance(native, dict) or (
        native.get("source_commit") != "08659901c43b51de735740f1cf61bb82fbe0c4e4"
        or native.get("llama_server_sha256") != "1d374fdb717832ec01d4829eff9feb46dfc83b7ccbb9d867c15315dbd8aa4bbe"
        or native.get("local_state") != "staged_and_hash_verified"
    ):
        failures.append("fallback native runtime identity changed")

    docker_engine = record["docker_engine"]
    docker_model = record["docker_model"]
    expected_digests = {
        "fallback DMR image": (
            docker_engine,
            "sha256:bd94095bbc1ddc4266c3a88f582a92562c6b63eceb175572c9a60045663727c9",
        ),
        "fallback Docker model": (
            docker_model,
            "sha256:50b48bb3c9659db6d9615fe9ec91b6d2e6da64f333135987ea12e6965dbd84f6",
        ),
    }
    for label, (value, expected) in expected_digests.items():
        if not isinstance(value, dict) or not OCI_DIGEST.fullmatch(str(value.get("digest", ""))):
            failures.append(f"{label} digest is not immutable")
        elif value.get("digest") != expected:
            failures.append(f"{label} digest substitution detected")
    if isinstance(docker_model, dict) and isinstance(selected, dict):
        if docker_model.get("model_layer_digest") != "sha256:" + str(selected.get("sha256")):
            failures.append("fallback Docker model layer differs from selected GGUF")
        projector = selected.get("multimodal_projector", {})
        if docker_model.get("projector_layer_digest") != "sha256:" + str(projector.get("sha256")):
            failures.append("fallback Docker projector differs from selected GGUF")
        if docker_model.get("local_state") != "remote_immutable_metadata_resolved_not_staged":
            failures.append("initial fallback Docker artifact state changed")

    host = record["evaluation_host"]
    if not isinstance(host, dict) or host.get("docker_available") is not False:
        failures.append("fallback admission overstates Docker availability")
    if isinstance(host, dict) and (
        host.get("podman_available") is not True or host.get("podman_version") != "5.8.4"
    ):
        failures.append("fallback Podman compatibility environment changed")

    decision = record["decision"]
    if not isinstance(decision, dict):
        failures.append("fallback artifact decision must be an object")
    else:
        if decision.get("status") != "BLOCKED":
            failures.append("fallback artifact admission must remain BLOCKED")
        blocker_codes = {
            item.get("code") for item in decision.get("blockers", []) if isinstance(item, dict)
        }
        expected_blockers = {
            "SOURCE-ADMISSION-BLOCKED",
            "SELECTED-GGUF-CONVERSION-NOT-REPRODUCIBLE",
            "FALLBACK-ARTIFACTS-NOT-STAGED",
            "DOCKER-ENGINE-NOT-VERIFIED",
        }
        if blocker_codes != expected_blockers:
            failures.append("fallback artifact blockers changed")
        if "automatic fallback" not in decision.get("prohibited_actions", []):
            failures.append("fallback artifact admission permits automatic fallback")
        if decision.get("independent_review_performed") is not False or decision.get("release_approval") is not False:
            failures.append("fallback artifact admission overstates review or approval")
    return failures


def validate_plan(
    plan: dict[str, object],
    source: dict[str, object],
    artifact: dict[str, object],
) -> list[str]:
    failures: list[str] = []
    expected_fields = {
        "schema_version",
        "record_type",
        "profile_id",
        "created_at",
        "data_classification",
        "candidate_state",
        "trigger_disposition",
        "admission",
        "corpus",
        "adapters",
        "state",
    }
    if set(plan) != expected_fields:
        return ["fallback evaluation-plan top-level fields do not match the schema"]
    if plan["schema_version"] != 1 or plan["record_type"] != "fallback_model_evaluation_plan":
        failures.append("unsupported fallback evaluation-plan schema identity")
    if plan["profile_id"] != "gemma-4-12b-unified-it":
        failures.append("fallback evaluation-plan profile changed")
    if plan["data_classification"] != "public_synthetic_only":
        failures.append("fallback evaluation plan permits non-synthetic data")
    if plan["candidate_state"] != "EVALUATION_REQUIRED_DISABLED":
        failures.append("fallback candidate is not disabled for evaluation")

    trigger = plan["trigger_disposition"]
    trigger_record = read_json(TRIGGER)
    if not isinstance(trigger, dict) or (
        trigger.get("path") != str(TRIGGER.relative_to(ROOT))
        or trigger.get("sha256") != sha256_file(TRIGGER)
        or trigger.get("status") != trigger_record.get("decision", {}).get("status")
        or trigger.get("status") != "REJECTED"
    ):
        failures.append("fallback trigger does not bind the rejected E4B decision")

    admission = plan["admission"]
    if not isinstance(admission, dict) or (
        admission.get("source_path") != str(SOURCE_RECORD.relative_to(ROOT))
        or admission.get("source_sha256") != sha256_file(SOURCE_RECORD)
        or admission.get("source_status") != source.get("decision", {}).get("status")
        or admission.get("artifact_path") != str(ARTIFACT_RECORD.relative_to(ROOT))
        or admission.get("artifact_sha256") != sha256_file(ARTIFACT_RECORD)
        or admission.get("artifact_status") != artifact.get("decision", {}).get("status")
    ):
        failures.append("fallback evaluation plan admission binding changed")

    corpus = plan["corpus"]
    corpus_record = read_json(CORPUS)
    if not isinstance(corpus, dict) or (
        corpus.get("path") != str(CORPUS.relative_to(ROOT))
        or corpus.get("sha256") != sha256_file(CORPUS)
        or corpus.get("version") != corpus_record.get("version")
        or corpus.get("threshold_change_authorized") is not False
    ):
        failures.append("fallback evaluation does not preserve the fixed corpus")

    expected_adapters = [
        "linux-native-vulkan",
        "linux-docker-model-runner-cuda",
        "macos-native-metal",
    ]
    if plan["adapters"] != expected_adapters:
        failures.append("fallback cross-adapter evaluation scope changed")
    state = plan["state"]
    if not isinstance(state, dict) or state != {
        "activation_authorized": False,
        "automatic_switch": False,
        "evaluation_complete": False,
        "independent_review_performed": False,
        "release_approval": False,
    }:
        failures.append("fallback evaluation plan overstates completion or authority")
    return failures


def validate_all() -> list[str]:
    source = read_json(SOURCE_RECORD)
    artifact = read_json(ARTIFACT_RECORD)
    plan = read_json(EVALUATION_PLAN)
    return validate_source(source) + validate_artifact(artifact) + validate_plan(
        plan, source, artifact
    )


def main() -> int:
    try:
        failures = validate_all()
    except (OSError, json.JSONDecodeError, ValueError) as error:
        print(f"Fallback admission validation failed: {error}", file=sys.stderr)
        return 1
    if failures:
        print("Fallback admission validation failed:", file=sys.stderr)
        for failure in failures:
            print(f"- {failure}", file=sys.stderr)
        return 1
    print("Validated disabled Gemma 4 12B Unified admission and evaluation plan.")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
