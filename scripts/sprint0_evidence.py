#!/usr/bin/env python3
"""Build and verify the public synthetic evidence bundle for Story 0.1."""

from __future__ import annotations

import argparse
import copy
import hashlib
import io
import json
import platform
import subprocess
import sys
import tarfile
import tempfile
from contextlib import contextmanager
from dataclasses import asdict
from pathlib import Path
from typing import Final, Iterator

try:
    from scripts.additions_only import audit_additions_only, load_json
    from scripts.requirement_conflicts import resolve_conflict
except ModuleNotFoundError:
    from additions_only import audit_additions_only, load_json
    from requirement_conflicts import resolve_conflict


ROOT: Final = Path(__file__).resolve().parents[1]
DEFAULT_OUTPUT: Final = ROOT / "artifacts" / "sprints" / "sprint-0" / "story-0.1"
EVIDENCE_DATE: Final = "2026-08-10"
REQUIRED_CONTROLS: Final = tuple(
    [f"SR-GOV-{number:03d}" for number in range(1, 11)]
    + ["SR-SUP-001", "SR-TST-001", "SR-TST-010"]
)
CANONICAL_FILES: Final = (
    "README.md",
    "PRD.md",
    "IMPLEMENTATION-PLAN.md",
    "Agent-Scaffolding-Inventory.md",
    "TASKS.md",
    "SECURITY-REVIEW.md",
    "requirements/registry.json",
    "requirements/normative-map.json",
    "requirements/policy-expectations.json",
    "requirements/additions-only-baseline.json",
    "requirements/security-references.json",
    "requirements/security-reference-baseline.json",
    "requirements/traceability-report.json",
)
RAW_COMMANDS: Final = (
    ("python3", "scripts/requirement_registry.py", "--check"),
    ("python3", "scripts/additions_only.py", "--json"),
    ("python3", "scripts/policy_expectations.py", "--check"),
    ("python3", "scripts/security_references.py", "--json"),
    ("python3", "scripts/traceability_report.py", "--check"),
    ("python3", "scripts/requirement_coverage.py", "--json"),
    (
        "python3",
        "-m",
        "unittest",
        "tests.test_additions_only",
        "tests.test_policy_expectations",
        "tests.test_requirement_conflicts",
        "tests.test_requirement_coverage",
        "tests.test_requirement_registry",
        "tests.test_security_references",
        "tests.test_traceability_report",
    ),
)
ARTIFACT_NAMES: Final = (
    "conflict-report.json",
    "control-map.json",
    "exclusion-diff.json",
    "raw-checker-output.json",
    "reviewer-disposition.json",
    "source-hashes.json",
    "summary.md",
)


class Sprint0EvidenceError(ValueError):
    """Raised when Sprint 0 evidence cannot be built or reconciled."""


def json_bytes(value: object) -> bytes:
    return (json.dumps(value, indent=2, sort_keys=True, ensure_ascii=True) + "\n").encode(
        "utf-8"
    )


def sha256(value: bytes) -> str:
    return hashlib.sha256(value).hexdigest()


@contextmanager
def archived_revision(revision: str) -> Iterator[Path]:
    archive = subprocess.run(
        ["git", "archive", "--format=tar", revision],
        cwd=ROOT,
        check=False,
        capture_output=True,
    )
    if archive.returncode != 0:
        detail = archive.stderr.decode("utf-8", errors="replace").strip()
        raise Sprint0EvidenceError(f"cannot archive source revision {revision}: {detail}")
    with tempfile.TemporaryDirectory() as temp_dir:
        checkout = Path(temp_dir) / "checkout"
        checkout.mkdir()
        with tarfile.open(fileobj=io.BytesIO(archive.stdout), mode="r:") as bundle:
            bundle.extractall(checkout, filter="data")
        yield checkout


def command_result(checkout: Path, command: tuple[str, ...]) -> dict[str, object]:
    result = subprocess.run(
        command,
        cwd=checkout,
        check=False,
        capture_output=True,
        text=True,
    )
    return {
        "command": list(command),
        "exit_code": result.returncode,
        "stdout": result.stdout,
        "stderr": result.stderr,
    }


def summarize_raw_checks(checks: list[dict[str, object]]) -> dict[str, int]:
    passed = sum(check.get("exit_code") == 0 for check in checks)
    failed = sum(check.get("exit_code") != 0 for check in checks)
    return {
        "total": len(checks),
        "passed": passed,
        "failed": failed,
        "skipped": 0,
    }


def raw_checker_output(checkout: Path, source_revision: str) -> dict[str, object]:
    checks = [command_result(checkout, command) for command in RAW_COMMANDS]
    return {
        "schema_version": 1,
        "source_revision": source_revision,
        "environment": {
            "system": platform.system(),
            "release": platform.release(),
            "machine": platform.machine(),
            "python": platform.python_version(),
        },
        "tool_versions": {
            "git": subprocess.run(
                ["git", "--version"], check=True, capture_output=True, text=True
            ).stdout.strip(),
            "node": subprocess.run(
                ["node", "--version"], check=True, capture_output=True, text=True
            ).stdout.strip(),
            "npm": subprocess.run(
                ["npm", "--version"], check=True, capture_output=True, text=True
            ).stdout.strip(),
        },
        "checks": checks,
        "summary": summarize_raw_checks(checks),
    }


def conflict_report(checkout: Path, source_revision: str) -> dict[str, object]:
    policy = load_json(checkout / "requirements" / "conflict-policy.json")
    left = {
        "id": "S-000-FIXTURE-LEFT",
        "allow": {"actions": ["read", "search"], "releases": ["v0.1", "v0.2"]},
        "deny": {"capabilities": ["shell"]},
        "required_controls": ["receipt"],
        "ceilings": {"max_bytes": 1024, "max_network_bytes": 0},
        "exact": {"authority_model": "capability_grant"},
    }
    right = copy.deepcopy(left)
    right["id"] = "S-000-FIXTURE-RIGHT"
    right["allow"] = {"actions": ["read"], "releases": ["v0.2"]}
    right["deny"] = {"capabilities": ["browser"]}
    right["required_controls"] = ["sandbox"]
    right["ceilings"] = {"max_bytes": 512, "max_network_bytes": 0}
    forward = resolve_conflict(left, right, policy)
    reverse = resolve_conflict(right, left, policy)
    return {
        "schema_version": 1,
        "source_revision": source_revision,
        "fixture": {"left": left, "right": right},
        "forward": forward,
        "reverse": reverse,
        "order_invariant": forward == reverse,
    }


def exclusion_diff(checkout: Path, source_revision: str) -> dict[str, object]:
    baseline = load_json(checkout / "requirements" / "additions-only-baseline.json")
    registry = load_json(checkout / "requirements" / "registry.json")
    removed = copy.deepcopy(registry)
    removed_record = removed["requirements"].pop()
    requirement_diagnostics = audit_additions_only(
        baseline,
        removed,
        checkout / "Agent-Scaffolding-Inventory.md",
    )

    inventory_text = (checkout / "Agent-Scaffolding-Inventory.md").read_text(
        encoding="utf-8"
    )
    protected_line = next(
        line for line in inventory_text.splitlines() if line.startswith("- [ ] `DEFER`")
    )
    with tempfile.TemporaryDirectory() as temp_dir:
        mutated_inventory = Path(temp_dir) / "Agent-Scaffolding-Inventory.md"
        mutated_inventory.write_text(
            inventory_text.replace(protected_line, "", 1),
            encoding="utf-8",
        )
        exclusion_diagnostics = audit_additions_only(
            baseline,
            registry,
            mutated_inventory,
        )
    return {
        "schema_version": 1,
        "source_revision": source_revision,
        "fixtures": {
            "removed_requirement_id": removed_record["id"],
            "removed_exclusion_statement": protected_line,
        },
        "requirement_diagnostics": [asdict(item) for item in requirement_diagnostics],
        "exclusion_diagnostics": [asdict(item) for item in exclusion_diagnostics],
        "blocked": bool(requirement_diagnostics and exclusion_diagnostics),
    }


def control_map(source_revision: str) -> dict[str, object]:
    status = {
        "SR-GOV-001": ("planning_scope_pass", ["PRD.md", "requirements/registry.json"]),
        "SR-GOV-002": (
            "planning_scope_pass",
            ["PRD.md", "requirements/policy-expectations.json"],
        ),
        "SR-GOV-003": ("planning_scope_pass", ["scripts/validate_docs.py"]),
        "SR-GOV-004": (
            "foundation_only",
            ["SECURITY.md", "schemas/planning/release-manifest.schema.json"],
        ),
        "SR-GOV-005": (
            "foundation_only",
            ["schemas/planning/risk-register.schema.json"],
        ),
        "SR-GOV-006": (
            "planning_scope_pass",
            ["RUNTIME-BOUNDARIES.md", "PRD.md"],
        ),
        "SR-GOV-007": ("planning_scope_pass", ["SECURITY-REVIEW.md"]),
        "SR-GOV-008": (
            "planning_scope_pass",
            ["requirements/policy-expectations.json", "exclusion-diff.json"],
        ),
        "SR-GOV-009": (
            "planning_scope_pass",
            ["requirements/security-references.json", "source-hashes.json"],
        ),
        "SR-GOV-010": (
            "foundation_only",
            ["requirements/security-reference-baseline.json", "conflict-report.json"],
        ),
        "SR-SUP-001": (
            "planning_scope_pass",
            ["requirements/security-references.json", "control-map.json"],
        ),
        "SR-TST-001": (
            "foundation_only",
            ["raw-checker-output.json", "TASKS.md"],
        ),
        "SR-TST-010": (
            "planning_scope_pass",
            ["raw-checker-output.json", "evidence-manifest.json"],
        ),
    }
    return {
        "schema_version": 1,
        "source_revision": source_revision,
        "scope": "Story 0.1 planning and traceability controls only",
        "controls": [
            {
                "id": identifier,
                "status": status[identifier][0],
                "evidence": status[identifier][1],
                "release_control_satisfied": False,
            }
            for identifier in REQUIRED_CONTROLS
        ],
    }


def reviewer_disposition(source_revision: str) -> dict[str, object]:
    return {
        "schema_version": 1,
        "source_revision": source_revision,
        "review_date": EVIDENCE_DATE,
        "scope": "Story 0.1 canonical scope and traceability baseline",
        "reviewer_type": "automated implementation self-review",
        "independent_review_performed": False,
        "disposition": "pass_for_sprint_0_planning_scope",
        "release_approval": False,
        "blocking_findings": [],
        "limitations": [
            "This disposition covers planning artifacts and their deterministic checks only.",
            "Foundation-only controls require later implementation and release evidence.",
            "No independent reviewer has reproduced this bundle.",
            "This is not product certification, publisher endorsement, or deployment approval.",
        ],
    }


def source_hashes(checkout: Path, source_revision: str) -> dict[str, object]:
    files = []
    for relative in CANONICAL_FILES:
        content = (checkout / relative).read_bytes()
        files.append({"path": relative, "sha256": sha256(content), "size": len(content)})
    return {
        "schema_version": 1,
        "source_revision": source_revision,
        "files": files,
    }


def summary_markdown(
    source_revision: str,
    raw: dict[str, object],
    controls: dict[str, object],
    disposition: dict[str, object],
) -> bytes:
    control_counts: dict[str, int] = {}
    for control in controls["controls"]:
        state = str(control["status"])
        control_counts[state] = control_counts.get(state, 0) + 1
    text = f"""# Story 0.1 Evidence Summary

| Field | Value |
|---|---|
| Source revision | `{source_revision}` |
| Evidence date | {EVIDENCE_DATE} |
| Scope | Story 0.1 planning and traceability baseline |
| Checker results | {raw['summary']['passed']} passed, {raw['summary']['failed']} failed, {raw['summary']['skipped']} skipped |
| Control mapping | {len(controls['controls'])} controls: {control_counts.get('planning_scope_pass', 0)} planning-scope pass, {control_counts.get('foundation_only', 0)} foundation only |
| Disposition | `{disposition['disposition']}` |
| Independent review | Not performed |
| Product release approval | No |

The raw checker output is authoritative over this summary. Every listed artifact is hash-pinned by `evidence-manifest.json`. Foundation-only controls remain open for their owning implementation and release sprints.
"""
    return text.encode("utf-8")


def build_bundle(source_revision: str) -> dict[str, bytes]:
    with archived_revision(source_revision) as checkout:
        raw = raw_checker_output(checkout, source_revision)
        conflict = conflict_report(checkout, source_revision)
        exclusion = exclusion_diff(checkout, source_revision)
        controls = control_map(source_revision)
        disposition = reviewer_disposition(source_revision)
        hashes = source_hashes(checkout, source_revision)
    return {
        "raw-checker-output.json": json_bytes(raw),
        "conflict-report.json": json_bytes(conflict),
        "exclusion-diff.json": json_bytes(exclusion),
        "control-map.json": json_bytes(controls),
        "reviewer-disposition.json": json_bytes(disposition),
        "source-hashes.json": json_bytes(hashes),
        "summary.md": summary_markdown(source_revision, raw, controls, disposition),
    }


def build_manifest(source_revision: str, bundle: dict[str, bytes]) -> dict[str, object]:
    return {
        "schema_version": 1,
        "bundle_id": "sprint-0-story-0.1-product-security-evidence",
        "evidence_date": EVIDENCE_DATE,
        "source_revision": source_revision,
        "scope": "Story 0.1 planning and traceability baseline",
        "files": [
            {"path": name, "sha256": sha256(bundle[name]), "size": len(bundle[name])}
            for name in ARTIFACT_NAMES
        ],
    }


def write_bundle(output: Path, source_revision: str) -> None:
    if output.exists():
        raise Sprint0EvidenceError(f"refusing to overwrite existing evidence: {output}")
    bundle = build_bundle(source_revision)
    output.mkdir(parents=True)
    for name, content in bundle.items():
        (output / name).write_bytes(content)
    (output / "evidence-manifest.json").write_bytes(
        json_bytes(build_manifest(source_revision, bundle))
    )


def check_bundle(output: Path = DEFAULT_OUTPUT) -> list[str]:
    failures: list[str] = []
    try:
        manifest = load_json(output / "evidence-manifest.json")
    except (OSError, json.JSONDecodeError, ValueError) as error:
        return [f"cannot load evidence manifest: {error}"]
    expected_files = list(ARTIFACT_NAMES)
    entries = manifest.get("files")
    if not isinstance(entries, list):
        return ["evidence manifest files must be an array"]
    if [entry.get("path") for entry in entries if isinstance(entry, dict)] != expected_files:
        failures.append("evidence manifest file order or membership is invalid")
    for entry in entries:
        if not isinstance(entry, dict):
            failures.append("evidence manifest contains a malformed file entry")
            continue
        path = output / str(entry.get("path"))
        try:
            content = path.read_bytes()
        except OSError as error:
            failures.append(f"cannot read evidence artifact {path.name}: {error}")
            continue
        if entry.get("sha256") != sha256(content):
            failures.append(f"evidence hash mismatch: {path.name}")
        if entry.get("size") != len(content):
            failures.append(f"evidence size mismatch: {path.name}")

    try:
        raw = load_json(output / "raw-checker-output.json")
        checks = raw.get("checks")
        if not isinstance(checks, list) or raw.get("summary") != summarize_raw_checks(checks):
            failures.append("raw checker summary does not reconcile")
        elif raw["summary"]["failed"] or raw["summary"]["skipped"]:
            failures.append("raw checker evidence contains a failed or skipped check")
        conflict = load_json(output / "conflict-report.json")
        if conflict.get("order_invariant") is not True or conflict.get("forward") != conflict.get("reverse"):
            failures.append("conflict evidence is not order-invariant")
        exclusion = load_json(output / "exclusion-diff.json")
        if exclusion.get("blocked") is not True:
            failures.append("exclusion evidence does not show both mutations blocked")
        controls = load_json(output / "control-map.json")
        control_ids = [item.get("id") for item in controls.get("controls", [])]
        if control_ids != list(REQUIRED_CONTROLS):
            failures.append("product-security control mapping is incomplete or reordered")
        disposition = load_json(output / "reviewer-disposition.json")
        if (
            disposition.get("independent_review_performed") is not False
            or disposition.get("release_approval") is not False
            or disposition.get("disposition") != "pass_for_sprint_0_planning_scope"
        ):
            failures.append("reviewer disposition overstates the evidence scope")
    except (OSError, json.JSONDecodeError, ValueError, TypeError, KeyError) as error:
        failures.append(f"cannot reconcile evidence bundle: {error}")
    return failures


def main(argv: list[str] | None = None) -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--output", type=Path, default=DEFAULT_OUTPUT)
    parser.add_argument("--source-revision")
    parser.add_argument("--write", action="store_true")
    args = parser.parse_args(argv)
    try:
        if args.write:
            if not args.source_revision:
                raise Sprint0EvidenceError("--write requires --source-revision")
            write_bundle(args.output, args.source_revision)
            return 0
        failures = check_bundle(args.output)
    except (OSError, tarfile.TarError, Sprint0EvidenceError) as error:
        print(f"Sprint 0 evidence error: {error}", file=sys.stderr)
        return 1
    if failures:
        print("Sprint 0 evidence validation failed:", file=sys.stderr)
        for failure in failures:
            print(f"- {failure}", file=sys.stderr)
        return 1
    print(f"Validated Story 0.1 evidence bundle at {args.output.relative_to(ROOT)}.")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
