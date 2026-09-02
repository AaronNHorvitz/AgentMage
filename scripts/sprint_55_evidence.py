#!/usr/bin/env python3
"""Build and validate truthful locally executable Sprint 55 evidence."""

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
OUTPUT: Final = ROOT / "artifacts/sprints/sprint-55/local-evidence-report.json"
REVISION: Final = re.compile(r"^[0-9a-f]{40}$")
SHA256: Final = re.compile(r"^[0-9a-f]{64}$")
IGNORED_TESTS: Final = re.compile(
    rb"test result: (?:ok|FAILED)\. \d+ passed; \d+ failed; (\d+) ignored;"
)
SOURCE_PATHS: Final = (
    "kernel/contracts/src/meeting.rs",
    "kernel/contracts/src/lib.rs",
    "kernel/engine/src/meeting_continuity.rs",
    "kernel/engine/src/lib.rs",
    "capabilities/knowledge/src/meeting_skills.rs",
    "capabilities/knowledge/src/lib.rs",
    "capabilities/knowledge/examples/meeting_skill_pack.rs",
    "shells/host/src/meeting_coordinator.rs",
    "shells/host/src/headless.rs",
    "shells/host/src/cli.rs",
    "shells/host/src/lib.rs",
    "schemas/runtime/meeting-plan-draft.schema.json",
    "schemas/runtime/examples/meeting-plan-draft.valid.json",
    "schemas/runtime/meeting-transcript-cleanup.schema.json",
    "schemas/runtime/examples/meeting-transcript-cleanup.valid.json",
    "schemas/runtime/meeting-minutes.schema.json",
    "schemas/runtime/examples/meeting-minutes.valid.json",
    "schemas/runtime/meeting-continuity-record.schema.json",
    "schemas/runtime/examples/meeting-continuity-record.valid.json",
    "scripts/validate_planning_schemas.mjs",
    "tests/test_planning_schemas.mjs",
    "docs/verification/sprint-55-meeting-corpus.json",
    "scripts/meeting_continuity_contract.py",
    "tests/test_meeting_continuity_contract.py",
    "scripts/meeting_skill_contract.py",
    "tests/test_meeting_skill_contract.py",
    "artifacts/sprints/sprint-55/meeting-skill-pack.json",
    "docs/architecture/meeting-records-and-continuity.md",
    "docs/guides/meeting-records-local-workflows.md",
    "docs/verification/sprint-55-local-results.md",
    "scripts/sprint_55_evidence.py",
    "tests/test_sprint_55_evidence.py",
)
COMMANDS: Final = (
    (
        "meeting-kernel-unit",
        (
            "cargo", "test", "-p", "agentmage-kernel-engine",
            "meeting_continuity::tests", "--lib", "--locked",
        ),
    ),
    (
        "meeting-skill-unit",
        (
            "cargo", "test", "-p", "agentmage-capability-knowledge",
            "meeting_skills::tests", "--lib", "--locked",
        ),
    ),
    (
        "meeting-host-coordinator",
        (
            "cargo", "test", "-p", "agentmage-host",
            "meeting_coordinator", "--lib", "--locked",
        ),
    ),
    (
        "meeting-native-protocol",
        (
            "cargo", "test", "-p", "agentmage-host",
            "headless::tests", "--lib", "--locked",
        ),
    ),
    (
        "meeting-native-cli",
        (
            "cargo", "test", "-p", "agentmage-host",
            "cli::tests", "--lib", "--locked",
        ),
    ),
    (
        "meeting-runtime-schemas",
        ("node", "--test", "tests/test_planning_schemas.mjs"),
    ),
    (
        "meeting-acceptance-corpus",
        ("python3", "-m", "unittest", "tests.test_meeting_continuity_contract"),
    ),
    (
        "meeting-skill-contract",
        ("python3", "scripts/meeting_skill_contract.py"),
    ),
    (
        "meeting-strict-clippy",
        (
            "cargo", "clippy", "-p", "agentmage-kernel-contracts", "-p",
            "agentmage-kernel-engine", "-p", "agentmage-capability-knowledge",
            "--all-targets", "--locked", "--", "-D", "warnings",
        ),
    ),
    ("documentation-gate", ("npm", "run", "docs:check")),
    ("product-ci-contract", ("python3", "scripts/product_ci.py", "--check")),
    ("supply-chain", ("python3", "scripts/supply_chain.py")),
    (
        "evidence-tests",
        ("python3", "-m", "unittest", "tests.test_sprint_55_evidence"),
    ),
)
FOCUSED_COMMANDS: Final = tuple(item[0] for item in COMMANDS[:8])
RUST_FOCUSED_COMMANDS: Final = {
    "meeting-kernel-unit",
    "meeting-skill-unit",
    "meeting-host-coordinator",
    "meeting-native-protocol",
    "meeting-native-cli",
}
SECURITY_REQUIREMENTS: Final = [
    "SR-DAT-001",
    "SR-DAT-002",
    "SR-DAT-003",
    "SR-AI-003",
    "SR-AI-007",
    "SR-AI-010",
    "SR-CIV-003",
    "SR-CIV-004",
    "SR-CIV-005",
    "SR-CIV-006",
    "SR-CIV-007",
    "SR-CIV-008",
    "SR-CIV-009",
    "SR-TST-006",
]
IMPLEMENTED: Final = {
    "meeting_draft_kind_count": 2,
    "invitation_state_count": 6,
    "attendance_state_count": 5,
    "runtime_schema_count": 4,
    "declarative_skill_count": 6,
    "acceptance_corpus_case_count": 86,
    "source_bound_preparation_drafts": True,
    "independent_attendee_states": True,
    "verbatim_preserving_cleanup": True,
    "explicit_attribution_and_unclear_markers": True,
    "confirmed_proposed_minutes_separation": True,
    "unknown_owner_and_date_refusal": True,
    "immutable_recurring_continuity": True,
    "local_follow_up_draft": True,
    "invite_effect_capability": False,
    "assignment_effect_capability": False,
    "notification_effect_capability": False,
    "send_effect_capability": False,
    "schedule_effect_capability": False,
    "inbox_access_capability": False,
    "network_access_capability": False,
    "calendar_mutation_capability": False,
    "source_mutation_capability": False,
    "product_coordinator": True,
    "native_interface_integration": True,
    "accessibility_acceptance": False,
    "installed_cross_platform_acceptance": False,
    "trusted_package_execution": False,
    "independent_records_review": False,
    "filing_control_complete": False,
    "manual_fuzzing": False,
}
BLOCKERS: Final = [
    {"code": "UPSTREAM-SPRINT-54-BLOCKED", "owner": "55.1"},
    {"code": "ACCESSIBILITY-ACCEPTANCE-ABSENT", "owner": "55.1.3.4"},
    {"code": "SPRINT-56-FILING-CONTROL-PENDING", "owner": "55.1.3.4"},
    {"code": "INSTALLED-CROSS-PLATFORM-ACCEPTANCE-ABSENT", "owner": "55.1.3.4"},
    {"code": "TRUSTED-INSTALLED-PACKAGE-EXECUTION-ABSENT", "owner": "55.1.3.4"},
    {"code": "INDEPENDENT-RECORDS-REVIEW-ABSENT", "owner": "55.1.3.4"},
    {"code": "MANUAL-FUZZING-DEFERRED", "owner": "SR-TST-006"},
]


def digest(value: bytes) -> str:
    return hashlib.sha256(value).hexdigest()


def git_file(revision: str, path: str) -> bytes:
    result = subprocess.run(
        ["git", "show", f"{revision}:{path}"], cwd=ROOT, check=False,
        capture_output=True, timeout=30,
    )
    if result.returncode != 0:
        raise ValueError(f"committed Sprint 55 source is absent: {path}")
    return result.stdout


def version(executable: str, *arguments: str) -> str:
    resolved = shutil.which(executable)
    if resolved is None:
        return "unavailable"
    result = subprocess.run(
        (resolved, *arguments), cwd=ROOT, check=False, capture_output=True,
        text=True, timeout=30,
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
                (executable, *argv[1:]), cwd=ROOT, check=False,
                capture_output=True, timeout=1800,
            )
            code, output = result.returncode, result.stdout + result.stderr
        blocking_skip_count = None
        if identifier in RUST_FOCUSED_COMMANDS:
            matches = IGNORED_TESTS.findall(output)
            blocking_skip_count = sum(int(value) for value in matches) if matches else -1
        elif identifier in FOCUSED_COMMANDS:
            blocking_skip_count = 0 if code == 0 else None
        records.append({
            "id": identifier,
            "argv": list(argv),
            "exit_code": code,
            "output_sha256": digest(output),
            "blocking_skip_count": blocking_skip_count,
        })
    return records


def expected_verification(local_pass: bool = True) -> dict[str, Any]:
    return {
        "focused_local_contracts": local_pass,
        "focused_blocking_skip_count": 0 if local_pass else None,
        "acceptance_corpus_case_count": 86 if local_pass else None,
        "accepted_external_effect_count": 0 if local_pass else None,
        "inferred_attendee_state_count": 0 if local_pass else None,
        "invented_owner_or_date_count": 0 if local_pass else None,
        "verbatim_source_mutation_count": 0 if local_pass else None,
        "product_coordinator": True,
        "native_interface_integration": True,
        "accessibility_acceptance": False,
        "installed_cross_platform_acceptance": False,
        "trusted_package_execution": False,
        "independent_records_review": False,
        "filing_control_complete": False,
        "manual_fuzzing": False,
        "sprint_gate_closed": False,
    }


def expected_summary(local_pass: bool = True) -> dict[str, Any]:
    return {
        "local_sprint_55_contract_passed": local_pass,
        "sprint_status": "BLOCKED",
        "upstream_sprint_54_closed": False,
        "meeting_workflow_integrated": True,
        "native_interface_integrated": True,
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
        "record_type": "sprint_55_local_evidence",
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
        cwd=ROOT, check=False, capture_output=True, timeout=30,
    )
    return result.returncode == 0


def validate_report(report: dict[str, Any], verify_ancestry: bool = True) -> list[str]:
    failures: list[str] = []
    revision = str(report.get("source_revision", ""))
    if not REVISION.fullmatch(revision):
        failures.append("source revision invalid")
    if report.get("schema_version") != 1 or report.get("record_type") != "sprint_55_local_evidence":
        failures.append("report identity invalid")
    if report.get("security_requirement_ids") != SECURITY_REQUIREMENTS:
        failures.append("security mapping drift")
    if report.get("implemented_contracts") != IMPLEMENTED:
        failures.append("implemented contract drift")
    if report.get("blockers") != BLOCKERS:
        failures.append("blocker drift")
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
        "credential_value", "secret_value", "private_key", "raw_output",
        "raw_result", "prompt_text", "raw_transcript", "remote_url",
        "repository_path", "access_token",
    ):
        if prohibited in encoded:
            failures.append(f"prohibited evidence field: {prohibited}")
    return failures


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser()
    parser.add_argument("--write", action="store_true")
    parser.add_argument("--source-revision")
    return parser.parse_args()


def main() -> int:
    args = parse_args()
    if args.write:
        revision = args.source_revision or subprocess.run(
            ["git", "rev-parse", "HEAD"], cwd=ROOT, check=True,
            capture_output=True, text=True, timeout=30,
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
    print("Sprint 55 local evidence validated")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
