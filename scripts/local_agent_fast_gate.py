#!/usr/bin/env python3
"""Check the changed mission components without regenerating evidence."""

from __future__ import annotations

import argparse
import ast
import io
import json
import os
from pathlib import Path, PurePosixPath
import select
import shutil
import signal
import subprocess
import sys
import tabnanny
import time
import tokenize


ROOT = Path(__file__).resolve().parents[1]
CRATE_INPUT_SUFFIXES = {".rs", ".toml", ".json", ".sql", ".txt"}
CRATE_EXTENSIONLESS_INPUTS = {
    "kernel/contracts/LICENSE",
    "shells/host/fixtures/extension-sample/packages/sample-report/source",
    "shells/host/fixtures/extension-sample/packages/sample-linter/source",
    "shells/host/fixtures/extension-sample/packages/sample-formatter/source",
}


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


def untracked_whitespace(root: Path) -> None:
    """Apply Git's whitespace rules to new files, without staging them."""
    names = git(root, "ls-files", "--others", "--exclude-standard", "-z")
    for item in names.split(b"\0"):
        if not item:
            continue
        name = os.fsdecode(item)
        result = subprocess.run(
            ["git", "diff", "--no-index", "--check", "--", "/dev/null", "./" + name],
            cwd=root, capture_output=True,
        )
        # A binary addition can return 1 with no diagnostics. Whitespace and
        # command errors have diagnostics or a different nonzero status.
        if result.returncode not in (0, 1) or result.stdout or result.stderr:
            raise ValueError(f"{name}: untracked whitespace check failed: "
                             + os.fsdecode(result.stdout + result.stderr))


def review_text_lint(root: Path, paths: list[str]) -> None:
    """Validate retained text report format, not reviewer independence or truth."""
    for name in paths:
        source = (root / name).read_text(encoding="utf-8")
        lines = source.splitlines()
        if not source.endswith("\n") or any(line.rstrip() != line for line in lines):
            raise ValueError(f"{name}: missing final newline or trailing whitespace")
        if len(lines) < 3 or not lines[0].startswith("Commit: "):
            raise ValueError(f"{name}: review must start with its exact commit")
        commit = lines[0].removeprefix("Commit: ")
        if len(commit) != 40 or any(c not in "0123456789abcdef" for c in commit):
            raise ValueError(f"{name}: review commit must be a full lowercase SHA")
        if not lines[1].startswith("Scope: ") or not lines[1].removeprefix("Scope: ").strip():
            raise ValueError(f"{name}: review scope is missing")
        if lines[-1] not in {"Verdict: PASS", "Verdict: FINDINGS", "Verdict: FAIL", "Verdict: BLOCKED"}:
            raise ValueError(f"{name}: review must end with its explicit verdict")


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
    review_paths = [p for p in paths if PurePosixPath(p).parent == PurePosixPath("docs/reviews")
                    and p.endswith(".txt")]
    for name in paths:
        if name in known_global or name in review_paths or name.endswith((".md", ".py")):
            continue
        if any(name.startswith(directory) for directory in package_roots) and (
            PurePosixPath(name).suffix in CRATE_INPUT_SUFFIXES or name in CRATE_EXTENSIONLESS_INPUTS
        ):
            continue
        raise ValueError(f"{name}: no fast check registered; add coverage before proceeding")
    commands = [["git", "diff", "--check", "--"]]
    commands.append([sys.executable, "scripts/local_agent_fast_gate.py", "--lint-untracked"])
    reviews = [p for p in review_paths if (root / p).is_file()]
    if reviews:
        commands.append([sys.executable, "scripts/local_agent_fast_gate.py", "--lint-reviews", *reviews])
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


def stop_check(process: subprocess.Popen) -> None:
    """Clean the check's group even when its leader exited successfully."""
    try:
        os.killpg(process.pid, signal.SIGTERM)
    except ProcessLookupError:
        pass
    try:
        process.wait(timeout=5)
    except subprocess.TimeoutExpired:
        pass
    try:
        os.killpg(process.pid, signal.SIGKILL)
    except ProcessLookupError:
        pass
    process.wait()


def supervise(lifeline: int, timeout: int, command: list[str]) -> int:
    """A separate session watches EOF from the gate, including after SIGKILL.

    Only the gate holds the pipe's write end. Checks get neither end. The
    supervisor owns a separate check process group and always cleans that group.
    """
    stopping = False

    def interrupted(_signum, _frame):
        nonlocal stopping
        stopping = True

    for signum in (signal.SIGINT, signal.SIGTERM, signal.SIGHUP):
        signal.signal(signum, interrupted)
    process = None
    try:
        deadline = time.monotonic() + timeout
        if stopping or select.select([lifeline], [], [], 0)[0]:
            return 130
        process = subprocess.Popen(command, process_group=0)
        while True:
            if stopping or select.select([lifeline], [], [], 0.05)[0]:
                print("Check interrupted; this check did not pass", file=sys.stderr)
                return 130
            code = process.poll()
            if code is not None:
                return code if code >= 0 else 128 - code
            if time.monotonic() >= deadline:
                print("TIMEOUT; this check did not pass", file=sys.stderr)
                return 124
    except FileNotFoundError:
        print(f"Missing executable: {command[0]}; no automatic download", file=sys.stderr)
        return 127
    finally:
        if process is not None:
            stop_check(process)
        os.close(lifeline)


def execute(root: Path, commands: list[list[str]], timeout: int) -> int:
    stopping = False

    def interrupted(_signum, _frame):
        nonlocal stopping
        stopping = True

    previous = {s: signal.signal(s, interrupted) for s in (signal.SIGINT, signal.SIGTERM, signal.SIGHUP)}
    try:
        return execute_checks(root, commands, timeout, lambda: stopping)
    finally:
        for signum, handler in previous.items():
            signal.signal(signum, handler)


def execute_checks(root: Path, commands: list[list[str]], timeout: int, stopping) -> int:
    environment = os.environ.copy()
    environment.update(CARGO_BUILD_JOBS="2", RUST_TEST_THREADS="4")
    failures = 0
    for command in commands:
        if stopping():
            return 1
        actual = command
        if command[0] == "cargo" and Path("/tools/build-slot").is_file():
            actual = ["bash", "/tools/build-slot", *command]
        print("RUN " + json.dumps(command), flush=True)
        started = time.monotonic()
        read_end, write_end = os.pipe()
        process = None
        try:
            process = subprocess.Popen(
                [sys.executable, str(Path(__file__).resolve()), "--supervise", str(read_end),
                 str(timeout), *actual], cwd=root, env=environment, start_new_session=True,
                pass_fds=(read_end,),
            )
            os.close(read_end)
            read_end = None
            while process.poll() is None and not stopping():
                time.sleep(0.05)
        finally:
            if read_end is not None:
                os.close(read_end)
            os.close(write_end)
            if process is not None:
                process.wait()
        if stopping():
            print("Interrupted; this gate did not pass", file=sys.stderr)
            return 1
        code = process.returncode
        print(f"RESULT exit={code} seconds={time.monotonic() - started:.2f}", flush=True)
        failures += code != 0
    print("FAST GATE: " + ("FAIL" if failures else "PASS"), flush=True)
    return int(bool(failures))


def main() -> int:
    if len(sys.argv) > 1 and sys.argv[1] == "--supervise":
        return supervise(int(sys.argv[2]), int(sys.argv[3]), sys.argv[4:])
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--base", default="HEAD", help="compare working tree to this commit, including untracked files")
    parser.add_argument("--plan", action="store_true", help="print checks without executing; not a passing gate")
    parser.add_argument("--timeout", type=int, default=600, help="per-command seconds; a timeout fails the gate")
    parser.add_argument("--lint-python", nargs="+", help=argparse.SUPPRESS)
    parser.add_argument("--lint-untracked", action="store_true", help=argparse.SUPPRESS)
    parser.add_argument("--lint-reviews", nargs="+", help=argparse.SUPPRESS)
    args = parser.parse_args()
    try:
        if args.lint_python:
            python_lint(ROOT, args.lint_python)
            return 0
        if args.lint_untracked:
            untracked_whitespace(ROOT)
            return 0
        if args.lint_reviews:
            review_text_lint(ROOT, args.lint_reviews)
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
