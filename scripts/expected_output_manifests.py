#!/usr/bin/env python3
"""Build deterministic expected-output manifests for synthetic fixtures."""

from __future__ import annotations

import argparse
import copy
import hashlib
import json
import os
import shutil
import sys
import tempfile
from dataclasses import dataclass
from pathlib import Path, PurePosixPath
from typing import Any

ROOT = Path(__file__).resolve().parents[1]
if str(ROOT) not in sys.path:
    sys.path.insert(0, str(ROOT))

from scripts.document_fixture_generator import (  # noqa: E402
    materialize_specs as materialize_document_specs,
)
from scripts.document_fixture_generator import (  # noqa: E402
    validate_profile as validate_document_profile,
)


PROFILE_PATH = ROOT / "fixtures" / "expected-output-profile.json"
REPORT_PATH = (
    ROOT
    / "artifacts"
    / "sprints"
    / "sprint-2"
    / "story-2.1"
    / "expected-output-report.json"
)
EXPECTED_STATES = ("Observed", "Derived", "Inferred", "Unknown/Blocked")
EXPECTED_CASES = (
    ("observed-bounded-read", "Observed", "success"),
    ("derived-task-count", "Derived", "success"),
    ("inferred-testing-preference", "Inferred", "success"),
    ("unknown-adjacent-workspace", "Unknown/Blocked", "denied"),
)
EXPECTED_PROHIBITED_SIDE_EFFECTS = (
    "workspace-write",
    "command-execution",
    "network-access",
    "credential-read",
    "grant-mint",
    "external-delivery",
)
ZERO_HASH = "0" * 64


@dataclass(frozen=True)
class ExpectedManifest:
    case_id: str
    content: bytes


def read_json(path: Path) -> Any:
    return json.loads(path.read_text(encoding="utf-8"))


def canonical_json(value: Any) -> bytes:
    return (json.dumps(value, indent=2, sort_keys=True) + "\n").encode("utf-8")


def sha256_bytes(value: bytes) -> str:
    return hashlib.sha256(value).hexdigest()


def safe_relative(value: str) -> bool:
    path = PurePosixPath(value)
    return bool(value) and not path.is_absolute() and ".." not in path.parts


def validate_profile(profile: Any) -> list[str]:
    failures: list[str] = []
    if not isinstance(profile, dict):
        return ["expected-output profile must be an object"]
    if (
        profile.get("schema_version") != 1
        or profile.get("profile_id") != "agentmage-expected-output-manifests-v1"
        or profile.get("status") != "synthetic-expected-output-contract"
    ):
        failures.append("expected-output profile identity is invalid")
    if profile.get("source_fixture_profile") != {
        "id": "agentmage-document-fixtures-v1",
        "path": "fixtures/document-fixture-profile.json",
    }:
        failures.append("expected-output source fixture profile drifted")
    if profile.get("fixed_timestamp") != "2024-01-01T00:00:00Z":
        failures.append("expected-output timestamp is not pinned")
    if profile.get("source_range_semantics") != {
        "byte_offsets": "zero-based-half-open-utf8",
        "line_numbers": "one-based-inclusive",
    }:
        failures.append("expected-output source-range semantics drifted")
    if tuple(profile.get("evidence_states", [])) != EXPECTED_STATES:
        failures.append("expected-output evidence-state closure drifted")
    if tuple(profile.get("prohibited_side_effects", [])) != EXPECTED_PROHIBITED_SIDE_EFFECTS:
        failures.append("expected-output prohibited-side-effect closure drifted")
    cases = profile.get("cases")
    if not isinstance(cases, list):
        failures.append("expected-output cases must be a list")
        cases = []
    observed_cases = tuple(
        (case.get("id"), case.get("evidence_state"), case.get("outcome"))
        for case in cases
        if isinstance(case, dict)
    )
    if observed_cases != EXPECTED_CASES:
        failures.append("expected-output case closure drifted")
    for case in cases:
        if not isinstance(case, dict):
            continue
        state = case.get("evidence_state")
        source = case.get("source")
        if state == "Unknown/Blocked":
            if source is not None or case.get("reason") != "out-of-scope":
                failures.append("Unknown/Blocked case has invalid source or reason")
        else:
            if not isinstance(source, dict):
                failures.append(f"expected-output case lacks source: {case.get('id')}")
                continue
            if not safe_relative(source.get("path", "")):
                failures.append(f"expected-output case has unsafe source: {case.get('id')}")
            start = source.get("line_start")
            end = source.get("line_end")
            if not isinstance(start, int) or not isinstance(end, int) or start < 1 or end < start:
                failures.append(f"expected-output case has invalid range: {case.get('id')}")
        if state == "Derived" and case.get("method_id") != "count-json-array-v1":
            failures.append("Derived case method drifted")
        if state == "Inferred" and (
            case.get("model_id") != "fake-model-v1"
            or case.get("runtime_id") != "fake-inference-runtime-v1"
            or case.get("prompt_template_id") != "synthetic-inference-v1"
        ):
            failures.append("Inferred case provenance drifted")
    if profile.get("authority_claim") != "none":
        failures.append("expected-output profile cannot claim authority")
    if profile.get("product_receipt_implementation_claim") != "none":
        failures.append("expected-output profile cannot claim a product receipt implementation")
    if (
        profile.get("versioned_golden_manifest_status")
        != "fulfilled-by-agentmage-versioned-synthetic-corpus-v1"
    ):
        failures.append("expected-output profile lost its versioned golden disposition")
    return failures


def source_range(content: bytes, line_start: int, line_end: int) -> dict[str, Any]:
    lines = content.splitlines(keepends=True)
    if line_start < 1 or line_end < line_start or line_end > len(lines):
        raise ValueError("expected-output source range is outside the fixture")
    byte_start = sum(len(line) for line in lines[: line_start - 1])
    byte_end = sum(len(line) for line in lines[:line_end])
    excerpt = content[byte_start:byte_end]
    return {
        "byte_end": byte_end,
        "byte_start": byte_start,
        "line_end": line_end,
        "line_start": line_start,
        "range_sha256": sha256_bytes(excerpt),
    }


def citation_for(case: dict[str, Any], source_specs: dict[str, Any]) -> dict[str, Any]:
    source = case["source"]
    path = source["path"]
    if path not in source_specs:
        raise ValueError(f"expected-output source fixture does not exist: {path}")
    fixture = source_specs[path]
    range_record = source_range(fixture.content, source["line_start"], source["line_end"])
    citation_identity = canonical_json(
        {
            "case_id": case["id"],
            "content_sha256": sha256_bytes(fixture.content),
            "path": path,
            "range": range_record,
        }
    )
    return {
        "citation_id": f"am-synthetic-citation-{sha256_bytes(citation_identity)[:16]}",
        "content_sha256": sha256_bytes(fixture.content),
        "media_type": fixture.media_type,
        "path": path,
        "range": range_record,
    }


def expected_output(case: dict[str, Any], citation: dict[str, Any] | None) -> dict[str, Any]:
    state = case["evidence_state"]
    output: dict[str, Any] = {
        "claim_id": f"am-synthetic-claim-{sha256_bytes(case['id'].encode())[:16]}",
        "evidence": {
            "blocked_reason": None,
            "citations": [] if citation is None else [citation],
            "derivation": None,
            "inference": None,
            "state": state,
        },
        "statement": case["statement"],
    }
    evidence = output["evidence"]
    if state == "Derived":
        evidence["derivation"] = {
            "input_citation_ids": [citation["citation_id"]],
            "method_id": case["method_id"],
        }
    elif state == "Inferred":
        evidence["inference"] = {
            "model_id": case["model_id"],
            "prompt_template_id": case["prompt_template_id"],
            "runtime_id": case["runtime_id"],
            "supporting_citation_ids": [citation["citation_id"]],
        }
    elif state == "Unknown/Blocked":
        evidence["blocked_reason"] = case["reason"]
    return output


def receipt_for(
    case: dict[str, Any],
    output: dict[str, Any],
    previous_receipt_sha256: str,
    timestamp: str,
) -> dict[str, Any]:
    receipt = {
        "action": case["action"],
        "attempt": 1,
        "completed_at": timestamp,
        "expected_side_effect_count": 0,
        "outcome": case["outcome"],
        "output_sha256": sha256_bytes(canonical_json(output)),
        "previous_receipt_sha256": previous_receipt_sha256,
        "receipt_id": f"am-synthetic-receipt-{sha256_bytes(case['id'].encode())[:16]}",
        "request_sha256": sha256_bytes(case["request"].encode("utf-8")),
        "synthetic": True,
    }
    receipt["receipt_sha256"] = sha256_bytes(canonical_json(receipt))
    return receipt


def build_manifest_records(profile: dict[str, Any], root: Path = ROOT) -> list[dict[str, Any]]:
    source_profile_path = root / profile["source_fixture_profile"]["path"]
    source_profile = read_json(source_profile_path)
    failures = validate_document_profile(source_profile)
    if failures:
        raise ValueError("; ".join(failures))
    source_specs = materialize_document_specs(source_profile)
    source_profile_sha256 = sha256_bytes(source_profile_path.read_bytes())
    records: list[dict[str, Any]] = []
    previous_receipt_sha256 = ZERO_HASH
    for case in profile["cases"]:
        citation = None
        if case["evidence_state"] != "Unknown/Blocked":
            citation = citation_for(case, source_specs)
        output = expected_output(case, citation)
        receipt = receipt_for(case, output, previous_receipt_sha256, profile["fixed_timestamp"])
        record = {
            "schema_version": 1,
            "manifest_id": f"agentmage-expected-{case['id']}-v1",
            "status": "synthetic-expected-output",
            "source_fixture_profile": {
                "id": source_profile["profile_id"],
                "sha256": source_profile_sha256,
            },
            "request": {
                "request_sha256": sha256_bytes(case["request"].encode("utf-8")),
                "text": case["request"],
            },
            "expected_output": output,
            "expected_receipt": receipt,
            "expected_side_effects": [],
            "prohibited_side_effects": [
                {"expected_count": 0, "id": item}
                for item in profile["prohibited_side_effects"]
            ],
            "authority_claim": "none",
            "product_implementation_claim": "none",
        }
        records.append(record)
        previous_receipt_sha256 = receipt["receipt_sha256"]
    return records


def materialize_specs(
    profile: dict[str, Any], root: Path = ROOT
) -> dict[str, ExpectedManifest]:
    failures = validate_profile(profile)
    if failures:
        raise ValueError("; ".join(failures))
    files = {
        f"expected-outputs/{case['id']}.json": ExpectedManifest(
            case_id=case["id"],
            content=canonical_json(record),
        )
        for case, record in zip(profile["cases"], build_manifest_records(profile, root))
    }
    return dict(sorted(files.items()))


def manifest_set_identity(files: dict[str, ExpectedManifest]) -> str:
    digest = hashlib.sha256()
    for path, manifest in sorted(files.items()):
        for field in (path.encode(), manifest.case_id.encode(), manifest.content):
            digest.update(len(field).to_bytes(8, "big"))
            digest.update(field)
    return digest.hexdigest()


def validate_records(
    profile: dict[str, Any],
    records: list[dict[str, Any]],
    root: Path = ROOT,
) -> list[str]:
    failures: list[str] = []
    if len(records) != len(profile["cases"]):
        return ["expected-output record count does not match the profile"]
    previous_hash = ZERO_HASH
    source_profile = read_json(root / profile["source_fixture_profile"]["path"])
    source_specs = materialize_document_specs(source_profile)
    for case, record in zip(profile["cases"], records):
        receipt = record["expected_receipt"]
        receipt_without_hash = copy.deepcopy(receipt)
        recorded_hash = receipt_without_hash.pop("receipt_sha256")
        if sha256_bytes(canonical_json(receipt_without_hash)) != recorded_hash:
            failures.append(f"receipt hash mismatch: {case['id']}")
        if receipt["previous_receipt_sha256"] != previous_hash:
            failures.append(f"receipt chain mismatch: {case['id']}")
        previous_hash = recorded_hash
        output = record["expected_output"]
        if sha256_bytes(canonical_json(output)) != receipt["output_sha256"]:
            failures.append(f"receipt output hash mismatch: {case['id']}")
        citations = output["evidence"]["citations"]
        if case["evidence_state"] == "Unknown/Blocked":
            if citations:
                failures.append("Unknown/Blocked expected output cannot cite unavailable content")
            continue
        citation = citations[0]
        source = source_specs[citation["path"]].content
        expected_range = source_range(
            source,
            case["source"]["line_start"],
            case["source"]["line_end"],
        )
        if citation["content_sha256"] != sha256_bytes(source) or citation["range"] != expected_range:
            failures.append(f"citation source binding mismatch: {case['id']}")
    return failures


def validate_manifest_records(profile: dict[str, Any], root: Path = ROOT) -> list[str]:
    return validate_records(profile, build_manifest_records(profile, root), root)


def generate(profile: dict[str, Any], destination: Path, root: Path = ROOT) -> dict[str, Any]:
    failures = validate_profile(profile) + validate_manifest_records(profile, root)
    if failures:
        raise ValueError("; ".join(failures))
    if destination.exists():
        raise FileExistsError("expected-output destination already exists")
    destination.parent.mkdir(parents=True, exist_ok=True)
    staging = Path(tempfile.mkdtemp(prefix=".agentmage-expected-", dir=destination.parent))
    files = materialize_specs(profile, root)
    try:
        for relative, manifest in files.items():
            target = staging / relative
            target.parent.mkdir(parents=True, exist_ok=True)
            target.write_bytes(manifest.content)
            target.chmod(0o644)
        os.replace(staging, destination)
    except BaseException:
        shutil.rmtree(staging, ignore_errors=True)
        raise
    return {
        "manifest_count": len(files),
        "manifest_set_sha256": manifest_set_identity(files),
    }


def build_report(root: Path = ROOT) -> dict[str, Any]:
    profile_path = root / PROFILE_PATH.relative_to(ROOT)
    profile = read_json(profile_path)
    failures = validate_profile(profile) + validate_manifest_records(profile, root)
    if failures:
        raise ValueError("; ".join(failures))
    files = materialize_specs(profile, root)
    return {
        "schema_version": 1,
        "task_id": "2.1.1.6",
        "status": "pass",
        "profile": {
            "id": profile["profile_id"],
            "sha256": sha256_bytes(profile_path.read_bytes()),
        },
        "manifest_preview": {
            "persisted": False,
            "manifest_count": len(files),
            "manifest_set_sha256": manifest_set_identity(files),
            "manifests": [
                {
                    "case_id": manifest.case_id,
                    "path": path,
                    "sha256": sha256_bytes(manifest.content),
                }
                for path, manifest in sorted(files.items())
            ],
        },
        "evidence_states": list(EXPECTED_STATES),
        "receipt_chain": {
            "algorithm": "sha256-canonical-json",
            "genesis_sha256": ZERO_HASH,
            "receipt_count": len(files),
            "validated": True,
        },
        "source_range_semantics": profile["source_range_semantics"],
        "prohibited_side_effects": list(EXPECTED_PROHIBITED_SIDE_EFFECTS),
        "product_receipt_implementation_claim": "none",
        "versioned_golden_manifest_status": "fulfilled-by-agentmage-versioned-synthetic-corpus-v1",
        "macos_support_claim": "none",
    }


def validate_report(report: Any, root: Path = ROOT) -> list[str]:
    if not isinstance(report, dict):
        return ["expected-output report must be an object"]
    failures: list[str] = []
    if report.get("schema_version") != 1 or report.get("task_id") != "2.1.1.6":
        failures.append("expected-output report identity is invalid")
    if report.get("status") != "pass":
        failures.append("expected-output manifest generation did not pass")
    if report.get("product_receipt_implementation_claim") != "none":
        failures.append("expected-output report made a product receipt implementation claim")
    if (
        report.get("versioned_golden_manifest_status")
        != "fulfilled-by-agentmage-versioned-synthetic-corpus-v1"
    ):
        failures.append("expected-output report lost its versioned golden disposition")
    if report.get("macos_support_claim") != "none":
        failures.append("expected-output report made a macOS support claim")
    if report != build_report(root):
        failures.append("expected-output report is stale or non-deterministic")
    return failures


def write_report(root: Path = ROOT) -> None:
    output = root / REPORT_PATH.relative_to(ROOT)
    output.parent.mkdir(parents=True, exist_ok=True)
    output.write_bytes(canonical_json(build_report(root)))


def check_report(root: Path = ROOT) -> list[str]:
    try:
        report = read_json(root / REPORT_PATH.relative_to(ROOT))
    except (OSError, json.JSONDecodeError) as error:
        return [f"cannot read expected-output report: {error}"]
    return validate_report(report, root)


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--output", type=Path)
    parser.add_argument("--write-report", action="store_true")
    args = parser.parse_args()
    try:
        profile = read_json(PROFILE_PATH)
        if args.output is not None:
            result = generate(profile, args.output)
            print(json.dumps(result, indent=2, sort_keys=True))
        if args.write_report:
            write_report()
        failures = check_report()
    except (OSError, ValueError) as error:
        print(f"expected-output manifest generation failed: {error}", file=sys.stderr)
        return 1
    if failures:
        for failure in failures:
            print(f"expected-output manifest generation failed: {failure}", file=sys.stderr)
        return 1
    if args.output is None:
        print("deterministic expected-output manifests validated")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
