#!/usr/bin/env python3
"""Validate the versioned, content-free Sprint 12 agent failure corpus."""

import argparse
import hashlib
import json
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
DEFAULT_CORPUS = ROOT / "fixtures/agent-policy/v1/failure-corpus.json"

EXPECTED_DISPOSITIONS = {
    "restart": "transition_locked_until_reconciled",
    "uncertain_effect": "blocked_uncertain_effect",
    "consumed_grant": "reject_grant_replay",
    "false_completion": "reject_non_deterministic_completion",
    "classifier_disagreement": "require_user_decision",
    "no_progress": "terminal_stalled",
}
REQUIRED_CASE_FIELDS = {
    "case_id",
    "category",
    "setup",
    "trigger",
    "expected_disposition",
    "prohibited_outcomes",
    "source_contract",
    "source_test",
    "evidence",
}
REQUIRED_EVIDENCE_FIELDS = {"path", "sha256"}


def sha256_file(path: Path) -> str:
    digest = hashlib.sha256()
    with path.open("rb") as handle:
        for chunk in iter(lambda: handle.read(1024 * 1024), b""):
            digest.update(chunk)
    return digest.hexdigest()


def valid_bounded_text(value: object) -> bool:
    return isinstance(value, str) and 0 < len(value) <= 512


def validate_corpus(payload: object, *, verify_files: bool = True) -> list[str]:
    errors: list[str] = []
    if not isinstance(payload, dict):
        return ["corpus must be an object"]
    if set(payload) != {"schema_version", "corpus_id", "content_class", "cases"}:
        errors.append("top-level fields are not closed")
    if payload.get("schema_version") != 1:
        errors.append("schema_version must be 1")
    if payload.get("corpus_id") != "agentmage-agent-policy-failure-corpus-v1":
        errors.append("corpus_id mismatch")
    if payload.get("content_class") != "synthetic_content_free":
        errors.append("content_class must be synthetic_content_free")

    cases = payload.get("cases")
    if not isinstance(cases, list):
        return errors + ["cases must be an array"]
    if len(cases) != len(EXPECTED_DISPOSITIONS):
        errors.append("case count mismatch")

    seen_ids: set[str] = set()
    seen_categories: set[str] = set()
    for index, case in enumerate(cases, start=1):
        label = f"case[{index}]"
        if not isinstance(case, dict):
            errors.append(f"{label} must be an object")
            continue
        if set(case) != REQUIRED_CASE_FIELDS:
            errors.append(f"{label} fields are not closed")
        expected_id = f"APF-{index:03}"
        if case.get("case_id") != expected_id:
            errors.append(f"{label} case_id must be {expected_id}")
        if expected_id in seen_ids:
            errors.append(f"{label} case_id is duplicated")
        seen_ids.add(expected_id)

        category = case.get("category")
        if category not in EXPECTED_DISPOSITIONS:
            errors.append(f"{label} category is unknown")
        elif case.get("expected_disposition") != EXPECTED_DISPOSITIONS[category]:
            errors.append(f"{label} disposition does not fail closed")
        if isinstance(category, str):
            if category in seen_categories:
                errors.append(f"{label} category is duplicated")
            seen_categories.add(category)

        for field in ("setup", "trigger", "source_contract", "source_test"):
            if not valid_bounded_text(case.get(field)):
                errors.append(f"{label} {field} is invalid")
        prohibited = case.get("prohibited_outcomes")
        if (
            not isinstance(prohibited, list)
            or len(prohibited) < 3
            or len(prohibited) != len(set(prohibited))
            or "success" not in prohibited
            or any(not valid_bounded_text(value) for value in prohibited)
        ):
            errors.append(f"{label} prohibited_outcomes are invalid")

        evidence = case.get("evidence")
        if not isinstance(evidence, dict) or set(evidence) != REQUIRED_EVIDENCE_FIELDS:
            errors.append(f"{label} evidence fields are not closed")
            continue
        relative = evidence.get("path")
        expected_sha = evidence.get("sha256")
        if not isinstance(relative, str) or relative.startswith("/") or ".." in Path(relative).parts:
            errors.append(f"{label} evidence path is unsafe")
            continue
        if not isinstance(expected_sha, str) or len(expected_sha) != 64:
            errors.append(f"{label} evidence hash is invalid")
            continue
        evidence_path = ROOT / relative
        if verify_files and (
            not evidence_path.is_file() or sha256_file(evidence_path) != expected_sha
        ):
            errors.append(f"{label} evidence artifact mismatch")

        source_contract = case.get("source_contract")
        source_test = case.get("source_test")
        if verify_files and isinstance(source_contract, str):
            source_path = ROOT / source_contract
            if not source_path.is_file():
                errors.append(f"{label} source contract is missing")
            elif isinstance(source_test, str) and source_test not in source_path.read_text():
                errors.append(f"{label} source test is missing")

    if seen_categories != set(EXPECTED_DISPOSITIONS):
        errors.append("category closure mismatch")
    return errors


def load_corpus(path: Path = DEFAULT_CORPUS) -> object:
    return json.loads(path.read_text())


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--corpus", type=Path, default=DEFAULT_CORPUS)
    args = parser.parse_args()
    errors = validate_corpus(load_corpus(args.corpus))
    if errors:
        for error in errors:
            print(f"ERROR: {error}")
        return 1
    print("Agent failure corpus: pass (6 cases)")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
