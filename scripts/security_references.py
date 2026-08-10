#!/usr/bin/env python3
"""Validate public security-reference provenance without network access."""

from __future__ import annotations

import argparse
import hashlib
import json
import re
import sys
from collections import Counter
from datetime import date
from pathlib import Path
from typing import Final
from urllib.parse import urlparse


ROOT: Final = Path(__file__).resolve().parents[1]
DEFAULT_REGISTER: Final = ROOT / "requirements" / "security-references.json"
DEFAULT_BASELINE: Final = ROOT / "requirements" / "security-reference-baseline.json"
DEFAULT_SECURITY_REVIEW: Final = ROOT / "SECURITY-REVIEW.md"
SCHEMA_VERSION: Final = 1
SECTION_START: Final = "## 5. Public Product-Security Reference Baseline"
SECTION_END_PREFIX: Final = "## 6."
MARKDOWN_LINK: Final = re.compile(r"(?<!!)\[[^\]]+\]\((https://[^)]+)\)")
REFERENCE_ID: Final = re.compile(r"^PSR-\d{3}$")
SHA256: Final = re.compile(r"^[a-f0-9]{64}$")
DATE_VALUE: Final = re.compile(r"^\d{4}(?:-\d{2}(?:-\d{2})?)?$")
SNAPSHOT_DECISIONS: Final = {
    "metadata_only_dynamic_source",
    "metadata_only_planning_baseline",
    "metadata_only_reuse_restricted",
}
TOP_LEVEL_FIELDS: Final = {
    "schema_version",
    "register_id",
    "as_of",
    "governance",
    "references",
}
GOVERNANCE_FIELDS: Final = {
    "decision_record",
    "external_certification_claimed",
    "publisher_endorsement_claimed",
}
REFERENCE_FIELDS: Final = {
    "id",
    "publisher",
    "title",
    "version",
    "publication_date",
    "effective_date",
    "date_notes",
    "source_url",
    "retrieval_date",
    "retrieval_http_status",
    "source_sha256",
    "hash_scope",
    "status",
    "superseded_by",
    "snapshot",
    "claims",
}
SNAPSHOT_FIELDS: Final = {
    "decision",
    "approval_status",
    "decision_record",
    "local_path",
    "rationale",
}
CLAIM_FIELDS: Final = {"external_certification", "publisher_endorsement"}
BASELINE_FIELDS: Final = {
    "schema_version",
    "baseline_id",
    "decision_record",
    "references",
}
BASELINE_REFERENCE_FIELDS: Final = {
    "id",
    "publisher",
    "title",
    "version",
    "source_url",
    "source_sha256",
    "status",
    "superseded_by",
    "snapshot_decision",
}


def _relative(path: Path, root: Path) -> str:
    try:
        return path.resolve().relative_to(root.resolve()).as_posix()
    except ValueError:
        return str(path)


def _field_contract(
    value: object,
    expected: set[str],
    location: str,
    failures: list[str],
) -> bool:
    if not isinstance(value, dict):
        failures.append(f"{location}: expected an object")
        return False
    actual = set(value)
    for field in sorted(expected - actual):
        failures.append(f"{location}: missing field {field}")
    for field in sorted(actual - expected):
        failures.append(f"{location}: unknown field {field}")
    return actual == expected


def _valid_date(value: object, *, nullable: bool) -> bool:
    if value is None:
        return nullable
    if not isinstance(value, str) or DATE_VALUE.fullmatch(value) is None:
        return False
    try:
        if len(value) == 4:
            date(int(value), 1, 1)
        elif len(value) == 7:
            date.fromisoformat(f"{value}-01")
        else:
            date.fromisoformat(value)
    except ValueError:
        return False
    return True


def _nonempty_string(value: object) -> bool:
    return isinstance(value, str) and bool(value.strip())


def _repo_path_exists(relative: object, root: Path) -> bool:
    if not _nonempty_string(relative):
        return False
    candidate = Path(str(relative))
    if candidate.is_absolute() or ".." in candidate.parts:
        return False
    resolved = (root / candidate).resolve()
    try:
        resolved.relative_to(root.resolve())
    except ValueError:
        return False
    return resolved.is_file()


def extract_reference_urls(security_review_path: Path = DEFAULT_SECURITY_REVIEW) -> list[str]:
    """Return HTTPS links in the canonical public-reference section, in order."""
    text = security_review_path.read_text(encoding="utf-8")
    start = text.find(SECTION_START)
    if start < 0:
        raise ValueError(f"missing section: {SECTION_START}")
    end = text.find(SECTION_END_PREFIX, start + len(SECTION_START))
    if end < 0:
        raise ValueError(f"missing section boundary: {SECTION_END_PREFIX}")
    return MARKDOWN_LINK.findall(text[start:end])


def _candidate_pin(record: dict[str, object]) -> dict[str, object]:
    snapshot = record.get("snapshot")
    snapshot_decision = snapshot.get("decision") if isinstance(snapshot, dict) else None
    return {
        "id": record.get("id"),
        "publisher": record.get("publisher"),
        "title": record.get("title"),
        "version": record.get("version"),
        "source_url": record.get("source_url"),
        "source_sha256": record.get("source_sha256"),
        "status": record.get("status"),
        "superseded_by": record.get("superseded_by"),
        "snapshot_decision": snapshot_decision,
    }


def _impact_review(
    reference_id: str,
    change_types: list[str],
    baseline: dict[str, object] | None,
    candidate: dict[str, object] | None,
) -> dict[str, object]:
    identity = json.dumps(
        {
            "reference_id": reference_id,
            "change_types": sorted(change_types),
            "baseline": baseline,
            "candidate": candidate,
        },
        sort_keys=True,
        separators=(",", ":"),
    )
    review_id = "IR-" + hashlib.sha256(identity.encode("utf-8")).hexdigest()[:16].upper()
    return {
        "id": review_id,
        "reference_id": reference_id,
        "status": "required",
        "change_types": sorted(change_types),
        "baseline": baseline,
        "candidate": candidate,
        "required_actions": [
            "verify publisher, source, version, and artifact identity",
            "review affected requirements, controls, tests, and release evidence",
            "record an accepted decision before updating the pinned baseline",
        ],
        "silent_replacement_blocked": True,
    }


def audit_reference_baseline(
    references: list[object],
    baseline_path: Path = DEFAULT_BASELINE,
    root: Path = ROOT,
) -> tuple[list[dict[str, object]], list[str]]:
    """Compare candidate records with the accepted baseline and create impact reviews."""
    failures: list[str] = []
    try:
        baseline = json.loads(baseline_path.read_text(encoding="utf-8"))
    except (OSError, json.JSONDecodeError) as error:
        return [], [f"unable to load security-reference baseline: {error}"]
    if not _field_contract(baseline, BASELINE_FIELDS, "baseline", failures):
        return [], failures
    assert isinstance(baseline, dict)
    if baseline["schema_version"] != 1:
        failures.append("baseline: unsupported schema_version")
    if not _nonempty_string(baseline["baseline_id"]):
        failures.append("baseline: baseline_id must be non-empty")
    if not _repo_path_exists(baseline["decision_record"], root):
        failures.append("baseline: decision_record must be an existing repository file")
    pins = baseline["references"]
    if not isinstance(pins, list):
        failures.append("baseline: references must be an array")
        return [], failures

    baseline_by_id: dict[str, dict[str, object]] = {}
    for index, pin in enumerate(pins):
        location = f"baseline.references[{index}]"
        if not _field_contract(pin, BASELINE_REFERENCE_FIELDS, location, failures):
            continue
        assert isinstance(pin, dict)
        identifier = pin["id"]
        if not isinstance(identifier, str) or REFERENCE_ID.fullmatch(identifier) is None:
            failures.append(f"{location}: invalid id")
            continue
        if identifier in baseline_by_id:
            failures.append(f"baseline: duplicate reference id {identifier}")
            continue
        baseline_by_id[identifier] = pin

    candidate_by_id: dict[str, dict[str, object]] = {}
    for record in references:
        if not isinstance(record, dict) or not isinstance(record.get("id"), str):
            continue
        candidate_by_id[str(record["id"])] = _candidate_pin(record)

    impacts: list[dict[str, object]] = []
    all_ids = sorted(set(baseline_by_id) | set(candidate_by_id))
    for identifier in all_ids:
        expected = baseline_by_id.get(identifier)
        observed = candidate_by_id.get(identifier)
        if expected is None:
            impacts.append(_impact_review(identifier, ["reference_added"], None, observed))
            continue
        if observed is None:
            impacts.append(_impact_review(identifier, ["reference_removed"], expected, None))
            continue

        change_types: list[str] = []
        if expected["publisher"] != observed["publisher"] or expected["title"] != observed["title"]:
            change_types.append("identity_substituted")
        if expected["version"] != observed["version"]:
            change_types.append("version_changed")
        if expected["source_url"] != observed["source_url"]:
            change_types.append("source_redirected")
        if expected["source_sha256"] != observed["source_sha256"]:
            change_types.append("integrity_changed")
        if expected["status"] != observed["status"]:
            change_types.append("status_changed")
        if expected["superseded_by"] != observed["superseded_by"]:
            change_types.append("supersession_changed")
        if expected["snapshot_decision"] != observed["snapshot_decision"]:
            change_types.append("snapshot_decision_changed")
        if change_types:
            impacts.append(_impact_review(identifier, change_types, expected, observed))

    for impact in impacts:
        failures.append(
            f"impact review required: {impact['id']} {impact['reference_id']} "
            f"({', '.join(impact['change_types'])})"
        )
    return impacts, failures


def audit_reference_register(
    register_path: Path = DEFAULT_REGISTER,
    security_review_path: Path = DEFAULT_SECURITY_REVIEW,
    baseline_path: Path = DEFAULT_BASELINE,
    root: Path = ROOT,
) -> dict[str, object]:
    """Return a deterministic validation report and never mutate source files."""
    failures: list[str] = []
    try:
        data = json.loads(register_path.read_text(encoding="utf-8"))
    except (OSError, json.JSONDecodeError) as error:
        return {
            "ok": False,
            "citation_count": 0,
            "reference_count": 0,
            "impact_reviews": [],
            "failures": [f"unable to load reference register: {error}"],
        }

    try:
        citation_urls = extract_reference_urls(security_review_path)
    except (OSError, ValueError) as error:
        citation_urls = []
        failures.append(f"unable to load public-reference citations: {error}")

    top_contract = _field_contract(data, TOP_LEVEL_FIELDS, "register", failures)
    if not top_contract:
        references = data.get("references", []) if isinstance(data, dict) else []
    else:
        references = data["references"]

    if not isinstance(data, dict):
        references = []
    else:
        if data.get("schema_version") != SCHEMA_VERSION:
            failures.append(
                f"register: unsupported schema_version {data.get('schema_version')!r}"
            )
        if data.get("register_id") != "agentmage-public-product-security-references":
            failures.append("register: invalid register_id")
        if not _valid_date(data.get("as_of"), nullable=False):
            failures.append("register: as_of must be a valid ISO date")

        governance = data.get("governance")
        if _field_contract(governance, GOVERNANCE_FIELDS, "governance", failures):
            assert isinstance(governance, dict)
            if not _repo_path_exists(governance["decision_record"], root):
                failures.append("governance: decision_record must be an existing repository file")
            if governance["external_certification_claimed"] is not False:
                failures.append("governance: external certification claims are prohibited")
            if governance["publisher_endorsement_claimed"] is not False:
                failures.append("governance: publisher endorsement claims are prohibited")

    if not isinstance(references, list):
        failures.append("register: references must be an array")
        references = []

    record_ids: list[str] = []
    record_urls: list[str] = []
    for index, record in enumerate(references, 1):
        location = f"references[{index - 1}]"
        if not _field_contract(record, REFERENCE_FIELDS, location, failures):
            continue
        assert isinstance(record, dict)
        identifier = record["id"]
        source_url = record["source_url"]
        if not isinstance(identifier, str) or REFERENCE_ID.fullmatch(identifier) is None:
            failures.append(f"{location}: invalid id")
        else:
            record_ids.append(identifier)
            expected_id = f"PSR-{index:03d}"
            if identifier != expected_id:
                failures.append(f"{location}: expected ordered id {expected_id}")

        for field in ("publisher", "title", "date_notes", "hash_scope"):
            if not _nonempty_string(record[field]):
                failures.append(f"{location}: {field} must be a non-empty string")
        if record["version"] is not None and not _nonempty_string(record["version"]):
            failures.append(f"{location}: version must be null or a non-empty string")
        for field in ("publication_date", "effective_date"):
            if not _valid_date(record[field], nullable=True):
                failures.append(f"{location}: {field} must be null or a valid ISO date")
        if not _valid_date(record["retrieval_date"], nullable=False):
            failures.append(f"{location}: retrieval_date must be a valid ISO date")
        elif isinstance(data, dict) and _valid_date(data.get("as_of"), nullable=False):
            if str(record["retrieval_date"]) > str(data["as_of"]):
                failures.append(f"{location}: retrieval_date is later than register as_of")

        if not isinstance(source_url, str) or urlparse(source_url).scheme != "https":
            failures.append(f"{location}: source_url must use HTTPS")
        else:
            record_urls.append(source_url)
        http_status = record["retrieval_http_status"]
        if isinstance(http_status, bool) or not isinstance(http_status, int):
            failures.append(f"{location}: retrieval_http_status must be an integer")
        elif not 100 <= http_status <= 599:
            failures.append(f"{location}: retrieval_http_status is outside HTTP range")
        if not isinstance(record["source_sha256"], str) or SHA256.fullmatch(
            record["source_sha256"]
        ) is None:
            failures.append(f"{location}: source_sha256 must be lowercase SHA-256")

        status = record["status"]
        superseded_by = record["superseded_by"]
        if status not in {"current", "superseded"}:
            failures.append(f"{location}: status must be current or superseded")
        elif status == "current" and superseded_by is not None:
            failures.append(f"{location}: a current reference cannot name superseded_by")
        elif status == "superseded" and not _nonempty_string(superseded_by):
            failures.append(f"{location}: a superseded reference must name superseded_by")

        snapshot = record["snapshot"]
        if _field_contract(snapshot, SNAPSHOT_FIELDS, f"{location}.snapshot", failures):
            assert isinstance(snapshot, dict)
            if snapshot["decision"] not in SNAPSHOT_DECISIONS:
                failures.append(f"{location}.snapshot: unsupported decision")
            if snapshot["approval_status"] != "accepted":
                failures.append(f"{location}.snapshot: decision is not accepted")
            if not _repo_path_exists(snapshot["decision_record"], root):
                failures.append(
                    f"{location}.snapshot: decision_record must be an existing repository file"
                )
            if snapshot["local_path"] is not None:
                failures.append(f"{location}.snapshot: metadata-only decisions require null local_path")
            if not _nonempty_string(snapshot["rationale"]):
                failures.append(f"{location}.snapshot: rationale must be non-empty")

        claims = record["claims"]
        if _field_contract(claims, CLAIM_FIELDS, f"{location}.claims", failures):
            assert isinstance(claims, dict)
            if claims["external_certification"] is not False:
                failures.append(f"{location}.claims: external certification claims are prohibited")
            if claims["publisher_endorsement"] is not False:
                failures.append(f"{location}.claims: publisher endorsement claims are prohibited")

    for identifier, count in sorted(Counter(record_ids).items()):
        if count > 1:
            failures.append(f"duplicate reference id: {identifier}")
    for source_url, count in sorted(Counter(record_urls).items()):
        if count > 1:
            failures.append(f"duplicate source_url: {source_url}")
    for source_url, count in sorted(Counter(citation_urls).items()):
        if count > 1:
            failures.append(f"duplicate Section 5 citation: {source_url}")

    citation_set = set(citation_urls)
    record_set = set(record_urls)
    for source_url in sorted(citation_set - record_set):
        failures.append(f"unregistered Section 5 citation: {source_url}")
    for source_url in sorted(record_set - citation_set):
        failures.append(f"orphaned reference record: {source_url}")
    if citation_set == record_set and citation_urls != record_urls:
        failures.append("reference records must follow Section 5 citation order")

    known_ids = set(record_ids)
    for index, record in enumerate(references):
        if not isinstance(record, dict) or record.get("status") != "superseded":
            continue
        replacement = record.get("superseded_by")
        if replacement not in known_ids or replacement == record.get("id"):
            failures.append(
                f"references[{index}]: superseded_by must name a different registered reference"
            )

    impacts, baseline_failures = audit_reference_baseline(references, baseline_path, root)
    failures.extend(baseline_failures)

    return {
        "ok": not failures,
        "citation_count": len(citation_urls),
        "reference_count": len(references),
        "impact_reviews": impacts,
        "failures": sorted(failures),
    }


def main(argv: list[str] | None = None) -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--register", type=Path, default=DEFAULT_REGISTER)
    parser.add_argument("--security-review", type=Path, default=DEFAULT_SECURITY_REVIEW)
    parser.add_argument("--baseline", type=Path, default=DEFAULT_BASELINE)
    parser.add_argument("--json", action="store_true", dest="json_output")
    args = parser.parse_args(argv)

    report = audit_reference_register(args.register, args.security_review, args.baseline)
    if args.json_output:
        print(json.dumps(report, indent=2, sort_keys=True))
    elif report["ok"]:
        print(
            f"Validated {report['reference_count']} public security references "
            "without network access."
        )
    else:
        print("Public security-reference validation failed:", file=sys.stderr)
        for failure in report["failures"]:
            print(f"- {failure}", file=sys.stderr)
    return 0 if report["ok"] else 1


if __name__ == "__main__":
    raise SystemExit(main())
