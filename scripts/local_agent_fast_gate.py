#!/usr/bin/env python3
"""Check the changed mission components without regenerating evidence."""

from __future__ import annotations

import argparse
import ast
import io
import json
import os
from pathlib import Path, PurePosixPath
import shutil
import signal
import subprocess
import sys
import tabnanny
import time
import tokenize


ROOT = Path(__file__).resolve().parents[1]


def git(root: Path, *args: str) -> bytes:
    return subprocess.check_output(["git", *args], cwd=root)


def changed_paths(root: Path, base: str) -> list[str]:
    revision = git(root, "rev-parse", "--verify", "--end-of-options", base + "^{commit}")
    revision = revision.decode().strip()
    tracked = git(root, "diff", "--name-only", "--no-renames", "-z", revision, "--")
    untracked = git(root, "ls-files", "--others", "--exclude-standard", "-z")
    return sorted(os.fsdecode(item) for item in set((tracked + untracked).split(b"\0")) if item)


def affected_packages(metadata: dict, paths: list[str], root: Path) -> list[str]:
    packages = {p["name"]: p for p in metadata["packages"]}
    roots = {
        name: Path(p["manifest_path"]).parent.relative_to(root).as_posix()
        for name, p in packages.items()
    }
    global_inputs = {"Cargo.toml", "Cargo.lock", "rust-toolchain.toml", "rustfmt.toml"}
    if global_inputs.intersection(paths):
        return sorted(packages)
    affected = {
        name for name, directory in roots.items()
        if any(path == directory or path.startswith(directory + "/") for path in paths)
    }
    while True:
        expanded = affected | {
            name for name, p in packages.items()
            if any(d["name"] in affected for d in p["dependencies"])
        }
        if expanded == affected:
            return sorted(affected)
        affected = expanded


def python_lint(root: Path, paths: list[str]) -> None:
    """Dependency-free syntax, indentation and whitespace checks, no writes."""
    for name in paths:
        path = root / name
        if not path.is_file():
            continue
        source = path.read_text(encoding="utf-8")
        ast.parse(source, filename=name)
        tabnanny.process_tokens(tokenize.generate_tokens(io.StringIO(source).readline))
        if not source.endswith("\n") or any(line.rstrip() != line for line in source.splitlines()):
            raise ValueError(f"{name}: missing final newline or trailing whitespace")


def plan(root: Path, paths: list[str], metadata: dict) -> list[list[str]]:
    packages = affected_packages(metadata, paths, root)
    package_roots = [
        Path(p["manifest_path"]).parent.relative_to(root).as_posix() + "/"
        for p in metadata["packages"]
    ]
    python_paths = [p for p in paths if p.endswith(".py")]
    modules = set()
    for name in python_paths:
        path = PurePosixPath(name)
        if path.parent == PurePosixPath("tests") and path.name.startswith("test_"):
            module = "tests." + path.stem
        elif path.parent == PurePosixPath("scripts"):
            module = "tests.test_" + path.stem
        else:
            raise ValueError(f"{name}: add an explicit test mapping before using this gate")
        if not (root / (module.replace(".", "/") + ".py")).is_file():
            raise ValueError(f"{name}: required test module {module} is missing")
        modules.add(module)
    known_global = {"Cargo.toml", "Cargo.lock", "rust-toolchain.toml", "rustfmt.toml"}
    for name in paths:
        if name in known_global or name.endswith((".md", ".py")):
            continue
        if any(name.startswith(directory) for directory in package_roots):
            continue
        raise ValueError(f"{name}: no fast check registered; add coverage before proceeding")
    commands = [["git", "diff", "--check", "--"]]
    markdown = ["./" + p for p in paths if p.endswith(".md") and (root / p).is_file()]
    if markdown:
        commands.append(["node", "node_modules/.bin/markdownlint-cli2", *markdown])
    if python_paths:
        commands.append([sys.executable, "scripts/local_agent_fast_gate.py", "--lint-python", *python_paths])
        commands.append([sys.executable, "-m", "unittest", *sorted(modules)])
    if packages:
        selection = [item for name in packages for item in ("-p", name)]
        commands.extend([
            ["cargo", "fmt", *selection, "--", "--check"],
            ["cargo", "clippy", "--locked", "--offline", *selection,
             "--all-targets", "--", "-D", "warnings"],
            ["cargo", "test", "--locked", "--offline", *selection,
             "--all-targets", "--no-fail-fast"],
        ])
    return commands


def execute(root: Path, commands: list[list[str]], timeout: int) -> int:
    environment = os.environ.copy()
    environment.update(CARGO_BUILD_JOBS="2", RUST_TEST_THREADS="4")
    failures = 0
    for command in commands:
        actual = command
        if command[0] == "cargo" and Path("/tools/build-slot").is_file():
            actual = ["bash", "/tools/build-slot", *command]
        print("RUN " + json.dumps(command), flush=True)
        started = time.monotonic()
        try:
            with subprocess.Popen(actual, cwd=root, env=environment, start_new_session=True) as process:
                try:
                    code = process.wait(timeout=timeout)
                except (subprocess.TimeoutExpired, KeyboardInterrupt):
                    os.killpg(process.pid, signal.SIGTERM)
                    try:
                        process.wait(timeout=5)
                    except subprocess.TimeoutExpired:
                        os.killpg(process.pid, signal.SIGKILL)
                        process.wait()
                    # The leader may exit before descendants that ignore TERM.
                    try:
                        os.killpg(process.pid, signal.SIGKILL)
                    except ProcessLookupError:
                        pass
                    print("TIMEOUT or interrupted; this check did not pass", file=sys.stderr)
                    return 1
        except FileNotFoundError:
            print(f"Missing executable: {command[0]}; no automatic download", file=sys.stderr)
            return 1
        print(f"RESULT exit={code} seconds={time.monotonic() - started:.2f}", flush=True)
        failures += code != 0
    print("FAST GATE: " + ("FAIL" if failures else "PASS"), flush=True)
    return int(bool(failures))


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--base", default="HEAD", help="compare working tree to this commit, including untracked files")
    parser.add_argument("--plan", action="store_true", help="print checks without executing; not a passing gate")
    parser.add_argument("--timeout", type=int, default=600, help="per-command seconds; a timeout fails the gate")
    parser.add_argument("--lint-python", nargs="+", help=argparse.SUPPRESS)
    args = parser.parse_args()
    try:
        if args.lint_python:
            python_lint(ROOT, args.lint_python)
            return 0
        if args.timeout <= 0:
            raise ValueError("timeout must be positive")
        paths = changed_paths(ROOT, args.base)
        if not paths:
            raise ValueError("no changed paths; use --base with the commit before the change")
        if shutil.which("cargo") is None:
            raise ValueError("cargo is required to determine workspace ownership")
        metadata = json.loads(subprocess.check_output(
            ["cargo", "metadata", "--no-deps", "--format-version", "1", "--locked", "--offline"], cwd=ROOT
        ))
        commands = plan(ROOT, paths, metadata)
        # Check the entire requested range, including committed and staged edits.
        revision = git(ROOT, "rev-parse", "--verify", "--end-of-options", args.base + "^{commit}").decode().strip()
        commands[0] = ["git", "diff", "--check", revision, "--"]
        print(json.dumps({"paths": paths, "commands": commands}, indent=2), flush=True)
        if args.plan:
            return 0
        return execute(ROOT, commands, args.timeout)
    except (ValueError, SyntaxError, tabnanny.NannyNag, OSError, subprocess.CalledProcessError) as error:
        print(f"FAST GATE: FAIL: {error}", file=sys.stderr)
        return 1


if __name__ == "__main__":
    raise SystemExit(main())
