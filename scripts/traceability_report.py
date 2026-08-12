#!/usr/bin/env python3
"""Build the deterministic cross-document requirement traceability report."""

from __future__ import annotations

import argparse
import json
import re
import sys
from collections import Counter, defaultdict
from pathlib import Path
from typing import Final

try:
    from scripts.evidence_applicability import build_report as build_evidence_report
    from scripts.evidence_core import atomic_write, canonical_json_bytes, sha256_bytes
except ModuleNotFoundError:
    from evidence_applicability import build_report as build_evidence_report
    from evidence_core import atomic_write, canonical_json_bytes, sha256_bytes


ROOT: Final = Path(__file__).resolve().parents[1]
DEFAULT_REGISTRY: Final = ROOT / "requirements" / "registry.json"
DEFAULT_NORMATIVE_MAP: Final = ROOT / "requirements" / "normative-map.json"
DEFAULT_POLICY_REGISTER: Final = ROOT / "requirements" / "policy-expectations.json"
DEFAULT_TASKS: Final = ROOT / "TASKS.md"
DEFAULT_OUTPUT: Final = ROOT / "requirements" / "traceability-report.json"
DEFAULT_EVIDENCE_CATALOG: Final = ROOT / "evidence" / "catalog.json"
SCHEMA_VERSION: Final = 2
SPRINT_HEADING: Final = re.compile(r"^### \[[ x]\] Sprint (\d+) - (.+?)\s*$")
STORY_HEADING: Final = re.compile(r"^#### \[[ x]\] Story (\d+\.\d+) - (.+?)\s*$")
REFERENCE_ID: Final = re.compile(r"\b(?:AM|AT|CR)-[A-Z0-9.-]+\b")


class TraceabilityError(ValueError):
    """Raised when traceability inputs cannot produce an unambiguous report."""


def load_object(path: Path) -> dict[str, object]:
    value = json.loads(path.read_text(encoding="utf-8"))
    if not isinstance(value, dict):
        raise TraceabilityError(f"{path}: expected a JSON object")
    return value


def source_name(path: Path, root: Path = ROOT) -> str:
    try:
        return path.resolve().relative_to(root.resolve()).as_posix()
    except ValueError:
        return path.name


def input_identity(path: Path, root: Path = ROOT) -> dict[str, str]:
    return {
        "document": source_name(path, root),
        "sha256": sha256_bytes(path.read_bytes()),
    }


def evidence_claim_index(
    report: dict[str, object], registry_ids: set[str]
) -> dict[str, list[dict[str, object]]]:
    """Index only historically valid catalog records by exact registry claim."""

    records = report.get("records")
    if not isinstance(records, list):
        raise TraceabilityError("evidence applicability records must be an array")
    indexed: dict[str, list[dict[str, object]]] = defaultdict(list)
    for record in records:
        if not isinstance(record, dict):
            raise TraceabilityError("evidence applicability record must be an object")
        evidence_id = record.get("evidence_id")
        claims = record.get("claims")
        artifact = record.get("artifact")
        historical = record.get("historical_validity")
        applicability = record.get("current_applicability")
        if (
            not isinstance(evidence_id, str)
            or not isinstance(claims, list)
            or not isinstance(artifact, dict)
            or historical not in {"valid", "invalid", "unverifiable-legacy"}
            or applicability
            not in {
                "current",
                "stale",
                "blocked",
                "superseded",
                "historical-only",
                "produced",
                "rejected",
            }
        ):
            raise TraceabilityError("evidence applicability record is malformed")
        if historical != "valid":
            continue
        path = artifact.get("path")
        if not isinstance(path, str):
            raise TraceabilityError("evidence artifact path is malformed")
        for claim in claims:
            if not isinstance(claim, str) or claim not in registry_ids:
                raise TraceabilityError(f"unknown evidence claim: {claim}")
            indexed[claim].append(
                {
                    "evidence_id": evidence_id,
                    "path": path,
                    "historical_validity": historical,
                    "current_applicability": applicability,
                    "platform_lanes": record.get("platform_lanes", []),
                    "revision": artifact.get("revision"),
                    "sha256": artifact.get("sha256"),
                }
            )
    for claim, items in indexed.items():
        current = [item for item in items if item["current_applicability"] == "current"]
        if len(current) > 1:
            raise TraceabilityError(f"ambiguous current evidence claim: {claim}")
        items.sort(key=lambda item: (str(item["path"]), str(item["evidence_id"])))
    return dict(indexed)


def evidence_state(
    records: list[dict[str, object]], *, outside_selected_release: bool
) -> dict[str, object]:
    """Render evidence without converting a produced artifact into completion."""

    if not records:
        return {
            "status": "absent",
            "absence_disposition": (
                "expected-outside-release"
                if outside_selected_release
                else "missing-required"
            ),
            "paths": [],
            "records": [],
        }
    priority = (
        "current",
        "blocked",
        "stale",
        "produced",
        "historical-only",
        "superseded",
        "rejected",
    )
    states = {str(record["current_applicability"]) for record in records}
    status = next(state for state in priority if state in states)
    return {
        "status": status,
        "absence_disposition": "not-applicable",
        "paths": sorted({str(record["path"]) for record in records}),
        "records": records,
    }


def parse_sprint_coverage(tasks_path: Path) -> dict[str, list[dict[str, object]]]:
    """Map stable IDs in sprint source-coverage declarations to their stories."""
    current_sprint: int | None = None
    current_sprint_title = ""
    current_story_id: str | None = None
    current_story_title = ""
    mapping: dict[str, list[dict[str, object]]] = defaultdict(list)

    for line_number, line in enumerate(tasks_path.read_text(encoding="utf-8").splitlines(), 1):
        sprint_match = SPRINT_HEADING.match(line)
        if sprint_match:
            current_sprint = int(sprint_match.group(1))
            current_sprint_title = sprint_match.group(2)
            current_story_id = None
            current_story_title = ""
            continue
        story_match = STORY_HEADING.match(line)
        if story_match:
            current_story_id = story_match.group(1)
            current_story_title = story_match.group(2)
            continue
        if not line.startswith("**Source coverage:**"):
            continue
        if current_sprint is None:
            raise TraceabilityError(f"source coverage at line {line_number} has no sprint")

        story_id = current_story_id or f"{current_sprint}.1"
        story_title = current_story_title or current_sprint_title
        item = {
            "sprint": current_sprint,
            "story_id": story_id,
            "story_title": story_title,
            "source": {
                "document": source_name(tasks_path),
                "heading": f"Sprint {current_sprint} - {current_sprint_title}",
                "line": line_number,
            },
        }
        for identifier in REFERENCE_ID.findall(line):
            if item not in mapping[identifier]:
                mapping[identifier].append(item)

    return {
        identifier: sorted(items, key=lambda item: (int(item["sprint"]), str(item["story_id"])))
        for identifier, items in mapping.items()
    }


def reverse_normative_mappings(normative_map: dict[str, object]) -> dict[str, list[dict[str, object]]]:
    mappings = normative_map.get("mappings")
    source = normative_map.get("source")
    if not isinstance(mappings, list) or not isinstance(source, dict):
        raise TraceabilityError("normative map must contain source and mappings")
    document = source.get("document")
    if not isinstance(document, str) or not document:
        raise TraceabilityError("normative map source document is missing")

    reverse: dict[str, list[dict[str, object]]] = defaultdict(list)
    for mapping in mappings:
        if not isinstance(mapping, dict):
            raise TraceabilityError("normative mapping must be an object")
        requirement_ids = mapping.get("requirement_ids")
        if not isinstance(requirement_ids, list):
            raise TraceabilityError("normative mapping requirement_ids must be an array")
        statement = {
            "document": document,
            "heading": mapping.get("heading"),
            "line": mapping.get("line"),
            "statement_sha256": mapping.get("statement_sha256"),
        }
        for identifier in requirement_ids:
            if not isinstance(identifier, str):
                raise TraceabilityError("normative requirement ID must be a string")
            reverse[identifier].append(statement)
    return {
        identifier: sorted(items, key=lambda item: (int(item["line"]), str(item["statement_sha256"])))
        for identifier, items in reverse.items()
    }


def policy_references(policy_register: dict[str, object]) -> dict[str, list[str]]:
    expectations = policy_register.get("expectations")
    if not isinstance(expectations, list):
        raise TraceabilityError("policy register expectations must be an array")
    result: dict[str, list[str]] = defaultdict(list)
    for expectation in expectations:
        if not isinstance(expectation, dict):
            raise TraceabilityError("policy expectation must be an object")
        identifier = expectation.get("id")
        statement = expectation.get("statement")
        if not isinstance(identifier, str) or not isinstance(statement, str):
            raise TraceabilityError("policy expectation identity or statement is malformed")
        for referenced in REFERENCE_ID.findall(statement):
            result[referenced].append(identifier)
    return {identifier: sorted(set(values)) for identifier, values in result.items()}


def _requirements(registry: dict[str, object]) -> list[dict[str, object]]:
    records = registry.get("requirements")
    if not isinstance(records, list) or not records:
        raise TraceabilityError("registry requirements must be a non-empty array")
    if not all(isinstance(record, dict) for record in records):
        raise TraceabilityError("every registry requirement must be an object")
    identifiers = [record.get("id") for record in records]
    if not all(isinstance(identifier, str) for identifier in identifiers):
        raise TraceabilityError("every registry requirement needs a string ID")
    duplicates = sorted(
        identifier for identifier, count in Counter(identifiers).items() if count > 1
    )
    if duplicates:
        raise TraceabilityError("duplicate requirement IDs: " + ", ".join(duplicates))
    if identifiers != sorted(identifiers):
        raise TraceabilityError("registry requirements are not in canonical ID order")
    return records


def build_traceability_report(
    registry_path: Path = DEFAULT_REGISTRY,
    normative_map_path: Path = DEFAULT_NORMATIVE_MAP,
    policy_register_path: Path = DEFAULT_POLICY_REGISTER,
    tasks_path: Path = DEFAULT_TASKS,
    evidence_catalog_path: Path = DEFAULT_EVIDENCE_CATALOG,
) -> dict[str, object]:
    """Build complete planning traceability without editing any input."""
    registry = load_object(registry_path)
    normative_map = load_object(normative_map_path)
    policy_register = load_object(policy_register_path)
    records = _requirements(registry)
    by_id = {str(record["id"]): record for record in records}
    evidence_report = build_evidence_report(ROOT, evidence_catalog_path)
    evidence_by_requirement = evidence_claim_index(evidence_report, set(by_id))
    selected_release = str(evidence_report["catalog"]["selected_release"])
    sprint_coverage = parse_sprint_coverage(tasks_path)
    normative_by_requirement = reverse_normative_mappings(normative_map)
    policy_by_requirement = policy_references(policy_register)

    consumers_by_test: dict[str, list[str]] = defaultdict(list)
    for record in records:
        for test_id in record.get("acceptance_tests", []):
            consumers_by_test[str(test_id)].append(str(record["id"]))

    traceability: list[dict[str, object]] = []
    for record in records:
        identifier = str(record["id"])
        plan_items = sprint_coverage.get(identifier, [])
        derivation = "explicit_source_coverage"
        if not plan_items and record.get("kind") == "acceptance_test":
            owners = consumers_by_test.get(identifier, [])
            inherited: list[dict[str, object]] = []
            for owner in owners:
                inherited.extend(sprint_coverage.get(owner, []))
            unique = {
                (int(item["sprint"]), str(item["story_id"])): item for item in inherited
            }
            plan_items = [unique[key] for key in sorted(unique)]
            derivation = "assigned_product_requirement"
        if not plan_items:
            raise TraceabilityError(f"{identifier} has no implementation-plan mapping")

        release = str(record.get("release"))
        expectation_ids = policy_by_requirement.get(identifier, [])
        if expectation_ids:
            exclusion_state = "explicit_policy_expectation"
        elif release == "v0.1":
            exclusion_state = "not_excluded"
        else:
            exclusion_state = "outside_v0.1_release"

        evidence_roots = [
            f"artifacts/sprints/sprint-{int(item['sprint'])}" for item in plan_items
        ]
        discovered_evidence = evidence_state(
            evidence_by_requirement.get(identifier, []),
            outside_selected_release=release != selected_release,
        )
        discovered_evidence["expected_roots"] = sorted(set(evidence_roots))
        traceability.append(
            {
                "id": identifier,
                "kind": record.get("kind"),
                "title": record.get("title"),
                "release": release,
                "status": record.get("status"),
                "source": record.get("source"),
                "dependencies": record.get("dependencies"),
                "acceptance_tests": record.get("acceptance_tests"),
                "verifies_requirement_ids": sorted(consumers_by_test.get(identifier, [])),
                "normative_statements": normative_by_requirement.get(identifier, []),
                "implementation": {
                    "derivation": derivation,
                    "planning_items": plan_items,
                    "external_issue_ids": [],
                    "external_issue_status": "not_created",
                },
                "exclusion": {
                    "state": exclusion_state,
                    "policy_expectation_ids": expectation_ids,
                },
                "evidence": discovered_evidence,
            }
        )

    by_kind = Counter(str(item["kind"]) for item in traceability)
    by_exclusion = Counter(str(item["exclusion"]["state"]) for item in traceability)
    by_evidence = Counter(str(item["evidence"]["status"]) for item in traceability)
    return {
        "schema_version": SCHEMA_VERSION,
        "report_id": "agentmage-cross-document-traceability",
        "generated_from": [
            input_identity(registry_path),
            input_identity(normative_map_path),
            input_identity(policy_register_path),
            input_identity(tasks_path),
            input_identity(evidence_catalog_path),
        ],
        "evidence_view": {
            "report_id": evidence_report["report_id"],
            "sha256": sha256_bytes(canonical_json_bytes(evidence_report)),
            "selected_release": selected_release,
        },
        "counts": {
            "total": len(traceability),
            "by_kind": {kind: by_kind[kind] for kind in sorted(by_kind)},
            "by_exclusion_state": {
                state: by_exclusion[state] for state in sorted(by_exclusion)
            },
            "by_evidence_status": {
                state: by_evidence[state] for state in sorted(by_evidence)
            },
        },
        "requirements": traceability,
    }


def render_traceability_report(report: dict[str, object]) -> str:
    return canonical_json_bytes(report).decode("ascii")


def write_traceability_report(output_path: Path = DEFAULT_OUTPUT, **paths: Path) -> None:
    atomic_write(
        output_path,
        canonical_json_bytes(build_traceability_report(**paths)),
    )


def check_traceability_report(output_path: Path = DEFAULT_OUTPUT, **paths: Path) -> bool:
    expected = render_traceability_report(build_traceability_report(**paths))
    try:
        actual = output_path.read_text(encoding="utf-8")
    except FileNotFoundError:
        print(f"Traceability report is missing: {output_path}", file=sys.stderr)
        return False
    if actual != expected:
        print(
            "Traceability report is stale; run `npm run traceability:build`.",
            file=sys.stderr,
        )
        return False
    return True


def main(argv: list[str] | None = None) -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--output", type=Path, default=DEFAULT_OUTPUT)
    parser.add_argument("--check", action="store_true")
    args = parser.parse_args(argv)
    try:
        if args.check:
            return 0 if check_traceability_report(args.output) else 1
        write_traceability_report(args.output)
    except (OSError, UnicodeError, json.JSONDecodeError, TraceabilityError) as error:
        print(f"Traceability report error: {error}", file=sys.stderr)
        return 1
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
