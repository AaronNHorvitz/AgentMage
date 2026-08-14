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
    args = parser.parse_args()
    if args.refresh:
        snapshot = refresh_source_snapshot()
    else:
        snapshot = read_json(SOURCE_SNAPSHOT)
    inventory, matrix = normalize(snapshot)
    if args.write:
        write_json(SOURCE_SNAPSHOT, snapshot)
        write_json(INVENTORY, inventory)
        write_json(ROLE_MATRIX, matrix)
    stored_inventory = read_json(INVENTORY) if INVENTORY.exists() else inventory
    stored_matrix = read_json(ROLE_MATRIX) if ROLE_MATRIX.exists() else matrix
    failures = validate(snapshot, stored_inventory, stored_matrix)
    if failures:
        print("Sprint 14 candidate inventory: invalid")
        for failure in failures:
            print(f"- {failure}")
        return 1
    print(
        "Sprint 14 candidate inventory: valid "
        f"({len(snapshot['entries'])} exact source entries; zero acquisition authority)"
    )
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
