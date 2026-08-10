#!/usr/bin/env python3
"""Build and verify the public synthetic evidence bundle for Story 0.2."""

from __future__ import annotations

import argparse
import json
import platform
import re
import subprocess
import sys
import tarfile
from pathlib import Path
from typing import Final

try:
    from scripts.additions_only import load_json
    from scripts.sprint0_evidence import (
        archived_revision,
        json_bytes,
        sha256,
        summarize_raw_checks,
    )
except ModuleNotFoundError:
    from additions_only import load_json
    from sprint0_evidence import (
        archived_revision,
        json_bytes,
        sha256,
        summarize_raw_checks,
    )


ROOT: Final = Path(__file__).resolve().parents[1]
DEFAULT_OUTPUT: Final = ROOT / "artifacts" / "sprints" / "sprint-0" / "story-0.2"
EVIDENCE_DATE: Final = "2026-08-10"
REQUIRED_CONTROLS: Final = (
    "SR-GOV-003",
    "SR-GOV-004",
    "SR-GOV-006",
    "SR-GOV-009",
    "SR-GOV-010",
    "SR-SUP-001",
    "SR-SUP-003",
    "SR-SUP-006",
    "SR-SUP-010",
    "SR-TST-001",
)
POLICY_FILES: Final = (
    "LICENSE",
    "SECURITY.md",
    "MODEL-PROVENANCE-POLICY.md",
    "RUNTIME-BOUNDARIES.md",
    "docs/decisions/0001-product-security-and-runtime-baseline.md",
    "README.md",
    "PRD.md",
    "IMPLEMENTATION-PLAN.md",
    "Agent-Scaffolding-Inventory.md",
    "TASKS.md",
    "SECURITY-REVIEW.md",
    "package.json",
    "package-lock.json",
    ".github/workflows/documentation.yml",
)
RAW_COMMANDS: Final = (
    ("npm", "ci", "--ignore-scripts"),
    ("npm", "run", "docs:lint"),
    ("npm", "run", "docs:mermaid"),
    ("python3", "scripts/validate_docs.py"),
    (
        "python3",
        "-m",
        "unittest",
        "-v",
        "tests.test_documentation_controls",
        "tests.test_documentation_mutations",
        "tests.test_public_policy_baseline",
    ),
)
ARTIFACT_NAMES: Final = (
    "control-map.json",
    "decision-record.json",
    "policy-hashes.json",
    "raw-checker-output.json",
    "reviewer-disposition.json",
    "summary.md",
    "workflow-identity.json",
)


class Story02EvidenceError(ValueError):
    """Raised when Story 0.2 evidence cannot be built or reconciled."""


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


def policy_hashes(checkout: Path, source_revision: str) -> dict[str, object]:
    files = []
    for relative in POLICY_FILES:
        content = (checkout / relative).read_bytes()
        files.append({"path": relative, "sha256": sha256(content), "size": len(content)})
    return {
        "schema_version": 1,
        "source_revision": source_revision,
        "files": files,
    }


def workflow_identity(checkout: Path, source_revision: str) -> dict[str, object]:
    workflow_path = checkout / ".github" / "workflows" / "documentation.yml"
    workflow = workflow_path.read_text(encoding="utf-8")
    package = load_json(checkout / "package.json")
    actions = re.findall(r"^\s*uses:\s*([^\s]+)$", workflow, re.MULTILINE)
    return {
        "schema_version": 1,
        "source_revision": source_revision,
        "path": ".github/workflows/documentation.yml",
        "sha256": sha256(workflow.encode("utf-8")),
        "runner": "ubuntu-24.04",
        "permissions": {"contents": "read"},
        "actions": actions,
        "actions_immutable": bool(actions)
        and all(re.fullmatch(r"[^@\s]+@[0-9a-f]{40}", item) for item in actions),
        "install_command": "npm ci --ignore-scripts",
        "gate_command": "npm run docs:clean-check",
        "gate_script": package["scripts"]["docs:clean-check"],
        "lockfile_sha256": sha256((checkout / "package-lock.json").read_bytes()),
    }


def decision_record(checkout: Path, source_revision: str) -> dict[str, object]:
    relative = "docs/decisions/0001-product-security-and-runtime-baseline.md"
    content = (checkout / relative).read_bytes()
    text = content.decode("utf-8")
    numbered_items = re.findall(r"^(\d+)\. ", text, re.MULTILINE)
    return {
        "schema_version": 1,
        "source_revision": source_revision,
        "path": relative,
        "sha256": sha256(content),
        "size": len(content),
        "decision_id": "0001",
        "status": "accepted" if "| Status | Accepted |" in text else "unknown",
        "date": "2026-08-10" if "| Date | 2026-08-10 |" in text else "unknown",
        "numbered_decisions": numbered_items,
    }


def control_map(source_revision: str) -> dict[str, object]:
    mapping = {
        "SR-GOV-003": ("policy_scope_pass", ["scripts/validate_docs.py", "raw-checker-output.json"]),
        "SR-GOV-004": ("policy_scope_pass", ["SECURITY.md", "decision-record.json"]),
        "SR-GOV-006": ("policy_scope_pass", ["RUNTIME-BOUNDARIES.md", "policy-hashes.json"]),
        "SR-GOV-009": ("policy_scope_pass", ["policy-hashes.json", "workflow-identity.json"]),
        "SR-GOV-010": ("policy_scope_pass", ["docs/decisions/0001-product-security-and-runtime-baseline.md"]),
        "SR-SUP-001": ("policy_scope_pass", ["SECURITY-REVIEW.md", "control-map.json"]),
        "SR-SUP-003": ("policy_scope_pass", ["package-lock.json", "workflow-identity.json"]),
        "SR-SUP-006": ("foundation_only", ["MODEL-PROVENANCE-POLICY.md"]),
        "SR-SUP-010": ("policy_scope_pass", ["SECURITY.md", "reviewer-disposition.json"]),
        "SR-TST-001": ("policy_scope_pass", ["raw-checker-output.json", "TASKS.md"]),
    }
    return {
        "schema_version": 1,
        "source_revision": source_revision,
        "scope": "Story 0.2 public policy and documentation controls only",
        "controls": [
            {
                "id": identifier,
                "status": mapping[identifier][0],
                "evidence": mapping[identifier][1],
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
        "scope": "Story 0.2 public policy and documentation controls",
        "reviewer_type": "automated implementation self-review",
        "independent_review_performed": False,
        "disposition": "pass_for_story_0_2_policy_scope",
        "release_approval": False,
        "blocking_findings": [],
        "limitations": [
            "This disposition covers public policy and documentation controls only.",
            "Supplier due diligence and product-runtime controls require later evidence.",
            "No independent reviewer has reproduced this bundle.",
            "This is not product certification, publisher endorsement, or deployment approval.",
        ],
    }


def summary_markdown(
    source_revision: str,
    raw: dict[str, object],
    controls: dict[str, object],
    disposition: dict[str, object],
) -> bytes:
    policy_pass = sum(
        item["status"] == "policy_scope_pass" for item in controls["controls"]
    )
    foundation_only = sum(
        item["status"] == "foundation_only" for item in controls["controls"]
    )
    text = f"""# Story 0.2 Evidence Summary

| Field | Value |
|---|---|
| Source revision | `{source_revision}` |
| Evidence date | {EVIDENCE_DATE} |
| Scope | Story 0.2 public policy and documentation controls |
| Checker results | {raw['summary']['passed']} passed, {raw['summary']['failed']} failed, {raw['summary']['skipped']} skipped |
| Control mapping | {len(controls['controls'])} controls: {policy_pass} policy-scope pass, {foundation_only} foundation only |
| Disposition | `{disposition['disposition']}` |
| Independent review | Not performed |
| Product release approval | No |

The raw checker output is authoritative over this summary. Every listed artifact is hash-pinned by `evidence-manifest.json`. Product-runtime, supplier, independent-review, and release controls remain open for their owning sprints.
"""
    return text.encode("utf-8")


def build_bundle(source_revision: str) -> dict[str, bytes]:
    with archived_revision(source_revision) as checkout:
        raw = raw_checker_output(checkout, source_revision)
        controls = control_map(source_revision)
        disposition = reviewer_disposition(source_revision)
        decision = decision_record(checkout, source_revision)
        hashes = policy_hashes(checkout, source_revision)
        workflow = workflow_identity(checkout, source_revision)
    return {
        "control-map.json": json_bytes(controls),
        "decision-record.json": json_bytes(decision),
        "policy-hashes.json": json_bytes(hashes),
        "raw-checker-output.json": json_bytes(raw),
        "reviewer-disposition.json": json_bytes(disposition),
        "summary.md": summary_markdown(source_revision, raw, controls, disposition),
        "workflow-identity.json": json_bytes(workflow),
    }


def build_manifest(source_revision: str, bundle: dict[str, bytes]) -> dict[str, object]:
    return {
        "schema_version": 1,
        "bundle_id": "sprint-0-story-0.2-product-security-evidence",
        "evidence_date": EVIDENCE_DATE,
        "source_revision": source_revision,
        "scope": "Story 0.2 public policy and documentation controls",
        "files": [
            {"path": name, "sha256": sha256(bundle[name]), "size": len(bundle[name])}
            for name in ARTIFACT_NAMES
        ],
    }


def write_bundle(output: Path, source_revision: str) -> None:
    if output.exists():
        raise Story02EvidenceError(f"refusing to overwrite existing evidence: {output}")
    bundle = build_bundle(source_revision)
    output.mkdir(parents=True)
    for name, content in bundle.items():
        (output / name).write_bytes(content)
    (output / "evidence-manifest.json").write_bytes(
        json_bytes(build_manifest(source_revision, bundle))
    )


def revision_content(revision: str, relative: str) -> bytes:
    result = subprocess.run(
        ["git", "show", f"{revision}:{relative}"],
        cwd=ROOT,
        check=False,
        capture_output=True,
    )
    if result.returncode != 0:
        raise Story02EvidenceError(f"cannot read {relative} from source revision")
    return result.stdout


def check_bundle(output: Path = DEFAULT_OUTPUT) -> list[str]:
    failures: list[str] = []
    try:
        manifest = load_json(output / "evidence-manifest.json")
    except (OSError, json.JSONDecodeError, ValueError) as error:
        return [f"cannot load evidence manifest: {error}"]
    entries = manifest.get("files")
    if not isinstance(entries, list):
        return ["evidence manifest files must be an array"]
    if [item.get("path") for item in entries if isinstance(item, dict)] != list(ARTIFACT_NAMES):
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
        source_revision = str(manifest["source_revision"])
        raw = load_json(output / "raw-checker-output.json")
        checks = raw.get("checks")
        if not isinstance(checks, list) or raw.get("summary") != summarize_raw_checks(checks):
            failures.append("raw checker summary does not reconcile")
        elif raw["summary"]["failed"] or raw["summary"]["skipped"]:
            failures.append("raw checker evidence contains a failed or skipped check")

        hashes = load_json(output / "policy-hashes.json")
        if [item.get("path") for item in hashes.get("files", [])] != list(POLICY_FILES):
            failures.append("policy hash membership is incomplete or reordered")
        for item in hashes.get("files", []):
            content = revision_content(source_revision, str(item["path"]))
            if item.get("sha256") != sha256(content) or item.get("size") != len(content):
                failures.append(f"policy source mismatch: {item.get('path')}")

        workflow = load_json(output / "workflow-identity.json")
        if workflow.get("actions_immutable") is not True:
            failures.append("workflow actions are not immutably identified")
        if workflow.get("gate_command") != "npm run docs:clean-check":
            failures.append("workflow does not identify the canonical clean gate")

        decision = load_json(output / "decision-record.json")
        if decision.get("status") != "accepted" or decision.get("numbered_decisions") != [
            str(number) for number in range(1, 13)
        ]:
            failures.append("accepted decision evidence is incomplete")

        controls = load_json(output / "control-map.json")
        if [item.get("id") for item in controls.get("controls", [])] != list(REQUIRED_CONTROLS):
            failures.append("product-security control mapping is incomplete or reordered")
        disposition = load_json(output / "reviewer-disposition.json")
        if (
            disposition.get("independent_review_performed") is not False
            or disposition.get("release_approval") is not False
            or disposition.get("disposition") != "pass_for_story_0_2_policy_scope"
        ):
            failures.append("reviewer disposition overstates the evidence scope")
    except (OSError, json.JSONDecodeError, ValueError, TypeError, KeyError, Story02EvidenceError) as error:
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
                raise Story02EvidenceError("--write requires --source-revision")
            write_bundle(args.output, args.source_revision)
            return 0
        failures = check_bundle(args.output)
    except (OSError, tarfile.TarError, Story02EvidenceError) as error:
        print(f"Story 0.2 evidence error: {error}", file=sys.stderr)
        return 1
    if failures:
        print("Story 0.2 evidence validation failed:", file=sys.stderr)
        for failure in failures:
            print(f"- {failure}", file=sys.stderr)
        return 1
    print(f"Validated Story 0.2 evidence bundle at {args.output.relative_to(ROOT)}.")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
