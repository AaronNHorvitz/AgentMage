#!/usr/bin/env python3
"""Freeze and validate the first-party Sprint 14 model candidate inventory."""

from __future__ import annotations

import argparse
import hashlib
import json
import re
import urllib.request
from pathlib import Path
from typing import Any, Final


ROOT: Final = Path(__file__).resolve().parents[1]
CATALOG_ROOT: Final = ROOT / "model-profiles/catalogs/2026-08-14"
SOURCE_SNAPSHOT: Final = CATALOG_ROOT / "first-party-source-snapshot.json"
INVENTORY: Final = CATALOG_ROOT / "candidate-inventory.json"
ROLE_MATRIX: Final = CATALOG_ROOT / "candidate-role-suite-matrix.json"
PREFLIGHT_MATRIX: Final = CATALOG_ROOT / "reference-machine-preflight-matrix.json"
FREEZE_DATE: Final = "2026-08-14"
GOOGLE_OWNER: Final = "google"
META_OWNER: Final = "meta-models"
COMMIT: Final = re.compile(r"^[0-9a-f]{40}$")
SHA256: Final = re.compile(r"^[0-9a-f]{64}$")
ALLOWED_DISPOSITIONS: Final = {
    "CANDIDATE",
    "INELIGIBLE",
    "BLOCKED",
    "BLOCKED-HARDWARE",
    "REJECTED",
}
PROTECTED_SPECIALIST_MARKERS: Final = (
    "embeddinggemma",
    "shieldgemma",
    "paligemma",
    "medgemma",
    "txgemma",
    "translategemma",
    "datagemma",
    "gemma-scope",
    "gemma-aps",
    "diffusiongemma",
)

PREFLIGHT_DIMENSIONS: Final = (
    "architecture",
    "runtime",
    "format",
    "acceleration",
    "disk",
    "memory",
    "context",
    "modality",
    "expected_working_set",
)

PREFLIGHT_STATUSES: Final = {
    "CANDIDATE",
    "BLOCKED",
    "BLOCKED-HARDWARE",
}

REFERENCE_MACHINE_ENVELOPES: Final = (
    {
        "envelope_id": "fedora-kinoite-44-x86_64-cuda-llamacpp-native",
        "profile_source": "README.md#platform-support",
        "platform": "Fedora Kinoite 44 x86_64",
        "architecture": "x86_64",
        "acceleration": "cuda",
        "runtime_family": "llama.cpp",
        "adapter_gate": "native-security-reference",
        "supported_runtime_families": ("llama.cpp",),
        "supported_artifact_formats": ("GGUF",),
        "supported_modalities": ("text-generation",),
        "available_disk_bytes": 512 * 1024**3,
        "available_memory_bytes": 128 * 1024**3,
        "available_accelerator_bytes": 24 * 1024**3,
        "maximum_context_tokens": 32768,
    },
    {
        "envelope_id": "fedora-kinoite-44-x86_64-cuda-docker-model-runner",
        "profile_source": "PRD.md#7.2-fedora-and-ubuntu-runtime",
        "platform": "Fedora Kinoite 44 x86_64",
        "architecture": "x86_64",
        "acceleration": "cuda",
        "runtime_family": "docker-model-runner",
        "adapter_gate": "separately-gated-compatibility-adapter",
        "supported_runtime_families": ("docker-model-runner",),
        "supported_artifact_formats": ("GGUF",),
        "supported_modalities": ("text-generation",),
        "available_disk_bytes": 512 * 1024**3,
        "available_memory_bytes": 128 * 1024**3,
        "available_accelerator_bytes": 24 * 1024**3,
        "maximum_context_tokens": 32768,
    },
    {
        "envelope_id": "ubuntu-24-04-x86_64-cpu-llamacpp-native",
        "profile_source": "README.md#platform-support",
        "platform": "Ubuntu 24.04 x86_64",
        "architecture": "x86_64",
        "acceleration": "cpu",
        "runtime_family": "llama.cpp",
        "adapter_gate": "native-security-reference",
        "supported_runtime_families": ("llama.cpp",),
        "supported_artifact_formats": ("GGUF",),
        "supported_modalities": ("text-generation",),
        "available_disk_bytes": 256 * 1024**3,
        "available_memory_bytes": 64 * 1024**3,
        "available_accelerator_bytes": 0,
        "maximum_context_tokens": 8192,
    },
    {
        "envelope_id": "windows-11-x86_64-cuda-llamacpp-native",
        "profile_source": "PRD.md#24-windows-11-first-ga-scope",
        "platform": "Windows 11 x64",
        "architecture": "x86_64",
        "acceleration": "cuda",
        "runtime_family": "llama.cpp",
        "adapter_gate": "native-only-docker-inference-disabled",
        "supported_runtime_families": ("llama.cpp",),
        "supported_artifact_formats": ("GGUF",),
        "supported_modalities": ("text-generation",),
        "available_disk_bytes": 512 * 1024**3,
        "available_memory_bytes": 64 * 1024**3,
        "available_accelerator_bytes": 16 * 1024**3,
        "maximum_context_tokens": 16384,
    },
    {
        "envelope_id": "macbook-pro-m5-arm64-metal-llamacpp-native",
        "profile_source": "README.md#platform-support",
        "platform": "MacBook Pro M5 (Apple Silicon)",
        "architecture": "arm64",
        "acceleration": "metal",
        "runtime_family": "llama.cpp",
        "adapter_gate": "post-ga-blocked-macos-until-hardware-evidence",
        "supported_runtime_families": ("llama.cpp",),
        "supported_artifact_formats": ("GGUF",),
        "supported_modalities": ("text-generation",),
        "available_disk_bytes": 1024 * 1024**3,
        "available_memory_bytes": 48 * 1024**3,
        "available_accelerator_bytes": 48 * 1024**3,
        "maximum_context_tokens": 16384,
    },
)

ARCHITECTURE_NEUTRAL: Final = "architecture-neutral"

EXACT_ARTIFACT_PROFILES: Final = (
    {
        "profile_id": "google/gemma-2-2b-GGUF@df5cd638ad27cf1be1a6266c0618397f82e8787e#2b_pt_v2.gguf",
        "repository": "google/gemma-2-2b-GGUF",
        "revision": "df5cd638ad27cf1be1a6266c0618397f82e8787e",
        "artifact_path": "2b_pt_v2.gguf",
        "architectures": (ARCHITECTURE_NEUTRAL,),
        "runtime_bindings": (
            {"runtime_family": "llama.cpp", "artifact_format": "GGUF"},
        ),
        "supported_accelerations": ("cpu", "cuda", "metal"),
        "artifact_size_bytes": 2600000000,
        "required_disk_bytes": 3000000000,
        "required_memory_bytes": 4000000000,
        "required_accelerator_bytes": 3000000000,
        "expected_working_set_bytes": 6000000000,
        "context_tokens": 4096,
        "modality": "text-generation",
    },
    {
        "profile_id": "google/codegemma-7b-it-GGUF@29ea2a44db5fd40a502119a477664692f2f04d0d#codegemma-7b-it-f16.gguf",
        "repository": "google/codegemma-7b-it-GGUF",
        "revision": "29ea2a44db5fd40a502119a477664692f2f04d0d",
        "artifact_path": "codegemma-7b-it-f16.gguf",
        "architectures": (ARCHITECTURE_NEUTRAL,),
        "runtime_bindings": (
            {"runtime_family": "llama.cpp", "artifact_format": "GGUF"},
        ),
        "supported_accelerations": ("cpu", "cuda", "metal"),
        "artifact_size_bytes": 17100000000,
        "required_disk_bytes": 18500000000,
        "required_memory_bytes": 20000000000,
        "required_accelerator_bytes": 18000000000,
        "expected_working_set_bytes": 24000000000,
        "context_tokens": 16384,
        "modality": "text-generation",
    },
    {
        "profile_id": "google/gemma-2-2b-it@299a8560bedf22ed1c72a8a11e7dce4a7f9f51f8#model-00001-of-00002.safetensors",
        "repository": "google/gemma-2-2b-it",
        "revision": "299a8560bedf22ed1c72a8a11e7dce4a7f9f51f8",
        "artifact_path": "model-00001-of-00002.safetensors",
        "architectures": ("arm64", "x86_64"),
        "runtime_bindings": (
            {"runtime_family": "transformers", "artifact_format": "safetensors"},
        ),
        "supported_accelerations": ("cpu", "cuda"),
        "artifact_size_bytes": 4900000000,
        "required_disk_bytes": 5500000000,
        "required_memory_bytes": 8000000000,
        "required_accelerator_bytes": 6000000000,
        "expected_working_set_bytes": 10000000000,
        "context_tokens": 8192,
        "modality": "text-generation",
    },
    {
        "profile_id": "google/paligemma-3b-mix-448@ead2d9a35598cb89119af004f5d023b311d1c4a1#model-00001-of-00002.safetensors",
        "repository": "google/paligemma-3b-mix-448",
        "revision": "ead2d9a35598cb89119af004f5d023b311d1c4a1",
        "artifact_path": "model-00001-of-00002.safetensors",
        "architectures": ("arm64", "x86_64"),
        "runtime_bindings": (
            {"runtime_family": "transformers", "artifact_format": "safetensors"},
        ),
        "supported_accelerations": ("cuda",),
        "artifact_size_bytes": 5800000000,
        "required_disk_bytes": 6500000000,
        "required_memory_bytes": 12000000000,
        "required_accelerator_bytes": 8000000000,
        "expected_working_set_bytes": 14000000000,
        "context_tokens": 8192,
        "modality": "image-text-to-text",
    },
    {
        "profile_id": "google/embeddinggemma-300m@57c266a740f537b4dc058e1b0cda161fd15afa75#model.safetensors",
        "repository": "google/embeddinggemma-300m",
        "revision": "57c266a740f537b4dc058e1b0cda161fd15afa75",
        "artifact_path": "model.safetensors",
        "architectures": ("arm64", "x86_64"),
        "runtime_bindings": (
            {"runtime_family": "sentence-transformers", "artifact_format": "safetensors"},
        ),
        "supported_accelerations": ("cpu", "cuda", "metal"),
        "artifact_size_bytes": 620000000,
        "required_disk_bytes": 700000000,
        "required_memory_bytes": 1500000000,
        "required_accelerator_bytes": 900000000,
        "expected_working_set_bytes": 2000000000,
        "context_tokens": 2048,
        "modality": "sentence-similarity",
    },
)


def canonical_bytes(value: object) -> bytes:
    return (json.dumps(value, sort_keys=True, separators=(",", ":")) + "\n").encode()


def sha256_bytes(value: bytes) -> str:
    return hashlib.sha256(value).hexdigest()


def read_json(path: Path) -> dict[str, Any]:
    value = json.loads(path.read_text(encoding="utf-8"))
    if not isinstance(value, dict):
        raise ValueError(f"{path.relative_to(ROOT)} must contain one object")
    return value


def fetch_json(url: str) -> Any:
    request = urllib.request.Request(
        url,
        headers={"User-Agent": "AgentMage-catalog-freeze/1"},
    )
    with urllib.request.urlopen(request, timeout=60) as response:
        return json.load(response)


def compact_model(model: dict[str, Any], catalogs: set[str]) -> dict[str, Any]:
    tags = sorted({str(item) for item in model.get("tags", [])})
    siblings = sorted(
        {
            str(item.get("rfilename"))
            for item in model.get("siblings", [])
            if isinstance(item, dict) and item.get("rfilename")
        }
    )
    return {
        "repository": str(model.get("id", "")),
        "revision": str(model.get("sha", "")),
        "private": bool(model.get("private", False)),
        "gated": model.get("gated", False),
        "pipeline_tag": model.get("pipeline_tag"),
        "library_name": model.get("library_name"),
        "tags": tags,
        "artifact_listing": siblings,
        "source_catalogs": sorted(catalogs),
    }


def refresh_source_snapshot() -> dict[str, Any]:
    google_search_url = (
        "https://huggingface.co/api/models?author=google&search=gemma&limit=1000&full=true"
    )
    collection_index_url = "https://huggingface.co/api/collections?owner=google&limit=100"
    meta_url = "https://huggingface.co/api/models?author=meta-models&limit=1000&full=true"
    google_search = fetch_json(google_search_url)
    collection_index = fetch_json(collection_index_url)
    meta_models = fetch_json(meta_url)

    source_payloads: list[dict[str, Any]] = [
        {
            "id": "google-gemma-model-search",
            "url": google_search_url,
            "sha256": sha256_bytes(canonical_bytes(google_search)),
        },
        {
            "id": "google-first-party-collection-index",
            "url": collection_index_url,
            "sha256": sha256_bytes(canonical_bytes(collection_index)),
        },
        {
            "id": "meta-models-first-party-index",
            "url": meta_url,
            "sha256": sha256_bytes(canonical_bytes(meta_models)),
        },
    ]
    records: dict[str, dict[str, Any]] = {}
    memberships: dict[str, set[str]] = {}
    for model in google_search:
        repository = str(model.get("id", ""))
        if repository.startswith("google/"):
            records[repository] = model
            memberships.setdefault(repository, set()).add("google-gemma-model-search")

    selected_collections = []
    for summary in collection_index:
        title = str(summary.get("title", ""))
        slug = str(summary.get("slug", ""))
        if "gemma" not in title.lower() or not slug.startswith("google/"):
            continue
        url = f"https://huggingface.co/api/collections/{slug}"
        collection = fetch_json(url)
        collection_id = f"google-collection:{slug}"
        source_payloads.append(
            {
                "id": collection_id,
                "url": url,
                "sha256": sha256_bytes(canonical_bytes(collection)),
            }
        )
        model_ids = []
        for item in collection.get("items", []):
            if item.get("type") != "model":
                continue
            repository = str(item.get("id", ""))
            if not repository.startswith("google/"):
                continue
            model_ids.append(repository)
            memberships.setdefault(repository, set()).add(collection_id)
            if repository not in records:
                records[repository] = fetch_json(
                    f"https://huggingface.co/api/models/{repository}"
                )
        selected_collections.append(
            {
                "slug": slug,
                "title": title,
                "last_updated": collection.get("lastUpdated"),
                "model_count": len(set(model_ids)),
            }
        )

    for model in meta_models:
        repository = str(model.get("id", ""))
        if repository.startswith("meta-models/Muse-Glimmer-"):
            records[repository] = model
            memberships.setdefault(repository, set()).add("meta-models-first-party-index")

    entries = [
        compact_model(records[repository], memberships.get(repository, set()))
        for repository in sorted(records)
    ]
    snapshot = {
        "schema_version": 1,
        "record_type": "first_party_model_source_snapshot",
        "frozen_on": FREEZE_DATE,
        "eligibility_boundary": {
            "google": (
                "Union of the official Google Gemma model search and every model item in "
                "a Google-owned collection whose title contains Gemma."
            ),
            "meta": (
                "Every repository beginning meta-models/Muse-Glimmer- in the official "
                "meta-models namespace response."
            ),
            "unit": "one repository at one immutable revision",
            "community_derivatives_included": False,
        },
        "sources": sorted(source_payloads, key=lambda item: item["id"]),
        "selected_google_collections": sorted(
            selected_collections, key=lambda item: item["slug"]
        ),
        "entries": entries,
        "counts": {
            "google": sum(item["repository"].startswith("google/") for item in entries),
            "meta": sum(item["repository"].startswith("meta-models/") for item in entries),
            "total": len(entries),
        },
    }
    snapshot["snapshot_sha256"] = sha256_bytes(canonical_bytes(snapshot))
    return snapshot


def license_identity(tags: list[str]) -> str | None:
    licenses = sorted(tag.removeprefix("license:") for tag in tags if tag.startswith("license:"))
    return licenses[0] if len(licenses) == 1 else None


def roles_for(repository: str, pipeline: object) -> tuple[list[str], list[str]]:
    name = repository.split("/", 1)[-1].lower()
    roles: set[str] = set()
    suites: set[str] = {"provenance", "resource", "security"}
    if "embeddinggemma" in name:
        roles.add("embedding")
        suites.update(("retrieval", "contamination", "invalidation"))
    elif "shieldgemma" in name:
        roles.add("safety_classification")
        suites.update(("advisory_classification", "deny_only_authority"))
    elif "functiongemma" in name:
        roles.add("tool_selection")
        suites.update(("structured_proposal", "tool_selection"))
    elif "translategemma" in name:
        roles.add("translation_specialist")
        suites.add("translation")
    elif "medgemma" in name or "txgemma" in name:
        roles.add("domain_specialist")
        suites.add("specialist_advisory")
    elif "gemma-scope" in name:
        roles.add("interpretability_research")
        suites.add("research_artifact")
    elif "gemma-aps" in name or "datagemma" in name:
        roles.add("research_specialist")
        suites.add("specialist_advisory")
    elif "paligemma" in name or "diffusiongemma" in name:
        roles.add("multimodal")
        suites.add("multimodal")
    else:
        roles.add("dialogue")
        suites.update(("repository", "planning", "evidence", "context"))
        if "codegemma" in name or "-it" in name:
            roles.add("coding_planner")
            suites.update(("coding", "tool", "false_completion"))
    if pipeline in {"image-text-to-text", "any-to-any"}:
        roles.add("multimodal")
        suites.add("multimodal")
    return sorted(roles), sorted(suites)


def artifact_formats(paths: list[str]) -> list[str]:
    formats = set()
    for path in paths:
        lower = path.lower()
        if lower.endswith(".gguf"):
            formats.add("GGUF")
        elif lower.endswith(".safetensors"):
            formats.add("safetensors")
        elif lower.endswith(".tflite"):
            formats.add("TFLite")
        elif lower.endswith(".pte"):
            formats.add("ExecuTorch-PTE")
    return sorted(formats)


def intake_disposition(
    *,
    owner: str,
    revision: str,
    private: bool,
    first_party: bool,
    mirrored: bool = False,
    community_converted: bool = False,
) -> tuple[str, list[str]]:
    reasons = []
    if owner not in {GOOGLE_OWNER, META_OWNER} or not first_party:
        reasons.append("not-first-party")
    if mirrored:
        reasons.append("mirror-prohibited")
    if community_converted:
        reasons.append("community-conversion-lab-only")
    if private:
        reasons.append("private-source")
    if not COMMIT.fullmatch(revision):
        reasons.append("mutable-or-missing-revision")
    return ("INELIGIBLE", reasons) if reasons else ("CANDIDATE", [])


def normalize(snapshot: dict[str, Any]) -> tuple[dict[str, Any], dict[str, Any]]:
    normalized = []
    for source in snapshot["entries"]:
        repository = source["repository"]
        owner = repository.split("/", 1)[0]
        roles, suites = roles_for(repository, source.get("pipeline_tag"))
        intake, reasons = intake_disposition(
            owner=owner,
            revision=source["revision"],
            private=source["private"],
            first_party=True,
        )
        model_name = repository.split("/", 1)[-1].lower()
        if owner == GOOGLE_OWNER and "gemma" not in model_name:
            intake = "INELIGIBLE"
            reasons.append("collection-adjacent-non-gemma-model")
        license_name = license_identity(source["tags"])
        has_card = "README.md" in source["artifact_listing"]
        formats = artifact_formats(source["artifact_listing"])
        blockers = list(reasons)
        if license_name is None:
            blockers.append("license-use-terms-unresolved")
        if not has_card:
            blockers.append("model-card-absent")
        if not formats:
            blockers.append("runtime-artifact-format-unresolved")
        blockers.append("exact-artifact-bytes-and-digest-not-selected")
        disposition = intake if intake == "INELIGIBLE" else "BLOCKED"
        prohibited = []
        if any(marker in repository.lower() for marker in PROTECTED_SPECIALIST_MARKERS):
            if "coding_planner" not in roles:
                prohibited.append("coding_planner")
        normalized.append(
            {
                "entry_id": sha256_bytes(f"{repository}@{source['revision']}".encode()),
                "repository": repository,
                "revision": source["revision"],
                "immutable_source_url": (
                    f"https://huggingface.co/{repository}/tree/{source['revision']}"
                ),
                "developer": "Google" if owner == GOOGLE_OWNER else "Meta",
                "publisher": owner,
                "publisher_control": "United States",
                "source_catalogs": source["source_catalogs"],
                "origin": "official-first-party-namespace",
                "lineage": sorted(
                    tag.removeprefix("base_model:")
                    for tag in source["tags"]
                    if tag.startswith("base_model:")
                ),
                "license_use_terms": license_name,
                "model_card_url": (
                    f"https://huggingface.co/{repository}/blob/{source['revision']}/README.md"
                    if has_card
                    else None
                ),
                "artifact_listing_sha256": sha256_bytes(
                    canonical_bytes(source["artifact_listing"])
                ),
                "artifact_formats": formats,
                "transformation": "none-recorded-at-source-entry",
                "tokenizer": "present" if any("tokenizer" in p.lower() for p in source["artifact_listing"]) else "unresolved",
                "template": "present" if any("chat_template" in p.lower() for p in source["artifact_listing"]) else "unresolved",
                "codec": "requires-exact-profile-admission",
                "runtime": "requires-exact-artifact-format-admission",
                "modalities": [source["pipeline_tag"]] if source.get("pipeline_tag") else [],
                "context_tokens": None,
                "roles": roles,
                "prohibited_roles": prohibited,
                "applicable_suites": suites,
                "hardware_preflight": {
                    "status": "BLOCKED",
                    "reason": "exact-artifact-size-working-set-and-runtime-envelope-required",
                    "acquisition_started": False,
                },
                "disposition": disposition,
                "blockers": sorted(set(blockers)),
                "lifecycle": "candidate" if disposition != "INELIGIBLE" else "excluded",
                "enabled": False,
                "automatic_fallback": False,
                "acquisition_allowed": False,
            }
        )
    inventory = {
        "schema_version": 1,
        "record_type": "normalized_candidate_inventory",
        "frozen_on": snapshot["frozen_on"],
        "source_snapshot_sha256": snapshot["snapshot_sha256"],
        "eligibility_policy": {
            "first_party_only": True,
            "immutable_revision_required": True,
            "community_mirrors_and_conversions": "post-GA-experimental-model-lab-only",
            "normalization_unit": "repository-revision candidate; exact artifact remains blocked until separately admitted",
        },
        "entries": normalized,
        "counts": {
            "total": len(normalized),
            "by_disposition": {
                disposition: sum(item["disposition"] == disposition for item in normalized)
                for disposition in sorted(ALLOWED_DISPOSITIONS)
            },
        },
        "product_state": {
            "enabled_models": 0,
            "automatic_fallbacks": 0,
            "acquisitions_authorized": 0,
        },
    }
    inventory["inventory_sha256"] = sha256_bytes(canonical_bytes(inventory))
    matrix = {
        "schema_version": 1,
        "record_type": "candidate_role_suite_matrix",
        "frozen_on": snapshot["frozen_on"],
        "inventory_sha256": inventory["inventory_sha256"],
        "entries": [
            {
                "entry_id": item["entry_id"],
                "repository": item["repository"],
                "roles": item["roles"],
                "prohibited_roles": item["prohibited_roles"],
                "applicable_suites": item["applicable_suites"],
                "hardware_status": item["hardware_preflight"]["status"],
                "disposition": item["disposition"],
            }
            for item in normalized
        ],
    }
    matrix["matrix_sha256"] = sha256_bytes(canonical_bytes(matrix))
    return inventory, matrix


def _serialize_envelope(envelope: dict[str, Any]) -> dict[str, Any]:
    return {
        key: (sorted(value) if isinstance(value, (list, tuple)) else value)
        for key, value in envelope.items()
    }


def _serialize_profile(profile: dict[str, Any]) -> dict[str, Any]:
    result: dict[str, Any] = {}
    for key, value in profile.items():
        if key == "runtime_bindings":
            result[key] = [
                {"artifact_format": b["artifact_format"], "runtime_family": b["runtime_family"]}
                for b in value
            ]
        elif isinstance(value, (list, tuple)):
            result[key] = sorted(value)
        else:
            result[key] = value
    return result


def _profile_bindings(profile: dict[str, Any]) -> list[dict[str, str]]:
    return [
        {"artifact_format": b["artifact_format"], "runtime_family": b["runtime_family"]}
        for b in profile["runtime_bindings"]
    ]


def reference_machine_preflight(
    profile: dict[str, Any], envelope: dict[str, Any]
) -> dict[str, Any]:
    """Preflight one exact artifact profile against one reference-machine envelope.

    All dimensions produce CANDIDATE or BLOCKED-HARDWARE from actual profile
    requirements compared with envelope capacities. Never acquires bytes.
    """

    architectures = list(profile["architectures"])
    envelope_runtimes = tuple(envelope["supported_runtime_families"])
    envelope_formats = tuple(envelope["supported_artifact_formats"])
    envelope_modalities = tuple(envelope["supported_modalities"])
    profile_bindings = _profile_bindings(profile)

    dimensions: dict[str, dict[str, Any]] = {}

    architecture_compatible = (
        ARCHITECTURE_NEUTRAL in architectures
        or envelope["architecture"] in architectures
    )
    dimensions["architecture"] = {
        "status": "CANDIDATE" if architecture_compatible else "BLOCKED-HARDWARE",
        "envelope_architecture": envelope["architecture"],
        "profile_architectures": sorted(set(architectures)),
    }

    matched_binding: dict[str, str] | None = None
    for binding in profile_bindings:
        if (
            binding["runtime_family"] in envelope_runtimes
            and binding["artifact_format"] in envelope_formats
        ):
            matched_binding = binding
            break
    binding_status = "CANDIDATE" if matched_binding is not None else "BLOCKED-HARDWARE"
    dimensions["runtime"] = {
        "status": binding_status,
        "matched_binding": matched_binding,
        "profile_bindings": profile_bindings,
        "envelope_supported_runtime_families": sorted(envelope_runtimes),
    }
    dimensions["format"] = {
        "status": binding_status,
        "matched_binding": matched_binding,
        "profile_bindings": profile_bindings,
        "envelope_supported_formats": sorted(envelope_formats),
    }

    acceleration = envelope["acceleration"]
    supported_accelerations = sorted(set(profile["supported_accelerations"]))
    envelope_accel_bytes = envelope["available_accelerator_bytes"]
    required_accel_bytes = profile["required_accelerator_bytes"]
    if acceleration not in supported_accelerations:
        acceleration_status = "BLOCKED-HARDWARE"
        effective_required_accel = required_accel_bytes
    elif acceleration == "cpu":
        acceleration_status = "CANDIDATE"
        effective_required_accel = 0
    else:
        acceleration_status = (
            "CANDIDATE" if envelope_accel_bytes >= required_accel_bytes else "BLOCKED-HARDWARE"
        )
        effective_required_accel = required_accel_bytes
    dimensions["acceleration"] = {
        "status": acceleration_status,
        "envelope_acceleration": acceleration,
        "envelope_available_accelerator_bytes": envelope_accel_bytes,
        "profile_supported_accelerations": supported_accelerations,
        "profile_required_accelerator_bytes": effective_required_accel,
    }

    dimensions["disk"] = {
        "status": (
            "CANDIDATE"
            if envelope["available_disk_bytes"] >= profile["required_disk_bytes"]
            else "BLOCKED-HARDWARE"
        ),
        "envelope_available_bytes": envelope["available_disk_bytes"],
        "profile_required_bytes": profile["required_disk_bytes"],
    }

    dimensions["memory"] = {
        "status": (
            "CANDIDATE"
            if envelope["available_memory_bytes"] >= profile["required_memory_bytes"]
            else "BLOCKED-HARDWARE"
        ),
        "envelope_available_bytes": envelope["available_memory_bytes"],
        "profile_required_bytes": profile["required_memory_bytes"],
    }

    dimensions["context"] = {
        "status": (
            "CANDIDATE"
            if envelope["maximum_context_tokens"] >= profile["context_tokens"]
            else "BLOCKED-HARDWARE"
        ),
        "envelope_maximum_context_tokens": envelope["maximum_context_tokens"],
        "profile_context_tokens": profile["context_tokens"],
    }

    modality_compatible = profile["modality"] in envelope_modalities
    dimensions["modality"] = {
        "status": "CANDIDATE" if modality_compatible else "BLOCKED-HARDWARE",
        "profile_modality": profile["modality"],
        "envelope_supported_modalities": sorted(envelope_modalities),
    }

    if acceleration == "cpu":
        available_working_set = envelope["available_memory_bytes"]
    else:
        available_working_set = (
            envelope["available_memory_bytes"] + envelope["available_accelerator_bytes"]
        )
    dimensions["expected_working_set"] = {
        "status": (
            "CANDIDATE"
            if available_working_set >= profile["expected_working_set_bytes"]
            else "BLOCKED-HARDWARE"
        ),
        "envelope_available_working_set_bytes": available_working_set,
        "profile_required_working_set_bytes": profile["expected_working_set_bytes"],
    }

    statuses = {dimension["status"] for dimension in dimensions.values()}
    if "BLOCKED-HARDWARE" in statuses:
        overall = "BLOCKED-HARDWARE"
    else:
        overall = "CANDIDATE"

    return {
        "envelope_id": envelope["envelope_id"],
        "profile_id": profile["profile_id"],
        "repository": profile["repository"],
        "revision": profile["revision"],
        "artifact_path": profile["artifact_path"],
        "acquisition_started": False,
        "status": overall,
        "dimensions": dimensions,
    }


def preflight_matrix(
    snapshot: dict[str, Any],
    envelopes: tuple[dict[str, Any], ...] = REFERENCE_MACHINE_ENVELOPES,
    profiles: tuple[dict[str, Any], ...] = EXACT_ARTIFACT_PROFILES,
) -> dict[str, Any]:
    envelope_records = [_serialize_envelope(envelope) for envelope in envelopes]
    profile_records = [_serialize_profile(profile) for profile in profiles]
    entries = []
    status_counts = {status: 0 for status in sorted(PREFLIGHT_STATUSES)}
    for profile in profiles:
        results = [
            reference_machine_preflight(profile, envelope) for envelope in envelopes
        ]
        for result in results:
            status_counts[result["status"]] += 1
        entries.append(
            {
                "profile_id": profile["profile_id"],
                "repository": profile["repository"],
                "revision": profile["revision"],
                "artifact_path": profile["artifact_path"],
                "results": results,
            }
        )
    return {
        "schema_version": 1,
        "record_type": "reference_machine_preflight_matrix",
        "frozen_on": snapshot.get("frozen_on"),
        "source_snapshot_sha256": snapshot.get("snapshot_sha256"),
        "acquisition_authorized": False,
        "envelope_count": len(envelope_records),
        "profile_count": len(profile_records),
        "envelopes": envelope_records,
        "profiles": profile_records,
        "entries": entries,
        "counts": {
            "profiles": len(entries),
            "envelopes": len(envelope_records),
            "results": len(entries) * len(envelope_records),
            "by_status": status_counts,
        },
    }


def _canonicalize(value: Any) -> Any:
    if isinstance(value, dict):
        return {key: _canonicalize(value[key]) for key in sorted(value)}
    if isinstance(value, (list, tuple)):
        return [_canonicalize(item) for item in value]
    return value


def validate_preflight_matrix(
    snapshot: dict[str, Any],
    matrix: dict[str, Any],
    envelopes: tuple[dict[str, Any], ...] = REFERENCE_MACHINE_ENVELOPES,
    profiles: tuple[dict[str, Any], ...] = EXACT_ARTIFACT_PROFILES,
) -> list[str]:
    """Regenerate the deterministic matrix from bound inputs and deep-compare."""

    failures: list[str] = []
    expected = preflight_matrix(snapshot, envelopes=envelopes, profiles=profiles)

    expected_canonical = _canonicalize(expected)
    actual_canonical = _canonicalize(matrix)

    for key in sorted(expected_canonical):
        if key == "entries":
            continue
        if actual_canonical.get(key) != expected_canonical.get(key):
            failures.append(f"preflight matrix {key} drift")

    expected_entries = expected_canonical.get("entries", [])
    actual_entries = actual_canonical.get("entries", [])
    if len(actual_entries) != len(expected_entries):
        failures.append("preflight matrix entry count drift")
    else:
        for expected_entry, actual_entry in zip(expected_entries, actual_entries):
            profile_id = expected_entry.get("profile_id")
            if actual_entry.get("profile_id") != profile_id:
                failures.append(f"preflight matrix profile_id drift: {profile_id}")
                continue
            for scalar_key in ("repository", "revision", "artifact_path"):
                if actual_entry.get(scalar_key) != expected_entry.get(scalar_key):
                    failures.append(
                        f"preflight matrix entry {scalar_key} drift: {profile_id}"
                    )
            expected_results = expected_entry.get("results", [])
            actual_results = actual_entry.get("results", [])
            if len(actual_results) != len(expected_results):
                failures.append(f"preflight matrix envelope count drift: {profile_id}")
                continue
            for expected_result, actual_result in zip(expected_results, actual_results):
                envelope_id = expected_result.get("envelope_id")
                if actual_result != expected_result:
                    if actual_result.get("envelope_id") != envelope_id:
                        failures.append(
                            f"preflight matrix envelope_id drift: {profile_id}"
                        )
                    if actual_result.get("status") != expected_result.get("status"):
                        failures.append(
                            f"preflight matrix overall status drift: {profile_id}/{envelope_id}"
                        )
                    expected_dims = expected_result.get("dimensions", {})
                    actual_dims = actual_result.get("dimensions", {})
                    if set(actual_dims) != set(expected_dims):
                        failures.append(
                            f"preflight matrix dimension set drift: {profile_id}/{envelope_id}"
                        )
                    for dim_name in expected_dims:
                        if actual_dims.get(dim_name) != expected_dims.get(dim_name):
                            failures.append(
                                f"preflight matrix dimension drift: {profile_id}/{envelope_id}/{dim_name}"
                            )
                    for scalar_key in (
                        "repository",
                        "revision",
                        "artifact_path",
                        "profile_id",
                        "acquisition_started",
                    ):
                        if actual_result.get(scalar_key) != expected_result.get(scalar_key):
                            failures.append(
                                f"preflight matrix result {scalar_key} drift: {profile_id}/{envelope_id}"
                            )

    if actual_canonical != expected_canonical and not failures:
        failures.append("preflight matrix deep-compare mismatch")

    return failures


def hardware_fit(
    *,
    required_disk: int,
    required_memory: int,
    required_accelerator: int,
    available_disk: int,
    available_memory: int,
    available_accelerator: int,
) -> str:
    values = (
        required_disk,
        required_memory,
        required_accelerator,
        available_disk,
        available_memory,
        available_accelerator,
    )
    if any(value < 0 for value in values):
        return "BLOCKED"
    if (
        available_disk < required_disk
        or available_memory < required_memory
        or available_accelerator < required_accelerator
    ):
        return "BLOCKED-HARDWARE"
    return "CANDIDATE"


def validate(
    snapshot: dict[str, Any], inventory: dict[str, Any], matrix: dict[str, Any]
) -> list[str]:
    failures = []
    source_entries = snapshot.get("entries", [])
    inventory_entries = inventory.get("entries", [])
    matrix_entries = matrix.get("entries", [])
    source_ids = [f"{item.get('repository')}@{item.get('revision')}" for item in source_entries]
    inventory_ids = [item.get("entry_id") for item in inventory_entries]
    matrix_ids = [item.get("entry_id") for item in matrix_entries]
    expected_entry_ids = [sha256_bytes(item.encode()) for item in source_ids]
    if snapshot.get("frozen_on") != FREEZE_DATE:
        failures.append("source freeze date changed")
    snapshot_copy = dict(snapshot)
    observed_snapshot_sha = snapshot_copy.pop("snapshot_sha256", None)
    if observed_snapshot_sha != sha256_bytes(canonical_bytes(snapshot_copy)):
        failures.append("source snapshot digest mismatch")
    if len(source_ids) != len(set(source_ids)):
        failures.append("source snapshot contains duplicate identity")
    if inventory_ids != expected_entry_ids:
        failures.append("inventory does not exactly reconcile to source order")
    if len(inventory_ids) != len(set(inventory_ids)):
        failures.append("inventory contains duplicate identity")
    if matrix_ids != inventory_ids:
        failures.append("role matrix does not exactly reconcile to inventory")
    if inventory.get("source_snapshot_sha256") != snapshot.get("snapshot_sha256"):
        failures.append("inventory source binding changed")
    inventory_copy = dict(inventory)
    observed_inventory_sha = inventory_copy.pop("inventory_sha256", None)
    if observed_inventory_sha != sha256_bytes(canonical_bytes(inventory_copy)):
        failures.append("inventory digest mismatch")
    if matrix.get("inventory_sha256") != inventory.get("inventory_sha256"):
        failures.append("role matrix inventory binding changed")
    matrix_copy = dict(matrix)
    observed_matrix_sha = matrix_copy.pop("matrix_sha256", None)
    if observed_matrix_sha != sha256_bytes(canonical_bytes(matrix_copy)):
        failures.append("role matrix digest mismatch")
    for item in inventory_entries:
        repository = str(item.get("repository", ""))
        if item.get("disposition") not in ALLOWED_DISPOSITIONS:
            failures.append(f"invalid disposition: {repository}")
        if item.get("enabled") is not False or item.get("automatic_fallback") is not False:
            failures.append(f"candidate gained product selection: {repository}")
        if item.get("acquisition_allowed") is not False:
            failures.append(f"candidate gained acquisition authority: {repository}")
        preflight = item.get("hardware_preflight", {})
        if preflight.get("acquisition_started") is not False:
            failures.append(f"hardware preflight acquired bytes: {repository}")
        if "coding_planner" in item.get("prohibited_roles", []) and "coding_planner" in item.get("roles", []):
            failures.append(f"specialist role escalation: {repository}")
        if not COMMIT.fullmatch(str(item.get("revision", ""))):
            failures.append(f"mutable inventory identity: {repository}")
    product = inventory.get("product_state", {})
    if product != {
        "enabled_models": 0,
        "automatic_fallbacks": 0,
        "acquisitions_authorized": 0,
    }:
        failures.append("inventory changed zero-authority product state")
    return failures


def write_json(path: Path, value: object) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_text(json.dumps(value, indent=2, sort_keys=True) + "\n", encoding="utf-8")


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--refresh", action="store_true")
    parser.add_argument("--write", action="store_true")
    parser.add_argument("--preflight", action="store_true")
    args = parser.parse_args()
    if args.refresh:
        snapshot = refresh_source_snapshot()
    else:
        snapshot = read_json(SOURCE_SNAPSHOT)
    inventory, matrix = normalize(snapshot)
    preflight = preflight_matrix(snapshot) if args.preflight else None
    if args.write:
        write_json(SOURCE_SNAPSHOT, snapshot)
        write_json(INVENTORY, inventory)
        write_json(ROLE_MATRIX, matrix)
        if preflight is not None:
            write_json(PREFLIGHT_MATRIX, preflight)
    stored_inventory = read_json(INVENTORY) if INVENTORY.exists() else inventory
    stored_matrix = read_json(ROLE_MATRIX) if ROLE_MATRIX.exists() else matrix
    failures = validate(snapshot, stored_inventory, stored_matrix)
    if args.preflight:
        if not PREFLIGHT_MATRIX.exists():
            failures.append(
                "preflight matrix missing: model-profiles/catalogs/2026-08-14/"
                "reference-machine-preflight-matrix.json is required"
            )
        else:
            stored_preflight = read_json(PREFLIGHT_MATRIX)
            failures.extend(validate_preflight_matrix(snapshot, stored_preflight))
    if failures:
        print("Sprint 14 candidate inventory: invalid")
        for failure in failures:
            print(f"- {failure}")
        return 1
    print(
        "Sprint 14 candidate inventory: valid "
        f"({len(snapshot['entries'])} exact source entries; zero acquisition authority)"
    )
    if args.preflight and preflight is not None:
        counts = preflight["counts"]
        by_status = counts["by_status"]
        print(
            "Sprint 14 reference-machine preflight: "
            f"{counts['profiles']} exact artifact profiles against "
            f"{counts['envelopes']} envelopes; "
            f"CANDIDATE={by_status['CANDIDATE']}, "
            f"BLOCKED={by_status['BLOCKED']}, "
            f"BLOCKED-HARDWARE={by_status['BLOCKED-HARDWARE']} "
            "(zero acquisition authority)"
        )
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
