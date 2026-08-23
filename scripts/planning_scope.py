#!/usr/bin/env python3
"""Validate decision-aware planning scope and emit a reviewable report."""

from __future__ import annotations

import argparse
import hashlib
import json
import re
import subprocess
import sys
from collections import Counter
from pathlib import Path
from typing import Any, Final

try:
    from scripts.additions_only import audit_additions_only
except ModuleNotFoundError:  # Direct execution adds scripts/, not the repository root.
    from additions_only import audit_additions_only


ROOT: Final = Path(__file__).resolve().parents[1]
DEFAULT_MANIFEST: Final = ROOT / "requirements" / "planning-scope-decisions.json"
DEFAULT_OUTPUT: Final = ROOT / "artifacts" / "sprints" / "sprint-12" / "story-12.3" / "planning-scope-report.json"
DEFAULT_REGISTRY: Final = ROOT / "requirements" / "registry.json"
DEFAULT_NORMATIVE_MAP: Final = ROOT / "requirements" / "normative-map.json"
DEFAULT_BASELINE: Final = ROOT / "requirements" / "additions-only-baseline.json"
DEFAULT_TRACEABILITY: Final = ROOT / "requirements" / "traceability-report.json"
DEFAULT_STATUS: Final = ROOT / "architecture" / "status-model.json"
DEFAULT_TASKS: Final = ROOT / "TASKS.md"

EXPECTED_DECISIONS: Final = ("ADR-0027", "ADR-0040")
EXPECTED_FOUNDATIONAL_DECISIONS: Final = ("ADR-0042", "ADR-0043", "ADR-0044")
DECISION_0043_REQUIREMENTS: Final = (
    "AM-CAP-001",
    "AM-CAP-002",
    "AM-CTX-001",
    "AM-CTX-002",
    "AM-CTX-003",
    "AM-DEG-001",
    "AM-ERT-001",
    "AM-MAG-001",
    "AM-OBS-002",
    "AM-PERF-001",
    "AM-SES-002",
    "AM-TIO-001",
    "AM-VER-001",
    "AM-VSC-004",
    "AM-VSC-005",
    "AM-VSC-006",
    "AM-WKF-001",
    "AM-WKF-002",
    "AT-CAP-001",
    "AT-CAP-002",
    "AT-CTX-001",
    "AT-CTX-002",
    "AT-CTX-003",
    "AT-CTX-004",
    "AT-DEG-001",
    "AT-ERT-001",
    "AT-MAG-001",
    "AT-MAG-002",
    "AT-OBS-002",
    "AT-PERF-002",
    "AT-PERF-003",
    "AT-RESUME-002",
    "AT-TIO-001",
    "AT-TIO-002",
    "AT-VER-001",
    "AT-VSC-004",
    "AT-VSC-005",
    "AT-VSC-006",
    "AT-WKF-001",
    "AT-WKF-002",
    "AT-WKF-003",
)
DECISION_0044_REQUIREMENTS: Final = (
    "AM-GWY-001",
    "AM-GWY-002",
    "AM-GWY-003",
    "AM-REM-001",
    "AM-REM-002",
    "AM-REM-003",
    "AT-GWY-001",
    "AT-GWY-002",
    "AT-GWY-003",
    "AT-REM-001",
    "AT-REM-002",
    "AT-REM-003",
)
EXPECTED_FOUNDATIONAL_ADDITIONS: Final = {
    "ADR-0042": ((), ("FRE-INGEST", "FRE-WORKFLOW")),
    "ADR-0043": (DECISION_0043_REQUIREMENTS, ("FRE-ENGINEERING-RUNTIME",)),
    "ADR-0044": (DECISION_0044_REQUIREMENTS, ("FRE-MODEL-GATEWAY",)),
}
EXPECTED_NEGATIVE_CONTROLS: Final = (
    "mutated-preserved-requirement",
    "deleted-preserved-requirement",
    "duplicated-preserved-requirement",
    "reordered-preserved-requirement",
    "renumbered-preserved-requirement",
    "unapproved-added-requirement",
    "missing-decision-approval",
    "corrupt-appended-id-set",
    "corrupt-status-count",
    "corrupt-supersession-boundary",
    "corrupt-generated-registry-linkage",
)


class PlanningScopeError(ValueError):
    """Raised when planning-scope input cannot be validated."""


def load_object(path: Path) -> dict[str, Any]:
    value = json.loads(path.read_text(encoding="utf-8"))
    if not isinstance(value, dict):
        raise PlanningScopeError(f"{path}: expected a JSON object")
    return value


def sha256_bytes(value: bytes) -> str:
    return hashlib.sha256(value).hexdigest()


def sha256_file(path: Path) -> str:
    return sha256_bytes(path.read_bytes())


def render(value: dict[str, Any]) -> str:
    return json.dumps(value, indent=2, sort_keys=True, ensure_ascii=True) + "\n"


def _records(value: Any, label: str, failures: list[str]) -> list[dict[str, Any]]:
    if not isinstance(value, list) or any(not isinstance(item, dict) for item in value):
        failures.append(f"{label} must be an array of objects")
        return []
    return value


def _ordered_unique_ids(
    records: list[dict[str, Any]], label: str, failures: list[str]
) -> list[str]:
    identifiers = [item.get("id") for item in records]
    if any(not isinstance(item, str) or not item for item in identifiers):
        failures.append(f"{label} contains a missing or invalid id")
        return []
    normalized = [str(item) for item in identifiers]
    duplicates = sorted(
        identifier for identifier, count in Counter(normalized).items() if count > 1
    )
    if duplicates:
        failures.append(f"{label} contains duplicate ids: {', '.join(duplicates)}")
    if normalized != sorted(normalized):
        failures.append(f"{label} ids are reordered; canonical order is lexical")
    return normalized


def _normative_identity(record: dict[str, Any]) -> dict[str, Any]:
    return {
        "heading": record.get("heading"),
        "requirement_ids": record.get("requirement_ids"),
        "statement_sha256": record.get("statement_sha256"),
    }


def _validate_counts(
    registry: dict[str, Any], normative_map: dict[str, Any], tasks_text: str,
    failures: list[str]
) -> dict[str, int]:
    requirements = _records(registry.get("requirements"), "registry requirements", failures)
    mappings = _records(normative_map.get("mappings"), "normative mappings", failures)
    counts = {
        "stable_requirements": len(requirements),
        "normative_mappings": len(mappings),
        "epics": len(re.findall(r"^## \[[ xX]\] Epic \d+", tasks_text, re.MULTILINE)),
        "foundational_runtime_epics": len(
            re.findall(
                r"^## \[[ xX]\] Foundational Runtime Epic F\d+ - ",
                tasks_text,
                re.MULTILINE,
            )
        ),
        "sprints": len(
            re.findall(r"^### \[[ xX]\] Sprint \d+", tasks_text, re.MULTILINE)
        ),
    }
    declared = registry.get("counts", {}).get("total")
    if declared != counts["stable_requirements"]:
        failures.append(
            "generated registry linkage count mismatch: "
            f"declared {declared!r}, observed {counts['stable_requirements']}"
        )
    return counts


def _validate_snapshot(
    snapshot: Any, label: str, failures: list[str]
) -> tuple[list[str], list[dict[str, Any]]]:
    if not isinstance(snapshot, dict):
        failures.append(f"{label} snapshot must be an object")
        return [], []
    requirement_ids = snapshot.get("requirement_ids")
    normative = snapshot.get("normative_mappings")
    if not isinstance(requirement_ids, list) or any(
        not isinstance(item, str) for item in requirement_ids
    ):
        failures.append(f"{label} requirement_ids must be strings")
        requirement_ids = []
    if requirement_ids != sorted(requirement_ids):
        failures.append(f"{label} requirement_ids must be sorted")
    if len(requirement_ids) != len(set(requirement_ids)):
        failures.append(f"{label} requirement_ids contain duplicates")
    normative_records = _records(normative, f"{label} normative_mappings", failures)
    expected = snapshot.get("counts")
    actual = {
        "stable_requirements": len(requirement_ids),
        "normative_mappings": len(normative_records),
    }
    if expected != actual:
        failures.append(f"{label} snapshot counts changed: expected {expected}, observed {actual}")
    return requirement_ids, normative_records


def _validate_decision(
    decision: dict[str, Any], root: Path, failures: list[str]
) -> str:
    decision_id = decision.get("id", "<unknown>")
    if decision.get("status") != "accepted":
        failures.append(f"{decision_id}: decision approval is missing or not accepted")
    document = decision.get("document")
    if not isinstance(document, str) or Path(document).is_absolute():
        failures.append(f"{decision_id}: decision document path is invalid")
        return ""
    path = root / document
    if not path.is_file():
        failures.append(f"{decision_id}: decision document is missing: {document}")
        return ""
    text = path.read_text(encoding="utf-8")
    observed_hash = sha256_file(path)
    if observed_hash != decision.get("source_sha256"):
        failures.append(
            f"{decision_id}: decision source hash changed from "
            f"{decision.get('source_sha256')} to {observed_hash}"
        )
    markers = decision.get("approval_markers")
    if not isinstance(markers, list) or not markers:
        failures.append(f"{decision_id}: approval markers are missing")
    else:
        for marker in markers:
            if not isinstance(marker, str) or marker not in text:
                failures.append(f"{decision_id}: decision approval marker is missing: {marker!r}")
    return text


def validate_planning_scope(
    manifest: dict[str, Any], registry: dict[str, Any], normative_map: dict[str, Any],
    additions_baseline: dict[str, Any], traceability: dict[str, Any],
    status_model: dict[str, Any], tasks_text: str, root: Path = ROOT,
) -> tuple[list[str], dict[str, Any]]:
    """Validate the accepted planning chain without mutating any input."""
    failures: list[str] = []
    if manifest.get("schema_version") != 1:
        failures.append("planning scope schema_version must equal 1")

    decisions = _records(manifest.get("decisions"), "planning decisions", failures)
    decision_ids = [item.get("id") for item in decisions]
    if tuple(decision_ids) != EXPECTED_DECISIONS:
        failures.append(
            "accepted decision chain must be exactly " + ", ".join(EXPECTED_DECISIONS)
        )
    decision_text = {
        str(item.get("id")): _validate_decision(item, root, failures) for item in decisions
    }

    foundational_decisions = _records(
        manifest.get("foundational_runtime_decisions"),
        "foundational runtime decisions",
        failures,
    )
    foundational_ids = [item.get("id") for item in foundational_decisions]
    if tuple(foundational_ids) != EXPECTED_FOUNDATIONAL_DECISIONS:
        failures.append(
            "accepted foundational decision set must be exactly "
            + ", ".join(EXPECTED_FOUNDATIONAL_DECISIONS)
        )
    foundational_additions: list[str] = []
    for item in foundational_decisions:
        decision_text_value = _validate_decision(item, root, failures)
        decision_id = str(item.get("id"))
        if item.get("release_epic_delta") != 0 or item.get("sprint_delta") != 0:
            failures.append(f"{decision_id}: release epic or sprint delta must remain zero")
        expected_ids, expected_epics = EXPECTED_FOUNDATIONAL_ADDITIONS.get(
            decision_id, ((), ())
        )
        actual_ids = item.get("appended_requirement_ids")
        if actual_ids != list(expected_ids):
            failures.append(f"{decision_id}: appended requirement id set is corrupt")
            actual_ids = []
        if item.get("stable_requirement_delta") != len(expected_ids):
            failures.append(f"{decision_id}: stable requirement delta is corrupt")
        if item.get("foundational_runtime_epics") != list(expected_epics):
            failures.append(f"{decision_id}: foundational runtime epic identities are corrupt")
        if decision_id != "ADR-0042" and not decision_text_value:
            failures.append(f"{decision_id}: additive decision text is unavailable")
        foundational_additions.extend(str(identifier) for identifier in actual_ids)
    if len(foundational_additions) != len(set(foundational_additions)):
        failures.append("foundational decisions contain duplicate appended requirement ids")

    snapshots = manifest.get("snapshots")
    if not isinstance(snapshots, dict):
        failures.append("planning snapshots must be an object")
        snapshots = {}
    names = ("pre-0027", "post-0027", "post-0040")
    parsed = {
        name: _validate_snapshot(snapshots.get(name), name, failures) for name in names
    }
    pre_ids, pre_normative = parsed["pre-0027"]
    d27_ids, d27_normative = parsed["post-0027"]
    d40_ids, d40_normative = parsed["post-0040"]

    d27 = decisions[0] if len(decisions) > 0 else {}
    d40 = decisions[1] if len(decisions) > 1 else {}
    appended = d27.get("appended_requirement_ids")
    if not isinstance(appended, list) or any(not isinstance(item, str) for item in appended):
        failures.append("ADR-0027: appended requirement id set is malformed")
        appended = []
    expected_d27 = sorted(set(pre_ids) | set(appended))
    if len(expected_d27) != len(pre_ids) + len(appended) or expected_d27 != d27_ids:
        failures.append("ADR-0027: appended requirement id set does not reconcile snapshots")
    if d40.get("appended_requirement_ids") != [] or d40_ids != d27_ids:
        failures.append("ADR-0040: requirement identities changed without an additive decision")

    expected_transitions = (
        (d27, "pre-0027", "post-0027", pre_normative, d27_normative),
        (d40, "post-0027", "post-0040", d27_normative, d40_normative),
    )
    for decision, before_name, after_name, before, after in expected_transitions:
        decision_id = decision.get("id", "<unknown>")
        if decision.get("before_snapshot") != before_name or decision.get(
            "after_snapshot"
        ) != after_name:
            failures.append(f"{decision_id}: snapshot linkage is corrupt")
        before_hashes = [item.get("statement_sha256") for item in before]
        after_hashes = [item.get("statement_sha256") for item in after]
        appended_hashes = decision.get("appended_normative_hashes")
        superseded_hashes = decision.get("superseded_normative_hashes")
        if not isinstance(appended_hashes, list) or not isinstance(
            superseded_hashes, list
        ):
            failures.append(f"{decision_id}: normative reconciliation sets are malformed")
            continue
        expected_after = [item for item in before_hashes if item not in superseded_hashes]
        expected_after.extend(appended_hashes)
        if len(expected_after) != len(set(expected_after)) or set(expected_after) != set(
            after_hashes
        ):
            failures.append(f"{decision_id}: normative mapping reconciliation is corrupt")

    requirements = _records(registry.get("requirements"), "registry requirements", failures)
    current_ids = _ordered_unique_ids(requirements, "registry requirements", failures)
    current_normative = [
        _normative_identity(item)
        for item in _records(normative_map.get("mappings"), "normative mappings", failures)
    ]
    accepted_current_ids = sorted(set(d40_ids) | set(foundational_additions))
    if len(accepted_current_ids) != len(d40_ids) + len(foundational_additions):
        failures.append("foundational appended requirement ids overlap accepted history")
    if current_ids != accepted_current_ids:
        missing = sorted(set(accepted_current_ids) - set(current_ids))
        added = sorted(set(current_ids) - set(accepted_current_ids))
        failures.append(
            "current requirement identities do not match accepted decisions; "
            f"missing={missing}, unapproved={added}"
        )
    if current_normative != d40_normative:
        failures.append("current normative mappings do not match the ordered post-0040 snapshot")

    counts = _validate_counts(registry, normative_map, tasks_text, failures)
    expected_counts = {
        "stable_requirements": 294,
        "normative_mappings": 31,
        "epics": 17,
        "foundational_runtime_epics": 4,
        "sprints": 169,
    }
    if counts != expected_counts:
        failures.append(f"current planning counts changed: expected {expected_counts}, observed {counts}")
    if status_model.get("scope_control", {}).get("stable_requirements") != counts[
        "stable_requirements"
    ]:
        failures.append("status count does not match the accepted requirement registry")
    if status_model.get("scope_control", {}).get(
        "accepted_foundational_runtime_epics"
    ) != counts["foundational_runtime_epics"]:
        failures.append("status count does not match the accepted foundational runtime epics")

    trace_records = _records(
        traceability.get("requirements"), "traceability requirements", failures
    )
    trace_ids = [item.get("id") for item in trace_records]
    if traceability.get("counts", {}).get("total") != len(trace_records):
        failures.append("generated traceability count does not match its records")
    if trace_ids != current_ids:
        failures.append("generated registry linkage does not preserve ordered requirement ids")

    baseline_requirements = _records(
        additions_baseline.get("requirements"), "additions-only requirements", failures
    )
    baseline_ids = [item.get("id") for item in baseline_requirements]
    if baseline_ids != current_ids:
        failures.append("additions-only baseline does not preserve the accepted requirement ids")
    try:
        for diagnostic in audit_additions_only(additions_baseline, registry):
            failures.append(
                f"additions-only {diagnostic.category} at {diagnostic.location}: "
                f"{diagnostic.message}"
            )
    except (OSError, ValueError) as error:
        failures.append(f"additions-only validation failed: {error}")

    model_direction = manifest.get("model_direction")
    if not isinstance(model_direction, dict):
        failures.append("model direction contract is missing")
        model_direction = {}
    historical_path = model_direction.get("historical_document")
    if not isinstance(historical_path, str) or not (root / historical_path).is_file():
        failures.append("model direction historical document is missing")
        historical_text = ""
    else:
        historical_text = (root / historical_path).read_text(encoding="utf-8")
    for marker in model_direction.get("historical_markers", []):
        if marker not in historical_text:
            failures.append(f"historical model direction marker is missing: {marker!r}")
    current_text = decision_text.get("ADR-0027", "")
    for marker in model_direction.get("supersession_markers", []):
        if marker not in current_text:
            failures.append(f"current model supersession marker is missing: {marker!r}")

    source_hashes = manifest.get("current_source_hashes")
    if not isinstance(source_hashes, dict):
        failures.append("current source hashes are missing")
        source_hashes = {}
    for relative, expected_hash in source_hashes.items():
        path = root / relative
        if not path.is_file():
            failures.append(f"current source is missing: {relative}")
            continue
        observed = sha256_file(path)
        if observed != expected_hash:
            failures.append(
                f"current source hash changed for {relative}: expected {expected_hash}, "
                f"observed {observed}"
            )

    controls = manifest.get("required_negative_controls")
    if tuple(controls) != EXPECTED_NEGATIVE_CONTROLS:
        failures.append("required negative-control registry is missing, reordered, or changed")

    report = {
        "schema_version": 1,
        "status": "pass" if not failures else "fail",
        "accepted_decisions": decision_ids + foundational_ids,
        "accepted_scope_decisions": decision_ids,
        "accepted_foundational_decisions": foundational_ids,
        "historical_baseline": snapshots.get("post-0027", {}).get("counts", {}),
        "current_counts": counts,
        "appended_requirement_ids": sorted(set(appended) | set(foundational_additions)),
        "foundational_appended_requirement_ids": foundational_additions,
        "preserved_requirement_ids": pre_ids,
        "superseded_assumptions": model_direction.get("superseded_assumptions", []),
        "source_hashes": source_hashes,
        "required_negative_controls": list(EXPECTED_NEGATIVE_CONTROLS),
        "failures": failures,
    }
    return failures, report


def build_report(root: Path = ROOT) -> tuple[list[str], dict[str, Any]]:
    return validate_planning_scope(
        load_object(root / "requirements" / "planning-scope-decisions.json"),
        load_object(root / "requirements" / "registry.json"),
        load_object(root / "requirements" / "normative-map.json"),
        load_object(root / "requirements" / "additions-only-baseline.json"),
        load_object(root / "requirements" / "traceability-report.json"),
        load_object(root / "architecture" / "status-model.json"),
        (root / "TASKS.md").read_text(encoding="utf-8"),
        root,
    )


def _git_json(revision: str, relative: str) -> dict[str, Any]:
    result = subprocess.run(
        ["git", "show", f"{revision}:{relative}"],
        cwd=ROOT,
        check=True,
        capture_output=True,
    )
    value = json.loads(result.stdout)
    if not isinstance(value, dict):
        raise PlanningScopeError(f"{revision}:{relative} is not an object")
    return value


def _git_hash(revision: str, relative: str) -> str:
    result = subprocess.run(
        ["git", "show", f"{revision}:{relative}"],
        cwd=ROOT,
        check=True,
        capture_output=True,
    )
    return sha256_bytes(result.stdout)


def _snapshot(registry: dict[str, Any], normative: dict[str, Any]) -> dict[str, Any]:
    ids = sorted(str(item["id"]) for item in registry["requirements"])
    mappings = [_normative_identity(item) for item in normative["mappings"]]
    return {
        "counts": {
            "stable_requirements": len(ids),
            "normative_mappings": len(mappings),
        },
        "requirement_ids": ids,
        "normative_mappings": mappings,
    }


def build_manifest() -> dict[str, Any]:
    """Build the one-time manifest from the exact accepted Git snapshots."""
    revisions = {
        "pre-0027": "99d5d93c9656c25cfd40f7586b38a6e5d8cfb6da",
        "post-0027": "4a180115f697e4ccf6a1816f9b4bd6f275358324",
        "post-0040": "d0ab2ca9842a5ebd13d489c5595abd832110eea1",
    }
    snapshots: dict[str, dict[str, Any]] = {}
    for name, revision in revisions.items():
        snapshots[name] = _snapshot(
            _git_json(revision, "requirements/registry.json"),
            _git_json(revision, "requirements/normative-map.json"),
        )
        snapshots[name]["source_revision"] = revision

    def hashes(name: str) -> set[str]:
        return {
            str(item["statement_sha256"])
            for item in snapshots[name]["normative_mappings"]
        }

    pre_ids = set(snapshots["pre-0027"]["requirement_ids"])
    d27_ids = set(snapshots["post-0027"]["requirement_ids"])
    d27_before, d27_after = hashes("pre-0027"), hashes("post-0027")
    d40_before, d40_after = hashes("post-0027"), hashes("post-0040")
    return {
        "schema_version": 1,
        "decisions": [
            {
                "id": "ADR-0027",
                "status": "accepted",
                "document": "docs/decisions/0027-muse-first-model-neutral-runtime-and-evaluation.md",
                "source_sha256": sha256_file(
                    ROOT / "docs/decisions/0027-muse-first-model-neutral-runtime-and-evaluation.md"
                ),
                "approval_markers": [
                    "| Status | Accepted scope and architecture refinement |",
                    "On 2026-08-12, the user explicitly approved Decision 0027",
                ],
                "before_snapshot": "pre-0027",
                "after_snapshot": "post-0027",
                "appended_requirement_ids": sorted(d27_ids - pre_ids),
                "appended_normative_hashes": sorted(d27_after - d27_before),
                "superseded_normative_hashes": sorted(d27_before - d27_after),
            },
            {
                "id": "ADR-0040",
                "status": "accepted",
                "document": "docs/decisions/0040-local-platform-validation-and-manual-macos.md",
                "source_sha256": sha256_file(
                    ROOT / "docs/decisions/0040-local-platform-validation-and-manual-macos.md"
                ),
                "approval_markers": [
                    "Accepted execution-policy decision.",
                    "The execution venue must not change evidence truth.",
                ],
                "before_snapshot": "post-0027",
                "after_snapshot": "post-0040",
                "appended_requirement_ids": [],
                "appended_normative_hashes": sorted(d40_after - d40_before),
                "superseded_normative_hashes": sorted(d40_before - d40_after),
            },
        ],
        "foundational_runtime_decisions": [
            {
                "id": "ADR-0042",
                "status": "accepted",
                "document": "docs/decisions/0042-universal-artifact-ingestion-and-verified-workflow-execution.md",
                "source_sha256": sha256_file(
                    ROOT
                    / "docs/decisions/0042-universal-artifact-ingestion-and-verified-workflow-execution.md"
                ),
                "approval_markers": [
                    "| Status | Accepted additive architecture and planning refinement |",
                    "On 2026-08-21, the user explicitly directed AgentMage",
                ],
                "foundational_runtime_epics": ["FRE-INGEST", "FRE-WORKFLOW"],
                "release_epic_delta": 0,
                "sprint_delta": 0,
                "stable_requirement_delta": 0,
                "appended_requirement_ids": [],
            },
            {
                "id": "ADR-0043",
                "status": "accepted",
                "document": "docs/decisions/0043-engineering-runtime-foundations-and-verified-chat.md",
                "source_sha256": sha256_file(
                    ROOT
                    / "docs/decisions/0043-engineering-runtime-foundations-and-verified-chat.md"
                ),
                "approval_markers": [
                    "| Status | Accepted additive architecture and planning refinement |",
                    "On 2026-08-22, the user explicitly instructed AgentMage",
                ],
                "foundational_runtime_epics": ["FRE-ENGINEERING-RUNTIME"],
                "release_epic_delta": 0,
                "sprint_delta": 0,
                "stable_requirement_delta": len(DECISION_0043_REQUIREMENTS),
                "appended_requirement_ids": list(DECISION_0043_REQUIREMENTS),
            },
            {
                "id": "ADR-0044",
                "status": "accepted",
                "document": "docs/decisions/0044-local-and-remote-open-weight-inference-profiles.md",
                "source_sha256": sha256_file(
                    ROOT
                    / "docs/decisions/0044-local-and-remote-open-weight-inference-profiles.md"
                ),
                "approval_markers": [
                    "| Status | Accepted additive architecture and planning refinement |",
                    "On 2026-08-22, the user explicitly instructed AgentMage",
                ],
                "foundational_runtime_epics": ["FRE-MODEL-GATEWAY"],
                "release_epic_delta": 0,
                "sprint_delta": 0,
                "stable_requirement_delta": len(DECISION_0044_REQUIREMENTS),
                "appended_requirement_ids": list(DECISION_0044_REQUIREMENTS),
            }
        ],
        "snapshots": snapshots,
        "model_direction": {
            "historical_document": "docs/decisions/0001-product-security-and-runtime-baseline.md",
            "historical_markers": [
                "Gemma 4 E4B remains the initial candidate.",
                "Gemma 4 12B Unified is the named, disabled fallback candidate",
            ],
            "supersession_markers": [
                "Muse-first implementation focus",
                "Candidate-neutral model boundary",
                "No model is enabled, downloaded, supported, or released by this decision.",
            ],
            "superseded_assumptions": [
                "one approved Gemma 4 E4B profile is the runtime prerequisite",
                "the model picker is hard-coded to E4B",
                "first-GA success depends on a Gemma candidate passing",
            ],
        },
        "current_source_hashes": {
            "architecture/status-model.json": sha256_file(
                ROOT / "architecture/status-model.json"
            ),
            "requirements/additions-only-baseline.json": sha256_file(
                ROOT / "requirements/additions-only-baseline.json"
            ),
            "requirements/normative-map.json": sha256_file(
                ROOT / "requirements/normative-map.json"
            ),
            "requirements/registry.json": sha256_file(
                ROOT / "requirements/registry.json"
            ),
        },
        "historical_source_hashes": {
            "pre-0027/requirements/registry.json": _git_hash(
                revisions["pre-0027"], "requirements/registry.json"
            ),
            "pre-0027/requirements/normative-map.json": _git_hash(
                revisions["pre-0027"], "requirements/normative-map.json"
            ),
            "post-0027/requirements/registry.json": _git_hash(
                revisions["post-0027"], "requirements/registry.json"
            ),
            "post-0027/requirements/normative-map.json": _git_hash(
                revisions["post-0027"], "requirements/normative-map.json"
            ),
        },
        "required_negative_controls": list(EXPECTED_NEGATIVE_CONTROLS),
    }


def main(argv: list[str] | None = None) -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    action = parser.add_mutually_exclusive_group()
    action.add_argument("--bootstrap-manifest", action="store_true")
    action.add_argument("--refresh-manifest", action="store_true")
    action.add_argument("--write", action="store_true")
    action.add_argument("--check", action="store_true")
    args = parser.parse_args(argv)
    try:
        if args.bootstrap_manifest:
            if DEFAULT_MANIFEST.exists():
                raise PlanningScopeError(
                    f"refusing to overwrite existing manifest: {DEFAULT_MANIFEST}"
                )
            DEFAULT_MANIFEST.write_text(render(build_manifest()), encoding="utf-8")
            return 0
        if args.refresh_manifest:
            if not DEFAULT_MANIFEST.is_file():
                raise PlanningScopeError(
                    f"cannot refresh missing manifest: {DEFAULT_MANIFEST}"
                )
            current = load_object(DEFAULT_MANIFEST)
            if current.get("schema_version") != 1:
                raise PlanningScopeError("refusing to refresh an unknown manifest version")
            DEFAULT_MANIFEST.write_text(render(build_manifest()), encoding="utf-8")
            return 0
        failures, report = build_report()
        if args.write:
            DEFAULT_OUTPUT.parent.mkdir(parents=True, exist_ok=True)
            DEFAULT_OUTPUT.write_text(render(report), encoding="utf-8")
        elif args.check and DEFAULT_OUTPUT.is_file():
            committed = load_object(DEFAULT_OUTPUT)
            comparable = dict(committed)
            comparable.pop("source_revision", None)
            if comparable != report:
                failures.append("planning scope report is stale")
        if failures:
            for failure in failures:
                print(f"- {failure}", file=sys.stderr)
            return 1
        print(
            "Planning scope validation passed: Decision 0027 baseline 241/30; "
            "current accepted chain 294/31; Decisions 0042-0044 add 4 foundational "
            "runtime epics, 53 stable requirements, and 0 release epics or sprints."
        )
        return 0
    except (OSError, ValueError, subprocess.CalledProcessError) as error:
        print(f"Planning scope validation failed: {error}", file=sys.stderr)
        return 1


if __name__ == "__main__":
    raise SystemExit(main())
