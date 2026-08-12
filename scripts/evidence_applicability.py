#!/usr/bin/env python3
"""Validate immutable evidence and build its separate current-applicability view."""

from __future__ import annotations

import argparse
import json
import sys
from collections import Counter, defaultdict
from pathlib import Path
from typing import Any, Final

try:
    from scripts.evidence_core import (
        EvidenceError,
        atomic_write,
        canonical_json_bytes,
        git_blob,
        git_source_identity,
        read_json_object,
        redacted_diagnostic,
        safe_relative_path,
        sha256_bytes,
        sha256_file,
        valid_sha256,
    )
except ModuleNotFoundError:
    from evidence_core import (  # type: ignore[no-redef]
        EvidenceError,
        atomic_write,
        canonical_json_bytes,
        git_blob,
        git_source_identity,
        read_json_object,
        redacted_diagnostic,
        safe_relative_path,
        sha256_bytes,
        sha256_file,
        valid_sha256,
    )


ROOT: Final = Path(__file__).resolve().parents[1]
CATALOG_PATH: Final = ROOT / "evidence" / "catalog.json"
REPORT_PATH: Final = ROOT / "evidence" / "current" / "applicability-report.json"
RECORD_FIELDS: Final = {
    "evidence_id",
    "artifact",
    "claims",
    "owned_inputs",
    "signatures",
    "dependencies",
    "disposition",
    "applicability_mode",
    "platform_lanes",
    "supersedes",
}
ARTIFACT_FIELDS: Final = {"path", "sha256", "revision", "tree"}
INPUT_FIELDS: Final = {"path", "sha256"}
DISPOSITIONS: Final = {"accepted", "blocked", "rejected"}
MODES: Final = {"evaluate-current", "historical-only", "produced-only"}
LANES: Final = {
    "shared",
    "fedora-x86_64",
    "ubuntu-x86_64",
    "windows-x86_64",
    "macos-arm64-retained",
}
CLAIM_KEYS: Final = {"acceptance_test_id", "requirement_id", "test_id"}


class CatalogError(ValueError):
    """Raised when the current evidence catalog is ambiguous or unsafe."""


def _strings(value: Any, label: str) -> list[str]:
    if not isinstance(value, list) or not all(isinstance(item, str) for item in value):
        raise CatalogError(f"{label}.invalid")
    if len(value) != len(set(value)) or value != sorted(value):
        raise CatalogError(f"{label}.noncanonical")
    return value


def _claim_values(value: Any) -> set[str]:
    claims: set[str] = set()
    if isinstance(value, dict):
        for key, item in value.items():
            if key in CLAIM_KEYS and isinstance(item, str):
                claims.add(item)
            claims.update(_claim_values(item))
    elif isinstance(value, list):
        for item in value:
            claims.update(_claim_values(item))
    return claims


def validate_catalog_shape(catalog: Any) -> list[str]:
    """Validate catalog closure without reading an artifact or Git object."""

    failures: list[str] = []
    if not isinstance(catalog, dict):
        return ["catalog.not_object"]
    if set(catalog) != {"schema_version", "catalog_id", "selected_release", "records"}:
        failures.append("catalog.field_closure")
    if catalog.get("schema_version") != 1 or catalog.get("catalog_id") != "agentmage-evidence-catalog-v1":
        failures.append("catalog.identity")
    if not isinstance(catalog.get("selected_release"), str) or not catalog.get("selected_release"):
        failures.append("catalog.release")
    records = catalog.get("records")
    if not isinstance(records, list):
        return [*failures, "catalog.records"]
    identifiers: list[str] = []
    artifact_keys: list[tuple[str, str]] = []
    for record in records:
        if not isinstance(record, dict) or set(record) != RECORD_FIELDS:
            failures.append("record.field_closure")
            continue
        evidence_id = record.get("evidence_id")
        if not isinstance(evidence_id, str) or not evidence_id:
            failures.append("record.identity")
        else:
            identifiers.append(evidence_id)
        artifact = record.get("artifact")
        if not isinstance(artifact, dict) or set(artifact) != ARTIFACT_FIELDS:
            failures.append(f"{evidence_id}.artifact_closure")
        else:
            if not safe_relative_path(artifact.get("path")):
                failures.append(f"{evidence_id}.artifact_path")
            if not valid_sha256(artifact.get("sha256")):
                failures.append(f"{evidence_id}.artifact_hash")
            if not isinstance(artifact.get("revision"), str) or not isinstance(
                artifact.get("tree"), str
            ):
                failures.append(f"{evidence_id}.artifact_source")
            else:
                artifact_keys.append((artifact["revision"], str(artifact.get("path"))))
        try:
            claims = _strings(record.get("claims"), f"{evidence_id}.claims")
            dependencies = _strings(
                record.get("dependencies"), f"{evidence_id}.dependencies"
            )
            supersedes = _strings(record.get("supersedes"), f"{evidence_id}.supersedes")
            lanes = _strings(record.get("platform_lanes"), f"{evidence_id}.platform_lanes")
        except CatalogError as error:
            failures.append(str(error))
            claims, dependencies, supersedes, lanes = [], [], [], []
        if not claims:
            failures.append(f"{evidence_id}.claims_empty")
        if evidence_id in dependencies or evidence_id in supersedes:
            failures.append(f"{evidence_id}.self_edge")
        if not set(lanes).issubset(LANES) or not lanes:
            failures.append(f"{evidence_id}.lanes")
        if record.get("disposition") not in DISPOSITIONS:
            failures.append(f"{evidence_id}.disposition")
        if record.get("applicability_mode") not in MODES:
            failures.append(f"{evidence_id}.mode")
        owned = record.get("owned_inputs")
        if not isinstance(owned, list) or not owned:
            failures.append(f"{evidence_id}.owned_inputs")
        else:
            paths: list[str] = []
            for item in owned:
                if not isinstance(item, dict) or set(item) != INPUT_FIELDS:
                    failures.append(f"{evidence_id}.owned_input_closure")
                    continue
                path = item.get("path")
                if not safe_relative_path(path) or not valid_sha256(item.get("sha256")):
                    failures.append(f"{evidence_id}.owned_input")
                else:
                    paths.append(path)
            if paths != sorted(set(paths)):
                failures.append(f"{evidence_id}.owned_input_order")
        signatures = record.get("signatures")
        if not isinstance(signatures, list):
            failures.append(f"{evidence_id}.signatures")
        else:
            signature_paths: list[str] = []
            for item in signatures:
                if not isinstance(item, dict) or set(item) != INPUT_FIELDS:
                    failures.append(f"{evidence_id}.signature_closure")
                    continue
                path = item.get("path")
                if not safe_relative_path(path) or not valid_sha256(item.get("sha256")):
                    failures.append(f"{evidence_id}.signature")
                else:
                    signature_paths.append(path)
            if signature_paths != sorted(set(signature_paths)):
                failures.append(f"{evidence_id}.signature_order")
    if identifiers != sorted(set(identifiers)):
        failures.append("catalog.record_order_or_duplicate")
    if len(artifact_keys) != len(set(artifact_keys)):
        failures.append("catalog.duplicate_artifact")
    known = set(identifiers)
    for record in records:
        if isinstance(record, dict):
            for edge in (*record.get("dependencies", []), *record.get("supersedes", [])):
                if edge not in known:
                    failures.append(f"{record.get('evidence_id')}.unknown_edge")
    return sorted(set(failures))


def _cycle_failures(records: list[dict[str, Any]], field: str) -> list[str]:
    graph = {record["evidence_id"]: record[field] for record in records}
    visiting: set[str] = set()
    visited: set[str] = set()

    def visit(node: str) -> bool:
        if node in visiting:
            return True
        if node in visited:
            return False
        visiting.add(node)
        if any(visit(child) for child in graph[node]):
            return True
        visiting.remove(node)
        visited.add(node)
        return False

    return [f"catalog.{field}_cycle"] if any(visit(node) for node in graph) else []


def _historical_result(root: Path, record: dict[str, Any]) -> tuple[str, list[dict[str, str]]]:
    evidence_id = record["evidence_id"]
    artifact = record["artifact"]
    diagnostics: list[dict[str, str]] = []
    try:
        identity = git_source_identity(root, artifact["revision"])
        if identity["tree"] != artifact["tree"]:
            diagnostics.append(redacted_diagnostic("historical.tree_mismatch", subject=evidence_id))
        artifact_bytes = git_blob(root, artifact["revision"], artifact["path"])
        if sha256_bytes(artifact_bytes) != artifact["sha256"]:
            diagnostics.append(redacted_diagnostic("historical.artifact_hash", subject=evidence_id))
        artifact_json = json.loads(artifact_bytes.decode("utf-8"))
        found_claims = _claim_values(artifact_json)
        if not set(record["claims"]).issubset(found_claims):
            diagnostics.append(redacted_diagnostic("historical.claim_missing", subject=evidence_id))
        for owned in record["owned_inputs"]:
            content = git_blob(root, artifact["revision"], owned["path"])
            if sha256_bytes(content) != owned["sha256"]:
                diagnostics.append(redacted_diagnostic("historical.input_hash", subject=evidence_id))
        for signature in record["signatures"]:
            content = git_blob(root, artifact["revision"], signature["path"])
            if sha256_bytes(content) != signature["sha256"]:
                diagnostics.append(
                    redacted_diagnostic("historical.signature_hash", subject=evidence_id)
                )
    except (EvidenceError, UnicodeDecodeError, json.JSONDecodeError):
        diagnostics.append(redacted_diagnostic("historical.source_unavailable", subject=evidence_id))
    return ("valid" if not diagnostics else "invalid", diagnostics)


def evaluate_catalog(catalog: dict[str, Any], root: Path = ROOT) -> dict[str, Any]:
    """Build separate historical-validity and current-applicability states."""

    failures = validate_catalog_shape(catalog)
    if failures:
        raise CatalogError("; ".join(failures))
    records = catalog["records"]
    cycles = _cycle_failures(records, "dependencies") + _cycle_failures(records, "supersedes")
    if cycles:
        raise CatalogError("; ".join(cycles))
    superseded_by: dict[str, list[str]] = defaultdict(list)
    for record in records:
        for old in record["supersedes"]:
            superseded_by[old].append(record["evidence_id"])
    if any(len(values) > 1 for values in superseded_by.values()):
        raise CatalogError("catalog.ambiguous_supersession")

    evaluated: dict[str, dict[str, Any]] = {}
    for record in records:
        evidence_id = record["evidence_id"]
        historical, diagnostics = _historical_result(root, record)
        artifact_current = False
        inputs_current = False
        try:
            artifact_current = sha256_file(root, record["artifact"]["path"]) == record["artifact"]["sha256"]
        except EvidenceError:
            diagnostics.append(redacted_diagnostic("current.artifact_unavailable", subject=evidence_id))
        try:
            inputs_current = all(
                sha256_file(root, item["path"]) == item["sha256"]
                for item in record["owned_inputs"]
            )
        except EvidenceError:
            diagnostics.append(redacted_diagnostic("current.input_unavailable", subject=evidence_id))
        signatures_current = False
        try:
            signatures_current = all(
                sha256_file(root, item["path"]) == item["sha256"]
                for item in record["signatures"]
            )
        except EvidenceError:
            diagnostics.append(
                redacted_diagnostic("current.signature_unavailable", subject=evidence_id)
            )
        if evidence_id in superseded_by:
            state = "superseded"
        elif record["disposition"] == "rejected" or historical != "valid":
            state = "rejected"
        elif record["disposition"] == "blocked":
            state = "blocked"
        elif record["applicability_mode"] == "historical-only":
            state = "historical-only"
        elif record["applicability_mode"] == "produced-only":
            state = "produced"
        elif not artifact_current or not inputs_current or not signatures_current:
            state = "stale"
        else:
            state = "current"
        evaluated[evidence_id] = {
            "evidence_id": evidence_id,
            "artifact": record["artifact"],
            "claims": record["claims"],
            "platform_lanes": record["platform_lanes"],
            "historical_validity": historical,
            "artifact_current": artifact_current,
            "owned_inputs_current": inputs_current,
            "signatures_current": signatures_current,
            "direct_state": state,
            "current_applicability": state,
            "stale_via": [],
            "superseded_by": sorted(superseded_by.get(evidence_id, [])),
            "diagnostics": diagnostics,
        }

    changed = True
    while changed:
        changed = False
        for record in records:
            result = evaluated[record["evidence_id"]]
            if result["current_applicability"] not in {"current", "produced"}:
                continue
            stale_dependencies = sorted(
                dependency
                for dependency in record["dependencies"]
                if evaluated[dependency]["current_applicability"] not in {"current", "produced"}
            )
            if stale_dependencies:
                result["current_applicability"] = "stale"
                result["stale_via"] = stale_dependencies
                changed = True

    ordered = [evaluated[record["evidence_id"]] for record in records]
    counts = Counter(item["current_applicability"] for item in ordered)
    return {
        "schema_version": 1,
        "report_id": "agentmage-current-evidence-applicability-v1",
        "catalog": {
            "path": CATALOG_PATH.relative_to(ROOT).as_posix(),
            "sha256": sha256_bytes(canonical_json_bytes(catalog)),
            "selected_release": catalog["selected_release"],
        },
        "counts": {state: counts[state] for state in sorted(counts)},
        "records": ordered,
    }


def build_report(root: Path = ROOT, catalog_path: Path | None = None) -> dict[str, Any]:
    relative = (catalog_path or CATALOG_PATH).relative_to(root).as_posix()
    return evaluate_catalog(read_json_object(root, relative), root)


def check_report(root: Path = ROOT) -> list[str]:
    try:
        expected = build_report(root)
        actual = read_json_object(root, REPORT_PATH.relative_to(root).as_posix())
    except (EvidenceError, CatalogError, ValueError) as error:
        return [str(error)]
    return [] if actual == expected else ["current.applicability_report_stale"]


def main(argv: list[str] | None = None) -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--write", action="store_true")
    args = parser.parse_args(argv)
    try:
        if args.write:
            atomic_write(REPORT_PATH, canonical_json_bytes(build_report()))
        failures = check_report()
    except (EvidenceError, CatalogError, OSError, ValueError) as error:
        failures = [str(error)]
    if failures:
        for failure in failures:
            print(f"evidence applicability failed: {failure}", file=sys.stderr)
        return 1
    print("historical evidence and current applicability validated")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
