#!/usr/bin/env python3
"""Run one bounded Docker topology acceptance probe inside a disposable KVM guest."""

from __future__ import annotations

import hashlib
import json
import os
import platform
import re
import stat
import struct
import subprocess
import sys
import time
from pathlib import Path
from typing import Any, Final


RUNNER_DIGEST: Final = (
    "sha256:bd94095bbc1ddc4266c3a88f582a92562c6b63eceb175572c9a60045663727c9"
)
MODEL_DIGEST: Final = (
    "sha256:08fa7b1d44f255be48cfc12359211725bfd659742612ed4b221cd5be90d14444"
)
RUNTIME_UID: Final = 10001
RUNTIME_GID: Final = 10001
GUARD_UID: Final = 10002
GUARD_PATH: Final = Path("/usr/libexec/agentmage/agentmage-docker-guard")
COLLECTOR_PATH: Final = Path(
    "/usr/libexec/agentmage/agentmage-docker-topology-collector"
)
NATIVE_PATH: Final = Path("/usr/libexec/agentmage/agentmage-native-inference")
RUNTIME_UNIT: Final = "agentmage-runtime-peer.service"
GUARD_UNIT: Final = "agentmage-dmr-guard.service"
CONTAINER_NAME: Final = "agentmage-dmr"


class GuestEvidenceError(ValueError):
    """Raised when the live guest topology is incomplete or drifted."""


def run(
    arguments: list[str],
    *,
    input_value: bytes | None = None,
    check: bool = True,
    timeout: int = 120,
) -> subprocess.CompletedProcess[bytes]:
    completed = subprocess.run(
        arguments,
        input=input_value,
        stdout=subprocess.PIPE,
        stderr=subprocess.PIPE,
        timeout=timeout,
        check=False,
    )
    if check and completed.returncode != 0:
        detail = completed.stderr.decode("utf-8", errors="replace").strip().splitlines()
        suffix = detail[-1] if detail else ""
        if len(suffix) > 160 or re.fullmatch(r"[A-Za-z0-9 ./_:=@()-]*", suffix) is None:
            suffix = ""
        diagnostic = f" ({suffix})" if suffix else ""
        raise GuestEvidenceError(
            f"guest command failed: {Path(arguments[0]).name}{diagnostic}"
        )
    return completed


def text(arguments: list[str], *, timeout: int = 120) -> str:
    return run(arguments, timeout=timeout).stdout.decode("utf-8").strip()


def sha256_file(path: Path) -> str:
    digest = hashlib.sha256()
    with path.open("rb") as stream:
        while block := stream.read(1024 * 1024):
            digest.update(block)
    return digest.hexdigest()


def sha256_bytes(value: bytes) -> str:
    return hashlib.sha256(value).hexdigest()


def process_start(pid: int) -> int:
    record = Path(f"/proc/{pid}/stat").read_text(encoding="ascii")
    fields = record[record.rfind(")") + 2 :].split()
    value = int(fields[19])
    if value <= 0:
        raise GuestEvidenceError("process start identity is invalid")
    return value


def process_record(pid: int) -> dict[str, Any]:
    status: dict[str, str] = {}
    for line in Path(f"/proc/{pid}/status").read_text(encoding="ascii").splitlines():
        if ":" in line:
            key, value = line.split(":", 1)
            status[key] = value.strip()
    uid = [int(value) for value in status["Uid"].split()]
    gid = [int(value) for value in status["Gid"].split()]
    if len(set(uid)) != 1 or len(set(gid)) != 1:
        raise GuestEvidenceError("process identity is not stable")
    executable = Path(f"/proc/{pid}/exe")
    cgroup = Path(f"/proc/{pid}/cgroup").read_bytes()
    return {
        "uid": uid[0],
        "gid": gid[0],
        "groups": [int(value) for value in status["Groups"].split()],
        "start_time_ticks": process_start(pid),
        "executable_sha256": sha256_file(executable),
        "cgroup_sha256": sha256_bytes(cgroup),
        "network_namespace_sha256": object_identity(Path(f"/proc/{pid}/ns/net")),
        "mount_namespace_sha256": object_identity(Path(f"/proc/{pid}/ns/mnt")),
        "effective_capabilities": int(status["CapEff"], 16),
        "no_new_privileges": status["NoNewPrivs"] == "1",
    }


def object_identity(path: Path) -> str:
    metadata = path.stat()
    digest = hashlib.sha256()
    digest.update(struct.pack(">Q", metadata.st_dev))
    digest.update(struct.pack(">Q", metadata.st_ino))
    digest.update(struct.pack(">I", metadata.st_uid))
    digest.update(struct.pack(">I", metadata.st_gid))
    digest.update(struct.pack(">I", metadata.st_mode))
    return digest.hexdigest()


def wait_for(predicate: Any, description: str, *, timeout: int = 60) -> None:
    deadline = time.monotonic() + timeout
    while time.monotonic() < deadline:
        if predicate():
            return
        time.sleep(0.2)
    raise GuestEvidenceError(f"timed out waiting for {description}")


def unit_pid(unit: str) -> int:
    value = text(["systemctl", "show", "--property=MainPID", "--value", unit])
    pid = int(value)
    if pid <= 1:
        raise GuestEvidenceError(f"unit has no held process: {unit}")
    return pid


def start_runtime_peer() -> int:
    run(
        [
            "systemd-run",
            "--quiet",
            "--unit=agentmage-runtime-peer",
            "--property=User=agentmage",
            "--property=Group=agentmage",
            "--property=NoNewPrivileges=yes",
            "--property=PrivateTmp=yes",
            "/usr/bin/sleep",
            "900",
        ]
    )
    wait_for(
        lambda: text(
            ["systemctl", "show", "--property=ActiveState", "--value", RUNTIME_UNIT]
        )
        == "active",
        "runtime peer",
    )
    return unit_pid(RUNTIME_UNIT)


def start_runner() -> tuple[str, int]:
    container_id = text(
        [
            "docker",
            "run",
            "--detach",
            "--name",
            CONTAINER_NAME,
            "--pull=never",
            "--network=none",
            "--read-only",
            "--security-opt=no-new-privileges",
            "--cap-drop=ALL",
            "--memory=64g",
            "--memory-swap=64g",
            "--cpus=32",
            "--pids-limit=64",
            "--tmpfs=/run:rw,nosuid,nodev,noexec,size=64m",
            "--volume=agentmage-models:/models:ro",
            "--env=DO_NOT_TRACK=1",
            f"--label=io.agentmage.model.manifest-digest={MODEL_DIGEST}",
            "--label=io.agentmage.runtime-seconds=3600",
            "--label=io.agentmage.output-bytes=16777216",
            "--label=io.agentmage.parallel-slots=1",
            "--label=io.agentmage.image-repull=false",
            f"docker.io/docker/model-runner@{RUNNER_DIGEST}",
        ],
        timeout=180,
    )
    if len(container_id) != 64:
        raise GuestEvidenceError("runner container identity is invalid")
    runner_pid = int(text(["docker", "inspect", "--format={{.State.Pid}}", container_id]))

    def listener_ready() -> bool:
        try:
            records = Path(f"/proc/{runner_pid}/net/tcp6").read_text(
                encoding="ascii"
            )
        except OSError:
            return False
        return any(
            fields[1] == "00000000000000000000000000000000:3092"
            and fields[3] == "0A"
            for line in records.splitlines()[1:]
            if len(fields := line.split()) >= 4
        )

    wait_for(listener_ready, "private Model Runner listener", timeout=120)
    return container_id, runner_pid


def create_guard_user() -> None:
    if run(["getent", "passwd", str(GUARD_UID)], check=False).returncode != 0:
        nologin = text(["sh", "-c", "command -v nologin"])
        run(
            [
                "useradd",
                "--uid",
                str(GUARD_UID),
                "--gid",
                str(RUNTIME_GID),
                "--no-create-home",
                "--shell",
                nologin,
                "agentmage-dmr-guard",
            ]
        )


def write_bootstrap(runtime: dict[str, Any], path: Path) -> None:
    secret = os.urandom(32)
    frame = b"".join(
        [
            b"AMDG0001",
            struct.pack(">H", 1),
            struct.pack(">I", RUNTIME_UID),
            struct.pack(">I", RUNTIME_GID),
            struct.pack(">I", runtime["pid"]),
            struct.pack(">Q", runtime["start_time_ticks"]),
            bytes.fromhex(runtime["executable_sha256"]),
            bytes.fromhex(runtime["cgroup_sha256"]),
            secret,
        ]
    )
    if len(frame) != 126 or secret == bytes(32):
        raise GuestEvidenceError("guard bootstrap construction failed")
    path.write_bytes(frame)
    os.chown(path, GUARD_UID, RUNTIME_GID)
    path.chmod(0o400)


def start_guard(runner_pid: int, runtime: dict[str, Any]) -> int:
    create_guard_user()
    parent = Path("/run/agentmage-dmr")
    parent.mkdir(mode=0o710)
    os.chown(parent, GUARD_UID, RUNTIME_GID)
    parent.chmod(0o710)
    replay = parent / "observations"
    replay.mkdir(mode=0o700)
    os.chown(replay, 0, 0)
    replay.chmod(0o700)
    bootstrap = parent / "bootstrap.bin"
    write_bootstrap(runtime, bootstrap)
    run(
        [
            "systemd-run",
            "--quiet",
            "--unit=agentmage-dmr-guard",
            "--property=User=agentmage-dmr-guard",
            "--property=Group=agentmage",
            "--property=NoNewPrivileges=yes",
            "--property=PrivateMounts=yes",
            f"--property=NetworkNamespacePath=/proc/{runner_pid}/ns/net",
            f"--property=StandardInput=file:{bootstrap}",
            str(GUARD_PATH),
            "--serve",
        ]
    )
    wait_for(lambda: (parent / "guard.sock").is_socket(), "guard socket")
    bootstrap.unlink()
    return unit_pid(GUARD_UNIT)


def package_record() -> dict[str, Any]:
    release: dict[str, str] = {}
    for line in Path("/etc/os-release").read_text(encoding="utf-8").splitlines():
        if "=" in line:
            key, value = line.split("=", 1)
            release[key] = value.strip('"')
    if release.get("ID") == "fedora":
        version = text(["rpm", "-q", "--qf", "%{VERSION}-%{RELEASE}", "agentmage"])
        package_format = "rpm"
    else:
        version = text(["dpkg-query", "-W", "-f=${Version}", "agentmage"])
        package_format = "deb"
    return {
        "format": package_format,
        "version": version,
        "payload": [
            {
                "path": str(path),
                "sha256": sha256_file(path),
                "mode": stat.S_IMODE(path.stat().st_mode),
            }
            for path in (NATIVE_PATH, GUARD_PATH, COLLECTOR_PATH)
        ],
    }


def distribution_id() -> str:
    for line in Path("/etc/os-release").read_text(encoding="utf-8").splitlines():
        if line.startswith("ID="):
            return line.split("=", 1)[1].strip('"')
    raise GuestEvidenceError("distribution identity is unavailable")


def host_listener_count() -> int:
    output = text(["ss", "-H", "-lnt"])
    return sum(1 for line in output.splitlines() if line.split()[3].endswith(":12434"))


def collect(revision: str) -> dict[str, Any]:
    native = json.loads(text([str(NATIVE_PATH), "--self-check"]))
    native_listener_before = host_listener_count()
    runtime_pid = start_runtime_peer()
    runtime = process_record(runtime_pid) | {"pid": runtime_pid}
    container_id, runner_pid = start_runner()
    guard_pid = start_guard(runner_pid, runtime)
    guard = process_record(guard_pid) | {"pid": guard_pid}
    runner = process_record(runner_pid) | {"pid": runner_pid}
    daemon_pid = int(text(["systemctl", "show", "--property=MainPID", "--value", "docker.service"]))
    daemon = process_record(daemon_pid) | {"pid": daemon_pid}
    socket_path = Path("/run/docker.sock")
    socket_metadata = socket_path.stat()
    collector_digest = sha256_file(COLLECTOR_PATH)
    request = {
        "protocol_version": 2,
        "preflight_contract_version": 3,
        "source_revision": revision,
        "os_release_sha256": sha256_file(Path("/etc/os-release")),
        "observation_started_unix_seconds": int(time.time()),
        "session_identity_sha256": hashlib.sha256(os.urandom(32)).hexdigest(),
        "collector_executable_sha256": collector_digest,
        "runtime_pid": runtime_pid,
        "runtime_start_time_ticks": runtime["start_time_ticks"],
        "guard_pid": guard_pid,
        "guard_start_time_ticks": guard["start_time_ticks"],
        "runner_container_id": container_id,
        "baseline": {
            "daemon_executable_sha256": daemon["executable_sha256"],
            "daemon_socket_identity_sha256": object_identity(socket_path),
            "docker_socket_gid": socket_metadata.st_gid,
            "runtime_uid": RUNTIME_UID,
            "runtime_gid": RUNTIME_GID,
            "runtime_executable_sha256": runtime["executable_sha256"],
            "runtime_cgroup_sha256": runtime["cgroup_sha256"],
            "guard_uid": GUARD_UID,
            "private_namespace_sha256": runner["network_namespace_sha256"],
            "guard_executable_sha256": guard["executable_sha256"],
            "guard_cgroup_sha256": guard["cgroup_sha256"],
        },
    }
    collector = run(
        [str(COLLECTOR_PATH), "--observe"],
        input_value=(json.dumps(request, sort_keys=True) + "\n").encode("ascii"),
        timeout=180,
    )
    observation = json.loads(collector.stdout)
    private_sockets = text(
        ["nsenter", "--target", str(runner_pid), "--net", "ss", "-H", "-lnt"]
    ).splitlines()
    return {
        "platform": {
            "distribution": distribution_id(),
            "architecture": platform.machine(),
            "kernel_release": platform.release(),
            "virtualization": text(["systemd-detect-virt"]),
            "cgroup_filesystem": "cgroup2" if Path("/sys/fs/cgroup/cgroup.controllers").is_file() else "other",
        },
        "package": package_record(),
        "native": {
            "descriptor": native,
            "mode": "packaged-inactive-self-check-only",
            "host_listener_count_before": native_listener_before,
            "host_listener_count_after": host_listener_count(),
            "inference_performed": False,
        },
        "docker": {
            "engine_version": text(["docker", "version", "--format={{.Server.Version}}"]),
            "container_id": container_id,
            "runner_image_digest": RUNNER_DIGEST,
            "model_manifest_digest": MODEL_DIGEST,
            "processes": {
                "daemon": daemon,
                "runtime_acceptance_peer": runtime,
                "guard": guard,
                "runner": runner,
            },
            "socket": {
                "identity_sha256": object_identity(socket_path),
                "owner_uid": socket_metadata.st_uid,
                "group_gid": socket_metadata.st_gid,
                "mode": stat.S_IMODE(socket_metadata.st_mode),
            },
            "private_tcp_listeners": private_sockets,
            "collector": observation,
        },
    }


def cleanup() -> dict[str, bool]:
    run(["docker", "rm", "--force", CONTAINER_NAME], check=False)
    for unit in (GUARD_UNIT, RUNTIME_UNIT):
        run(["systemctl", "stop", unit], check=False)
        run(["systemctl", "reset-failed", unit], check=False)
    parent = Path("/run/agentmage-dmr")
    if parent.exists():
        for child in sorted(parent.rglob("*"), reverse=True):
            child.unlink() if child.is_file() or child.is_socket() else child.rmdir()
        parent.rmdir()
    return {
        "runner_absent": run(["docker", "inspect", CONTAINER_NAME], check=False).returncode != 0,
        "guard_process_absent": not Path("/run/agentmage-dmr/guard.sock").exists(),
        "runtime_unit_inactive": run(
            ["systemctl", "is-active", RUNTIME_UNIT], check=False
        ).stdout.strip()
        != b"active",
        "host_raw_listener_absent": host_listener_count() == 0,
        "bootstrap_secret_absent": not Path("/run/agentmage-dmr/bootstrap.bin").exists(),
    }


def main() -> int:
    if os.geteuid() != 0 or len(sys.argv) != 2 or len(sys.argv[1]) != 40:
        print("guest evidence input refused", file=sys.stderr)
        return 2
    result: dict[str, Any] | None = None
    failure: BaseException | None = None
    try:
        result = collect(sys.argv[1])
    except BaseException as error:
        failure = error
    cleanup_record = cleanup()
    if failure is not None:
        print(f"guest evidence failed: {failure}", file=sys.stderr)
        return 1
    if result is None or not all(cleanup_record.values()):
        print("guest evidence cleanup failed", file=sys.stderr)
        return 1
    result["cleanup"] = cleanup_record
    print(json.dumps(result, sort_keys=True))
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
