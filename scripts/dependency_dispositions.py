#!/usr/bin/env python3
"""Validate the closed Task 1.2.4.1 artifact dependency disposition record."""

from __future__ import annotations

import json
import re
import sys
from pathlib import Path
from typing import Any, Final


ROOT: Final = Path(__file__).resolve().parents[1]
RECORD_PATH: Final = ROOT / "architecture" / "dependency-dispositions.json"
FAMILIES: Final = ["parser", "mime-detection", "tokenization", "archive", "ocr", "database"]
DISPOSITIONS: Final = ["accepted", "deferred", "rejected"]
PLATFORMS: Final = [
    "fedora-x86_64",
    "ubuntu-x86_64",
    "windows-11-x86_64",
    "macos-apple-silicon",
]
EXPECTED_IDS: Final = [
    "archive-general-formats",
    "archive-zip-8.6.0",
    "database-rusqlite-0.40.2",
    "database-tantivy-0.26.1",
    "mime-file-format-0.29.0",
    "mime-in-tree-bounded-probes-v1",
    "ocr-cloud-service",
    "ocr-tesseract-5.5.3",
    "parser-calamine-0.36.1",
    "parser-lopdf-0.44.0",
    "parser-quick-xml-0.41.0",
    "parser-tree-sitter-locked-family",
    "tokenization-huggingface-0.23.1",
    "tokenization-profile-owned-exact-counter",
]
ACCEPTED_COMPONENTS: Final = {
    "archive-zip-8.6.0": {"zip"},
    "database-rusqlite-0.40.2": {"rusqlite"},
    "parser-lopdf-0.44.0": {"lopdf"},
    "parser-quick-xml-0.41.0": {"quick-xml"},
    "parser-tree-sitter-locked-family": {
        "tree-sitter",
        "tree-sitter-javascript",
        "tree-sitter-python",
        "tree-sitter-rust",
        "tree-sitter-swift",
        "tree-sitter-typescript",
    },
}
INTERNAL_ACCEPTED: Final = {
    "mime-in-tree-bounded-probes-v1",
    "tokenization-profile-owned-exact-counter",
}
ROOT_KEYS: Final = {
    "schema_version",
    "decision_id",
    "task_id",
    "status",
    "reviewed_on",
    "effective_until",
    "authorities",
    "families",
    "dispositions",
    "platform_truth",
    "candidates",
    "product_truth",
}
CANDIDATE_KEYS: Final = {
    "id",
    "family",
    "disposition",
    "purpose",
    "components",
    "license",
    "provenance",
    "maintenance",
    "unsafe_code",
    "platforms",
    "resource_controls",
    "packaging",
    "cancellation",
    "absence_behavior",
    "rationale",
    "re_review_triggers",
}
COMPONENT_KEYS: Final = {"name", "version", "checksum_sha256"}
PROVENANCE_KEYS: Final = {
    "source_url",
    "source_sha256",
    "retrieved_on",
    "release_date",
    "authority",
}
PRODUCT_TRUTH: Final = {
    "adds_dependency": False,
    "changes_lockfile": False,
    "enables_capability": False,
    "admits_ocr": False,
    "enables_model": False,
    "claims_native_evidence": False,
    "claims_platform_support": False,
    "claims_release_readiness": False,
}
SHA256: Final = re.compile(r"[0-9a-f]{64}")


def load_record(path: Path = RECORD_PATH) -> dict[str, Any]:
    return json.loads(path.read_text(encoding="utf-8"))


def _closed(value: Any, keys: set[str], label: str, failures: list[str]) -> bool:
    if not isinstance(value, dict):
        failures.append(f"{label} must be an object")
        return False
    unknown = set(value) - keys
    missing = keys - set(value)
    if unknown or missing:
        failures.append(
            f"{label} fields are not closed: missing={sorted(missing)} extra={sorted(unknown)}"
        )
        return False
    return True


def _provenance(root: Path) -> dict[str, dict[str, Any]]:
    record = json.loads(
        (root / "supply-chain" / "dependency-provenance.json").read_text(encoding="utf-8")
    )
    return {
        item["name"]: item
        for item in record.get("components", [])
        if item.get("ecosystem") == "cargo"
    }


def validate_record(record: Any, root: Path = ROOT) -> list[str]:
    failures: list[str] = []
    if not _closed(record, ROOT_KEYS, "record", failures):
        return failures
    expected_header = {
        "schema_version": 1,
        "decision_id": "ADR-0042",
        "task_id": "1.2.4.1",
        "status": "review-complete-no-admission-expansion",
        "reviewed_on": "2026-08-29",
        "effective_until": "identity-or-evidence-change",
    }
    for key, expected in expected_header.items():
        if record.get(key) != expected:
            failures.append(f"{key} must equal {expected!r}")
    if record.get("families") != FAMILIES:
        failures.append("families must contain the six Task 1.2.4.1 families in order")
    if record.get("dispositions") != DISPOSITIONS:
        failures.append("dispositions must contain accepted, deferred, and rejected in order")
    if record.get("product_truth") != PRODUCT_TRUTH:
        failures.append("product truth was widened")
    if record.get("platform_truth") != {
        "first_ga_targets": PLATFORMS[:3],
        "release_qualified_targets": [],
        "macos": "blocked-post-ga",
    }:
        failures.append("platform truth was widened or reordered")
    if not isinstance(record.get("authorities"), list) or not record["authorities"]:
        failures.append("authorities must be a non-empty array")

    candidates = record.get("candidates")
    if not isinstance(candidates, list):
        return [*failures, "candidates must be an array"]
    ids = [item.get("id") for item in candidates if isinstance(item, dict)]
    if ids != EXPECTED_IDS:
        failures.append("candidate inventory must be complete, unique, and sorted")
    try:
        provenance = _provenance(root)
    except (OSError, json.JSONDecodeError, KeyError) as error:
        failures.append(f"cannot load dependency provenance: {error}")
        provenance = {}

    observed_families: set[str] = set()
    observed_dispositions: set[str] = set()
    for index, candidate in enumerate(candidates):
        label = f"candidate[{index}]"
        if not _closed(candidate, CANDIDATE_KEYS, label, failures):
            continue
        candidate_id = candidate["id"]
        family = candidate["family"]
        disposition = candidate["disposition"]
        observed_families.add(family)
        observed_dispositions.add(disposition)
        if family not in FAMILIES:
            failures.append(f"{candidate_id}: unknown family")
        if disposition not in DISPOSITIONS:
            failures.append(f"{candidate_id}: unknown disposition")
        for field in (
            "purpose",
            "license",
            "maintenance",
            "unsafe_code",
            "packaging",
            "cancellation",
            "absence_behavior",
            "rationale",
        ):
            if not isinstance(candidate[field], str) or not candidate[field].strip():
                failures.append(f"{candidate_id}: {field} must be non-empty")
        source = candidate["provenance"]
        if not _closed(source, PROVENANCE_KEYS, f"{candidate_id}.provenance", failures):
            continue
        source_sha = source.get("source_sha256")
        if not isinstance(source_sha, str) or not SHA256.fullmatch(source_sha):
            failures.append(f"{candidate_id}: source_sha256 must be lowercase SHA-256")
        if source.get("retrieved_on") != "2026-08-29":
            failures.append(f"{candidate_id}: retrieval date drifted")
        if not isinstance(candidate["resource_controls"], list) or not candidate["resource_controls"]:
            failures.append(f"{candidate_id}: resource controls must be non-empty")
        if not isinstance(candidate["re_review_triggers"], list) or not candidate["re_review_triggers"]:
            failures.append(f"{candidate_id}: re-review triggers must be non-empty")
        platforms = candidate["platforms"]
        if not isinstance(platforms, dict) or list(platforms) != PLATFORMS:
            failures.append(f"{candidate_id}: platform matrix must be complete and ordered")
        else:
            allowed = {"architecture-only", "native-evidence-pending", "blocked-post-ga"}
            if any(state not in allowed for state in platforms.values()):
                failures.append(f"{candidate_id}: platform matrix makes an unsupported claim")
            if platforms["macos-apple-silicon"] != "blocked-post-ga":
                failures.append(f"{candidate_id}: macOS must remain blocked-post-ga")

        components = candidate["components"]
        if not isinstance(components, list):
            failures.append(f"{candidate_id}: components must be an array")
            components = []
        names: set[str] = set()
        for component_index, component in enumerate(components):
            component_label = f"{candidate_id}.components[{component_index}]"
            if not _closed(component, COMPONENT_KEYS, component_label, failures):
                continue
            names.add(component["name"])
            checksum = component["checksum_sha256"]
            if not isinstance(checksum, str) or not SHA256.fullmatch(checksum):
                failures.append(f"{component_label}: checksum must be lowercase SHA-256")

        if disposition == "accepted":
            expected_names = ACCEPTED_COMPONENTS.get(candidate_id)
            if candidate_id not in set(ACCEPTED_COMPONENTS) | INTERNAL_ACCEPTED:
                failures.append(f"{candidate_id}: candidate cannot be promoted by status mutation")
            if expected_names is not None and names != expected_names:
                failures.append(f"{candidate_id}: accepted component closure drifted")
            if candidate_id in INTERNAL_ACCEPTED and components:
                failures.append(f"{candidate_id}: internal boundary must not declare a package")
            for component in components:
                actual = provenance.get(component["name"])
                if actual is None:
                    failures.append(f"{candidate_id}: accepted component missing from provenance")
                    continue
                actual_checksum = (actual.get("integrity") or "").removeprefix("sha256:")
                if component["version"] != actual.get("version"):
                    failures.append(f"{candidate_id}: accepted version differs from provenance")
                if component["checksum_sha256"] != actual_checksum:
                    failures.append(f"{candidate_id}: accepted checksum differs from provenance")
        elif components:
            failures.append(f"{candidate_id}: deferred/rejected package must remain unadmitted")

        controls = set(candidate["resource_controls"])
        required = {"input-bytes", "memory", "cpu-time", "wall-time", "cancellation", "cleanup"}
        if not required <= controls:
            failures.append(f"{candidate_id}: universal resource controls are incomplete")
        if family == "archive" and not {
            "entry-count",
            "expanded-bytes",
            "ratio",
            "recursion-depth",
            "path-safety",
        } <= controls:
            failures.append(f"{candidate_id}: archive controls are incomplete")
        if family == "ocr" and not {
            "model-provenance",
            "confidence",
            "language",
            "process-isolation",
            "residue",
        } <= controls:
            failures.append(f"{candidate_id}: OCR controls are incomplete")
        if family == "tokenization" and "exact-profile-identity" not in controls:
            failures.append(f"{candidate_id}: tokenizer must bind an exact profile")
        if family == "mime-detection" and "extension-advisory-only" not in controls:
            failures.append(f"{candidate_id}: extensions must remain advisory")
        if family == "database" and "single-sqlite-authority" not in controls:
            failures.append(f"{candidate_id}: database authority must remain singular")

    if observed_families != set(FAMILIES):
        failures.append("candidate inventory does not cover all families")
    if observed_dispositions != set(DISPOSITIONS):
        failures.append("candidate inventory does not retain all dispositions")
    return failures


def main() -> int:
    try:
        record = load_record()
    except (OSError, json.JSONDecodeError) as error:
        print(f"dependency disposition validation failed: {error}", file=sys.stderr)
        return 1
    failures = validate_record(record)
    if failures:
        for failure in failures:
            print(f"dependency disposition validation failed: {failure}", file=sys.stderr)
        return 1
    print("dependency disposition validation passed")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
