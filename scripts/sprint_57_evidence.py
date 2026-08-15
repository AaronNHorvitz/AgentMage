#!/usr/bin/env python3
"""Build and validate truthful locally executable Sprint 57 evidence."""

from __future__ import annotations

import argparse
import hashlib
import json
import platform
import re
import shutil
import subprocess
from pathlib import Path
from typing import Any, Final


ROOT: Final = Path(__file__).resolve().parents[1]
OUTPUT: Final = ROOT / "artifacts/sprints/sprint-57/local-evidence-report.json"
REVISION: Final = re.compile(r"^[0-9a-f]{40}$")
SHA256: Final = re.compile(r"^[0-9a-f]{64}$")
IGNORED_TESTS: Final = re.compile(
    rb"test result: (?:ok|FAILED)\. \d+ passed; \d+ failed; (\d+) ignored;"
)
SOURCE_PATHS: Final = (
    "kernel/contracts/src/display_link.rs",
    "kernel/contracts/src/lib.rs",
    "capabilities/knowledge/src/markdown_write.rs",
    "capabilities/knowledge/src/markdown_artifacts.rs",
    "capabilities/knowledge/src/markdown_artifact_skills.rs",
    "capabilities/knowledge/src/lib.rs",
    "capabilities/knowledge/examples/markdown_artifact_skill_pack.rs",
    "schemas/runtime/markdown-quality-report.schema.json",
    "schemas/runtime/examples/markdown-quality-report.valid.json",
    "schemas/runtime/generated-markdown-artifact.schema.json",
    "schemas/runtime/examples/generated-markdown-artifact.valid.json",
    "schemas/runtime/markdown-round-trip-result.schema.json",
    "schemas/runtime/examples/markdown-round-trip-result.valid.json",
    "scripts/validate_planning_schemas.mjs",
    "tests/test_planning_schemas.mjs",
    "docs/verification/sprint-57-markdown-artifact-corpus.json",
    "scripts/markdown_artifact_contract.py",
    "tests/test_markdown_artifact_contract.py",
    "scripts/markdown_artifact_skill_contract.py",
    "tests/test_markdown_artifact_skill_contract.py",
    "artifacts/sprints/sprint-57/markdown-artifact-skill-pack.json",
    "docs/architecture/markdown-artifacts-and-round-trip.md",
    "docs/guides/markdown-artifact-local-workflows.md",
    "docs/guides/skills.md",
    "docs/verification/sprint-57-local-results.md",
    "scripts/sprint_57_evidence.py",
    "tests/test_sprint_57_evidence.py",
)
COMMANDS: Final = (
    (
        "markdown-writer-unit",
        (
            "cargo", "test", "-p", "agentmage-capability-knowledge",
            "markdown_write::tests", "--lib", "--locked",
        ),
    ),
    (
        "markdown-artifact-unit",
        (
            "cargo", "test", "-p", "agentmage-capability-knowledge",
            "markdown_artifacts::tests", "--lib", "--locked",
        ),
    ),
    (
        "markdown-skill-unit",
        (
            "cargo", "test", "-p", "agentmage-capability-knowledge",
            "markdown_artifact_skills::tests", "--lib", "--locked",
        ),
    ),
    (
        "display-link-unit",
        (
            "cargo", "test", "-p", "agentmage-kernel-contracts",
            "display_link::tests", "--lib", "--locked",
        ),
    ),
    ("markdown-runtime-schemas", ("node", "--test", "tests/test_planning_schemas.mjs")),
    (
        "markdown-acceptance-corpus",
        ("python3", "-m", "unittest", "tests.test_markdown_artifact_contract"),
    ),
    ("markdown-skill-contract", ("python3", "scripts/markdown_artifact_skill_contract.py")),
    (
        "markdown-strict-clippy",
        (
            "cargo", "clippy", "-p", "agentmage-kernel-contracts",
            "-p", "agentmage-capability-knowledge", "--all-targets", "--locked",
            "--", "-D", "warnings",
        ),
    ),
    ("documentation-gate", ("npm", "run", "docs:check")),
    ("product-ci-contract", ("python3", "scripts/product_ci.py", "--check")),
    ("supply-chain", ("python3", "scripts/supply_chain.py")),
    ("evidence-tests", ("python3", "-m", "unittest", "tests.test_sprint_57_evidence")),
)
FOCUSED_COMMANDS: Final = tuple(item[0] for item in COMMANDS[:7])
RUST_FOCUSED_COMMANDS: Final = {
    "markdown-writer-unit",
    "markdown-artifact-unit",
    "markdown-skill-unit",
    "display-link-unit",
}
SECURITY_REQUIREMENTS: Final = [
    "SR-DAT-002",
    "SR-DAT-003",
    "SR-AI-010",
    "SR-TST-002",
    "SR-TST-004",
    "SR-CIV-006",
    "SR-CIV-007",
    "SR-CIV-008",
    "SR-CIV-009",
]
IMPLEMENTED: Final = {
    "parser_element_kind_count": 11,
    "fidelity_warning_kind_count": 4,
    "quality_finding_kind_count": 14,
    "artifact_kind_count": 7,
    "runtime_schema_count": 3,
    "declarative_skill_count": 7,
    "acceptance_corpus_case_count": 68,
    "byte_preserving_parse": True,
    "lf_and_crlf_preservation": True,
    "exact_source_ranges": True,
    "protected_code_and_raw_notes": True,
    "exact_scoped_edit_previews": True,
    "unknown_acronym_restraint": True,
    "citation_bound_factual_statements": True,
    "display_only_file_line_links": True,
    "inert_hostile_content": True,
    "byte_semantic_rendered_round_trip": True,
    "network_access_capability": False,
    "execution_capability": False,
    "filesystem_mutation_capability": False,
    "renderer_capability": False,
    "citation_invention_capability": False,
    "product_coordinator": False,
    "controlled_writer_integration": False,
    "native_interface_integration": False,
    "accessibility_acceptance": False,
    "installed_cross_platform_acceptance": False,
    "trusted_package_execution": False,
    "independent_content_review": False,
    "manual_fuzzing": False,
}
BLOCKERS: Final = [
    {"code": "UPSTREAM-SPRINT-56-BLOCKED", "owner": "57.1"},
    {"code": "MARKDOWN-PRODUCT-COORDINATOR-ABSENT", "owner": "57.1.3.4"},
    {"code": "CONTROLLED-WRITER-INTEGRATION-ABSENT", "owner": "57.1.3.4"},
    {"code": "LOCAL-RENDERER-INTEGRATION-ABSENT", "owner": "57.1.3.4"},
    {"code": "NATIVE-INTERFACE-INTEGRATION-ABSENT", "owner": "57.1.3.4"},
    {"code": "ACCESSIBILITY-ACCEPTANCE-ABSENT", "owner": "57.1.3.5"},
    {"code": "INSTALLED-CROSS-PLATFORM-ACCEPTANCE-ABSENT", "owner": "57.1.3.5"},
    {"code": "TRUSTED-INSTALLED-PACKAGE-EXECUTION-ABSENT", "owner": "57.1.3.5"},
    {"code": "INDEPENDENT-CONTENT-REVIEW-ABSENT", "owner": "57.1.3.5"},
    {"code": "MANUAL-FUZZING-DEFERRED", "owner": "SR-TST-004"},
]


def digest(value: bytes) -> str:
    return hashlib.sha256(value).hexdigest()


def git_file(revision: str, path: str) -> bytes:
    result = subprocess.run(
        ["git", "show", f"{revision}:{path}"],
        cwd=ROOT,
        check=False,
        capture_output=True,
        timeout=30,
    )
    if result.returncode != 0:
        raise ValueError(f"committed Sprint 57 source is absent: {path}")
    return result.stdout


def version(executable: str, *arguments: str) -> str:
    resolved = shutil.which(executable)
    if resolved is None:
        return "unavailable"
    result = subprocess.run(
        (resolved, *arguments),
        cwd=ROOT,
        check=False,
        capture_output=True,
        text=True,
        timeout=30,
    )
    output = (result.stdout + result.stderr).strip().splitlines()
    return output[0][:256] if result.returncode == 0 and output else "unavailable"


def environment_manifest() -> dict[str, str]:
    return {
        "system": platform.system(),
        "release": platform.release(),
        "machine": platform.machine(),
        "python": platform.python_version(),
        "rustc": version("rustc", "--version"),
        "cargo": version("cargo", "--version"),
        "node": version("node", "--version"),
        "npm": version("npm", "--version"),
    }


def run_commands() -> list[dict[str, Any]]:
    records = []
    for identifier, argv in COMMANDS:
        executable = shutil.which(argv[0])
        if executable is None:
            code, output = 127, b"executable-unavailable"
        else:
            result = subprocess.run(
                (executable, *argv[1:]),
                cwd=ROOT,
                check=False,
                capture_output=True,
                timeout=1800,
            )
            code, output = result.returncode, result.stdout + result.stderr
        blocking_skip_count = None
        if identifier in RUST_FOCUSED_COMMANDS:
            matches = IGNORED_TESTS.findall(output)
            blocking_skip_count = sum(int(value) for value in matches) if matches else -1
        elif identifier in FOCUSED_COMMANDS:
            blocking_skip_count = 0 if code == 0 else None
        records.append(
            {
                "id": identifier,
                "argv": list(argv),
                "exit_code": code,
                "output_sha256": digest(output),
                "blocking_skip_count": blocking_skip_count,
            }
        )
    return records


def expected_verification(local_pass: bool = True) -> dict[str, Any]:
    return {
        "focused_local_contracts": local_pass,
        "focused_blocking_skip_count": 0 if local_pass else None,
        "acceptance_corpus_case_count": 68 if local_pass else None,
        "accepted_network_effect_count": 0 if local_pass else None,
        "accepted_execution_effect_count": 0 if local_pass else None,
        "accepted_filesystem_effect_count": 0 if local_pass else None,
        "invented_citation_count": 0 if local_pass else None,
        "invented_acronym_expansion_count": 0 if local_pass else None,
        "product_coordinator": False,
        "controlled_writer_integration": False,
        "native_interface_integration": False,
        "accessibility_acceptance": False,
        "installed_cross_platform_acceptance": False,
        "trusted_package_execution": False,
        "independent_content_review": False,
        "manual_fuzzing": False,
        "sprint_gate_closed": False,
    }


def expected_summary(local_pass: bool = True) -> dict[str, Any]:
    return {
        "local_sprint_57_contract_passed": local_pass,
        "sprint_status": "BLOCKED",
        "upstream_sprint_56_closed": False,
        "Markdown_workflow_integrated": False,
        "external_effects_enabled": False,
        "cross_platform_acceptance_passed": False,
        "trusted_package_execution_complete": False,
        "independent_review_present": False,
        "manual_fuzzing_complete": False,
        "release_approval": False,
    }


def build_report(revision: str, commands: list[dict[str, Any]]) -> dict[str, Any]:
    focused = [item for item in commands if item["id"] in FOCUSED_COMMANDS]
    local_pass = (
        all(item["exit_code"] == 0 for item in commands)
        and len(focused) == len(FOCUSED_COMMANDS)
        and all(item.get("blocking_skip_count") == 0 for item in focused)
    )
    return {
        "schema_version": 1,
        "record_type": "sprint_57_local_evidence",
        "source_revision": revision,
        "source_sha256": {path: digest(git_file(revision, path)) for path in SOURCE_PATHS},
        "environment": environment_manifest(),
        "commands": commands,
        "security_requirement_ids": list(SECURITY_REQUIREMENTS),
        "implemented_contracts": dict(IMPLEMENTED),
        "verification_evidence": expected_verification(local_pass),
        "blockers": [dict(blocker) for blocker in BLOCKERS],
        "summary": expected_summary(local_pass),
    }


def revision_is_ancestor(revision: str) -> bool:
    result = subprocess.run(
        ["git", "merge-base", "--is-ancestor", revision, "HEAD"],
        cwd=ROOT,
        check=False,
        capture_output=True,
        timeout=30,
    )
    return result.returncode == 0


def validate_report(report: dict[str, Any], verify_ancestry: bool = True) -> list[str]:
    failures: list[str] = []
    revision = str(report.get("source_revision", ""))
    if not REVISION.fullmatch(revision):
        failures.append("source revision invalid")
    if report.get("schema_version") != 1 or report.get("record_type") != "sprint_57_local_evidence":
        failures.append("report identity invalid")
    for actual, expected, name in (
        (report.get("security_requirement_ids"), SECURITY_REQUIREMENTS, "security mapping drift"),
        (report.get("implemented_contracts"), IMPLEMENTED, "implemented contract drift"),
        (report.get("blockers"), BLOCKERS, "blocker drift"),
    ):
        if actual != expected:
            failures.append(name)
    commands = report.get("commands", [])
    if [item.get("id") for item in commands] != [item[0] for item in COMMANDS]:
        failures.append("command inventory drift")
    if len(commands) != len(COMMANDS) or any(
        item.get("argv") != list(expected[1])
        or item.get("exit_code") != 0
        or not SHA256.fullmatch(str(item.get("output_sha256", "")))
        for item, expected in zip(commands, COMMANDS, strict=False)
    ):
        failures.append("command result invalid")
    for identifier in FOCUSED_COMMANDS:
        focused = next((item for item in commands if item.get("id") == identifier), None)
        if focused is None or focused.get("blocking_skip_count") != 0:
            failures.append(f"focused skipped, suppressed, or unavailable check: {identifier}")
    if report.get("verification_evidence") != expected_verification():
        failures.append("verification claim drift")
    if report.get("summary") != expected_summary():
        failures.append("summary or release claim drift")
    source = report.get("source_sha256", {})
    if list(source) != list(SOURCE_PATHS):
        failures.append("source inventory drift")
    elif REVISION.fullmatch(revision):
        for path in SOURCE_PATHS:
            try:
                expected = digest(git_file(revision, path))
            except ValueError as error:
                failures.append(str(error))
                continue
            if source.get(path) != expected:
                failures.append(f"source digest drift: {path}")
    if verify_ancestry and REVISION.fullmatch(revision) and not revision_is_ancestor(revision):
        failures.append("report source revision is not an ancestor of current HEAD")
    encoded = json.dumps(report, sort_keys=True).lower()
    for prohibited in (
        "credential_value", "secret_value", "private_key", "raw_output", "raw_result",
        "prompt_text", "raw_document", "remote_url", "repository_path", "access_token",
    ):
        if prohibited in encoded:
            failures.append(f"prohibited evidence field: {prohibited}")
    return failures


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--write", action="store_true")
    parser.add_argument("--source-revision")
    args = parser.parse_args()
    if args.write:
        revision = args.source_revision or subprocess.run(
            ["git", "rev-parse", "HEAD"],
            cwd=ROOT,
            check=True,
            capture_output=True,
            text=True,
            timeout=30,
        ).stdout.strip()
        report = build_report(revision, run_commands())
        failures = validate_report(report, verify_ancestry=False)
        if failures:
            for failure in failures:
                print(f"error: {failure}")
            return 1
        OUTPUT.parent.mkdir(parents=True, exist_ok=True)
        OUTPUT.write_text(json.dumps(report, indent=2) + "\n", encoding="utf-8")
        print(f"wrote {OUTPUT.relative_to(ROOT)}")
        return 0
    if not OUTPUT.is_file():
        print(f"error: missing {OUTPUT.relative_to(ROOT)}")
        return 1
    failures = validate_report(json.loads(OUTPUT.read_text(encoding="utf-8")))
    if failures:
        for failure in failures:
            print(f"error: {failure}")
        return 1
    print("Sprint 57 local evidence validated")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
