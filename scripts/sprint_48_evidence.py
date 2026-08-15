#!/usr/bin/env python3
"""Build and validate truthful locally executable Sprint 48 evidence."""

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
OUTPUT: Final = ROOT / "artifacts/sprints/sprint-48/local-evidence-report.json"
REVISION: Final = re.compile(r"^[0-9a-f]{40}$")
SHA256: Final = re.compile(r"^[0-9a-f]{64}$")
IGNORED_TESTS: Final = re.compile(
    rb"test result: (?:ok|FAILED)\. \d+ passed; \d+ failed; (\d+) ignored;"
)
SOURCE_PATHS: Final = (
    "shells/host/src/headless.rs",
    "shells/host/src/cli.rs",
    "shells/host/src/bin/agent.rs",
    "shells/host/src/lib.rs",
    "shells/host/Cargo.toml",
    "shells/host/README.md",
    "shells/README.md",
    "schemas/runtime/thin-client-request.schema.json",
    "schemas/runtime/examples/thin-client-request.valid.json",
    "schemas/runtime/thin-client-event.schema.json",
    "schemas/runtime/examples/thin-client-event.valid.json",
    "scripts/validate_planning_schemas.mjs",
    "tests/test_planning_schemas.mjs",
    "docs/verification/sprint-48-headless-corpus.json",
    "tests/test_sprint_48_headless_corpus.py",
    "tests/test_sprint_48_cli_binary.py",
    "scripts/effect_boundary.py",
    "tests/test_effect_boundary.py",
    "docs/architecture/thin-client-boundary.md",
    "docs/guides/local-command-line-interface.md",
    "docs/verification/sprint-48-local-results.md",
    "README.md",
    "scripts/sprint_48_evidence.py",
    "tests/test_sprint_48_evidence.py",
)
COMMANDS: Final = (
    (
        "thin-client-unit",
        (
            "cargo",
            "test",
            "-p",
            "agentmage-host",
            "headless::tests",
            "--lib",
            "--locked",
        ),
    ),
    (
        "cli-parser-unit",
        (
            "cargo",
            "test",
            "-p",
            "agentmage-host",
            "cli::tests",
            "--lib",
            "--locked",
        ),
    ),
    (
        "cli-binary-contract",
        ("python3", "-m", "unittest", "tests.test_sprint_48_cli_binary"),
    ),
    (
        "host-library",
        ("cargo", "test", "-p", "agentmage-host", "--lib", "--locked"),
    ),
    (
        "agent-binary-build",
        (
            "cargo",
            "build",
            "-p",
            "agentmage-host",
            "--bin",
            "agent",
            "--locked",
        ),
    ),
    (
        "headless-security-corpus",
        ("python3", "-m", "unittest", "tests.test_sprint_48_headless_corpus"),
    ),
    ("runtime-schema-contract", ("node", "--test", "tests/test_planning_schemas.mjs")),
    (
        "host-strict-clippy",
        (
            "cargo",
            "clippy",
            "-p",
            "agentmage-host",
            "--all-targets",
            "--locked",
            "--",
            "-D",
            "warnings",
        ),
    ),
    ("effect-boundary", ("python3", "scripts/effect_boundary.py")),
    ("documentation-gate", ("npm", "run", "docs:check")),
    ("product-ci-contract", ("python3", "scripts/product_ci.py", "--check")),
    ("supply-chain", ("python3", "scripts/supply_chain.py")),
    (
        "evidence-tests",
        ("python3", "-m", "unittest", "tests.test_sprint_48_evidence"),
    ),
)
FOCUSED_COMMANDS: Final = ("thin-client-unit", "cli-parser-unit")
SECURITY_REQUIREMENTS: Final = [
    "SR-PLT-005",
    "SR-PLT-006",
    "SR-ACC-001",
    "SR-ACC-007",
    "SR-OPS-001",
    "SR-TST-001",
    "SR-TST-004",
]
IMPLEMENTED: Final = {
    "closed_thin_client_protocol": True,
    "closed_command_taxonomy": True,
    "shared_status_projection": True,
    "stable_exit_codes": True,
    "deterministic_shell_completion": True,
    "bounded_request_and_output": True,
    "exact_predeclared_headless_grants": True,
    "headless_interactive_approval_denied": True,
    "transport_only_clients": True,
    "surface_independent_kernel_identity": True,
    "hash_chained_event_stream": True,
    "replay_resume_and_cancellation_contracts": True,
    "closed_request_and_event_schemas": True,
    "executable_cli_fail_closed_fixture": True,
    "production_authenticated_cli_transport": False,
    "production_conversation_coordinator": False,
    "production_knowledge_coordinator": False,
    "production_operational_coordinator": False,
    "native_disconnect_process_tree_campaign": False,
    "native_cross_platform_acceptance": False,
    "trusted_package_execution": False,
    "independent_review": False,
    "manual_fuzzing": False,
}
BLOCKERS: Final = [
    {"code": "UPSTREAM-SPRINT-47-BLOCKED", "owner": "48.1"},
    {"code": "AUTHENTICATED-PRODUCT-TRANSPORT-ABSENT", "owner": "48.1.1.1"},
    {"code": "CONVERSATION-COORDINATOR-ABSENT", "owner": "48.1.1.2"},
    {"code": "KNOWLEDGE-COORDINATOR-ABSENT", "owner": "48.1.1.3"},
    {"code": "OPERATIONAL-COORDINATOR-ABSENT", "owner": "48.1.1.3"},
    {"code": "NATIVE-DISCONNECT-PROCESS-TREE-CAMPAIGN-ABSENT", "owner": "48.1.3.4"},
    {"code": "NATIVE-CROSS-PLATFORM-ACCEPTANCE-ABSENT", "owner": "48.1.3.5"},
    {"code": "TRUSTED-INSTALLED-PACKAGE-EXECUTION-ABSENT", "owner": "48.1.3.5"},
    {"code": "INDEPENDENT-THIN-CLIENT-REVIEW-ABSENT", "owner": "48.1.3.5"},
    {"code": "MANUAL-FUZZING-DEFERRED", "owner": "SR-TST-001"},
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
        raise ValueError(f"committed Sprint 48 source is absent: {path}")
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
        if identifier in FOCUSED_COMMANDS:
            matches = IGNORED_TESTS.findall(output)
            blocking_skip_count = sum(int(value) for value in matches) if matches else -1
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
        "client_surface_count": 5 if local_pass else None,
        "command_family_count": 4 if local_pass else None,
        "stable_exit_code_count": 9 if local_pass else None,
        "runtime_record_count": 23 if local_pass else None,
        "thin_client_schema_count": 2 if local_pass else None,
        "adversarial_corpus_case_count": 40 if local_pass else None,
        "executable_cli_fixture_passed": local_pass,
        "unauthorized_effect_acceptance_count": 0 if local_pass else None,
        "production_authenticated_cli_transport": False,
        "production_command_coordinators": False,
        "native_disconnect_process_tree_campaign": False,
        "native_cross_platform_acceptance": False,
        "trusted_package_execution": False,
        "independent_review": False,
        "manual_fuzzing": False,
    }


def expected_summary(local_pass: bool = True) -> dict[str, Any]:
    return {
        "local_sprint_48_contract_passed": local_pass,
        "sprint_status": "BLOCKED",
        "upstream_sprint_47_closed": False,
        "authenticated_product_transport_active": False,
        "production_command_coordinators_active": False,
        "native_disconnect_process_tree_campaign_complete": False,
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
        "record_type": "sprint_48_local_evidence",
        "source_revision": revision,
        "source_sha256": {
            path: digest(git_file(revision, path)) for path in SOURCE_PATHS
        },
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
    if report.get("schema_version") != 1 or report.get("record_type") != "sprint_48_local_evidence":
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
        focused = next(
            (item for item in commands if item.get("id") == identifier), None
        )
        if focused is None or focused.get("blocking_skip_count") != 0:
            failures.append(
                f"focused skipped, suppressed, or unavailable check: {identifier}"
            )
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
        "credential_value",
        "secret_value",
        "private_key",
        "raw_output",
        "stdout_content",
        "stderr_content",
        "remote_url",
        "repository_path",
        "socket_path",
        "prompt_text",
        "access_token",
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
    print("Sprint 48 local evidence validated")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
