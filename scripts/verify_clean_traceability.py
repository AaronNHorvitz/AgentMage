#!/usr/bin/env python3
"""Verify requirement traceability from an isolated Git archive."""

from __future__ import annotations

import argparse
import io
import json
import subprocess
import sys
import tarfile
import tempfile
from pathlib import Path
from typing import Final


ROOT: Final = Path(__file__).resolve().parents[1]
OFFLINE_COMMANDS: Final = (
    ("python3", "scripts/requirement_registry.py", "--check"),
    ("python3", "scripts/additions_only.py"),
    ("python3", "scripts/policy_expectations.py", "--check"),
    ("python3", "scripts/security_references.py"),
    ("python3", "scripts/traceability_report.py", "--check"),
    ("python3", "scripts/requirement_coverage.py", "--json"),
)


class CleanTraceabilityError(ValueError):
    """Raised when an isolated traceability verification cannot close."""


def load_object(path: Path) -> dict[str, object]:
    value = json.loads(path.read_text(encoding="utf-8"))
    if not isinstance(value, dict):
        raise CleanTraceabilityError(f"{path}: expected a JSON object")
    return value


def verify_traceability_closure(
    report: dict[str, object],
    normative_map: dict[str, object],
) -> dict[str, int]:
    """Verify every normative statement's complete planning chain."""
    requirements = report.get("requirements")
    mappings = normative_map.get("mappings")
    if not isinstance(requirements, list) or not isinstance(mappings, list):
        raise CleanTraceabilityError("traceability requirements and mappings must be arrays")
    if any(not isinstance(record, dict) for record in requirements):
        raise CleanTraceabilityError("traceability requirement records must be objects")
    by_id = {str(record.get("id")): record for record in requirements}
    if len(by_id) != len(requirements):
        raise CleanTraceabilityError("traceability report contains duplicate identities")

    mapped_hashes: set[str] = set()
    resolved_links = 0
    for index, mapping in enumerate(mappings):
        if not isinstance(mapping, dict):
            raise CleanTraceabilityError(f"normative mapping {index} is not an object")
        statement_hash = mapping.get("statement_sha256")
        requirement_ids = mapping.get("requirement_ids")
        if not isinstance(statement_hash, str) or not statement_hash:
            raise CleanTraceabilityError(f"normative mapping {index} has no statement hash")
        if statement_hash in mapped_hashes:
            raise CleanTraceabilityError(f"duplicate normative statement: {statement_hash}")
        mapped_hashes.add(statement_hash)
        if not isinstance(requirement_ids, list) or not requirement_ids:
            raise CleanTraceabilityError(
                f"normative statement {statement_hash} has no requirement"
            )

        for requirement_id in requirement_ids:
            requirement = by_id.get(str(requirement_id))
            if requirement is None:
                raise CleanTraceabilityError(
                    f"normative statement {statement_hash} has orphan requirement "
                    f"{requirement_id}"
                )
            if requirement.get("kind") != "product_requirement":
                raise CleanTraceabilityError(
                    f"normative statement {statement_hash} maps to non-product "
                    f"requirement {requirement_id}"
                )
            source = requirement.get("source")
            if (
                not isinstance(source, dict)
                or not source.get("heading")
                or not isinstance(source.get("line"), int)
                or not source.get("definition_sha256")
            ):
                raise CleanTraceabilityError(f"{requirement_id} has no exact source anchor")
            if not requirement.get("release") or not requirement.get("status"):
                raise CleanTraceabilityError(f"{requirement_id} has no release or status")

            implementation = requirement.get("implementation")
            planning_items = (
                implementation.get("planning_items")
                if isinstance(implementation, dict)
                else None
            )
            if not isinstance(planning_items, list) or not planning_items:
                raise CleanTraceabilityError(f"{requirement_id} has no planned issue")
            if any(
                not isinstance(item, dict)
                or not item.get("story_id")
                or not isinstance(item.get("sprint"), int)
                for item in planning_items
            ):
                raise CleanTraceabilityError(
                    f"{requirement_id} has a malformed planned issue"
                )

            acceptance_tests = requirement.get("acceptance_tests")
            if not isinstance(acceptance_tests, list) or not acceptance_tests:
                raise CleanTraceabilityError(f"{requirement_id} has no acceptance test")
            for test_id in acceptance_tests:
                test_record = by_id.get(str(test_id))
                if test_record is None or test_record.get("kind") != "acceptance_test":
                    raise CleanTraceabilityError(
                        f"{requirement_id} has orphan acceptance test {test_id}"
                    )

            evidence = requirement.get("evidence")
            if (
                not isinstance(evidence, dict)
                or not evidence.get("status")
                or not isinstance(evidence.get("expected_roots"), list)
                or not evidence["expected_roots"]
                or not isinstance(evidence.get("paths"), list)
            ):
                raise CleanTraceabilityError(f"{requirement_id} has no evidence state")
            reverse_hashes = {
                item.get("statement_sha256")
                for item in requirement.get("normative_statements", [])
                if isinstance(item, dict)
            }
            if statement_hash not in reverse_hashes:
                raise CleanTraceabilityError(
                    f"{requirement_id} is missing reverse link to {statement_hash}"
                )
            resolved_links += 1

    report_hashes = {
        item.get("statement_sha256")
        for requirement in requirements
        for item in (
            requirement.get("normative_statements", [])
            if isinstance(requirement, dict)
            else []
        )
        if isinstance(item, dict)
    }
    if report_hashes != mapped_hashes:
        missing = sorted(mapped_hashes - report_hashes)
        orphaned = sorted(report_hashes - mapped_hashes)
        raise CleanTraceabilityError(
            f"normative reverse-link mismatch; missing={missing}, orphaned={orphaned}"
        )
    return {
        "normative_statements": len(mapped_hashes),
        "resolved_requirement_links": resolved_links,
        "traceability_records": len(requirements),
    }


def verify_clean_checkout(revision: str = "HEAD", root: Path = ROOT) -> dict[str, object]:
    """Archive a Git tree, run offline gates, and verify traceability closure."""
    archive = subprocess.run(
        ["git", "archive", "--format=tar", revision],
        cwd=root,
        check=False,
        capture_output=True,
    )
    if archive.returncode != 0:
        message = archive.stderr.decode("utf-8", errors="replace").strip()
        raise CleanTraceabilityError(f"cannot archive revision {revision}: {message}")

    command_results: list[dict[str, object]] = []
    with tempfile.TemporaryDirectory() as temp_dir:
        checkout = Path(temp_dir) / "checkout"
        checkout.mkdir()
        with tarfile.open(fileobj=io.BytesIO(archive.stdout), mode="r:") as bundle:
            bundle.extractall(checkout, filter="data")

        for command in OFFLINE_COMMANDS:
            result = subprocess.run(
                command,
                cwd=checkout,
                check=False,
                capture_output=True,
                text=True,
            )
            command_results.append(
                {
                    "command": " ".join(command),
                    "exit_code": result.returncode,
                }
            )
            if result.returncode != 0:
                detail = (result.stderr or result.stdout).strip()
                raise CleanTraceabilityError(
                    f"clean-checkout command failed ({' '.join(command)}): {detail}"
                )

        closure = verify_traceability_closure(
            load_object(checkout / "requirements" / "traceability-report.json"),
            load_object(checkout / "requirements" / "normative-map.json"),
        )

    return {
        "schema_version": 1,
        "ok": True,
        "revision": revision,
        "commands": command_results,
        "closure": closure,
    }


def main(argv: list[str] | None = None) -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--revision", default="HEAD")
    parser.add_argument("--json", action="store_true", dest="json_output")
    args = parser.parse_args(argv)
    try:
        report = verify_clean_checkout(args.revision)
    except (OSError, tarfile.TarError, CleanTraceabilityError) as error:
        print(f"Clean traceability verification failed: {error}", file=sys.stderr)
        return 1
    if args.json_output:
        print(json.dumps(report, indent=2, sort_keys=True))
    else:
        closure = report["closure"]
        print(
            f"Verified {closure['normative_statements']} normative statements and "
            f"{closure['resolved_requirement_links']} requirement links from "
            f"clean Git revision {args.revision}."
        )
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
