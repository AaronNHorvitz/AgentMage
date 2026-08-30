#!/usr/bin/env python3
"""Build and verify complete source/range lineage fixtures for Story 2.4.2.1."""

from __future__ import annotations

import argparse
import copy
import json
import sys
from pathlib import Path
from typing import Any, Final


ROOT: Final = Path(__file__).resolve().parents[1]
if str(ROOT) not in sys.path:
    sys.path.insert(0, str(ROOT))

from scripts.engineering_artifact_admission_fixtures import (  # noqa: E402
    ZERO_SHA256,
    canonical_json,
    sealed,
    sha256_bytes,
    write_atomic,
)


CORPUS_DIR: Final = ROOT / "fixtures" / "artifact-admission" / "v1"
ADMISSION_PATH: Final = CORPUS_DIR / "manifest.json"
ADVERSARIAL_PATH: Final = CORPUS_DIR / "adversarial-manifest.json"
LINEAGE_PATH: Final = CORPUS_DIR / "lineage-manifest.json"
FIXED_TIME: Final = "2026-08-29T00:00:00Z"
CONTEXT_MANIFEST_ID: Final = "context-artifact-lineage-fixtures-v1"
CAPTURED_WARNING: Final = "fixture_identity_projection_only_no_product_parser_claim"
NONCAPTURED_WARNING: Final = "source_not_captured_no_derivative_created"
HOSTILE_CATEGORIES: Final = {
    "malformed_package",
    "active_content",
    "external_relationship",
    "archive_traversal",
    "decompression_bomb",
    "mixed_encoding",
    "concurrent_replacement",
    "truncation",
    "cancellation",
    "parser_failure",
}


def read_json(path: Path) -> Any:
    return json.loads(path.read_text(encoding="utf-8"))


def context_disposition(category: str, artifact_id: str, section_id: str | None) -> dict[str, Any]:
    if category == "duplicate":
        disposition, reason_code = "duplicate", "duplicate_content"
    elif category in {"stale", "concurrent_replacement"}:
        disposition, reason_code = "stale", "source_not_current"
    elif category == "unsupported":
        disposition, reason_code = "unsupported", "source_unsupported"
    elif category == "inaccessible":
        disposition, reason_code = "unavailable", "source_unavailable"
    else:
        disposition, reason_code = "omitted", "fixture_only_not_model_delivered"
    return {
        "schema_version": 1,
        "disposition_id": f"disposition-{artifact_id}",
        "context_manifest_id": CONTEXT_MANIFEST_ID,
        "source_artifact_id": artifact_id,
        "section_id": section_id,
        "disposition": disposition,
        "ranges": [],
        "token_count": 0,
        "reason_code": reason_code,
        "reason": "Synthetic lineage fixture; no model delivery or product parser support is claimed.",
        "terminal": True,
    }


def retention_record(case: dict[str, Any]) -> dict[str, Any]:
    source = case["records"]["source_artifact"]
    hostile = case["category"] in HOSTILE_CATEGORIES
    lifecycle = "quarantined" if hostile else "active"
    return sealed(
        {
            "schema_version": 1,
            "retention_id": f"retention-{source['source_artifact_id']}",
            "source_artifact_id": source["source_artifact_id"],
            "request_id": source["request_id"],
            "authority_id": source["authority_id"],
            "owner_class": "request",
            "owner_id": source["request_id"],
            "retention_class": "memory_only",
            "retention_policy_id": None,
            "retention_expires_at": None,
            "physical_store": "runtime_artifact_backend",
            "physical_binding": None,
            "encryption_state": "not_persisted",
            "protected_metadata_sha256": case["protected_descriptor_sha256"],
            "lifecycle_state": lifecycle,
            "reason_code": "hostile_fixture_quarantined" if hostile else None,
            "recorded_at": FIXED_TIME,
            "source_retention_sha256": ZERO_SHA256,
        },
        "source_retention_sha256",
    )


def captured_lineage(case: dict[str, Any]) -> dict[str, Any]:
    source = case["records"]["source_artifact"]
    artifact_id = source["source_artifact_id"]
    identity = case["offered_byte_identity"]
    byte_range = {"start_byte": 0, "end_byte_exclusive": identity["byte_length"]}
    extraction_id = f"extraction-{artifact_id}"
    section_id = f"section-{artifact_id}-root"
    transformation_id = f"transformation-{artifact_id}"
    warning = (
        case.get("expected_result", {}).get("error_code", CAPTURED_WARNING)
        if case["category"] in HOSTILE_CATEGORIES
        else CAPTURED_WARNING
    )
    transformation = {
        "schema_version": 2,
        "transformation_id": transformation_id,
        "artifact_id": artifact_id,
        "transformer_id": "fixture.byte_identity_projector",
        "transformer_version": "1.0.0",
        "input_sha256": identity["sha256"],
        "output_sha256": identity["sha256"],
        "source_ranges": [byte_range],
        "warnings": [warning],
        "reproducible": True,
    }
    extraction = {
        "schema_version": 1,
        "extraction_id": extraction_id,
        "source_artifact_id": artifact_id,
        "extractor_id": "fixture.byte_identity_projector",
        "extractor_version": "1.0.0",
        "source_sha256": identity["sha256"],
        "output_sha256": identity["sha256"],
        "media_type": case["media_type"],
        "disposition": "captured",
        "section_ids": [section_id],
        "warnings": [warning],
        "truncated": False,
        "reproducible": True,
        "terminal": True,
    }
    section = {
        "schema_version": 1,
        "section_id": section_id,
        "source_artifact_id": artifact_id,
        "extraction_id": extraction_id,
        "parent_section_id": None,
        "ordinal": 0,
        "kind": "document_root",
        "byte_range": byte_range,
        "line_range": None,
        "token_count": 0,
        "title": None,
        "content_sha256": identity["sha256"],
    }
    return {
        "parser_link": {
            "state": "fixture_identity_projection",
            "extractor_id": extraction["extractor_id"],
            "extractor_version": extraction["extractor_version"],
            "extraction_id": extraction_id,
            "product_parser_executed": False,
        },
        "transformation_link": {"state": "recorded", "transformation_id": transformation_id},
        "warning_links": [warning],
        "structure_links": [section_id],
        "records": {
            "transformation": transformation,
            "extraction": extraction,
            "sections": [section],
        },
        "ranges": [
            {
                "range_id": f"range-{artifact_id}-root",
                "source_range": byte_range,
                "derivative_range": byte_range,
                "source_sha256": identity["sha256"],
                "derivative_sha256": identity["sha256"],
                "parser_id": extraction["extractor_id"],
                "transformation_id": transformation_id,
                "warning_codes": [warning],
                "section_id": section_id,
            }
        ],
    }


def noncaptured_lineage(case: dict[str, Any]) -> dict[str, Any]:
    reason = case.get("expected_result", {}).get("error_code", NONCAPTURED_WARNING)
    return {
        "parser_link": {
            "state": "not_created",
            "reason_code": reason,
            "product_parser_executed": False,
        },
        "transformation_link": {"state": "not_created", "reason_code": reason},
        "warning_links": [reason],
        "structure_links": [],
        "records": {"transformation": None, "extraction": None, "sections": []},
        "ranges": [],
    }


def lineage_case(case: dict[str, Any]) -> dict[str, Any]:
    source = case["records"]["source_artifact"]
    provenance = case["records"]["source_provenance"]
    captured = source["capture_state"] == "captured"
    detail = captured_lineage(case) if captured else noncaptured_lineage(case)
    section_id = detail["structure_links"][0] if detail["structure_links"] else None
    retention = retention_record(case)
    disposition = context_disposition(case["category"], source["source_artifact_id"], section_id)
    for range_record in detail["ranges"]:
        range_record.update(
            {
                "provenance_id": provenance["provenance_id"],
                "retention_id": retention["retention_id"],
                "disposition_id": disposition["disposition_id"],
            }
        )
    return {
        "fixture_id": case["fixture_id"],
        "category": case["category"],
        "source_artifact_id": source["source_artifact_id"],
        "capture_state": source["capture_state"],
        "source_byte_identity": case["offered_byte_identity"],
        "provenance_link": provenance["provenance_id"],
        "retention_link": retention["retention_id"],
        "disposition_link": disposition["disposition_id"],
        **detail,
        "source_records": {
            "source_provenance": provenance,
            "source_retention": retention,
            "context_disposition": disposition,
        },
        "accounting_terminal": True,
    }


def build_manifest() -> dict[str, Any]:
    admission = read_json(ADMISSION_PATH)
    adversarial = read_json(ADVERSARIAL_PATH)
    cases = admission["cases"] + adversarial["cases"]
    value = {
        "schema_version": 1,
        "manifest_id": "engineering-artifact-lineage-v1",
        "task_id": "2.4.2.1",
        "generated_on": "2026-08-29",
        "status": "synthetic-lineage-contract",
        "synthetic_only": True,
        "product_parser_support_claim": "none",
        "model_delivery_claim": "none",
        "context_manifest_id": CONTEXT_MANIFEST_ID,
        "source_manifests": [
            {"path": str(ADMISSION_PATH.relative_to(ROOT)), "sha256": sha256_bytes(ADMISSION_PATH.read_bytes())},
            {"path": str(ADVERSARIAL_PATH.relative_to(ROOT)), "sha256": sha256_bytes(ADVERSARIAL_PATH.read_bytes())},
        ],
        "generator": {
            "path": "scripts/engineering_artifact_lineage_fixtures.py",
            "sha256": sha256_bytes(Path(__file__).read_bytes()),
        },
        "source_count": len(cases),
        "range_count": sum(case["records"]["source_artifact"]["capture_state"] == "captured" for case in cases),
        "cases": [lineage_case(case) for case in cases],
    }
    value["manifest_sha256"] = sha256_bytes(canonical_json(value))
    return value


def validate_manifest(value: Any) -> list[str]:
    if not isinstance(value, dict):
        return ["artifact lineage manifest must be an object"]
    failures: list[str] = []
    unhashed = copy.deepcopy(value)
    recorded = unhashed.pop("manifest_sha256", None)
    if recorded != sha256_bytes(canonical_json(unhashed)):
        failures.append("artifact lineage manifest self-hash is invalid")
    if value != build_manifest():
        failures.append("artifact lineage manifest is stale, incomplete, reordered, or widened")
    cases = value.get("cases", [])
    if value.get("source_count") != len(cases) or value.get("range_count") != sum(len(case.get("ranges", [])) for case in cases):
        failures.append("artifact lineage source or range accounting is incomplete")
    required_range_links = {
        "parser_id",
        "transformation_id",
        "warning_codes",
        "section_id",
        "provenance_id",
        "retention_id",
        "disposition_id",
    }
    for case in cases if isinstance(cases, list) else []:
        if case.get("accounting_terminal") is not True:
            failures.append(f"artifact lineage is nonterminal: {case.get('fixture_id')}")
        for link in ("parser_link", "transformation_link", "provenance_link", "retention_link", "disposition_link", "warning_links", "structure_links"):
            if link not in case:
                failures.append(f"artifact lineage link is missing: {case.get('fixture_id')}/{link}")
        for range_record in case.get("ranges", []):
            if not required_range_links.issubset(range_record):
                failures.append(f"artifact range lineage is incomplete: {case.get('fixture_id')}")
        if case.get("capture_state") != "captured" and case.get("ranges"):
            failures.append(f"uncaptured source invented derivative ranges: {case.get('fixture_id')}")
    if value.get("synthetic_only") is not True or value.get("product_parser_support_claim") != "none" or value.get("model_delivery_claim") != "none":
        failures.append("artifact lineage manifest makes an implementation or delivery overclaim")
    return failures


def check() -> list[str]:
    try:
        actual = read_json(LINEAGE_PATH)
    except (OSError, json.JSONDecodeError) as error:
        return [f"cannot validate artifact lineage manifest: {error}"]
    return validate_manifest(actual)


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--write", action="store_true")
    args = parser.parse_args()
    if args.write:
        write_atomic(LINEAGE_PATH, canonical_json(build_manifest()))
    failures = check()
    if failures:
        for failure in failures:
            print(f"Artifact lineage validation failed: {failure}", file=sys.stderr)
        return 1
    manifest = read_json(LINEAGE_PATH)
    print(f"Validated lineage for {manifest['source_count']} sources and {manifest['range_count']} ranges")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
