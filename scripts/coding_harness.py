"""Operate the explicit AgentMage disposable coding-development harness.

This wrapper creates only synthetic private repositories. Candidate selection is evaluation-only;
it does not activate a production model, relax AgentMage policy, or claim qualification.
"""
from __future__ import annotations

import argparse
import fcntl
import hashlib
import json
import os
from pathlib import Path
import signal
import stat
import subprocess
import sys
import time


ROOT = Path(__file__).resolve().parents[1]
PROFILE = "scripted-executable-fixture-32k-v1"
BRANCH = "agentmage/tasks/coding-fixture"
MARKER = ".agentmage-development-workspace"
RUN_RECORD = "coding-harness-run.json"
MAX_RECORD_BYTES = 16 * 1024
MAX_LINUX_SOCKET_PATH_BYTES = 107
MODEL_LAB_PROFILE = ROOT / "model-profiles/development/coding-model-lab.json"
MODEL_LAB_LOCK = Path.home() / ".local/state/agentmage-model-lab/gpu.lock"


class HarnessError(RuntimeError):
    """One content-minimized operator error."""


def file_identity(path: Path) -> dict:
    """Measure a stable regular file; this diagnostic does not admit an executable."""
    with path.open("rb") as stream:
        before = os.fstat(stream.fileno())
        if not stat.S_ISREG(before.st_mode):
            raise HarnessError("coding.harness.identity-not-regular")
        digest = hashlib.file_digest(stream, "sha256").hexdigest()
        after = os.fstat(stream.fileno())
    fields = ("st_dev", "st_ino", "st_size", "st_mtime_ns", "st_ctime_ns")
    if any(getattr(before, field) != getattr(after, field) for field in fields):
        raise HarnessError("coding.harness.identity-changed")
    return {"path": str(path), "bytes": after.st_size, "sha256": digest}


def implementation_identity() -> dict:
    def git(*arguments: str) -> bytes:
        return subprocess.run(
            ["git", "--no-optional-locks", *arguments], cwd=ROOT,
            check=True, capture_output=True, timeout=30,
        ).stdout

    return {
        "head_commit": git("rev-parse", "HEAD").decode("ascii").strip(),
        "worktree_status": git("status", "--porcelain=v1", "--untracked-files=all").decode("utf-8"),
        "tracked_diff_sha256": hashlib.sha256(git("diff", "HEAD", "--binary", "--no-ext-diff")).hexdigest(),
        "binaries": [file_identity(binary(name)) for name in (
            "agentmage", "agentmage-host", "agentmage-read-only-worker",
        )],
        "wrapper": file_identity(Path(__file__).resolve()),
    }


def scope_resources() -> dict[str, str]:
    group = Path("/proc/self/cgroup").read_text(encoding="utf-8").strip().split("::", 1)[1]
    root = Path("/sys/fs/cgroup") / group.lstrip("/")
    return {
        name: (root / name).read_text(encoding="utf-8").strip()
        for name in ("memory.high", "memory.max", "memory.swap.max", "memory.peak", "cpu.max")
    }


def gpu_memory() -> tuple[int, int]:
    result = subprocess.run(
        [
            "/usr/bin/nvidia-smi",
            "--query-gpu=memory.used,memory.free",
            "--format=csv,noheader,nounits",
        ],
        check=True,
        capture_output=True,
        text=True,
        timeout=10,
    )
    lines = result.stdout.strip().splitlines()
    if len(lines) != 1:
        raise HarnessError("coding.harness.candidate.single-gpu-required")
    try:
        used, free = (int(value.strip()) for value in lines[0].split(","))
    except (TypeError, ValueError) as error:
        raise HarnessError("coding.harness.candidate.gpu-observation-invalid") from error
    return used, free


class CandidateResourceGuard:
    """Cooperatively guards one exact candidate run without touching other processes."""

    def __init__(self, descriptor: int, profile: dict, resources: dict[str, str], used: int, free: int):
        self.descriptor = descriptor
        self.profile = profile
        self.resources = resources
        self.baseline_used_mib = used
        self.baseline_free_mib = free
        self.peak_total_gpu_used_mib_sampled = used
        self.error: str | None = None

    @classmethod
    def acquire(cls) -> "CandidateResourceGuard":
        profile = json.loads(MODEL_LAB_PROFILE.read_text(encoding="utf-8"))
        if profile.get("product_enabled") is not False or profile.get("parallel_slots") != 1:
            raise HarnessError("coding.harness.candidate.profile-denied")
        resources = scope_resources()
        for name, maximum in (
            ("memory.high", 5 * 1024**3),
            ("memory.max", 6 * 1024**3),
            ("memory.swap.max", 512 * 1024**2),
        ):
            if resources[name] == "max" or int(resources[name]) > maximum:
                raise HarnessError("coding.harness.candidate.scope-denied")
        quota, period = resources["cpu.max"].split()
        if quota == "max" or int(quota) > 2 * int(period):
            raise HarnessError("coding.harness.candidate.scope-denied")
        memory = {
            name: value
            for name, value in (
                line.split(":", 1) for line in Path("/proc/meminfo").read_text().splitlines()
            )
        }
        if int(memory["MemAvailable"].split()[0]) < profile["minimum_available_ram_gib"] * 1024**2:
            raise HarnessError("coding.harness.candidate.ram-contended")
        private_directory(MODEL_LAB_LOCK.parent)
        descriptor = os.open(MODEL_LAB_LOCK, os.O_RDWR | os.O_CREAT | os.O_NOFOLLOW, 0o600)
        try:
            info = os.fstat(descriptor)
            if not stat.S_ISREG(info.st_mode) or info.st_uid != os.getuid() or info.st_mode & 0o077:
                raise HarnessError("coding.harness.candidate.gpu-lock-denied")
            fcntl.flock(descriptor, fcntl.LOCK_EX | fcntl.LOCK_NB)
            used, free = gpu_memory()
            if free < profile["minimum_free_vram_mib"]:
                raise HarnessError("coding.harness.candidate.gpu-contended")
            return cls(descriptor, profile, resources, used, free)
        except BaseException:
            os.close(descriptor)
            raise

    def sample(self) -> bool:
        try:
            used, _ = gpu_memory()
            self.peak_total_gpu_used_mib_sampled = max(self.peak_total_gpu_used_mib_sampled, used)
            if used > self.profile["maximum_total_vram_used_mib"]:
                self.error = "coding.harness.candidate.gpu-headroom-exceeded"
        except (HarnessError, OSError, subprocess.SubprocessError):
            self.error = "coding.harness.candidate.gpu-observation-failed"
        return self.error is None

    def report(self) -> dict:
        return {
            "baseline_gpu_used_mib": self.baseline_used_mib,
            "baseline_gpu_free_mib": self.baseline_free_mib,
            "peak_total_gpu_used_mib_sampled": self.peak_total_gpu_used_mib_sampled,
            "resource_guard_error": self.error,
            "scope_resources": scope_resources(),
        }

    def close(self) -> None:
        fcntl.flock(self.descriptor, fcntl.LOCK_UN)
        os.close(self.descriptor)


def private_directory(path: Path) -> Path:
    path.mkdir(mode=0o700, parents=True, exist_ok=True)
    path.chmod(0o700)
    info = path.lstat()
    if not stat.S_ISDIR(info.st_mode) or info.st_uid != os.getuid() or info.st_mode & 0o077:
        raise HarnessError("coding.harness.private-directory-denied")
    return path


def private_file(path: Path, content: str) -> None:
    descriptor = os.open(path, os.O_WRONLY | os.O_CREAT | os.O_EXCL | os.O_NOFOLLOW, 0o600)
    with os.fdopen(descriptor, "w", encoding="utf-8") as target:
        target.write(content)
        target.flush()
        os.fsync(target.fileno())


def run_git(workspace: Path, *arguments: str, capture: bool = False) -> subprocess.CompletedProcess:
    return subprocess.run(
        ["/usr/bin/git", "-C", str(workspace), *arguments],
        check=True,
        stdin=subprocess.DEVNULL,
        text=True,
        capture_output=capture,
        timeout=30,
        env={"PATH": "/usr/bin", "LANG": "C", "LC_ALL": "C"},
    )


def paths(base: Path) -> tuple[Path, Path, Path]:
    exact = base.resolve(strict=False)
    if not exact.is_absolute() or exact == Path("/") or exact == Path.home():
        raise HarnessError("coding.harness.root-denied")
    return exact / "state", exact / "disposable", exact / "disposable/worktree"


def setup(base: Path, fixture: str = "repair") -> dict:
    base = base.resolve(strict=False)
    if base.exists() or base.is_symlink():
        raise HarnessError("coding.harness.setup-target-exists")
    base.mkdir(mode=0o700)
    base.chmod(0o700)
    state, disposable, workspace = paths(base)
    private_directory(state)
    private_directory(disposable)
    private_directory(workspace)
    private_directory(workspace / "src")
    private_directory(workspace / "tests")
    run_git(workspace, "init", "--initial-branch", BRANCH)
    run_git(workspace, "config", "user.name", "AgentMage Synthetic Fixture")
    run_git(workspace, "config", "user.email", "fixture.invalid@agentmage.local")
    if fixture not in ("repair", "new-file", "multi-file", "stable"):
        raise HarnessError("coding.harness.fixture-denied")
    tracked = ["tests/run_validation.py", MARKER]
    if fixture != "new-file":
        source = (
            "def add(left, right):\n    return left + right\n"
            if fixture == "stable"
            else "def broken_add(left, right):\n    return left + right\n"
        )
        private_file(workspace / "src/calc.py", source)
        tracked.append("src/calc.py")
    if fixture == "multi-file":
        private_file(
            workspace / "src/subtract.py",
            "def broken_subtract(left, right):\n    return left - right\n",
        )
        tracked.append("src/subtract.py")
    validation = """import json
import sys
from pathlib import Path

sys.dont_write_bytecode = True
sys.path.insert(0, str(Path(__file__).resolve().parents[1]))

try:
    from src.calc import add
except ImportError:
    result = {"schema_version": 1, "status": "assertion_failed", "passed": 0,
              "failed": 1, "skipped": 0, "duration_ms": 1,
              "failed_names": ["fixture::add"], "artifact_ids": [],
              "retry_count": 0, "initial_failure_sha256": None}
    print(json.dumps(result, sort_keys=True))
    raise SystemExit(1)

passed = add(2, 3) == 5
result = {"schema_version": 1, "status": "passed" if passed else "assertion_failed",
          "passed": 1 if passed else 0, "failed": 0 if passed else 1,
          "skipped": 0, "duration_ms": 1,
          "failed_names": [] if passed else ["fixture::add"], "artifact_ids": [],
          "retry_count": 0, "initial_failure_sha256": None}
print(json.dumps(result, sort_keys=True))
raise SystemExit(0 if passed else 1)
"""
    if fixture == "multi-file":
        validation = """import json
import sys
from pathlib import Path

sys.dont_write_bytecode = True
sys.path.insert(0, str(Path(__file__).resolve().parents[1]))

failed = []
try:
    from src.calc import add
except ImportError:
    add = None
    failed.append("fixture::add")
try:
    from src.subtract import subtract
except ImportError:
    subtract = None
    failed.append("fixture::subtract")
if add is not None and add(2, 3) != 5:
    failed.append("fixture::add")
if subtract is not None and subtract(7, 2) != 5:
    failed.append("fixture::subtract")
passed = 2 - len(failed)
result = {"schema_version": 1, "status": "passed" if not failed else "assertion_failed",
          "passed": passed, "failed": len(failed), "skipped": 0, "duration_ms": 1,
          "failed_names": failed, "artifact_ids": [], "retry_count": 0,
          "initial_failure_sha256": None}
print(json.dumps(result, sort_keys=True))
raise SystemExit(0 if not failed else 1)
"""
    private_file(
        workspace / "tests/run_validation.py",
        validation,
    )
    marker = f"activation=coding-development-v1\nworkspace={workspace}\n"
    private_file(workspace / MARKER, marker)
    run_git(workspace, "add", "--", *tracked)
    run_git(workspace, "commit", "-m", "test: initialize synthetic coding fixture")
    status = diagnose(base)
    print(json.dumps(status, sort_keys=True))
    return status


def binary(name: str) -> Path:
    candidate = ROOT / "target/debug" / name
    if not candidate.is_file() or not os.access(candidate, os.X_OK):
        raise HarnessError(f"coding.harness.binary-unavailable:{name}")
    return candidate.resolve()


def expected_marker(workspace: Path) -> str:
    return f"activation=coding-development-v1\nworkspace={workspace}\n"


def read_run_record(state: Path) -> dict | None:
    record = state / RUN_RECORD
    try:
        info = record.lstat()
    except FileNotFoundError:
        return None
    if not stat.S_ISREG(info.st_mode) or info.st_uid != os.getuid() or info.st_mode & 0o077:
        raise HarnessError("coding.harness.run-record-denied")
    if info.st_size <= 0 or info.st_size > MAX_RECORD_BYTES:
        raise HarnessError("coding.harness.run-record-denied")
    value = json.loads(record.read_text(encoding="utf-8"))
    if set(value) != {"pid", "base", "binary", "started_at_epoch_ms"}:
        raise HarnessError("coding.harness.run-record-denied")
    return value


def exact_running_process(record: dict, base: Path) -> bool:
    pid = record.get("pid")
    if not isinstance(pid, int) or pid <= 1 or record.get("base") != str(base):
        return False
    process = Path("/proc") / str(pid)
    try:
        executable = (process / "exe").resolve(strict=True)
        arguments = (process / "cmdline").read_bytes().split(b"\0")
    except (FileNotFoundError, PermissionError, ProcessLookupError):
        return False
    expected = Path(record.get("binary", ""))
    encoded_base = os.fsencode(base)
    owns_arguments = any(
        argument == encoded_base or argument.startswith(encoded_base + b"/")
        for argument in arguments
    )
    return executable == expected and owns_arguments and b"--development" in arguments


def diagnose(base: Path) -> dict:
    base = base.resolve(strict=True)
    state, disposable, workspace = paths(base)
    checks: dict[str, object] = {
        "schema_version": 1,
        "profile_id": PROFILE,
        "qualification": "executable-scripted-only",
        "base": str(base),
    }
    for label, directory in (("state", state), ("disposable", disposable), ("workspace", workspace)):
        info = directory.lstat()
        checks[f"{label}_private"] = stat.S_ISDIR(info.st_mode) and not info.st_mode & 0o077
    marker = workspace / MARKER
    marker_info = marker.lstat()
    checks["activation"] = (
        stat.S_ISREG(marker_info.st_mode)
        and marker_info.st_nlink == 1
        and not marker_info.st_mode & 0o077
        and marker.read_text(encoding="utf-8") == expected_marker(workspace)
    )
    checks["git_branch"] = run_git(
        workspace, "symbolic-ref", "--quiet", "HEAD", capture=True
    ).stdout.strip()
    checks["git_clean"] = not run_git(
        workspace, "status", "--porcelain=v1", "--untracked-files=all", capture=True
    ).stdout
    checks["confinement"] = all(Path(path).is_file() for path in (
        "/usr/bin/bwrap", "/usr/bin/systemd-run", "/usr/bin/systemctl", "/usr/bin/git"
    ))
    # The host suffix includes a 10-digit PID, separators, a 20-digit start time, and `.sock`.
    checks["transport_path"] = (
        len(os.fsencode(state)) + 1 + len("coding-development-4294967295-18446744073709551615.sock")
        <= MAX_LINUX_SOCKET_PATH_BYTES
    )
    checks["agentmage_binary"] = str(binary("agentmage"))
    checks["host_binary"] = str(binary("agentmage-host"))
    checks["read_worker_binary"] = str(binary("agentmage-read-only-worker"))
    record = read_run_record(state)
    state_key_present = (state / "operational-store-development-v1.key").exists()
    process_running = bool(record and exact_running_process(record, base))
    checks["lifecycle"] = (
        "running" if process_running and state_key_present
        else "starting" if process_running
        else "ready"
    )
    checks["state_key"] = "present" if state_key_present else "not-created"
    return checks


def status(base: Path) -> dict:
    result = diagnose(base)
    print(json.dumps(result, sort_keys=True))
    return result


def start(
    base: Path,
    scenario: str,
    objective: str,
    approve: bool,
    stale_approval_probe: bool,
    log_dir: Path | None,
    replay_approval_probe: bool = False,
    expired_cursor_probe: bool = False,
    approval_delay_ms: int = 0,
    model: str = "scripted",
    slow_subscriber_probe: bool = False,
    artifact_integrity_probe: bool = False,
    preauthorized_paths: tuple[str, ...] = (),
    preauthorized_commands: tuple[str, ...] = (),
    preauthorize_workspace_reads: bool = False,
    preauthorization_budget: int | None = None,
    preauthorization_minutes: int | None = None,
    revoke_preauthorization_before_follow_ups: bool = False,
    record_session: bool = False,
    artifact_release_probe_before_follow_ups: bool = False,
    follow_ups: tuple[str, ...] = (),
) -> int:
    base = base.resolve(strict=True)
    state, disposable, workspace = paths(base)
    current = diagnose(base)
    if (
        current["lifecycle"] != "ready"
        or not current["git_clean"]
        or not current["transport_path"]
    ):
        raise HarnessError("coding.harness.start-state-denied")
    executable = binary("agentmage")
    command = [
        str(executable), "--json", "code", "--development",
        "--state-root", str(state), "--disposable-root", str(disposable),
        "--workspace-root", str(workspace), "--scenario", scenario,
        "--model", model,
        "--objective", objective,
    ]
    if approve:
        command.append("--approve-this-run")
    if stale_approval_probe:
        command.append("--stale-approval-probe")
    if replay_approval_probe:
        command.append("--replay-approval-probe")
    if expired_cursor_probe:
        command.append("--expired-cursor-probe")
    if approval_delay_ms:
        command.extend(("--approval-delay-ms", str(approval_delay_ms)))
    if slow_subscriber_probe:
        command.append("--slow-subscriber-probe")
    if artifact_integrity_probe:
        command.append("--artifact-integrity-probe")
    if record_session:
        command.append("--record-session")
    if artifact_release_probe_before_follow_ups:
        command.append("--artifact-release-probe-before-follow-ups")
    for follow_up in follow_ups:
        command.extend(("--follow-up", follow_up))
    for path in preauthorized_paths:
        command.extend(("--preauthorize-path", path))
    for template in preauthorized_commands:
        command.extend(("--preauthorize-command", template))
    if preauthorize_workspace_reads:
        command.append("--preauthorize-workspace-reads")
    if preauthorization_budget is not None:
        command.extend(("--preauthorization-budget", str(preauthorization_budget)))
    if preauthorization_minutes is not None:
        command.extend(("--preauthorization-minutes", str(preauthorization_minutes)))
    if revoke_preauthorization_before_follow_ups:
        command.append("--revoke-preauthorization-before-follow-ups")
    resource_guard = None
    stdout_target = None
    stderr_target = None
    opened = []
    process = None
    record_path = state / RUN_RECORD
    started_at_epoch_ms = time.time_ns() // 1_000_000
    started_monotonic = time.monotonic()
    implementation = implementation_identity() if log_dir is not None else None
    try:
        if log_dir is not None:
            log_dir = log_dir.resolve(strict=False)
            if log_dir.exists() or log_dir.is_symlink():
                raise HarnessError("coding.harness.log-target-exists")
            private_directory(log_dir)
            stdout_target = os.fdopen(
                os.open(
                    log_dir / "stdout.jsonl",
                    os.O_WRONLY | os.O_CREAT | os.O_EXCL | os.O_NOFOLLOW,
                    0o600,
                ),
                "wb",
            )
            opened.append(stdout_target)
            stderr_target = os.fdopen(
                os.open(
                    log_dir / "stderr.log",
                    os.O_WRONLY | os.O_CREAT | os.O_EXCL | os.O_NOFOLLOW,
                    0o600,
                ),
                "wb",
            )
            opened.append(stderr_target)
        resource_guard = CandidateResourceGuard.acquire() if model != "scripted" else None
        process = subprocess.Popen(command, stdin=None, stdout=stdout_target, stderr=stderr_target)
        private_file(
            record_path,
            json.dumps(
                {
                    "pid": process.pid,
                    "base": str(base),
                    "binary": str(executable),
                    "started_at_epoch_ms": time.time_ns() // 1_000_000,
                },
                sort_keys=True,
            )
            + "\n",
        )
        if resource_guard is None:
            exit_code = process.wait()
        else:
            while process.poll() is None:
                time.sleep(1)
                if resource_guard.sample():
                    continue
                process.send_signal(signal.SIGINT)
                try:
                    exit_code = process.wait(timeout=30)
                except subprocess.TimeoutExpired:
                    process.terminate()
                    try:
                        exit_code = process.wait(timeout=10)
                    except subprocess.TimeoutExpired:
                        process.kill()
                        exit_code = process.wait(timeout=10)
                break
            else:
                exit_code = process.returncode
        if log_dir is not None:
            final_binaries = [file_identity(binary(name)) for name in (
                "agentmage", "agentmage-host", "agentmage-read-only-worker",
            )]
            result = {
                "schema_version": 1, "exit_code": exit_code, "scenario": scenario,
                "started_at_epoch_ms": started_at_epoch_ms,
                "finished_at_epoch_ms": time.time_ns() // 1_000_000,
                "elapsed_seconds": time.monotonic() - started_monotonic,
                "command": command,
                "implementation": implementation,
                "final_binaries": final_binaries,
                "binary_identity_unchanged": implementation["binaries"] == final_binaries,
                "model": model,
                "objective": objective, "approved_for_this_run": approve,
                "stale_approval_probe": stale_approval_probe,
                "replay_approval_probe": replay_approval_probe,
                "expired_cursor_probe": expired_cursor_probe,
                "approval_delay_ms": approval_delay_ms,
                "artifact_integrity_probe": artifact_integrity_probe,
                "record_session": record_session,
                "artifact_release_probe_before_follow_ups": artifact_release_probe_before_follow_ups,
                "follow_up_count": len(follow_ups),
                "session_preauthorization_requested": bool(
                    preauthorized_paths
                    or preauthorized_commands
                    or preauthorize_workspace_reads
                ),
            }
            if resource_guard is not None:
                result["candidate_resources"] = resource_guard.report()
            private_file(log_dir / "result.json", json.dumps(result, sort_keys=True) + "\n")
            print(json.dumps({"exit_code": exit_code, "log_dir": str(log_dir)}, sort_keys=True))
        return exit_code
    finally:
        if process is not None and process.poll() is None:
            process.send_signal(signal.SIGINT)
            try:
                process.wait(timeout=30)
            except subprocess.TimeoutExpired:
                process.terminate()
                try:
                    process.wait(timeout=10)
                except subprocess.TimeoutExpired:
                    process.kill()
                    process.wait(timeout=10)
        for target in opened:
            target.close()
        try:
            record_path.unlink()
        except FileNotFoundError:
            pass
        if resource_guard is not None:
            resource_guard.close()


def stop(base: Path) -> None:
    base = base.resolve(strict=True)
    state, _, _ = paths(base)
    record = read_run_record(state)
    if (
        record is None
        or not exact_running_process(record, base)
        or not (state / "operational-store-development-v1.key").is_file()
    ):
        raise HarnessError("coding.harness.not-running")
    os.kill(record["pid"], signal.SIGINT)
    print("coding.harness.cancellation-requested")


def approval_delay(value: str) -> int:
    try:
        delay = int(value)
    except ValueError as error:
        raise argparse.ArgumentTypeError("approval delay must be an integer") from error
    if not 1 <= delay <= 10_000:
        raise argparse.ArgumentTypeError("approval delay must be between 1 and 10000 milliseconds")
    return delay


def parser() -> argparse.ArgumentParser:
    result = argparse.ArgumentParser(description=__doc__)
    commands = result.add_subparsers(dest="command", required=True)
    for name in ("setup", "status", "diagnose", "stop"):
        command = commands.add_parser(name)
        command.add_argument("--root", type=Path, required=True)
        if name == "setup":
            command.add_argument(
                "--fixture", choices=("repair", "new-file", "multi-file", "stable"), default="repair"
            )
    start_command = commands.add_parser("start")
    start_command.add_argument("--root", type=Path, required=True)
    start_command.add_argument(
        "--scenario",
        choices=(
            "no-op", "failed-test-repair", "slow-cancel", "new-file", "multi-file", "rollback",
            "false-completion", "overflow", "disk-pressure", "output-pressure",
            "protocol-correction", "arguments-correction", "repeated-protocol-rejection",
            "native-command-repair",
        ),
        required=True,
    )
    start_command.add_argument("--objective", required=True)
    start_command.add_argument("--follow-up", action="append", default=[])
    start_command.add_argument("--model", choices=("scripted", "muse", "gpt-oss"), default="scripted")
    start_command.add_argument("--approve-this-run", action="store_true")
    start_command.add_argument("--stale-approval-probe", action="store_true")
    start_command.add_argument("--replay-approval-probe", action="store_true")
    start_command.add_argument("--expired-cursor-probe", action="store_true")
    start_command.add_argument("--approval-delay-ms", type=approval_delay, default=0)
    start_command.add_argument("--slow-subscriber-probe", action="store_true")
    start_command.add_argument("--artifact-integrity-probe", action="store_true")
    start_command.add_argument("--record-session", action="store_true")
    start_command.add_argument("--artifact-release-probe-before-follow-ups", action="store_true")
    start_command.add_argument("--preauthorize-path", action="append", default=[])
    start_command.add_argument("--preauthorize-command", action="append", default=[])
    start_command.add_argument("--preauthorize-workspace-reads", action="store_true")
    start_command.add_argument("--preauthorization-budget", type=int)
    start_command.add_argument("--preauthorization-minutes", type=int)
    start_command.add_argument(
        "--revoke-preauthorization-before-follow-ups", action="store_true"
    )
    start_command.add_argument("--log-dir", type=Path)
    return result


def main() -> int:
    arguments = parser().parse_args()
    try:
        if arguments.command == "setup":
            setup(arguments.root, arguments.fixture)
            return 0
        if arguments.command in ("status", "diagnose"):
            status(arguments.root)
            return 0
        if arguments.command == "stop":
            stop(arguments.root)
            return 0
        return start(
            arguments.root, arguments.scenario, arguments.objective,
            arguments.approve_this_run, arguments.stale_approval_probe, arguments.log_dir,
            arguments.replay_approval_probe, arguments.expired_cursor_probe,
            arguments.approval_delay_ms, arguments.model, arguments.slow_subscriber_probe,
            arguments.artifact_integrity_probe,
            tuple(arguments.preauthorize_path), tuple(arguments.preauthorize_command),
            arguments.preauthorize_workspace_reads, arguments.preauthorization_budget,
            arguments.preauthorization_minutes,
            arguments.revoke_preauthorization_before_follow_ups,
            arguments.record_session,
            arguments.artifact_release_probe_before_follow_ups,
            tuple(arguments.follow_up),
        )
    except (HarnessError, OSError, subprocess.SubprocessError, json.JSONDecodeError) as error:
        print(str(error), file=sys.stderr)
        return 1


if __name__ == "__main__":
    raise SystemExit(main())
