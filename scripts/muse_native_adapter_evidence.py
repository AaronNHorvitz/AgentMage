#!/usr/bin/env python3
"""Build closed evidence for the non-activated native Muse adapter contract."""

from __future__ import annotations

import argparse
import hashlib
import json
import os
import re
import shutil
import subprocess
from pathlib import Path
from typing import Any, Final


ROOT: Final = Path(__file__).resolve().parents[1]
OUTPUT: Final = ROOT / "artifacts/sprints/sprint-13/story-13.1/muse-native-adapter-contract.json"
PACKAGE: Final = ROOT / "release-output/agentmage-llama-cpp-b10423-muse-vulkan-linux-x86_64.tar.gz"
SOURCE_PATHS: Final = (
    "kernel/contracts/src/model.rs",
    "platforms/linux-inference/src/native_model_adapter.rs",
    "platforms/linux-inference/src/llama_server_driver.rs",
    "model-profiles/candidates/muse-glimmer-30b-text-8k/runtime-support.json",
    "model-profiles/runtimes/llama-cpp-b10423-muse-linux-x86_64.json",
    "model-profiles/exact-profile-catalog.json",
    "scripts/muse_native_adapter_evidence.py",
    "tests/test_muse_native_adapter_evidence.py",
)
REVISION: Final = re.compile(r"^[0-9a-f]{40}$")
COMMANDS: Final = (
    (
        "inference-tests",
        ("cargo", "test", "-p", "agentmage-platform-linux-inference", "--locked"),
    ),
    (
        "inference-clippy",
        (
            "cargo", "clippy", "-p", "agentmage-platform-linux-inference",
            "--all-targets", "--locked", "--", "-D", "warnings",
        ),
    ),
    ("candidate-admission", ("python3", "scripts/muse_candidate_admission.py")),
    (
        "runtime-package",
        (
            "python3", "scripts/muse_llama_runtime.py", "--verify-package",
            "release-output/agentmage-llama-cpp-b10423-muse-vulkan-linux-x86_64.tar.gz",
        ),
    ),
    ("profile-catalog", ("node", "scripts/model_profile_catalog.mjs")),
)


def sha256_bytes(value: bytes) -> str:
    return hashlib.sha256(value).hexdigest()


def sha256_file(path: Path) -> str:
    digest = hashlib.sha256()
    with path.open("rb") as stream:
        for block in iter(lambda: stream.read(1024 * 1024), b""):
            digest.update(block)
    return digest.hexdigest()


def execute_commands() -> list[dict[str, Any]]:
    results: list[dict[str, Any]] = []
    for identifier, command in COMMANDS:
        executable = shutil.which(command[0])
        if executable is None:
            results.append(
                {
                    "id": identifier,
                    "argv": list(command),
                    "exit_code": 127,
                    "output_sha256": sha256_bytes(b"executable-unavailable"),
                }
            )
            continue
        environment = {
            "PATH": "/usr/local/bin:/usr/bin:/bin",
            "HOME": str(Path.home()),
            "CARGO_TERM_COLOR": "never",
        }
        for name in ("CARGO_HOME", "RUSTUP_HOME"):
            if name in os.environ:
                environment[name] = os.environ[name]
        for name in ("RUSTC", "RUSTDOC"):
            resolved = shutil.which(name.lower())
            if resolved is not None:
                environment[name] = resolved
        process = subprocess.run(
            (executable, *command[1:]),
            cwd=ROOT,
            check=False,
            capture_output=True,
            timeout=600,
            env=environment,
        )
        combined = process.stdout + process.stderr
        results.append(
            {
                "id": identifier,
                "argv": list(command),
                "exit_code": process.returncode,
                "output_sha256": sha256_bytes(combined),
            }
        )
    return results


def build_report(source_revision: str, commands: list[dict[str, Any]]) -> dict[str, Any]:
    sources = {
        relative: sha256_file(ROOT / relative)
        for relative in SOURCE_PATHS
    }
    package = {
        "path": str(PACKAGE.relative_to(ROOT)),
        "bytes": PACKAGE.stat().st_size if PACKAGE.is_file() else None,
        "sha256": sha256_file(PACKAGE) if PACKAGE.is_file() else None,
    }
    checks = [
        {
            "id": command["id"],
            "result": "PASS" if command["exit_code"] == 0 else "FAIL",
            "code": f"{command['id']}-pass" if command["exit_code"] == 0 else f"{command['id']}-fail",
        }
        for command in commands
    ]
    package_exact = (
        package["bytes"] == 25_449_031
        and package["sha256"] == "83d08dd70d46d55c9f576368c0a7ad6861e74b9bc93fe510d1ad965bcf43d31c"
    )
    checks.append(
        {
            "id": "package-identity",
            "result": "PASS" if package_exact else "FAIL",
            "code": "package-identity-pass" if package_exact else "package-identity-fail",
        }
    )
    contract_pass = all(check["result"] == "PASS" for check in checks)
    return {
        "schema_version": 1,
        "record_type": "muse_native_adapter_contract_evidence",
        "source_revision": source_revision,
        "source_sha256": sources,
        "package": package,
        "commands": commands,
        "checks": checks,
        "disposition": {
            "status": "CONTRACT-PASS-LIVE-BLOCKED" if contract_pass else "BLOCKED",
            "adapter_contract_implemented": contract_pass,
            "adapter_live_proven": False,
            "model_loaded": False,
            "network_isolation_live_proven": False,
            "quality_evaluated": False,
            "enabled_models": 0,
            "activation": False,
            "automatic_fallback": False,
            "release_approval": False,
        },
        "blockers": [
            "exact-model-artifact-not-present",
            "live-model-load-not-executed",
            "live-zero-egress-campaign-not-executed",
            "quality-and-repeatability-not-measured",
        ],
        "limitations": [
            "Synthetic contract and process-boundary tests are not a real model execution.",
            "A verified runtime package does not admit or activate a model profile.",
            "Linux evidence cannot be reused as macOS, Windows, Docker, quality, or release evidence.",
        ],
    }


def validate_report(report: dict[str, Any]) -> list[str]:
    failures: list[str] = []
    if set(report) != {
        "schema_version", "record_type", "source_revision", "source_sha256", "package",
        "commands", "checks", "disposition", "blockers", "limitations",
    }:
        return ["adapter evidence fields are not closed"]
    if report.get("schema_version") != 1 or report.get("record_type") != "muse_native_adapter_contract_evidence":
        failures.append("adapter evidence identity changed")
    if not REVISION.fullmatch(str(report.get("source_revision", ""))):
        failures.append("adapter evidence source revision is not exact")
    sources = report.get("source_sha256", {})
    if set(sources) != set(SOURCE_PATHS) or any(
        not re.fullmatch(r"[0-9a-f]{64}", str(value)) for value in sources.values()
    ):
        failures.append("adapter evidence source closure changed")
    commands = report.get("commands", [])
    expected_commands = {identifier: list(command) for identifier, command in COMMANDS}
    if (
        len(commands) != len(COMMANDS)
        or {item.get("id") for item in commands} != set(expected_commands)
        or any(item.get("argv") != expected_commands.get(item.get("id")) for item in commands)
        or any(set(item) != {"id", "argv", "exit_code", "output_sha256"} for item in commands)
        or any(not isinstance(item.get("exit_code"), int) for item in commands)
        or any(not re.fullmatch(r"[0-9a-f]{64}", str(item.get("output_sha256", ""))) for item in commands)
    ):
        failures.append("adapter evidence command closure changed")
    checks = report.get("checks", [])
    if len(checks) != len(COMMANDS) + 1 or len({item.get("id") for item in checks}) != len(checks):
        failures.append("adapter evidence check closure changed")
    command_by_id = {item.get("id"): item for item in commands}
    check_by_id = {item.get("id"): item for item in checks}
    for identifier, command in command_by_id.items():
        expected_result = "PASS" if command.get("exit_code") == 0 else "FAIL"
        expected_code = f"{identifier}-pass" if expected_result == "PASS" else f"{identifier}-fail"
        check = check_by_id.get(identifier, {})
        if check.get("result") != expected_result or check.get("code") != expected_code:
            failures.append(f"adapter evidence command/check mismatch: {identifier}")
    package = report.get("package", {})
    package_exact = (
        package.get("path") == str(PACKAGE.relative_to(ROOT))
        and package.get("bytes") == 25_449_031
        and package.get("sha256") == "83d08dd70d46d55c9f576368c0a7ad6861e74b9bc93fe510d1ad965bcf43d31c"
    )
    package_check = check_by_id.get("package-identity", {})
    if package_check.get("result") != ("PASS" if package_exact else "FAIL"):
        failures.append("adapter evidence package/check mismatch")
    contract_pass = bool(checks) and all(check.get("result") == "PASS" for check in checks)
    disposition = report.get("disposition", {})
    expected_status = "CONTRACT-PASS-LIVE-BLOCKED" if contract_pass else "BLOCKED"
    if disposition.get("status") != expected_status:
        failures.append("adapter evidence status overstates checks")
    if disposition.get("adapter_contract_implemented") is not contract_pass:
        failures.append("adapter contract implementation status disagrees with checks")
    for field in (
        "adapter_live_proven", "model_loaded", "network_isolation_live_proven",
        "quality_evaluated", "activation", "automatic_fallback", "release_approval",
    ):
        if disposition.get(field) is not False:
            failures.append(f"adapter evidence overstates {field}")
    if disposition.get("enabled_models") != 0:
        failures.append("adapter evidence enables a model")
    if report.get("blockers") != [
        "exact-model-artifact-not-present",
        "live-model-load-not-executed",
        "live-zero-egress-campaign-not-executed",
        "quality-and-repeatability-not-measured",
    ]:
        failures.append("adapter evidence blockers changed")
    return failures


def current_revision() -> str:
    process = subprocess.run(
        ("git", "rev-parse", "HEAD"),
        cwd=ROOT,
        check=True,
        capture_output=True,
        text=True,
        timeout=10,
    )
    return process.stdout.strip()


def tracked_tree_is_clean() -> bool:
    process = subprocess.run(
        ("git", "status", "--porcelain", "--untracked-files=no"),
        cwd=ROOT,
        check=True,
        capture_output=True,
        text=True,
        timeout=10,
    )
    return not process.stdout.strip()


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--source-revision", required=True)
    parser.add_argument("--write", action="store_true")
    args = parser.parse_args()
    revision = current_revision() if args.source_revision == "HEAD" else args.source_revision
    if revision != current_revision() or not REVISION.fullmatch(revision):
        print("- source revision is not the current exact HEAD")
        return 1
    if args.write and not tracked_tree_is_clean():
        print("- tracked worktree must be clean before writing evidence")
        return 1
    commands = execute_commands()
    report = build_report(revision, commands)
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
