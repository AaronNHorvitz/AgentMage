#!/usr/bin/env python3
"""Score frozen repository development tasks without changing the agent harness.

Original evaluation tooling. Historical inputs remain pinned and are never edited.
This three-task development pilot is not the complete T-311 suite or a model score.
"""
from __future__ import annotations

import argparse
import hashlib
import io
import json
import os
from pathlib import Path
import resource
import selectors
import signal
import stat
import subprocess
import sys
import tarfile
import tempfile
import time

ROOT = Path(__file__).resolve().parents[1]
VERSION = "coding-development-pilot-v1"
MAX_FILE = 1024 * 1024
MAX_TREE = 16 * MAX_FILE
TIMEOUT = 90
# Freeze these inputs. Expansion must add tasks; never rewrite these tests or pins.
TASKS = [{'id': 'dev-001',
  'split': 'development',
  'base': 'aa3134b224e887fbf5962fbf724b8d3d54cbe8ec',
  'reference': '14e33a4565e34663fd84fe0939b3ccc5bc2e633d',
  'hidden_test': 'tests/test_coding_harness.py',
  'hidden_sha256': '781687002b761c6d8b0c5aae80d7ed4384cf1a7590655ea8fc1f4a03f19bd049',
  'source': 'scripts/coding_harness.py',
  'base_sha256': '38590b031cc6bed5c12d3c86bb923d383a006b2a6a00d6ea3653342b6982537d',
  'reference_sha256': '4b1c37aebb77d0d752e59fcba67f79ee05c826c63abdbf79c23009817f494672',
  'projection_sha256': '06ff2dcd1c9cf9c4a71c5f4e5c891c7c90d863248eef17a5486028248e4b056c',
  'statement': "Extend the development harness's implementation identity to bind untracked source "
               'bytes, not just their names. Enumerate untracked files using Git; ignore '
               'Git-ignored build outputs. Return stable path, byte-count and SHA-256 identities '
               'in sorted order. Refuse aliases, non-regular files, over 4096 paths, files over 16 '
               'MiB or aggregate content over 64 MiB, and concurrent changes. Include the '
               'identities and their canonical aggregate digest alongside the existing '
               'tracked-diff, HEAD and binary identities. Preserve existing authority checks.',
  'expected_tests': 12},
 {'id': 'dev-002',
  'split': 'development',
  'base': '6d1cb28424866cb95c1e5460427b4f0bbfe44384',
  'reference': '0898b05a6c82bd531704e3f13db412f537ca4f75',
  'hidden_test': 'tests/test_coding_harness_model_diagnostics.py',
  'hidden_sha256': '33247a099c00ce191309949f52623d9154b9a5b9a66aea4ef35373e01f57d9a4',
  'source': 'scripts/coding_harness.py',
  'base_sha256': '111c4ef587a76f0a70fb8a968c6448718481c3679eed905371ce31e263a826f0',
  'reference_sha256': '3f7f4e3d8c53fc7140324261ef0b501a2882fc2a570b6922b1ff4ff2a5b216eb',
  'projection_sha256': '7995b218e914be7443fb82b5e8fa8b8d7cffbe00882314f19da18b7fcd100445',
  'statement': 'Report the requested model separately from serving or qualification in development '
               'diagnostics. Infer it only from a currently owned process with exactly one --model '
               'argument and a closed supported value, never from an unowned process, missing or '
               'conflicting arguments. Do not echo unknown/private values, change the run-record '
               'schema or claim that a requested model is running or qualified. Preserve '
               'reservation, startup, running and stale-record lifecycle behavior.',
  'expected_tests': 3},
 {'id': 'dev-003',
  'split': 'development',
  'base': '4fda20f7d9f335cfe859cb3e01206573ea5e3e08',
  'reference': 'e88a9b42cd35821d81f6c8d706b65f151aadcbe2',
  'hidden_test': 'tests/test_local_agent_fast_gate.py',
  'hidden_sha256': 'efe56d376453b501c99f73aeee5a2813e9b9c0d9ab9c7b68df2eb7731a769c23',
  'source': 'scripts/local_agent_fast_gate.py',
  'base_sha256': 'ecdcfac774e042e975b5aeef1c487522b9770e9eb867642e82b267ce35fda965',
  'reference_sha256': '07b48ae80bd9d26a055e128184c289f583665634cf55d6da796cf42e0e034f04',
  'projection_sha256': 'b2e690696102c28664af7689b08372f9ac70761d9870c3c00d9e7a971ec015cf',
  'statement': "Correct the development fast gate's process cleanup: an exited check leader must "
               'remain unreaped while both process-group cleanup signals are sent, preventing '
               'group-ID reuse. Preserve the original exit or signal status, timeout and '
               'interruption behavior. Also accept the actual retained review endings PASS, '
               'FINDINGS, or INCONCLUSIVE followed by exactly one nonempty reason, while rejecting '
               'malformed endings and preserving report bytes.',
  'expected_tests': 23}]


class EvaluationError(ValueError):
    """A fixed diagnostic; task/candidate contents stay in private output."""


def sha(data: bytes) -> str:
    return hashlib.sha256(data).hexdigest()


def git(*args: str) -> bytes:
    return subprocess.check_output(
        ["git", "--no-optional-locks", *args], cwd=ROOT,
        stderr=subprocess.DEVNULL, timeout=30,
    )


def pinned_blob(revision: str, name: str, expected: str) -> bytes:
    data = git("show", revision + ":" + name)
    if sha(data) != expected:
        raise EvaluationError("evaluation.input-hash-mismatch")
    return data


def projection(task: dict) -> dict[str, bytes]:
    """Only tracked Python scripts; no Git history, hidden tests or reference code."""
    entries = [line for line in git("ls-tree", "-r", task["base"], "--", "scripts").splitlines()
               if line.endswith(b".py")]
    if sha(b"\n".join(entries) + b"\n") != task["projection_sha256"]:
        raise EvaluationError("evaluation.projection-hash-mismatch")
    archive = git("archive", "--format=tar", task["base"], "scripts")
    if len(archive) > 32 * MAX_FILE:
        raise EvaluationError("evaluation.archive-too-large")
    result = {}
    with tarfile.open(fileobj=io.BytesIO(archive)) as stream:
        for member in stream:
            path = Path(member.name)
            if path.suffix != ".py":
                continue
            if (not member.isfile() or path.is_absolute() or ".." in path.parts
                    or path.parts[0] != "scripts" or member.size > MAX_FILE):
                raise EvaluationError("evaluation.archive-entry-refused")
            result[member.name] = stream.extractfile(member).read()
    if sum(map(len, result.values())) > MAX_TREE:
        raise EvaluationError("evaluation.projection-too-large")
    if sha(result[task["source"]]) != task["base_sha256"]:
        raise EvaluationError("evaluation.base-hash-mismatch")
    return result


def new_directory(path: Path) -> None:
    # Existing paths (including dangling aliases) are never adopted or overwritten.
    path.mkdir(mode=0o700)


def write_file(path: Path, content: bytes) -> None:
    path.parent.mkdir(parents=True, exist_ok=True, mode=0o700)
    descriptor = os.open(path, os.O_WRONLY | os.O_CREAT | os.O_EXCL | os.O_NOFOLLOW, 0o600)
    with os.fdopen(descriptor, "wb") as stream:
        stream.write(content)


def publish(path: Path, value: dict) -> None:
    write_file(path, (json.dumps(value, sort_keys=True, indent=2) + "\n").encode())


def read_candidate(candidate: Path, original: dict[str, bytes], allowed: str) -> dict[str, bytes]:
    """Snapshot only an exact projection with one editable path; refuse extra inputs."""
    result = {}
    total = 0

    def walk(directory: int, prefix: str) -> None:
        nonlocal total
        for name in os.listdir(directory):
            relative = prefix + name
            info = os.stat(name, dir_fd=directory, follow_symlinks=False)
            if stat.S_ISDIR(info.st_mode):
                if not any(p.startswith(relative + "/") for p in original):
                    raise EvaluationError("evaluation.candidate-directory-refused")
                child = os.open(name, os.O_RDONLY | os.O_DIRECTORY | os.O_NOFOLLOW, dir_fd=directory)
                try:
                    walk(child, relative + "/")
                finally:
                    os.close(child)
                continue
            if stat.S_ISLNK(info.st_mode):
                raise EvaluationError("evaluation.candidate-directory-refused")
            if relative not in original:
                raise EvaluationError("evaluation.candidate-file-refused")
            descriptor = os.open(name, os.O_RDONLY | os.O_NOFOLLOW | os.O_NONBLOCK, dir_fd=directory)
            with os.fdopen(descriptor, "rb") as stream:
                before = os.fstat(stream.fileno())
                if not stat.S_ISREG(before.st_mode) or before.st_nlink != 1 or before.st_size > MAX_FILE:
                    raise EvaluationError("evaluation.candidate-file-refused")
                data = stream.read(MAX_FILE + 1)
                after = os.fstat(stream.fileno())
            fields = ("st_dev", "st_ino", "st_size", "st_mtime_ns", "st_ctime_ns")
            if (len(data) != before.st_size or len(data) > MAX_FILE
                    or any(getattr(before, f) != getattr(after, f) for f in fields)):
                raise EvaluationError("evaluation.candidate-changed")
            if relative != allowed and data != original[relative]:
                raise EvaluationError("evaluation.out-of-scope-edit")
            result[relative] = data
            total += len(data)
            if total > MAX_TREE:
                raise EvaluationError("evaluation.candidate-too-large")

    descriptor = os.open(candidate, os.O_RDONLY | os.O_DIRECTORY | os.O_NOFOLLOW)
    try:
        walk(descriptor, "")
    finally:
        os.close(descriptor)
    if result.keys() != original.keys():
        raise EvaluationError("evaluation.candidate-file-missing")
    return result


DRIVER = b'''import importlib.util
import json
import sys
import unittest
sys.dont_write_bytecode = True
sys.path.insert(0, "/workspace")
spec = importlib.util.spec_from_file_location("hidden_eval", "/grader/test.py")
module = importlib.util.module_from_spec(spec)
spec.loader.exec_module(module)
suite = unittest.defaultTestLoader.loadTestsFromModule(module)
result = unittest.TextTestRunner(stream=sys.stderr, verbosity=2).run(suite)
report = {"tests": result.testsRun, "failures": len(result.failures),
          "errors": len(result.errors), "skipped": len(result.skipped),
          "expected_failures": len(result.expectedFailures),
          "unexpected_successes": len(result.unexpectedSuccesses)}
with open("/output/tests.json", "x") as stream:
    json.dump(report, stream, sort_keys=True)
raise SystemExit(0 if result.wasSuccessful() and result.testsRun > 0
                 and not result.skipped and not result.expectedFailures else 1)
'''


def limits() -> None:
    resource.setrlimit(resource.RLIMIT_AS, (512 * MAX_FILE, 512 * MAX_FILE))
    resource.setrlimit(resource.RLIMIT_FSIZE, (32 * MAX_FILE, 32 * MAX_FILE))
    resource.setrlimit(resource.RLIMIT_CPU, (60, 60))
    resource.setrlimit(resource.RLIMIT_CORE, (0, 0))


def sandbox_command(workspace: Path, grader: Path, output: Path) -> list[str]:
    return [
        "/usr/bin/bwrap", "--unshare-all", "--unshare-user", "--die-with-parent", "--new-session",
        "--disable-userns", "--clearenv", "--cap-drop", "ALL",
        "--ro-bind", "/usr", "/usr", "--symlink", "usr/bin", "/bin",
        "--symlink", "usr/lib", "/lib", "--symlink", "usr/lib64", "/lib64",
        "--proc", "/proc", "--dev", "/dev", "--tmpfs", "/tmp",
        "--ro-bind", str(workspace), "/workspace",
        "--ro-bind", str(grader), "/grader", "--bind", str(output), "/output",
        "--setenv", "PATH", "/usr/bin", "--setenv", "HOME", "/tmp",
        "--setenv", "LANG", "C.UTF-8", "--chdir", "/workspace",
        "/usr/bin/python3", "-I", "-B", "/grader/run.py",
    ]


def read_regular(path: Path, maximum: int) -> bytes:
    descriptor = os.open(path, os.O_RDONLY | os.O_NONBLOCK | os.O_NOFOLLOW)
    with os.fdopen(descriptor, "rb") as stream:
        before = os.fstat(stream.fileno())
        if not stat.S_ISREG(before.st_mode) or before.st_nlink != 1 or before.st_size > maximum:
            raise EvaluationError("evaluation.result-file-refused")
        data = stream.read(maximum + 1)
        after = os.fstat(stream.fileno())
    fields = ("st_dev", "st_ino", "st_size", "st_mtime_ns", "st_ctime_ns")
    if (len(data) != before.st_size or len(data) > maximum
            or any(getattr(before, f) != getattr(after, f) for f in fields)):
        raise EvaluationError("evaluation.result-file-changed")
    return data


def initialize_grading_repository(workspace: Path) -> None:
    """One candidate-only commit for Git-dependent diagnostics, without source history."""
    env = {"PATH": "/usr/bin", "LANG": "C.UTF-8", "HOME": str(workspace),
           "GIT_CONFIG_NOSYSTEM": "1", "GIT_CONFIG_GLOBAL": "/dev/null",
           "GIT_TERMINAL_PROMPT": "0", "GIT_AUTHOR_DATE": "2000-01-01T00:00:00+00:00",
           "GIT_COMMITTER_DATE": "2000-01-01T00:00:00+00:00"}
    for arguments in (
        ["init", "--quiet", "--template=", "--initial-branch=evaluation-candidate"],
        ["add", "--", "scripts"],
        ["-c", "user.name=Synthetic", "-c", "user.email=synthetic@example.invalid",
         "-c", "core.hooksPath=/dev/null", "commit", "--quiet", "-m", "Candidate snapshot"],
    ):
        subprocess.run(["/usr/bin/git", *arguments], cwd=workspace, env=env, check=True,
                       stdin=subprocess.DEVNULL, stdout=subprocess.DEVNULL,
                       stderr=subprocess.DEVNULL, timeout=30)


def grade(task: dict, files: dict[str, bytes], output: Path) -> dict:
    new_directory(output)
    hidden = pinned_blob(task["reference"], task["hidden_test"], task["hidden_sha256"])
    started = time.monotonic()
    with tempfile.TemporaryDirectory(prefix="agentmage-eval-") as temporary:
        root = Path(temporary)
        workspace, grader, result_dir = root / "workspace", root / "grader", root / "results"
        workspace.mkdir(mode=0o700)
        grader.mkdir(mode=0o700)
        result_dir.mkdir(mode=0o700)
        for name, content in files.items():
            write_file(workspace / name, content)
        initialize_grading_repository(workspace)
        write_file(grader / "test.py", hidden)
        write_file(grader / "run.py", DRIVER)
        command = sandbox_command(workspace, grader, result_dir)
        timed_out = False
        output_limited = False
        process = None
        try:
            with (output / "stdout.log").open("xb") as stdout, (output / "stderr.log").open("xb") as stderr:
                process = subprocess.Popen(command, stdin=subprocess.DEVNULL, stdout=subprocess.PIPE,
                                           stderr=subprocess.PIPE, start_new_session=True, preexec_fn=limits)
                deadline = time.monotonic() + TIMEOUT
                exit_code = None
                totals = {stdout: 0, stderr: 0}
                with selectors.DefaultSelector() as events:
                    events.register(process.stdout, selectors.EVENT_READ, stdout)
                    events.register(process.stderr, selectors.EVENT_READ, stderr)
                    while True:
                        for key, _ in events.select(timeout=0.01):
                            data = os.read(key.fileobj.fileno(), 65536)
                            if not data:
                                events.unregister(key.fileobj)
                                key.fileobj.close()
                                continue
                            remaining = 2 * MAX_FILE - totals[key.data]
                            key.data.write(data[:remaining])
                            totals[key.data] += min(len(data), remaining)
                            if len(data) > remaining:
                                output_limited = True
                        status = os.waitid(os.P_PID, process.pid, os.WEXITED | os.WNOHANG | os.WNOWAIT)
                        if status is not None and not events.get_map():
                            exit_code = status.si_status if status.si_code == os.CLD_EXITED else -status.si_status
                            break
                        if output_limited:
                            break
                        if time.monotonic() >= deadline:
                            timed_out = True
                            break
        finally:
            if process is not None:
                # Bubblewrap's private PID namespace also kills escaped descendants.
                try:
                    os.killpg(process.pid, signal.SIGKILL)
                except ProcessLookupError:
                    pass
                process.wait()
                for pipe in (process.stdout, process.stderr):
                    if pipe is not None:
                        pipe.close()
        tests = None
        try:
            data = read_regular(result_dir / "tests.json", 4096)
            tests = json.loads(data)
            fields = {"tests", "failures", "errors", "skipped", "expected_failures", "unexpected_successes"}
            if (not isinstance(tests, dict) or tests.keys() != fields
                    or any(type(v) is not int or v < 0 for v in tests.values())
                    or tests["tests"] != task["expected_tests"]):
                raise ValueError
            write_file(output / "tests.json", data)
        except (OSError, ValueError):
            tests = None
    passed = (exit_code == 0 and not timed_out and not output_limited and tests is not None
              and all(tests[k] == 0 for k in tests if k != "tests"))
    cause = ("none" if passed else "output_limit" if output_limited else "timeout" if timed_out else "infrastructure"
             if tests is None else "test_failure")
    report = {"version": VERSION, "task": task["id"], "split": task["split"],
              "base": task["base"], "hidden_sha256": task["hidden_sha256"],
              "evaluator_sha256": sha(Path(__file__).read_bytes()),
              "task_sha256": sha(json.dumps(task, sort_keys=True, separators=(",", ":")).encode()),
              "candidate_sha256": sha(files[task["source"]]), "passed": passed,
              "cause": cause, "exit_code": exit_code, "tests": tests, "output_truncated": output_limited,
              "wall_seconds": round(time.monotonic() - started, 6),
              "gpu_minutes": 0, "tokens": None, "model_attempts": 0,
              "logs": {name: sha(read_regular(output / name, 2 * MAX_FILE)) for name in ("stdout.log", "stderr.log")}}
    publish(output / "report.json", report)
    return report


def verify(output: Path) -> dict:
    """Validate evaluator discrimination, never count reference patches as agent attempts."""
    new_directory(output)
    rows = []
    for task in TASKS:
        original = projection(task)
        base = grade(task, original, output / (task["id"] + "-base"))
        fixed = dict(original)
        fixed[task["source"]] = pinned_blob(task["reference"], task["source"], task["reference_sha256"])
        reference = grade(task, fixed, output / (task["id"] + "-reference"))
        rows.append({"task": task["id"], "base": base, "reference": reference,
                     "valid": not base["passed"] and base["cause"] == "test_failure" and reference["passed"]})
    summary = {"version": VERSION, "kind": "evaluator-validation-only", "tasks": rows,
               "passed": all(row["valid"] for row in rows), "model_score": None}
    publish(output / "summary.json", summary)
    return summary


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("operation", choices=("list", "prepare", "score", "verify"))
    parser.add_argument("--task", choices=[t["id"] for t in TASKS])
    parser.add_argument("--candidate", type=Path)
    parser.add_argument("--output", type=Path)
    args = parser.parse_args()
    if args.operation == "list":
        print(json.dumps({"version": VERSION, "tasks": [
            {k: task[k] for k in ("id", "split", "base", "source", "statement")} for task in TASKS
        ]}, indent=2))
        return 0
    if args.output is None:
        parser.error("--output is required")
    if args.operation == "verify":
        return 0 if verify(args.output)["passed"] else 1
    if args.task is None:
        parser.error("--task is required")
    task = next(t for t in TASKS if t["id"] == args.task)
    files = projection(task)
    if args.operation == "prepare":
        new_directory(args.output)
        for name, content in files.items():
            write_file(args.output / name, content)
        return 0
    if args.candidate is None:
        parser.error("--candidate is required")
    files = read_candidate(args.candidate, files, task["source"])
    return 0 if grade(task, files, args.output)["passed"] else 1


def interrupted(signum: int, frame: object) -> None:
    raise KeyboardInterrupt


if __name__ == "__main__":
    for termination_signal in (signal.SIGTERM, signal.SIGHUP):
        signal.signal(termination_signal, interrupted)
    try:
        raise SystemExit(main())
    except (EvaluationError, OSError, subprocess.SubprocessError):
        print("evaluation.refused; inspect private output", file=sys.stderr)
        raise SystemExit(2)
