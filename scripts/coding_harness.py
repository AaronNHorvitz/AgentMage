"""Operate the explicit AgentMage disposable coding-development harness.

This wrapper creates only synthetic private repositories. Candidate selection is evaluation-only;
it does not activate a production model, relax AgentMage policy, or claim qualification.
"""
from __future__ import annotations

import argparse
from contextlib import ExitStack
import fcntl
import hashlib
import json
import os
from pathlib import Path
import re
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
RUN_RECORD_VERSION = 2
DIAGNOSIS_VERSION = 2
MODEL_CHOICES = ("scripted", "muse", "gpt-oss")
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

    untracked = untracked_file_identities(ROOT)
    return {
        "head_commit": git("rev-parse", "HEAD").decode("ascii").strip(),
        "worktree_status": git("status", "--porcelain=v1", "--untracked-files=all").decode("utf-8"),
        "tracked_diff_sha256": hashlib.sha256(git("diff", "HEAD", "--binary", "--no-ext-diff")).hexdigest(),
        # Git's tracked diff omits new Rust modules. Retain their exact bytes'
        # identities too; a filename-only dirty status cannot bind new source.
        "untracked_files": untracked,
        "untracked_files_sha256": hashlib.sha256(json.dumps(
            untracked, sort_keys=True, separators=(",", ":"), ensure_ascii=True,
        ).encode("ascii")).hexdigest(),
        "binaries": [file_identity(binary(name)) for name in (
            "agentmage", "agentmage-host", "agentmage-read-only-worker",
        )],
        "wrapper": file_identity(Path(__file__).resolve()),
    }


def untracked_file_identities(root: Path) -> list[dict]:
    """Bound dirty diagnostic inputs without following aliases or reading content into logs.

    This supplements, never replaces, HEAD, the complete tracked diff and binary hashes.
    Ignored build/model/private outputs are outside Git's implementation inventory.
    It is not an atomic source snapshot or a substitute for a pinned clean campaign.
    """
    paths = subprocess.run(
        ["git", "--no-optional-locks", "ls-files", "--others", "--exclude-standard", "-z"],
        cwd=root, check=True, capture_output=True, timeout=30,
    ).stdout.split(b"\0")
    names = sorted(os.fsdecode(path) for path in paths if path)
    if len(names) > 4096:
        raise HarnessError("coding.harness.source-inventory-limit")
    identities = []
    total = 0
    for name in names:
        relative = Path(name)
        if relative.is_absolute() or any(part in {".", ".."} for part in relative.parts):
            raise HarnessError("coding.harness.source-inventory-path")
        path = root / relative
        if any(root.joinpath(*relative.parts[:end]).is_symlink()
               for end in range(1, len(relative.parts) + 1)):
            raise HarnessError("coding.harness.source-inventory-alias")
        descriptor = os.open(path, os.O_RDONLY | os.O_NOFOLLOW | os.O_NONBLOCK)
        with os.fdopen(descriptor, "rb") as stream:
            before = os.fstat(stream.fileno())
            if not stat.S_ISREG(before.st_mode):
                raise HarnessError("coding.harness.identity-not-regular")
            remaining = min(16 * 1024 * 1024, 64 * 1024 * 1024 - total)
            if before.st_size > remaining:
                raise HarnessError("coding.harness.source-inventory-limit")
            content = stream.read(remaining + 1)
            after = os.fstat(stream.fileno())
        if len(content) > remaining:
            raise HarnessError("coding.harness.source-inventory-limit")
        fields = ("st_dev", "st_ino", "st_size", "st_mtime_ns", "st_ctime_ns")
        if len(content) != before.st_size or any(
            getattr(before, field) != getattr(after, field) for field in fields
        ):
            raise HarnessError("coding.harness.identity-changed")
        total += len(content)
        identities.append({"path": name, "bytes": len(content),
                           "sha256": hashlib.sha256(content).hexdigest()})
    return identities


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


def run_record_bytes(descriptor: int) -> bytes:
    before = os.fstat(descriptor)
    if (not stat.S_ISREG(before.st_mode) or before.st_uid != os.getuid()
            or before.st_mode & 0o077 or before.st_nlink != 1
            or not 0 <= before.st_size <= MAX_RECORD_BYTES):
        raise HarnessError("coding.harness.run-record-denied")
    content = os.pread(descriptor, MAX_RECORD_BYTES + 1, 0)
    after = os.fstat(descriptor)
    fields = ("st_dev", "st_ino", "st_size", "st_mtime_ns", "st_ctime_ns", "st_mode", "st_nlink", "st_uid")
    if len(content) != before.st_size or any(
        getattr(before, field) != getattr(after, field) for field in fields
    ):
        raise HarnessError("coding.harness.run-record-changed")
    return content


def valid_run_record(value: object) -> bool:
    if not isinstance(value, dict) or set(value) != {
        "schema_version", "pid", "base", "binary", "started_at_epoch_ms", "process_identity",
    }:
        return False
    if type(value["schema_version"]) is not int or value["schema_version"] != RUN_RECORD_VERSION:
        return False
    if any(not isinstance(value[name], str) or not value[name].startswith("/")
           or "\0" in value[name] for name in ("base", "binary")):
        return False
    if type(value["started_at_epoch_ms"]) is not int or value["started_at_epoch_ms"] <= 0:
        return False
    identity = value["process_identity"]
    if value["pid"] is None:
        return identity is None
    return (
        type(value["pid"]) is int and 1 < value["pid"] <= 2**31 - 1
        and isinstance(identity, dict) and set(identity) == {"start_ticks", "uid", "boot_id"}
        and type(identity["start_ticks"]) is int and identity["start_ticks"] > 0
        and type(identity["uid"]) is int and identity["uid"] == os.getuid()
        and isinstance(identity["boot_id"], str)
        and re.fullmatch(r"[0-9a-f]{8}(?:-[0-9a-f]{4}){3}-[0-9a-f]{12}", identity["boot_id"]) is not None
    )


def read_run_record(state: Path) -> dict | None:
    try:
        descriptor = os.open(state / RUN_RECORD, os.O_RDONLY | os.O_NOFOLLOW | os.O_NONBLOCK)
    except FileNotFoundError:
        return None
    try:
        fcntl.flock(descriptor, fcntl.LOCK_SH | fcntl.LOCK_NB)
        content = run_record_bytes(descriptor)

        def closed_object(pairs: list) -> dict:
            result = dict(pairs)
            if len(result) != len(pairs):
                raise HarnessError("coding.harness.run-record-denied")
            return result

        value = json.loads(content, object_pairs_hook=closed_object)
        if not valid_run_record(value):
            raise HarnessError("coding.harness.run-record-denied")
        return value
    except (ValueError, UnicodeError, BlockingIOError) as error:
        raise HarnessError("coding.harness.run-record-denied") from error
    finally:
        os.close(descriptor)


class RunRecordReservation:
    """One invocation's held record; never adopts an existing reservation."""

    def __init__(self, state: Path, base: Path, executable: Path):
        self.directory = os.open(state, os.O_RDONLY | os.O_DIRECTORY | os.O_NOFOLLOW)
        self.descriptor = None
        self.expected = b""
        try:
            info = os.fstat(self.directory)
            if info.st_uid != os.getuid() or info.st_mode & 0o077:
                raise HarnessError("coding.harness.run-directory-denied")
            self.descriptor = os.open(
                RUN_RECORD, os.O_RDWR | os.O_CREAT | os.O_EXCL | os.O_NOFOLLOW,
                0o600, dir_fd=self.directory,
            )
            self.record = {
                "schema_version": RUN_RECORD_VERSION, "pid": None,
                "base": str(base), "binary": str(executable),
                "started_at_epoch_ms": time.time_ns() // 1_000_000,
                "process_identity": None,
            }
            self.publish()
        except BaseException:
            # An incomplete publication is preserved for inspection. It cannot
            # be mistaken for a completed child record or silently adopted.
            if self.descriptor is not None:
                os.close(self.descriptor)
            os.close(self.directory)
            raise

    def check_owned(self) -> None:
        held = os.fstat(self.descriptor)
        current = os.stat(RUN_RECORD, dir_fd=self.directory, follow_symlinks=False)
        if ((held.st_dev, held.st_ino) != (current.st_dev, current.st_ino)
                or run_record_bytes(self.descriptor) != self.expected):
            raise HarnessError("coding.harness.run-record-ownership-lost")

    def lock_for_update(self) -> None:
        deadline = time.monotonic() + 1
        while True:
            try:
                fcntl.flock(self.descriptor, fcntl.LOCK_EX | fcntl.LOCK_NB)
                return
            except BlockingIOError:
                if time.monotonic() >= deadline:
                    raise HarnessError("coding.harness.run-record-busy") from None
                time.sleep(0.01)

    def publish(self) -> None:
        content = (json.dumps(self.record, sort_keys=True) + "\n").encode("utf-8")
        if not valid_run_record(self.record) or len(content) > MAX_RECORD_BYTES:
            raise HarnessError("coding.harness.run-record-denied")
        self.lock_for_update()
        try:
            self.check_owned()
            offset = 0
            while offset < len(content):
                written = os.pwrite(self.descriptor, content[offset:], offset)
                if written <= 0:
                    raise HarnessError("coding.harness.run-record-write-failed")
                offset += written
            os.ftruncate(self.descriptor, len(content))
            os.fsync(self.descriptor)
            self.expected = content
            self.check_owned()
        finally:
            fcntl.flock(self.descriptor, fcntl.LOCK_UN)

    def identify(self, process: subprocess.Popen) -> None:
        try:
            identity = linux_process_identity(process.pid)
        except (OSError, ValueError, IndexError, HarnessError):
            # A fast child may already have exited before its /proc observation.
            # Reap our own child, retaining its actual exit and streams normally.
            if process.poll() is not None:
                return
            raise HarnessError("coding.harness.child-identity-unavailable") from None
        self.record.update(pid=process.pid, process_identity=identity)
        self.publish()

    def close(self, *, child_reaped: bool = True) -> None:
        try:
            if not child_reaped:
                raise HarnessError("coding.harness.child-cleanup-incomplete")
            self.lock_for_update()
            self.check_owned()
            os.unlink(RUN_RECORD, dir_fd=self.directory)
            os.fsync(self.directory)
        finally:
            try:
                os.close(self.descriptor)
            finally:
                os.close(self.directory)


def linux_process_identity(pid: int) -> dict:
    process = Path("/proc") / str(pid)
    # comm may contain spaces and parentheses; fields after its final ')' start
    # at field 3 (state). Start time is field 22, hence index 19 here.
    fields = (process / "stat").read_text().rsplit(") ", 1)[1].split()
    if fields[0] in {"Z", "X", "x"}:
        raise HarnessError("coding.harness.process-exited")
    return {
        "start_ticks": int(fields[19]), "uid": process.stat().st_uid,
        "boot_id": Path("/proc/sys/kernel/random/boot_id").read_text().strip(),
    }


def _owned_process_arguments(record: dict, base: Path) -> list[bytes] | None:
    """Observe arguments only while the existing exact process identity matches."""
    pid = record.get("pid")
    if not valid_run_record(record) or pid is None or record["base"] != str(base):
        return None
    process = Path("/proc") / str(pid)
    try:
        before = linux_process_identity(pid)
        executable = (process / "exe").resolve(strict=True)
        arguments = (process / "cmdline").read_bytes().split(b"\0")
        after = linux_process_identity(pid)
    except (OSError, ValueError, IndexError, HarnessError):
        return None
    expected = Path(record["binary"])
    roots = zip((b"--state-root", b"--disposable-root", b"--workspace-root"), paths(base))
    owns_arguments = all(
        arguments.count(flag) == 1
        and arguments.index(flag) + 1 < len(arguments)
        and arguments[arguments.index(flag) + 1] == os.fsencode(path)
        for flag, path in roots
    )
    if (before == after == record["process_identity"] and executable == expected
            and owns_arguments and arguments.count(b"--development") == 1):
        return arguments
    return None


def exact_running_process(record: dict, base: Path) -> bool:
    return _owned_process_arguments(record, base) is not None


def _model_request_diagnosis(arguments: list[bytes] | None) -> dict:
    """A process argument is requested intent, never serving or qualification proof."""
    result = {"selection": None, "observation": "unavailable",
              "serving": "not-observed", "qualification": "not-assessed"}
    if arguments is None:
        return result
    if arguments.count(b"--model") != 1 or any(
        value.startswith(b"--model=") for value in arguments
    ):
        return result
    position = arguments.index(b"--model") + 1
    if position < len(arguments) and arguments[position] in {
        choice.encode("ascii") for choice in MODEL_CHOICES
    }:
        result.update(selection=arguments[position].decode("ascii"),
                      observation="owned-process-arguments")
    return result


def native_executable_prerequisite(path: Path) -> str:
    """Diagnostic only: native Rust owners still hold and verify exact launch objects."""
    try:
        # Match the native root-owned path prerequisite without following aliases.
        for component in (*reversed(path.parents[:-1]), path):
            info = component.lstat()
            if info.st_uid != 0 or info.st_mode & 0o022 or stat.S_ISLNK(info.st_mode):
                return "untrusted-path"
        if not stat.S_ISREG(info.st_mode) or not info.st_mode & 0o111 or info.st_size <= 0:
            return "not-executable"
    except OSError:
        return "unavailable"
    return "available"


def confinement_diagnosis() -> dict:
    """Report necessary launch prerequisites, never platform or confinement admission."""
    executables = {
        name: native_executable_prerequisite(Path("/usr/bin") / name)
        for name in ("bwrap", "systemd-run", "systemctl", "git")
    }
    manager = "not-probed-untrusted-systemctl"
    if executables["systemctl"] == "available":
        try:
            probe = subprocess.run(
                ["/usr/bin/systemctl", "--user", "--no-pager", "show",
                 "--property=Version", "--value"],
                stdin=subprocess.DEVNULL, stdout=subprocess.DEVNULL,
                stderr=subprocess.DEVNULL, timeout=3, check=False,
                # Match the native supervisor's fixed local bus. Do not inherit
                # an ambient remote bus address or control-program overrides.
                env={"LANG": "C", "XDG_RUNTIME_DIR": f"/run/user/{os.getuid()}",
                     "DBUS_SESSION_BUS_ADDRESS": f"unix:path=/run/user/{os.getuid()}/bus"},
            )
            manager = "available" if probe.returncode == 0 else "unavailable"
        except subprocess.TimeoutExpired:
            manager = "timeout"
        except OSError:
            manager = "unavailable"
    return {
        "scope": "prerequisites-only",
        "native_executables": executables,
        "user_manager": manager,
        "ready": all(value == "available" for value in executables.values())
        and manager == "available",
    }


def diagnose(base: Path) -> dict:
    base = base.resolve(strict=True)
    state, disposable, workspace = paths(base)
    checks: dict[str, object] = {
        "schema_version": DIAGNOSIS_VERSION,
        "scope": "development-diagnostics-only",
        "fixture_profile_id": PROFILE,
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
    checks["confinement_prerequisites"] = confinement_diagnosis()
    checks["confinement"] = checks["confinement_prerequisites"]["ready"]
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
    arguments = _owned_process_arguments(record, base) if record else None
    process_running = arguments is not None
    checks["model_request"] = _model_request_diagnosis(arguments)
    checks["lifecycle"] = (
        "running" if process_running and state_key_present
        else "starting" if process_running
        else "reserved" if record and record["pid"] is None
        else "stale-record" if record
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
    suspend_resume_probe: bool = False,
    action_history_export: str | None = None,
    support_bundle: Path | None = None,
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
    if suspend_resume_probe:
        command.append("--suspend-resume-probe")
    if action_history_export is not None:
        command.extend(("--action-history-export", export_selection(action_history_export)))
    if support_bundle is not None:
        command.extend(("--support-bundle", str(support_bundle_directory(support_bundle))))
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
    opened = ExitStack()
    process = None
    reservation = None
    result = None
    started_at_epoch_ms = time.time_ns() // 1_000_000
    started_monotonic = time.monotonic()
    implementation = implementation_identity() if log_dir is not None else None
    try:
        reservation = RunRecordReservation(state, base, executable)
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
            opened.enter_context(stdout_target)
            stderr_target = os.fdopen(
                os.open(
                    log_dir / "stderr.log",
                    os.O_WRONLY | os.O_CREAT | os.O_EXCL | os.O_NOFOLLOW,
                    0o600,
                ),
                "wb",
            )
            opened.enter_context(stderr_target)
        resource_guard = CandidateResourceGuard.acquire() if model != "scripted" else None
        process = subprocess.Popen(command, stdin=None, stdout=stdout_target, stderr=stderr_target)
        reservation.identify(process)
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
    finally:
        try:
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
        finally:
            try:
                opened.close()
            finally:
                try:
                    if reservation is not None:
                        reservation.close(child_reaped=process is None or process.poll() is not None)
                finally:
                    if resource_guard is not None:
                        resource_guard.close()
    if result is not None:
        private_file(log_dir / "result.json", json.dumps(result, sort_keys=True) + "\n")
        print(json.dumps({"exit_code": exit_code, "log_dir": str(log_dir)}, sort_keys=True))
    return exit_code


RUN_ID = re.compile(r"[A-Za-z0-9._:-]{1,128}")


def ended_run(base: Path, run_id: str, action_history_export: str | None = None) -> int:
    """Reads an ended run's stored action histories back through the actual
    CLI and catalog host (Decisions 0129 and 0130). The catalog host composes
    no run; it opens the store only for the read."""
    if RUN_ID.fullmatch(run_id) is None:
        raise HarnessError("coding.harness.run-id-denied")
    arguments = ["--ended-run", run_id]
    if action_history_export is not None:
        arguments.extend(("--action-history-export", export_selection(action_history_export)))
    return catalog(base, arguments)


PACK_ID = re.compile(r"[a-z][a-z0-9.-]{0,63}")
LICENSE_ID = re.compile(r"[A-Za-z0-9.+-]{1,64}")
PACK_VERSION = re.compile(r"(0|[1-9][0-9]{0,9})\.(0|[1-9][0-9]{0,9})\.(0|[1-9][0-9]{0,9})")


def doc_pack_arguments(arguments: argparse.Namespace) -> list[str]:
    """The CLI arguments of one documentation pack operation (Decision 0130),
    in the closed forms the CLI accepts."""
    if arguments.import_directory is not None:
        directory = arguments.import_directory
        if not directory.is_absolute() or not directory.is_dir():
            raise HarnessError("coding.harness.doc-pack-directory-denied")
        licenses = arguments.allow_license or []
        if not licenses or any(LICENSE_ID.fullmatch(value) is None for value in licenses):
            raise HarnessError("coding.harness.doc-pack-license-denied")
        result = ["--doc-pack-import", str(directory)]
        for value in licenses:
            result.extend(("--allow-license", value))
        if arguments.refresh_version is not None:
            if PACK_VERSION.fullmatch(arguments.refresh_version) is None:
                raise HarnessError("coding.harness.doc-pack-version-denied")
            result.extend(("--refresh-version", arguments.refresh_version))
        return result
    if arguments.allow_license or arguments.refresh_version is not None:
        raise HarnessError("coding.harness.doc-pack-arguments-denied")
    if arguments.search is not None:
        if not arguments.search.split() or len(arguments.search) > 2_048:
            raise HarnessError("coding.harness.doc-pack-query-denied")
        result = ["--doc-pack-search", arguments.search]
        if arguments.pack is not None:
            if PACK_ID.fullmatch(arguments.pack) is None:
                raise HarnessError("coding.harness.doc-pack-id-denied")
            result.extend(("--doc-pack", arguments.pack))
        if arguments.include_history:
            result.append("--include-history")
        return result
    if arguments.pack is not None or arguments.include_history:
        raise HarnessError("coding.harness.doc-pack-arguments-denied")
    if arguments.list:
        return ["--doc-pack-list"]
    if arguments.inspect is not None:
        if PACK_ID.fullmatch(arguments.inspect) is None:
            raise HarnessError("coding.harness.doc-pack-id-denied")
        return ["--doc-pack-inspect", arguments.inspect]
    pack_id, _, version = arguments.delete.partition("@")
    if PACK_ID.fullmatch(pack_id) is None or "@" in arguments.delete and PACK_VERSION.fullmatch(version) is None:
        raise HarnessError("coding.harness.doc-pack-id-denied")
    return ["--doc-pack-delete", arguments.delete]


MEMORY_LABEL = re.compile(r"[a-z0-9._:-]{1,128}")
MEMORY_OBJECT = re.compile(r"[a-z0-9._:-]{1,256}")
MEMORY_ID = re.compile(r"memory-[a-z0-9-]{0,121}")
MEMORY_TYPES = ("semantic", "preference", "procedural", "episodic")


def memory_arguments(arguments: argparse.Namespace) -> list[str]:
    """The CLI arguments of one memory operation (Decision 0131), in the
    closed forms the CLI accepts."""
    workspace = arguments.workspace
    if workspace is not None and MEMORY_LABEL.fullmatch(workspace) is None:
        raise HarnessError("coding.harness.memory-workspace-denied")
    if arguments.remember is not None:
        text = arguments.remember
        if (
            not text.strip()
            or len(text.encode("utf-8")) > 16 * 1024
            or text.startswith("--")
            or any(ord(character) < 32 or 127 <= ord(character) < 160 for character in text)
        ):
            raise HarnessError("coding.harness.memory-text-denied")
        pack_id, _, rest = (arguments.cite or "").partition("@")
        version, _, path = rest.partition(":")
        if (
            PACK_ID.fullmatch(pack_id) is None
            or PACK_VERSION.fullmatch(version) is None
            or not path
            or len(path) > 4_096
        ):
            raise HarnessError("coding.harness.memory-citation-denied")
        if workspace is None or arguments.object is not None:
            raise HarnessError("coding.harness.memory-arguments-denied")
        result = [
            "--memory-remember", text, "--memory-workspace", workspace,
            "--memory-cite", arguments.cite,
        ]
        if arguments.type is not None:
            result.extend(("--memory-type", arguments.type))
        return result
    if arguments.cite is not None or arguments.type is not None:
        raise HarnessError("coding.harness.memory-arguments-denied")
    if arguments.revoke_source is not None:
        if MEMORY_LABEL.fullmatch(arguments.revoke_source) is None:
            raise HarnessError("coding.harness.memory-source-denied")
        if workspace is None:
            raise HarnessError("coding.harness.memory-arguments-denied")
        result = ["--memory-revoke-source", arguments.revoke_source, "--memory-workspace", workspace]
        if arguments.object is not None:
            if MEMORY_OBJECT.fullmatch(arguments.object) is None:
                raise HarnessError("coding.harness.memory-object-denied")
            result.extend(("--memory-object", arguments.object))
        return result
    if arguments.object is not None:
        raise HarnessError("coding.harness.memory-arguments-denied")
    if arguments.list:
        return ["--memory-list"] + (["--memory-workspace", workspace] if workspace else [])
    if workspace is not None:
        raise HarnessError("coding.harness.memory-arguments-denied")
    memory_id = arguments.revoke if arguments.revoke is not None else arguments.delete
    if MEMORY_ID.fullmatch(memory_id) is None:
        raise HarnessError("coding.harness.memory-id-denied")
    return ["--memory-revoke" if arguments.revoke is not None else "--memory-delete", memory_id]


EXTENSION_IDENTIFIER = re.compile(r"[A-Za-z0-9._:-]{1,128}")
EXTENSION_SHA256 = re.compile(r"[0-9a-f]{64}")
EXTENSION_SAMPLE = ROOT / "shells" / "host" / "fixtures" / "extension-sample"


def extension_arguments(arguments: argparse.Namespace) -> list[str]:
    """The CLI arguments of one extension operation (Decision 0132), in the
    closed forms the CLI accepts. Files and directories are absolute."""
    workspace = arguments.workspace
    if workspace is not None and MEMORY_LABEL.fullmatch(workspace) is None:
        raise HarnessError("coding.harness.extension-workspace-denied")
    if (arguments.allow_license is not None) != (arguments.install is not None):
        raise HarnessError("coding.harness.extension-arguments-denied")
    if arguments.list:
        return ["--extension-list"] + (["--extension-workspace", workspace] if workspace else [])
    if workspace is None:
        raise HarnessError("coding.harness.extension-arguments-denied")
    scope = ["--extension-workspace", workspace]
    for option, value in (
        ("--extension-trust", arguments.trust),
        ("--extension-revocations", arguments.revocations),
    ):
        if value is not None:
            if not value.is_absolute():
                raise HarnessError("coding.harness.extension-path-denied")
            return [option, str(value)] + scope
    if arguments.install is not None:
        if (
            not arguments.install.is_absolute()
            or EXTENSION_IDENTIFIER.fullmatch(arguments.allow_license) is None
        ):
            raise HarnessError("coding.harness.extension-install-denied")
        return [
            "--extension-install", str(arguments.install),
            "--extension-allow-license", arguments.allow_license,
        ] + scope
    if arguments.distrust is not None:
        if EXTENSION_SHA256.fullmatch(arguments.distrust) is None:
            raise HarnessError("coding.harness.extension-key-denied")
        return ["--extension-distrust", arguments.distrust] + scope
    if EXTENSION_IDENTIFIER.fullmatch(arguments.uninstall or "") is None:
        raise HarnessError("coding.harness.extension-package-denied")
    return ["--extension-uninstall", arguments.uninstall] + scope


def write_extension_sample(directory: Path) -> list[str]:
    """Copies the committed synthetic extension sample into a new private
    directory (Decision 0132). Its keys come from fixed, published seeds:
    trust them only inside a disposable demonstration root."""
    if not directory.is_absolute() or directory.exists() or directory.is_symlink():
        raise HarnessError("coding.harness.extension-sample-target-denied")
    files = sorted(
        path.relative_to(EXTENSION_SAMPLE)
        for path in EXTENSION_SAMPLE.rglob("*")
        if path.is_file() and not path.is_symlink()
    )
    if not files:
        raise HarnessError("coding.harness.extension-sample-missing")
    directory.mkdir(mode=0o700)
    for relative in files:
        target = directory / relative
        for parent in reversed(target.relative_to(directory).parents[:-1]):
            (directory / parent).mkdir(mode=0o700, exist_ok=True)
        private_file(target, (EXTENSION_SAMPLE / relative).read_text(encoding="utf-8"))
    return [str(relative) for relative in files]


def catalog(base: Path, arguments: list[str]) -> int:
    """Runs one catalog operation through the actual CLI and catalog host
    (Decision 0130). The same run record reservation as `start` keeps a
    second invocation out of the root while it runs."""
    base = base.resolve(strict=True)
    state, disposable, workspace = paths(base)
    current = diagnose(base)
    if current["lifecycle"] != "ready" or not current["transport_path"]:
        raise HarnessError("coding.harness.start-state-denied")
    executable = binary("agentmage")
    command = [
        str(executable), "--json", "code", "--development",
        "--state-root", str(state), "--disposable-root", str(disposable),
        "--workspace-root", str(workspace), *arguments,
    ]
    reservation = RunRecordReservation(state, base, executable)
    process = None
    try:
        process = subprocess.Popen(command, stdin=None)
        reservation.identify(process)
        return process.wait()
    finally:
        try:
            if process is not None and process.poll() is None:
                process.send_signal(signal.SIGINT)
                try:
                    process.wait(timeout=30)
                except subprocess.TimeoutExpired:
                    process.kill()
                    process.wait(timeout=10)
        finally:
            reservation.close(child_reaped=process is None or process.poll() is not None)


SAMPLE_PACK_FILES = {
    "guide/cache.md": (
        "# Build cache\n\n"
        "The build cache keeps compiled units between runs.\n"
        "Clear it with the clean step when a toolchain changes.\n\n"
        "## Remote cache\n\n"
        "A remote cache is not used by this sample; every entry stays local.\n"
    ),
    "guide/tests.md": (
        "# Running tests\n\n"
        "Run the validation script after each change to the calculator.\n"
        "A failing test names the function and the expected value.\n"
    ),
    "notes.txt": "Synthetic offline notes written for the AgentMage sample pack, version {version}.\n",
}


def write_sample_pack(directory: Path, version: str, retrieved_on: str) -> dict:
    """Writes a small sealed documentation pack of synthetic text into a new
    private directory, for trying the documentation pack commands (Decision
    0130). The manifest is sealed exactly as the knowledge component seals
    it: the SHA-256 of its compact encoding with the digest field empty."""
    match = PACK_VERSION.fullmatch(version)
    if match is None or re.fullmatch(r"[0-9]{4}-[0-9]{2}-[0-9]{2}", retrieved_on) is None:
        raise HarnessError("coding.harness.doc-pack-sample-denied")
    if not directory.is_absolute() or directory.exists() or directory.is_symlink():
        raise HarnessError("coding.harness.doc-pack-sample-target-denied")
    directory.mkdir(mode=0o700)
    files = []
    for path, template in sorted(SAMPLE_PACK_FILES.items()):
        # Each version's notes name it, so a search with history finds both.
        text = template.replace("{version}", version)
        target = directory / path
        target.parent.mkdir(mode=0o700, exist_ok=True)
        private_file(target, text)
        encoded = text.encode("utf-8")
        files.append({
            "path": path,
            "media_type": "markdown" if path.endswith(".md") else "plain_text",
            "byte_len": len(encoded),
            "sha256": hashlib.sha256(encoded).hexdigest(),
        })
    manifest = {
        "schema_version": 1,
        "pack_id": "agentmage-sample-guide",
        "version": {
            "major": int(match.group(1)), "minor": int(match.group(2)), "patch": int(match.group(3)),
        },
        "title": "AgentMage sample guide",
        "publisher": "AgentMage synthetic sample",
        "license": "LicenseRef-agentmage-sample",
        "retrieved_on": retrieved_on,
        "fresh_for_days": 30,
        "files": files,
        "total_bytes": sum(file["byte_len"] for file in files),
        "manifest_sha256": "",
    }
    manifest["manifest_sha256"] = hashlib.sha256(
        json.dumps(manifest, separators=(",", ":"), ensure_ascii=False).encode("utf-8")
    ).hexdigest()
    private_file(directory / "manifest.json", json.dumps(manifest, indent=2) + "\n")
    return manifest


def stop(base: Path) -> None:
    signal_running_cli(base, signal.SIGINT)
    print("coding.harness.cancellation-requested")


# Decision 0122: the development CLI maps SIGUSR1 to a suspension request and
# SIGUSR2 to a resumption request; the host decides each through its job ledger.
JOB_CONTROL_SIGNALS = {
    "pause": (signal.SIGUSR1, "coding.harness.suspension-requested"),
    "resume": (signal.SIGUSR2, "coding.harness.resumption-requested"),
}


def control(base: Path, command: str) -> None:
    number, message = JOB_CONTROL_SIGNALS[command]
    signal_running_cli(base, number)
    print(message)


def signal_running_cli(base: Path, number: signal.Signals) -> None:
    base = base.resolve(strict=True)
    state, _, _ = paths(base)
    record = read_run_record(state)
    if (
        record is None
        or record["pid"] is None
        or not (state / "operational-store-development-v1.key").is_file()
    ):
        raise HarnessError("coding.harness.not-running")
    if not hasattr(os, "pidfd_open") or not hasattr(signal, "pidfd_send_signal"):
        raise HarnessError("coding.harness.process-descriptor-unavailable")
    try:
        descriptor = os.pidfd_open(record["pid"])
    except OSError as error:
        raise HarnessError("coding.harness.process-descriptor-unavailable") from error
    try:
        if not exact_running_process(record, base) or read_run_record(state) != record:
            raise HarnessError("coding.harness.not-running")
        signal.pidfd_send_signal(descriptor, number)
    finally:
        os.close(descriptor)


def approval_delay(value: str) -> int:
    try:
        delay = int(value)
    except ValueError as error:
        raise argparse.ArgumentTypeError("approval delay must be an integer") from error
    if not 1 <= delay <= 10_000:
        raise argparse.ArgumentTypeError("approval delay must be between 1 and 10000 milliseconds")
    return delay


EXPORT_SELECTION = re.compile(r"(effects|job-control|routes):([1-9][0-9]{0,5}):([1-9][0-9]{0,5})")


def export_selection(value: str) -> str:
    """One action history chain and inclusive range, as the CLI accepts it
    (Decisions 0127 and 0128): `effects`, `job-control` or `routes`, then
    FROM:TO in order."""
    match = EXPORT_SELECTION.fullmatch(value)
    if match is None or int(match.group(2)) > int(match.group(3)) or int(match.group(3)) > 512:
        raise argparse.ArgumentTypeError("export selection must be CHAIN:FROM:TO")
    return value


def support_bundle_directory(value: Path) -> Path:
    """The existing absolute directory a support bundle may be written into
    (Decision 0128). The CLI's export workflow checks that it is private,
    owned by the person and not synchronized; the wrapper only refuses a
    relative or missing path before launch."""
    if not value.is_absolute() or not value.is_dir():
        raise HarnessError("coding.harness.support-bundle-directory-denied")
    return value


def parser() -> argparse.ArgumentParser:
    result = argparse.ArgumentParser(description=__doc__)
    commands = result.add_subparsers(dest="command", required=True)
    for name in ("setup", "status", "diagnose", "stop", "pause", "resume"):
        command = commands.add_parser(name)
        command.add_argument("--root", type=Path, required=True)
        if name == "setup":
            command.add_argument(
                "--fixture", choices=("repair", "new-file", "multi-file", "stable"), default="repair"
            )
    ended_run_command = commands.add_parser("ended-run")
    ended_run_command.add_argument("--root", type=Path, required=True)
    ended_run_command.add_argument("--run", required=True)
    ended_run_command.add_argument("--action-history-export", type=export_selection)
    doc_pack_command = commands.add_parser("doc-pack")
    doc_pack_command.add_argument("--root", type=Path, required=True)
    operation = doc_pack_command.add_mutually_exclusive_group(required=True)
    operation.add_argument("--import", dest="import_directory", type=Path)
    operation.add_argument("--list", action="store_true")
    operation.add_argument("--inspect")
    operation.add_argument("--delete")
    operation.add_argument("--search")
    doc_pack_command.add_argument("--allow-license", action="append")
    doc_pack_command.add_argument("--refresh-version")
    doc_pack_command.add_argument("--pack")
    doc_pack_command.add_argument("--include-history", action="store_true")
    memory_command = commands.add_parser("memory")
    memory_command.add_argument("--root", type=Path, required=True)
    memory_operation = memory_command.add_mutually_exclusive_group(required=True)
    memory_operation.add_argument("--remember")
    memory_operation.add_argument("--list", action="store_true")
    memory_operation.add_argument("--revoke")
    memory_operation.add_argument("--revoke-source")
    memory_operation.add_argument("--delete")
    memory_command.add_argument("--workspace")
    memory_command.add_argument("--cite")
    memory_command.add_argument("--type", choices=MEMORY_TYPES)
    memory_command.add_argument("--object")
    extension_command = commands.add_parser("extension")
    extension_command.add_argument("--root", type=Path, required=True)
    extension_operation = extension_command.add_mutually_exclusive_group(required=True)
    extension_operation.add_argument("--trust", type=Path)
    extension_operation.add_argument("--distrust")
    extension_operation.add_argument("--install", type=Path)
    extension_operation.add_argument("--uninstall")
    extension_operation.add_argument("--revocations", type=Path)
    extension_operation.add_argument("--list", action="store_true")
    extension_command.add_argument("--workspace")
    extension_command.add_argument("--allow-license")
    extension_sample_command = commands.add_parser("extension-sample")
    extension_sample_command.add_argument("--directory", type=Path, required=True)
    sample_command = commands.add_parser("doc-pack-sample")
    sample_command.add_argument("--directory", type=Path, required=True)
    sample_command.add_argument("--version", default="1.0.0")
    sample_command.add_argument("--retrieved-on", default=time.strftime("%Y-%m-%d", time.gmtime()))
    start_command = commands.add_parser("start")
    start_command.add_argument("--root", type=Path, required=True)
    start_command.add_argument(
        "--scenario",
        choices=(
            "no-op", "failed-test-repair", "slow-cancel", "new-file", "multi-file", "rollback",
            "false-completion", "overflow", "disk-pressure", "output-pressure",
            "protocol-correction", "arguments-correction", "repeated-protocol-rejection",
            "read-arguments-correction", "read-arguments-denied",
            "native-command-failure",
        ),
        required=True,
    )
    start_command.add_argument("--objective", required=True)
    start_command.add_argument("--follow-up", action="append", default=[])
    start_command.add_argument("--model", choices=MODEL_CHOICES, default="scripted")
    start_command.add_argument("--approve-this-run", action="store_true")
    start_command.add_argument("--stale-approval-probe", action="store_true")
    start_command.add_argument("--replay-approval-probe", action="store_true")
    start_command.add_argument("--expired-cursor-probe", action="store_true")
    start_command.add_argument("--approval-delay-ms", type=approval_delay, default=0)
    start_command.add_argument("--slow-subscriber-probe", action="store_true")
    start_command.add_argument("--artifact-integrity-probe", action="store_true")
    start_command.add_argument("--suspend-resume-probe", action="store_true")
    start_command.add_argument("--action-history-export", type=export_selection)
    start_command.add_argument("--support-bundle", type=Path)
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
        if arguments.command in JOB_CONTROL_SIGNALS:
            control(arguments.root, arguments.command)
            return 0
        if arguments.command == "ended-run":
            return ended_run(arguments.root, arguments.run, arguments.action_history_export)
        if arguments.command == "doc-pack":
            return catalog(arguments.root, doc_pack_arguments(arguments))
        if arguments.command == "memory":
            return catalog(arguments.root, memory_arguments(arguments))
        if arguments.command == "extension":
            return catalog(arguments.root, extension_arguments(arguments))
        if arguments.command == "extension-sample":
            print(json.dumps({
                "directory": str(arguments.directory),
                "files": write_extension_sample(arguments.directory),
            }, sort_keys=True))
            return 0
        if arguments.command == "doc-pack-sample":
            manifest = write_sample_pack(
                arguments.directory, arguments.version, arguments.retrieved_on
            )
            print(json.dumps({
                "directory": str(arguments.directory),
                "license": manifest["license"],
                "manifest_sha256": manifest["manifest_sha256"],
                "pack_id": manifest["pack_id"],
            }, sort_keys=True))
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
            arguments.suspend_resume_probe,
            arguments.action_history_export,
            arguments.support_bundle,
        )
    except (HarnessError, OSError, subprocess.SubprocessError, json.JSONDecodeError) as error:
        print(str(error), file=sys.stderr)
        return 1


if __name__ == "__main__":
    raise SystemExit(main())
