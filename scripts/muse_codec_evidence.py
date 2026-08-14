#!/usr/bin/env python3
"""Build D027-S13-MUSE-CODEC family-edge and kernel-neutrality evidence."""

from __future__ import annotations

import argparse
import hashlib
import json
import re
import shutil
import subprocess
from pathlib import Path
from typing import Any, Final


ROOT: Final = Path(__file__).resolve().parents[1]
OUTPUT: Final = ROOT / "artifacts/sprints/sprint-13/story-13.3/d027-s13-muse-codec.json"
REVISION: Final = re.compile(r"^[0-9a-f]{40}$")
SOURCE_PATHS: Final = (
    "kernel/contracts/src/model.rs",
    "kernel/engine/src/model_codec.rs",
    "kernel/engine/src/model_response.rs",
    "kernel/engine/src/model_runtime.rs",
    "platforms/linux-inference/src/muse_atem_codec.rs",
    "scripts/muse_codec_evidence.py",
    "tests/test_muse_codec_evidence.py",
)
COMMANDS: Final = (
    (
        "muse-edge-codec",
        (
            "cargo", "test", "-p", "agentmage-platform-linux-inference", "--locked",
            "muse_atem_codec::tests::",
        ),
    ),
    (
        "generic-codec",
        (
            "cargo", "test", "-p", "agentmage-kernel-engine", "--locked",
            "model_codec::tests_support::",
        ),
    ),
    (
        "bounded-response",
        (
            "cargo", "test", "-p", "agentmage-kernel-engine", "--locked",
            "model_response::tests::",
        ),
    ),
    (
        "fake-family-parity",
        (
            "cargo", "test", "-p", "agentmage-kernel-engine", "--locked",
            "fake_muse_and_gemma_cover_every_runtime_terminal_and_hostile_response_state",
        ),
    ),
)
MUTATIONS: Final = {
    "tokenizer": "rejects_every_identity_dimension_independently",
    "template": "rejects_every_identity_dimension_independently",
    "reasoning-flag": "rejects_every_identity_dimension_independently",
    "message-boundary": "rejects_context_response_and_unknown_field_drift",
    "end-token": "rejects_every_identity_dimension_independently",
    "tool-envelope": "rejects_context_response_and_unknown_field_drift",
    "tool-protocol": "rejects_every_identity_dimension_independently",
    "stream-split": "replay_missing_and_exhausted_repairs_have_distinct_closed_results",
    "unknown-field": "rejects_context_response_and_unknown_field_drift",
    "trailing-bytes": "rejects_context_response_and_unknown_field_drift",
    "proposal-identity": "malformed_stale_replayed_and_authority_seeking_envelopes_remain_inert",
}


def sha256_file(path: Path) -> str:
    return hashlib.sha256(path.read_bytes()).hexdigest()


def run_commands() -> list[dict[str, Any]]:
    results: list[dict[str, Any]] = []
    for identifier, command in COMMANDS:
        executable = shutil.which(command[0])
        if executable is None:
            exit_code = 127
            output = b"executable-unavailable"
        else:
            process = subprocess.run(
                (executable, *command[1:]),
                cwd=ROOT,
                check=False,
                capture_output=True,
                timeout=600,
            )
            exit_code = process.returncode
            output = process.stdout + process.stderr
        results.append(
            {
                "id": identifier,
                "argv": list(command),
                "exit_code": exit_code,
                "output_sha256": hashlib.sha256(output).hexdigest(),
            }
        )
    return results


def kernel_family_references() -> list[dict[str, Any]]:
    references: list[dict[str, Any]] = []
    for path in sorted((ROOT / "kernel/engine/src").glob("*.rs")):
        content = path.read_text(encoding="utf-8")
        production = content.split("#[cfg(test)]", maxsplit=1)[0]
        for line_number, line in enumerate(production.splitlines(), start=1):
            for family in ("muse", "gemma"):
                if family in line.lower():
                    references.append(
                        {
                            "path": str(path.relative_to(ROOT)),
                            "line": line_number,
                            "family": family,
                        }
                    )
    return references


def build_report(source_revision: str, commands: list[dict[str, Any]]) -> dict[str, Any]:
    references = kernel_family_references()
    checks = [
        {
            "id": command["id"],
            "result": "PASS" if command["exit_code"] == 0 else "FAIL",
            "code": f"{command['id']}-pass" if command["exit_code"] == 0 else f"{command['id']}-fail",
        }
        for command in commands
    ]
    checks.append(
        {
            "id": "production-kernel-family-neutrality",
            "result": "PASS" if not references else "FAIL",
            "code": "no-family-branch" if not references else "family-branch-present",
        }
    )
    passed = all(check["result"] == "PASS" for check in checks)
    return {
        "schema_version": 1,
        "record_type": "d027_s13_muse_codec_evidence",
        "source_revision": source_revision,
        "source_sha256": {path: sha256_file(ROOT / path) for path in SOURCE_PATHS},
        "commands": commands,
        "mutations": [
            {"id": identifier, "proving_test": test, "result": "PASS" if passed else "BLOCKED"}
            for identifier, test in MUTATIONS.items()
        ],
        "production_kernel_family_references": references,
        "checks": checks,
        "disposition": {
            "status": "PASS-CONTRACT" if passed else "BLOCKED",
            "codec_at_platform_edge": passed,
            "kernel_family_neutral": passed and not references,
            "authority_change": False,
            "model_loaded": False,
            "profile_activated": False,
            "release_approval": False,
        },
        "limitations": [
            "This campaign uses deterministic synthetic packets and proposals, not model output.",
            "Codec conformance does not prove model quality, runtime isolation, or release readiness.",
            "Test-only family labels exercise parity and are not production kernel branches.",
        ],
    }


def validate_report(report: dict[str, Any]) -> list[str]:
    failures: list[str] = []
    if set(report) != {
        "schema_version", "record_type", "source_revision", "source_sha256", "commands",
        "mutations", "production_kernel_family_references", "checks", "disposition", "limitations",
    }:
        return ["codec evidence fields are not closed"]
    if report.get("schema_version") != 1 or report.get("record_type") != "d027_s13_muse_codec_evidence":
        failures.append("codec evidence identity changed")
    if not REVISION.fullmatch(str(report.get("source_revision", ""))):
        failures.append("codec evidence revision is not exact")
    sources = report.get("source_sha256", {})
    if set(sources) != set(SOURCE_PATHS) or any(
        not re.fullmatch(r"[0-9a-f]{64}", str(value)) for value in sources.values()
    ):
        failures.append("codec source closure changed")
    commands = report.get("commands", [])
    expected_commands = {identifier: list(command) for identifier, command in COMMANDS}
    if (
        len(commands) != len(COMMANDS)
        or {item.get("id") for item in commands} != set(expected_commands)
        or any(item.get("argv") != expected_commands.get(item.get("id")) for item in commands)
    ):
        failures.append("codec command closure changed")
    checks = report.get("checks", [])
    check_by_id = {item.get("id"): item for item in checks}
    if len(checks) != len(COMMANDS) + 1 or len(check_by_id) != len(checks):
        failures.append("codec check closure changed")
    for command in commands:
        expected = "PASS" if command.get("exit_code") == 0 else "FAIL"
        if check_by_id.get(command.get("id"), {}).get("result") != expected:
            failures.append(f"codec command/check mismatch: {command.get('id')}")
    references = report.get("production_kernel_family_references", [])
    neutrality = check_by_id.get("production-kernel-family-neutrality", {})
    if neutrality.get("result") != ("PASS" if not references else "FAIL"):
        failures.append("kernel-neutrality result disagrees with references")
    passed = bool(checks) and all(check.get("result") == "PASS" for check in checks)
    mutations = report.get("mutations", [])
    if (
        [item.get("id") for item in mutations] != list(MUTATIONS)
        or any(item.get("proving_test") != MUTATIONS.get(item.get("id")) for item in mutations)
        or any(item.get("result") != ("PASS" if passed else "BLOCKED") for item in mutations)
    ):
        failures.append("codec mutation matrix changed")
    disposition = report.get("disposition", {})
    if disposition.get("status") != ("PASS-CONTRACT" if passed else "BLOCKED"):
        failures.append("codec disposition overstates checks")
    if disposition.get("codec_at_platform_edge") is not passed:
        failures.append("codec edge status disagrees with checks")
    if disposition.get("kernel_family_neutral") is not (passed and not references):
        failures.append("kernel-neutral status disagrees with checks")
    for field in ("authority_change", "model_loaded", "profile_activated", "release_approval"):
        if disposition.get(field) is not False:
            failures.append(f"codec evidence overstates {field}")
    return failures


def git_output(*arguments: str) -> str:
    process = subprocess.run(
        ("git", *arguments), cwd=ROOT, check=True, capture_output=True, text=True, timeout=10
    )
    return process.stdout.strip()


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--source-revision", required=True)
    parser.add_argument("--write", action="store_true")
    args = parser.parse_args()
    current = git_output("rev-parse", "HEAD")
    revision = current if args.source_revision == "HEAD" else args.source_revision
    if revision != current or not REVISION.fullmatch(revision):
        print("- source revision is not the current exact HEAD")
        return 1
    if args.write and git_output("status", "--porcelain", "--untracked-files=no"):
        print("- tracked worktree must be clean before writing evidence")
        return 1
    report = build_report(revision, run_commands())
    failures = validate_report(report)
    if failures:
        for failure in failures:
            print(f"- {failure}")
        return 1
    if args.write:
        OUTPUT.parent.mkdir(parents=True, exist_ok=True)
        OUTPUT.write_text(json.dumps(report, indent=2, sort_keys=True) + "\n", encoding="utf-8")
    print(json.dumps(report["disposition"], indent=2, sort_keys=True))
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
