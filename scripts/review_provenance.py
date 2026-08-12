#!/usr/bin/env python3
"""Validate exact review classes without promoting repository automation to human review."""

from __future__ import annotations

import sys
from pathlib import Path
from typing import Any, Final

try:
    from scripts.evidence_core import (
        EvidenceError,
        git_blob,
        read_json_object,
        safe_relative_path,
        sha256_bytes,
        valid_sha256,
    )
except ModuleNotFoundError:
    from evidence_core import (  # type: ignore[no-redef]
        EvidenceError,
        git_blob,
        read_json_object,
        safe_relative_path,
        sha256_bytes,
        valid_sha256,
    )


ROOT: Final = Path(__file__).resolve().parents[1]
REGISTRY_PATH: Final = ROOT / "evidence" / "review-provenance.json"
CLASSES: Final = {
    "deterministic-self-check",
    "automated-aggregation",
    "independent-automated-analysis",
    "native-execution",
    "independent-human-review",
    "external-review",
}
STATUSES: Final = {"completed", "blocked", "superseded"}
FINDING_STATES: Final = {"open", "resolved", "accepted-risk"}


def _cycle(records: list[dict[str, Any]]) -> bool:
    parent = {record["review_id"]: record["rereview_of"] for record in records}
    for start in parent:
        seen: set[str] = set()
        current: str | None = start
        while current is not None:
            if current in seen:
                return True
            seen.add(current)
            current = parent.get(current)
    return False


def validate_registry(value: Any, root: Path = ROOT) -> list[str]:
    failures: list[str] = []
    if not isinstance(value, dict) or set(value) != {
        "schema_version",
        "registry_id",
        "classes",
        "records",
    }:
        return ["review.registry.field_closure"]
    if value.get("schema_version") != 1 or value.get("registry_id") != "agentmage-review-provenance-v1":
        failures.append("review.registry.identity")
    if value.get("classes") != sorted(CLASSES):
        failures.append("review.class_closure")
    records = value.get("records")
    if not isinstance(records, list):
        return [*failures, "review.records.invalid"]
    identities: list[str] = []
    for record in records:
        if not isinstance(record, dict) or set(record) != {
            "review_id",
            "subject",
            "original_label",
            "review_class",
            "performer",
            "status",
            "findings",
            "rereview_of",
        }:
            failures.append("review.record.field_closure")
            continue
        review_id = record.get("review_id")
        identities.append(str(review_id))
        review_class = record.get("review_class")
        if review_class not in CLASSES:
            failures.append(f"review.{review_id}.class")
        if record.get("status") not in STATUSES:
            failures.append(f"review.{review_id}.status")
        subject = record.get("subject")
        if not isinstance(subject, dict) or set(subject) != {"path", "revision", "sha256"}:
            failures.append(f"review.{review_id}.subject")
        elif not safe_relative_path(subject.get("path")) or not valid_sha256(subject.get("sha256")):
            failures.append(f"review.{review_id}.subject_identity")
        else:
            try:
                if sha256_bytes(git_blob(root, subject["revision"], subject["path"])) != subject["sha256"]:
                    failures.append(f"review.{review_id}.historical_hash")
            except EvidenceError:
                failures.append(f"review.{review_id}.historical_unavailable")
        performer = record.get("performer")
        if not isinstance(performer, dict) or set(performer) != {
            "identity",
            "kind",
            "human",
            "distinct_implementation",
            "distinct_process",
            "external_organization",
        }:
            failures.append(f"review.{review_id}.performer")
            continue
        human = performer.get("human") is True
        if not isinstance(performer.get("identity"), str) or not performer.get("identity"):
            failures.append(f"review.{review_id}.performer_identity")
        if not isinstance(performer.get("kind"), str) or not performer.get("kind"):
            failures.append(f"review.{review_id}.performer_kind")
        independent = (
            performer.get("distinct_implementation") is True
            and performer.get("distinct_process") is True
        )
        if review_class in {"deterministic-self-check", "automated-aggregation"} and (
            human or performer.get("external_organization") is not None
        ):
            failures.append(f"review.{review_id}.automation_mislabel")
        if review_class == "independent-automated-analysis" and (human or not independent):
            failures.append(f"review.{review_id}.independence")
        if review_class == "native-execution" and performer.get("kind") != "native-runtime":
            failures.append(f"review.{review_id}.native_identity")
        if review_class == "independent-human-review" and (not human or not independent):
            failures.append(f"review.{review_id}.human_identity")
        if review_class == "external-review" and (
            not human or not independent or not performer.get("external_organization")
        ):
            failures.append(f"review.{review_id}.external_identity")
        findings = record.get("findings")
        if not isinstance(findings, list):
            failures.append(f"review.{review_id}.findings")
            continue
        finding_ids: list[str] = []
        for finding in findings:
            if not isinstance(finding, dict) or set(finding) != {
                "finding_id",
                "severity",
                "owner",
                "state",
            }:
                failures.append(f"review.{review_id}.finding_closure")
                continue
            finding_ids.append(str(finding.get("finding_id")))
            if not finding.get("owner") or finding.get("state") not in FINDING_STATES:
                failures.append(f"review.{review_id}.finding_state")
        if finding_ids != sorted(set(finding_ids)):
            failures.append(f"review.{review_id}.finding_order")
        open_findings = [item for item in findings if item.get("state") == "open"]
        if record.get("status") == "blocked" and not open_findings:
            failures.append(f"review.{review_id}.blocked_without_finding")
        if record.get("status") == "completed" and open_findings:
            failures.append(f"review.{review_id}.completed_with_open_finding")
    if identities != sorted(set(identities)):
        failures.append("review.record_order_or_duplicate")
    known = set(identities)
    if any(
        record.get("rereview_of") is not None and record.get("rereview_of") not in known
        for record in records
        if isinstance(record, dict)
    ):
        failures.append("review.unknown_rereview")
    if records and _cycle(records):
        failures.append("review.rereview_cycle")
    return sorted(set(failures))


def satisfies_required_class(
    value: dict[str, Any], subject_path: str, required_class: str
) -> bool:
    """Return whether one exact completed provenance record satisfies a control."""

    return any(
        record.get("subject", {}).get("path") == subject_path
        and record.get("review_class") == required_class
        and record.get("status") == "completed"
        and all(finding.get("state") != "open" for finding in record.get("findings", []))
        for record in value.get("records", [])
    )


def main() -> int:
    try:
        registry = read_json_object(ROOT, REGISTRY_PATH.relative_to(ROOT).as_posix())
        failures = validate_registry(registry)
    except EvidenceError as error:
        failures = [str(error)]
    if failures:
        for failure in failures:
            print(f"review provenance failed: {failure}", file=sys.stderr)
        return 1
    print("exact review provenance classes validated")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
