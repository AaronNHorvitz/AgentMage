#!/usr/bin/env python3
"""Verify deterministic locked resolution and fail-closed dependency mutations."""

from __future__ import annotations

import argparse
import hashlib
import json
import os
import shutil
import subprocess
import sys
import tempfile
import tomllib
from pathlib import Path
from typing import Any

try:
    from scripts.build_contract import load_contract
    from scripts.module_inventory import load_inventory
except ModuleNotFoundError:
    from build_contract import load_contract
    from module_inventory import load_inventory


ROOT = Path(__file__).resolve().parents[1]
REPORT_PATH = (
    ROOT
    / "artifacts"
    / "sprints"
    / "sprint-1"
    / "story-1.1"
    / "locked-resolution-report.json"
)
LOCK_PATHS = ("Cargo.lock", "package-lock.json", "platforms/macos/Package.resolved")
EXPECTED_MUTATIONS = {
    "substituted": "npm lock version mismatch: typescript expected 5.9.3 got 5.9.4",
    "missing": "missing locked npm package: node_modules/typescript",
    "revoked": "revoked dependency selected: npm:node_modules/typescript@5.9.3",
    "wrong-platform": (
        "wrong-platform dependency: platform-macos does not support fedora-x86_64"
    ),
}


def canonical_json(value: Any) -> str:
    return json.dumps(value, indent=2, sort_keys=True) + "\n"


def sha256_bytes(value: bytes) -> str:
    return hashlib.sha256(value).hexdigest()


def read_json(path: Path) -> Any:
    return json.loads(path.read_text(encoding="utf-8"))


def normalized_graph(root: Path) -> dict[str, Any]:
    cargo_lock = tomllib.loads((root / "Cargo.lock").read_text(encoding="utf-8"))
    npm_lock = read_json(root / "package-lock.json")
    swift_lock = read_json(root / "platforms/macos/Package.resolved")
    cargo = [
        {
            "name": package["name"],
            "version": package["version"],
            "dependencies": sorted(package.get("dependencies", [])),
        }
        for package in cargo_lock.get("package", [])
    ]
    npm = [
        {
            "path": path,
            "name": package.get("name"),
            "version": package.get("version"),
            "integrity": package.get("integrity"),
            "resolved": package.get("resolved"),
            "link": package.get("link", False),
        }
        for path, package in npm_lock.get("packages", {}).items()
    ]
    return {
        "cargo": sorted(cargo, key=lambda item: (item["name"], item["version"])),
        "npm": sorted(npm, key=lambda item: item["path"]),
        "swift": swift_lock,
    }


def graph_sha256(root: Path) -> str:
    return sha256_bytes(canonical_json(normalized_graph(root)).encode("utf-8"))


def expected_lock_hashes(root: Path) -> dict[str, str]:
    hashes = {}
    for line in (root / "supply-chain/dependency-hashes.sha256").read_text(
        encoding="utf-8"
    ).splitlines():
        digest, path = line.split("  ", 1)
        if path in LOCK_PATHS:
            hashes[path] = digest
    return hashes


def validate_candidate(
    root: Path,
    revoked: set[str] | None = None,
    platform: str = "fedora-x86_64",
    package_inputs: tuple[str, ...] = ("platform-linux",),
) -> list[str]:
    failures: list[str] = []
    expected_hashes = expected_lock_hashes(ROOT)
    for relative in LOCK_PATHS:
        path = root / relative
        if not path.is_file():
            failures.append(f"missing lockfile: {relative}")
            continue
        if sha256_bytes(path.read_bytes()) != expected_hashes.get(relative):
            failures.append(f"lock hash mismatch: {relative}")

    try:
        extension = read_json(root / "shells/vscode/package.json")
        npm_lock = read_json(root / "package-lock.json")
    except (OSError, json.JSONDecodeError) as error:
        failures.append(f"cannot parse npm resolution input: {error}")
        return failures
    packages = npm_lock.get("packages", {})
    for dependency, expected_version in extension.get("devDependencies", {}).items():
        lock_path = f"node_modules/{dependency}"
        locked = packages.get(lock_path)
        if not isinstance(locked, dict):
            failures.append(f"missing locked npm package: {lock_path}")
            continue
        actual_version = locked.get("version")
        if actual_version != expected_version:
            failures.append(
                f"npm lock version mismatch: {dependency} expected {expected_version} "
                f"got {actual_version}"
            )

    selected_components = {
        f"npm:{path}@{package.get('version')}"
        for path, package in packages.items()
        if path.startswith("node_modules/") and package.get("link") is not True
    }
    for component in sorted((revoked or set()) & selected_components):
        failures.append(f"revoked dependency selected: {component}")

    inventory = load_inventory()
    modules = {item["id"]: item for item in inventory["modules"]}
    for module_id in package_inputs:
        module = modules.get(module_id)
        if module is None:
            failures.append(f"unknown package input: {module_id}")
        elif platform not in module["platforms"]:
            failures.append(
                f"wrong-platform dependency: {module_id} does not support {platform}"
            )
    return failures


def copy_clean_inputs(destination: Path) -> None:
    contract = load_contract()
    for relative in contract["required_files"]:
        source = ROOT / relative
        target = destination / relative
        target.parent.mkdir(parents=True, exist_ok=True)
        shutil.copy2(source, target)
    supply_hash = ROOT / "supply-chain/dependency-hashes.sha256"
    target_hash = destination / "supply-chain/dependency-hashes.sha256"
    target_hash.parent.mkdir(parents=True, exist_ok=True)
    shutil.copy2(supply_hash, target_hash)


def _run(
    command: list[str], cwd: Path, environment: dict[str, str]
) -> tuple[int, str]:
    result = subprocess.run(
        command,
        cwd=cwd,
        env=environment,
        stdin=subprocess.DEVNULL,
        stdout=subprocess.PIPE,
        stderr=subprocess.STDOUT,
        text=True,
        timeout=30,
        check=False,
    )
    return result.returncode, result.stdout


def clean_resolution_run(run_id: str) -> dict[str, Any]:
    with tempfile.TemporaryDirectory(prefix="agentmage-resolution-") as temporary:
        root = Path(temporary)
        copy_clean_inputs(root)
        before = {path: (root / path).read_bytes() for path in LOCK_PATHS}
        original_home = Path.home()
        rustup_home = Path(os.environ.get("RUSTUP_HOME", original_home / ".rustup"))
        cargo_candidates = sorted(
            rustup_home.glob("toolchains/1.95.0-*/bin/cargo")
        )
        if len(cargo_candidates) != 1:
            raise OSError("exactly one installed Rust 1.95.0 Cargo binary is required")
        cargo_binary = cargo_candidates[0]
        rustc_binary = cargo_binary.with_name("rustc")
        environment = os.environ.copy()
        environment.update(
            {
                "CARGO_NET_OFFLINE": "true",
                "CARGO_HOME": str(root / "cargo-home"),
                "CARGO_TARGET_DIR": str(root / "target"),
                "HOME": str(root / "home"),
                "NPM_CONFIG_AUDIT": "false",
                "NPM_CONFIG_CACHE": str(root / "npm-cache"),
                "NPM_CONFIG_FUND": "false",
                "NPM_CONFIG_UPDATE_NOTIFIER": "false",
                "RUSTC": str(rustc_binary),
                "RUSTUP_HOME": str(rustup_home),
                "RUSTUP_NO_UPDATE_CHECK": "1",
            }
        )
        cargo_command = [
            str(cargo_binary),
            "metadata",
            "--locked",
            "--offline",
            "--format-version",
            "1",
            "--no-deps",
        ]
        npm_command = [
            "npm",
            "install",
            "--package-lock-only",
            "--ignore-scripts",
            "--offline",
            "--no-audit",
            "--no-fund",
        ]
        cargo_code, cargo_output = _run(cargo_command, root, environment)
        npm_code, npm_output = _run(npm_command, root, environment)
        after = {path: (root / path).read_bytes() for path in LOCK_PATHS}
        diagnostics = validate_candidate(root)
        status = (
            "pass"
            if cargo_code == 0
            and npm_code == 0
            and before == after
            and not diagnostics
            else "fail"
        )
        return {
            "run_id": run_id,
            "status": status,
            "graph_sha256": graph_sha256(root),
            "cargo": {
                "command": "cargo metadata --locked --offline --format-version 1 --no-deps",
                "exit_code": cargo_code,
                "output_captured": bool(cargo_output),
                "output_redacted": True,
            },
            "npm": {
                "command": (
                    "npm install --package-lock-only --ignore-scripts --offline "
                    "--no-audit --no-fund"
                ),
                "exit_code": npm_code,
                "output_captured": bool(npm_output),
                "output_redacted": True,
            },
            "locks_unchanged": before == after,
            "diagnostics": diagnostics,
            "swift_execution": "blocked-macos",
        }


def mutation_cases() -> list[dict[str, Any]]:
    cases = []
    with tempfile.TemporaryDirectory(prefix="agentmage-mutations-") as temporary:
        base = Path(temporary)
        for scenario in ("substituted", "missing", "revoked", "wrong-platform"):
            root = base / scenario
            copy_clean_inputs(root)
            revoked: set[str] = set()
            package_inputs = ("platform-linux",)
            if scenario in {"substituted", "missing"}:
                path = root / "package-lock.json"
                lock = read_json(path)
                if scenario == "substituted":
                    lock["packages"]["node_modules/typescript"]["version"] = "5.9.4"
                else:
                    del lock["packages"]["node_modules/typescript"]
                path.write_text(canonical_json(lock), encoding="utf-8")
            elif scenario == "revoked":
                revoked = {"npm:node_modules/typescript@5.9.3"}
            else:
                package_inputs = ("platform-macos",)
            diagnostics = validate_candidate(
                root,
                revoked=revoked,
                package_inputs=package_inputs,
            )
            expected = EXPECTED_MUTATIONS[scenario]
            cases.append(
                {
                    "scenario": scenario,
                    "expected_diagnostic": expected,
                    "observed_diagnostics": diagnostics,
                    "status": "pass" if expected in diagnostics else "fail",
                }
            )
    return cases


def build_report() -> dict[str, Any]:
    clean_runs = [clean_resolution_run("clean-a"), clean_resolution_run("clean-b")]
    mutations = mutation_cases()
    graph_equal = clean_runs[0]["graph_sha256"] == clean_runs[1]["graph_sha256"]
    return {
        "schema_version": 1,
        "test_id": "S-001-UT02",
        "status": (
            "pass"
            if graph_equal
            and all(item["status"] == "pass" for item in clean_runs)
            and all(item["status"] == "pass" for item in mutations)
            else "fail"
        ),
        "network_mode": "offline-command-flags",
        "clean_graphs_identical": graph_equal,
        "clean_runs": clean_runs,
        "mutation_cases": mutations,
        "macos_resolution": "blocked-macos",
        "macos_support_claim": "none",
    }


def validate_report(report: Any, expected: dict[str, Any] | None = None) -> list[str]:
    failures: list[str] = []
    if not isinstance(report, dict):
        return ["locked resolution report must be an object"]
    if report.get("schema_version") != 1 or report.get("test_id") != "S-001-UT02":
        failures.append("locked resolution report identity is invalid")
    if report.get("status") != "pass" or report.get("clean_graphs_identical") is not True:
        failures.append("locked resolution report is not passing deterministically")
    if report.get("network_mode") != "offline-command-flags":
        failures.append("locked resolution did not require offline command flags")
    if report.get("macos_resolution") != "blocked-macos":
        failures.append("locked resolution must retain blocked macOS status")
    if report.get("macos_support_claim") != "none":
        failures.append("Linux lock resolution cannot make a macOS support claim")
    clean_runs = report.get("clean_runs", [])
    if len(clean_runs) != 2:
        failures.append("exactly two clean resolution runs are required")
    for run in clean_runs:
        if run.get("status") != "pass" or run.get("locks_unchanged") is not True:
            failures.append(f"clean resolution run failed: {run.get('run_id')}")
        if run.get("cargo", {}).get("exit_code") != 0:
            failures.append(f"Cargo resolution failed: {run.get('run_id')}")
        if run.get("npm", {}).get("exit_code") != 0:
            failures.append(f"npm resolution failed: {run.get('run_id')}")
    mutations = report.get("mutation_cases", [])
    observed_scenarios = {item.get("scenario") for item in mutations}
    if observed_scenarios != set(EXPECTED_MUTATIONS) or len(mutations) != 4:
        failures.append("locked resolution mutation closure is incomplete")
    for mutation in mutations:
        scenario = mutation.get("scenario")
        expected_diagnostic = EXPECTED_MUTATIONS.get(scenario)
        if mutation.get("status") != "pass":
            failures.append(f"dependency mutation was not rejected: {scenario}")
        if expected_diagnostic not in mutation.get("observed_diagnostics", []):
            failures.append(f"dependency mutation diagnostic is imprecise: {scenario}")
    if expected is not None and report != expected:
        failures.append("locked resolution report is stale")
    return failures


def write_report() -> None:
    REPORT_PATH.parent.mkdir(parents=True, exist_ok=True)
    REPORT_PATH.write_text(canonical_json(build_report()), encoding="utf-8")


def check_report() -> list[str]:
    try:
        report = read_json(REPORT_PATH)
    except (OSError, json.JSONDecodeError) as error:
        return [f"cannot read locked resolution report: {error}"]
    expected = build_report()
    return validate_report(report, expected)


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--write", action="store_true")
    args = parser.parse_args()
    try:
        if args.write:
            write_report()
        failures = check_report()
    except (OSError, KeyError, json.JSONDecodeError, tomllib.TOMLDecodeError) as error:
        print(f"locked resolution validation failed: {error}", file=sys.stderr)
        return 1
    if failures:
        for failure in failures:
            print(f"locked resolution validation failed: {failure}", file=sys.stderr)
        return 1
    print("locked resolution and dependency mutations validated")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
