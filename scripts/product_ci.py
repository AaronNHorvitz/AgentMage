#!/usr/bin/env python3
"""Validate and execute deterministic AgentMage product CI lanes."""

from __future__ import annotations

import argparse
import json
import os
import re
import subprocess
import sys
import tomllib
from pathlib import Path
from typing import Any


ROOT = Path(__file__).resolve().parents[1]
POLICY_PATH = ROOT / "architecture" / "product-ci-policy.json"
WORKFLOW_PATH = ROOT / ".github" / "workflows" / "product.yml"
PACKAGE_PATH = ROOT / "package.json"
RUST_TOOLCHAIN_PATH = ROOT / "rust-toolchain.toml"
PINNED_ACTION = re.compile(r"^[A-Za-z0-9_.-]+/[A-Za-z0-9_.-]+@[0-9a-f]{40}$")
VERSION_PATTERNS = {
    "node": lambda version: re.compile(rf"^v{re.escape(version)}$"),
    "npm": lambda version: re.compile(rf"^{re.escape(version)}$"),
    "rust": lambda version: re.compile(rf"^rustc {re.escape(version)}\b"),
}
EXPECTED_JOBS = (
    "contract",
    "format",
    "lint",
    "build",
    "rust_unit_contract",
    "vscode_shell",
    "native_linux_status",
    "windows_contract",
)
EXPECTED_LANES = {
    "format": [
        ["cargo", "fmt", "--all", "--", "--check"],
        ["npm", "run", "format:check", "--workspace", "@agentmage/vscode-shell"],
    ],
    "lint": [
        [
            "cargo",
            "clippy",
            "--workspace",
            "--all-targets",
            "--locked",
            "--",
            "-D",
            "warnings",
        ],
        ["npm", "run", "lint", "--workspace", "@agentmage/vscode-shell"],
        ["npm", "run", "strict-local-source:check"],
        ["npm", "run", "hostile-network:check"],
    ],
    "build": [
        ["cargo", "build", "--workspace", "--all-targets", "--locked"],
        ["npm", "run", "build", "--workspace", "@agentmage/vscode-shell"],
    ],
    "rust-unit-contract": [["cargo", "test", "--workspace", "--locked"]],
    "vscode-shell": [
        ["npm", "run", "test", "--workspace", "@agentmage/vscode-shell"]
    ],
}
EXPECTED_NATIVE_TESTS = (
    "linux_read::tests::approved_read_is_receipted_replay_safe_and_restart_verifiable",
    "linux_read::tests::stale_and_cancelled_previews_start_no_worker_and_publish_no_receipt",
    "inventory::tests::live_self_inventory_attributes_executable_and_open_writable_descriptor",
    "platform::tests::production_discovery_never_self_supplies_release_trust",
    "sandbox::tests::bounded_scratch_cannot_escape_into_the_held_object_or_host_workspace",
    "sandbox::tests::directory_worker_receives_only_the_bounded_exclusion_safe_projection",
    "sandbox::tests::foreign_workspace_identity_never_starts_a_worker",
    "sandbox::tests::fresh_worker_reads_only_the_canonical_workspace_file",
    "sandbox::tests::transient_service_terminates_an_unbounded_worker",
    "sandbox::tests::worker_cannot_resolve_sibling_parent_or_hidden_descriptor_content",
    "sandbox::tests::worker_cannot_write_the_read_only_workspace",
    "sandbox::tests::worker_has_no_ambient_host_paths_devices_or_processes",
    "sandbox::tests::worker_kernel_status_confirms_no_new_privileges_and_seccomp",
    "sandbox::tests::worker_output_is_drained_but_never_retained_past_the_bound",
    "sandbox::tests::worker_receives_only_the_fixed_environment_and_no_network",
    "secret_service::tests::live_operational_key_provisioning_is_exact_non_overwriting_and_cleaned",
    "secret_service::tests::live_service_probe_returns_only_a_content_free_receipt",
    "secret_service::tests::live_service_round_trip_is_exact_and_cleanup_is_verified",
    "tests::isolated_bind_mount_swap_is_rejected_as_mount_changed",
)


def read_json(path: Path) -> Any:
    return json.loads(path.read_text(encoding="utf-8"))


def _workflow_jobs(workflow: str) -> set[str]:
    try:
        jobs = workflow.split("\njobs:\n", 1)[1]
    except IndexError:
        return set()
    return set(re.findall(r"^  ([a-z][a-z0-9_]*):\s*$", jobs, re.MULTILINE))


def validate_contract(
    policy: Any,
    workflow: str,
    package: Any,
    rust_toolchain: Any,
    documentation_workflow: str,
) -> list[str]:
    failures: list[str] = []
    if not isinstance(policy, dict):
        return ["product CI policy must be an object"]
    if policy.get("schema_version") != 1 or policy.get("status") != "enforced":
        failures.append("product CI policy identity is invalid")
    toolchains = policy.get("toolchains", {})
    if toolchains != {"node": "24.15.0", "npm": "11.12.1", "rust": "1.95.0"}:
        failures.append("product CI toolchain closure drifted")
    if tuple(policy.get("required_jobs", [])) != EXPECTED_JOBS:
        failures.append("product CI required-job policy closure drifted")
    if policy.get("lanes") != EXPECTED_LANES:
        failures.append("product CI command closure drifted")
    native = policy.get("native_linux", {})
    if native.get("generic_ci_disposition") != "pending-native-execution":
        failures.append("product CI native disposition drifted")
    if native.get("inventory_command") != [
        "cargo",
        "test",
        "--workspace",
        "--locked",
        "--",
        "--list",
        "--ignored",
    ]:
        failures.append("product CI native inventory command drifted")
    if tuple(native.get("expected_tests", [])) != EXPECTED_NATIVE_TESTS:
        failures.append("product CI native pending-test closure drifted")
    if policy.get("native_windows") != {
        "disposition": "partial-native-identity-evidence-other-controls-blocked",
        "runner": "windows-2022",
        "commands": [
            [
                "cargo",
                "+1.95.0",
                "test",
                "-p",
                "agentmage-platform-windows",
                "--locked",
            ],
            [
                "cargo",
                "+1.95.0",
                "run",
                "-p",
                "agentmage-platform-windows",
                "--bin",
                "native_evidence",
                "--locked",
            ],
        ],
    }:
        failures.append("product CI Windows native identity closure drifted")
    if policy.get("documentation_gate") != {
        "independent": True,
        "workflow": ".github/workflows/documentation.yml",
    }:
        failures.append("product CI documentation independence policy drifted")
    engines = package.get("engines", {}) if isinstance(package, dict) else {}
    if engines.get("node") != f">={toolchains.get('node')} <25":
        failures.append("product CI Node version differs from package.json")
    if engines.get("npm") != f">={toolchains.get('npm')} <12":
        failures.append("product CI npm version differs from package.json")
    if package.get("packageManager") != f"npm@{toolchains.get('npm')}":
        failures.append("product CI npm version differs from packageManager")
    rust = rust_toolchain.get("toolchain", {}) if isinstance(rust_toolchain, dict) else {}
    if rust.get("channel") != toolchains.get("rust"):
        failures.append("product CI Rust version differs from rust-toolchain.toml")
    if sorted(rust.get("components", [])) != ["clippy", "rustfmt"]:
        failures.append("product CI Rust component closure drifted")

    expected_jobs = set(EXPECTED_JOBS)
    actual_jobs = _workflow_jobs(workflow)
    if actual_jobs != expected_jobs:
        failures.append("product CI workflow job closure drifted")
    for lane in policy.get("lanes", {}):
        invocation = f"python3 scripts/product_ci.py --run-lane {lane}"
        if workflow.count(invocation) != 1:
            failures.append(f"product CI lane is not invoked exactly once: {lane}")
    native_invocation = "python3 scripts/product_ci.py --inventory-native"
    if workflow.count(native_invocation) != 1:
        failures.append("native Linux inventory lane is not invoked exactly once")
    windows_invocation = "cargo +1.95.0 test -p agentmage-platform-windows --locked"
    if workflow.count(windows_invocation) != 1:
        failures.append("Windows native test boundary is not invoked exactly once")
    windows_evidence = (
        "cargo +1.95.0 run -p agentmage-platform-windows "
        "--bin native_evidence --locked"
    )
    if workflow.count(windows_evidence) != 1:
        failures.append("Windows native evidence boundary is not invoked exactly once")
    if f'node-version: "{toolchains.get("node")}"' not in workflow:
        failures.append("product CI workflow does not install the declared Node version")
    npm_install = f"npm install --global npm@{toolchains.get('npm')}"
    if npm_install not in workflow:
        failures.append("product CI workflow does not install the declared npm version")
    rust_install = f"rustup toolchain install {toolchains.get('rust')}"
    if rust_install not in workflow:
        failures.append("product CI workflow does not install the declared Rust version")
    if "permissions:\n  contents: read" not in workflow:
        failures.append("product CI workflow permissions are not read-only")
    if "continue-on-error: true" in workflow:
        failures.append("product CI workflow weakens a required failure")
    if "docs:check" in workflow or "docs:clean-check" in workflow:
        failures.append("product CI workflow is not independent from documentation CI")
    if "product_ci.py" in documentation_workflow:
        failures.append("documentation CI is not independent from product CI")
    action_values = re.findall(r"^\s*uses:\s*(\S+)\s*$", workflow, re.MULTILINE)
    if not action_values or any(not PINNED_ACTION.fullmatch(item) for item in action_values):
        failures.append("product CI actions must be pinned to full commit identities")
    return failures


def check_contract(root: Path = ROOT) -> list[str]:
    try:
        policy = read_json(root / POLICY_PATH.relative_to(ROOT))
        package = read_json(root / PACKAGE_PATH.relative_to(ROOT))
        rust_toolchain = tomllib.loads(
            (root / RUST_TOOLCHAIN_PATH.relative_to(ROOT)).read_text(encoding="utf-8")
        )
        workflow = (root / WORKFLOW_PATH.relative_to(ROOT)).read_text(encoding="utf-8")
        documentation = (
            root / ".github" / "workflows" / "documentation.yml"
        ).read_text(encoding="utf-8")
    except (OSError, json.JSONDecodeError, tomllib.TOMLDecodeError) as error:
        return [f"cannot read product CI contract: {error}"]
    return validate_contract(policy, workflow, package, rust_toolchain, documentation)


def sanitize_output(output: str, root: Path = ROOT) -> str:
    replacements = {
        str(root): "<REPOSITORY>",
        str(Path.home()): "<HOME>",
        os.environ.get("RUNNER_TEMP", ""): "<RUNNER_TEMP>",
        os.environ.get("RUNNER_WORKSPACE", ""): "<RUNNER_WORKSPACE>",
    }
    sanitized = output.replace("\r\n", "\n")
    for value, replacement in sorted(
        ((key, value) for key, value in replacements.items() if key),
        key=lambda item: len(item[0]),
        reverse=True,
    ):
        sanitized = sanitized.replace(value, replacement)
    return sanitized


def _summary(message: str) -> None:
    destination = os.environ.get("GITHUB_STEP_SUMMARY")
    if destination:
        with Path(destination).open("a", encoding="utf-8") as summary:
            summary.write(message.rstrip() + "\n")


def _run(command: list[str], root: Path = ROOT) -> subprocess.CompletedProcess[str]:
    return subprocess.run(
        command,
        cwd=root,
        stdin=subprocess.DEVNULL,
        stdout=subprocess.PIPE,
        stderr=subprocess.STDOUT,
        text=True,
        timeout=1800,
        check=False,
    )


def run_lane(lane: str, root: Path = ROOT) -> int:
    policy = read_json(root / POLICY_PATH.relative_to(ROOT))
    commands = policy.get("lanes", {}).get(lane)
    if not isinstance(commands, list):
        print(f"product CI failed: unknown lane: {lane}", file=sys.stderr)
        return 2
    for index, command in enumerate(commands, start=1):
        result = _run(command, root)
        if result.returncode != 0:
            output = sanitize_output(result.stdout, root)
            tail = "\n".join(output.splitlines()[-80:])
            print(
                f"product CI failed: lane={lane} command={index} "
                f"exit={result.returncode}",
                file=sys.stderr,
            )
            if tail:
                print(tail, file=sys.stderr)
            _summary(f"Product lane `{lane}`: **FAILED** at command {index}.")
            return 1
    print(f"product CI lane passed: {lane}")
    _summary(f"Product lane `{lane}`: **PASSED**.")
    return 0


def inventory_native(root: Path = ROOT) -> int:
    policy = read_json(root / POLICY_PATH.relative_to(ROOT))
    native = policy["native_linux"]
    result = _run(native["inventory_command"], root)
    if result.returncode != 0:
        print("product CI failed: native Linux inventory command failed", file=sys.stderr)
        print("\n".join(sanitize_output(result.stdout, root).splitlines()[-80:]), file=sys.stderr)
        return 1
    actual = sorted(
        line.removesuffix(": test").strip()
        for line in result.stdout.splitlines()
        if line.strip().endswith(": test")
    )
    expected = sorted(native["expected_tests"])
    if actual != expected:
        print("product CI failed: native Linux pending-test inventory drifted", file=sys.stderr)
        return 1
    message = (
        f"Native Linux execution: **PENDING**. Inventory verified for {len(actual)} "
        "tests; generic CI did not execute them."
    )
    print(message)
    _summary(message)
    return 0


def check_environment(stack: str, root: Path = ROOT) -> list[str]:
    policy = read_json(root / POLICY_PATH.relative_to(ROOT))
    names = ("node", "npm", "rust") if stack == "all" else (stack,)
    commands = {"node": ["node", "--version"], "npm": ["npm", "--version"], "rust": ["rustc", "--version"]}
    failures: list[str] = []
    for name in names:
        result = _run(commands[name], root)
        version = result.stdout.strip().splitlines()[0] if result.stdout.strip() else ""
        expected = policy["toolchains"][name]
        if result.returncode != 0 or not VERSION_PATTERNS[name](expected).search(version):
            failures.append(f"product CI {name} toolchain does not match policy")
    return failures


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--check", action="store_true")
    parser.add_argument("--check-environment", choices=("all", "node", "npm", "rust"))
    parser.add_argument("--run-lane", choices=("format", "lint", "build", "rust-unit-contract", "vscode-shell"))
    parser.add_argument("--inventory-native", action="store_true")
    args = parser.parse_args()
    selected = sum(
        bool(item)
        for item in (args.check, args.check_environment, args.run_lane, args.inventory_native)
    )
    if selected != 1:
        parser.error("select exactly one product CI operation")
    if args.run_lane:
        return run_lane(args.run_lane)
    if args.inventory_native:
        return inventory_native()
    failures = check_environment(args.check_environment) if args.check_environment else check_contract()
    if failures:
        for failure in failures:
            print(f"product CI failed: {failure}", file=sys.stderr)
        return 1
    print("product CI contract passed" if args.check else "product CI environment passed")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
