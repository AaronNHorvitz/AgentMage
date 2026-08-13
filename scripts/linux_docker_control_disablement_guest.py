#!/usr/bin/env python3
"""Disable each Docker isolation control independently in a disposable guest."""

from __future__ import annotations

import grp
import hashlib
import json
import os
import subprocess
import sys
import time
from pathlib import Path
from typing import Any, Final

import linux_docker_kvm_guest as topology


MANAGEMENT_UNIT: Final = "agentmage-dmr-management-probe.service"
CONTROLS: Final = (
    ("daemon-privilege", "runtime-user-in-docker-group", "docker-preflight.daemon.privilege"),
    ("socket-ownership", "docker-socket-world-writable", "docker-preflight.socket.ownership"),
    ("api-binding", "private-management-listener", "docker-preflight.api.binding"),
    (
        "container-reachability",
        "guard-socket-world-writable",
        "docker-preflight.container.reachability",
    ),
    ("image-identity", "model-manifest-substitution", "docker-preflight.image.identity"),
    ("resource-limits", "runner-memory-limit-drift", "docker-preflight.resources.invalid"),
    ("zero-egress", "ambient-proxy-injection", "docker-preflight.egress.nonzero"),
)


def docker_group_name() -> str:
    socket_gid = Path("/run/docker.sock").stat().st_gid
    try:
        return grp.getgrgid(socket_gid).gr_name
    except KeyError as error:
        raise topology.GuestEvidenceError("Docker socket group is unnamed") from error


def set_runtime_docker_group(*, member: bool) -> None:
    group = docker_group_name()
    if member:
        topology.run(["usermod", "--append", "--groups", group, "agentmage"])
        return
    topology.run(["gpasswd", "--delete", "agentmage", group], check=False)


def start_runner_variant(control: str | None) -> tuple[str, int]:
    model_digest = topology.MODEL_DIGEST
    memory = "64g"
    environment = ["--env=DO_NOT_TRACK=1"]
    if control == "image-identity":
        model_digest = "sha256:" + "1" * 64
    elif control == "resource-limits":
        memory = "63g"
    elif control == "zero-egress":
        environment.append("--env=HTTP_PROXY=http://127.0.0.1:9")

    container_id = topology.text(
        [
            "docker",
            "run",
            "--detach",
            "--name",
            topology.CONTAINER_NAME,
            "--pull=never",
            "--network=none",
            "--read-only",
            "--security-opt=no-new-privileges",
            "--cap-drop=ALL",
            f"--memory={memory}",
            f"--memory-swap={memory}",
            "--cpus=32",
            "--pids-limit=64",
            "--tmpfs=/run:rw,nosuid,nodev,noexec,size=64m",
            "--volume=agentmage-models:/models:ro",
            *environment,
            f"--label=io.agentmage.model.manifest-digest={model_digest}",
            "--label=io.agentmage.runtime-seconds=3600",
            "--label=io.agentmage.output-bytes=16777216",
            "--label=io.agentmage.parallel-slots=1",
            "--label=io.agentmage.image-repull=false",
            f"docker.io/docker/model-runner@{topology.RUNNER_DIGEST}",
        ],
        timeout=180,
    )
    if len(container_id) != 64:
        raise topology.GuestEvidenceError("runner container identity is invalid")
    runner_pid = int(
        topology.text(["docker", "inspect", "--format={{.State.Pid}}", container_id])
    )

    def listener_ready() -> bool:
        try:
            records = Path(f"/proc/{runner_pid}/net/tcp6").read_text(encoding="ascii")
        except OSError:
            return False
        return any(
            fields[1] == "00000000000000000000000000000000:3092"
            and fields[3] == "0A"
            for line in records.splitlines()[1:]
            if len(fields := line.split()) >= 4
        )

    topology.wait_for(listener_ready, "private Model Runner listener", timeout=120)
    return container_id, runner_pid


def start_management_listener(runner_pid: int) -> None:
    program = (
        "import socket,time;"
        "s=socket.socket(socket.AF_INET6);"
        "s.setsockopt(socket.SOL_SOCKET,socket.SO_REUSEADDR,1);"
        "s.bind(('::1',12435));s.listen(1);time.sleep(900)"
    )
    topology.run(
        [
            "systemd-run",
            "--quiet",
            f"--unit={MANAGEMENT_UNIT.removesuffix('.service')}",
            f"--property=NetworkNamespacePath=/proc/{runner_pid}/ns/net",
            "--property=NoNewPrivileges=yes",
            "/usr/bin/python3",
            "-c",
            program,
        ]
    )
    topology.wait_for(
        lambda: topology.network_record(runner_pid)["management_listener_count"] == 1,
        "private management listener",
    )


def collector_request(
    revision: str,
    container_id: str,
    runtime: dict[str, Any],
    guard: dict[str, Any],
    runner: dict[str, Any],
) -> dict[str, Any]:
    daemon_pid = int(
        topology.text(
            ["systemctl", "show", "--property=MainPID", "--value", "docker.service"]
        )
    )
    daemon = topology.process_record(daemon_pid) | {"pid": daemon_pid}
    socket_path = Path("/run/docker.sock")
    socket_metadata = socket_path.stat()
    return {
        "protocol_version": 2,
        "preflight_contract_version": 3,
        "source_revision": revision,
        "os_release_sha256": topology.sha256_file(Path("/etc/os-release")),
        "observation_started_unix_seconds": int(time.time()),
        "session_identity_sha256": hashlib.sha256(os.urandom(32)).hexdigest(),
        "collector_executable_sha256": topology.sha256_file(topology.COLLECTOR_PATH),
        "runtime_pid": runtime["pid"],
        "runtime_start_time_ticks": runtime["start_time_ticks"],
        "guard_pid": guard["pid"],
        "guard_start_time_ticks": guard["start_time_ticks"],
        "runner_container_id": container_id,
        "baseline": {
            "daemon_executable_sha256": daemon["executable_sha256"],
            "daemon_socket_identity_sha256": topology.object_identity(socket_path),
            "docker_socket_gid": socket_metadata.st_gid,
            "runtime_uid": topology.RUNTIME_UID,
            "runtime_gid": topology.RUNTIME_GID,
            "runtime_executable_sha256": runtime["executable_sha256"],
            "runtime_cgroup_sha256": runtime["cgroup_sha256"],
            "guard_uid": topology.GUARD_UID,
            "private_namespace_sha256": runner["network_namespace_sha256"],
            "guard_executable_sha256": guard["executable_sha256"],
            "guard_cgroup_sha256": guard["cgroup_sha256"],
        },
    }


def invoke_collector(request: dict[str, Any]) -> subprocess.CompletedProcess[bytes]:
    return topology.run(
        [str(topology.COLLECTOR_PATH), "--observe"],
        input_value=(json.dumps(request, sort_keys=True) + "\n").encode("ascii"),
        check=False,
        timeout=180,
    )


def prepare_case(revision: str, control: str | None) -> dict[str, Any]:
    set_runtime_docker_group(member=control == "daemon-privilege")
    runtime_pid = topology.start_runtime_peer()
    runtime = topology.process_record(runtime_pid) | {"pid": runtime_pid}
    container_id, runner_pid = start_runner_variant(control)
    guard_pid = topology.start_guard(runner_pid, runtime)
    guard = topology.process_record(guard_pid) | {"pid": guard_pid}
    runner = topology.process_record(runner_pid) | {"pid": runner_pid}

    if control == "socket-ownership":
        Path("/run/docker.sock").chmod(0o666)
    elif control == "api-binding":
        start_management_listener(runner_pid)
    elif control == "container-reachability":
        Path("/run/agentmage-dmr/guard.sock").chmod(0o666)

    return collector_request(revision, container_id, runtime, guard, runner)


def cleanup_case() -> dict[str, bool]:
    topology.run(["systemctl", "stop", MANAGEMENT_UNIT], check=False)
    topology.run(["systemctl", "reset-failed", MANAGEMENT_UNIT], check=False)
    record = topology.cleanup()
    socket_path = Path("/run/docker.sock")
    if socket_path.exists():
        socket_path.chmod(0o660)
        set_runtime_docker_group(member=False)
    record["management_probe_absent"] = (
        topology.run(["systemctl", "is-active", MANAGEMENT_UNIT], check=False)
        .stdout.strip()
        != b"active"
    )
    record["runtime_docker_group_absent"] = not socket_path.exists() or (
        "agentmage" not in grp.getgrnam(docker_group_name()).gr_mem
    )
    return record


def run_case(revision: str, control: str, mutation: str, expected: str) -> dict[str, Any]:
    request: dict[str, Any] | None = None
    failure: BaseException | None = None
    completed: subprocess.CompletedProcess[bytes] | None = None
    try:
        request = prepare_case(revision, control)
        completed = invoke_collector(request)
    except BaseException as error:
        failure = error
    cleanup = cleanup_case()
    if failure is not None:
        raise failure
    if request is None or completed is None:
        raise topology.GuestEvidenceError("control case did not execute")
    refusal = completed.stderr.decode("ascii", errors="replace").strip()
    if completed.returncode == 0 or completed.stdout or refusal != expected:
        raise topology.GuestEvidenceError(f"control refusal changed: {control}")
    if not all(cleanup.values()):
        raise topology.GuestEvidenceError(f"control cleanup failed: {control}")
    return {
        "control": control,
        "mutation": mutation,
        "expected_refusal": expected,
        "observed_refusal": refusal,
        "docker_admitted": False,
        "native_fallback_selected": False,
        "inference_performed": False,
        "cleanup": cleanup,
    }


def baseline_control(revision: str) -> dict[str, Any]:
    completed: subprocess.CompletedProcess[bytes] | None = None
    failure: BaseException | None = None
    try:
        completed = invoke_collector(prepare_case(revision, None))
    except BaseException as error:
        failure = error
    cleanup = cleanup_case()
    if failure is not None:
        raise failure
    if completed is None or completed.returncode != 0 or completed.stderr:
        raise topology.GuestEvidenceError("control baseline was not admitted")
    output = json.loads(completed.stdout)
    if output.get("admission", {}).get("status") != "admitted" or not all(
        cleanup.values()
    ):
        raise topology.GuestEvidenceError("control baseline changed")
    return {
        "status": "admitted",
        "inference_performed": False,
        "cleanup": cleanup,
    }


def collect(revision: str) -> dict[str, Any]:
    daemon = topology.configure_direct_daemon()
    if daemon["socket_activation_active"]:
        raise topology.GuestEvidenceError("Docker socket activation remained active")
    native = json.loads(topology.text([str(topology.NATIVE_PATH), "--self-check"]))
    native_before = topology.host_listener_count()
    baseline = baseline_control(revision)
    cases = [run_case(revision, *case) for case in CONTROLS]
    native_after = topology.host_listener_count()
    if native_before != 0 or native_after != 0:
        raise topology.GuestEvidenceError("native fallback listener appeared")
    return {
        "source_revision": revision,
        "distribution": topology.distribution_id(),
        "daemon_configuration": daemon,
        "native_descriptor": native,
        "baseline": baseline,
        "cases": cases,
        "fallback_selected": False,
        "inference_performed": False,
    }


def main() -> int:
    if os.geteuid() != 0 or len(sys.argv) != 2 or len(sys.argv[1]) != 40:
        return 2
    result: dict[str, Any] | None = None
    failure: BaseException | None = None
    try:
        result = collect(sys.argv[1])
    except BaseException as error:
        failure = error
    final_cleanup = cleanup_case()
    if failure is not None:
        print(f"control disablement evidence failed: {failure}", file=sys.stderr)
        return 1
    if result is None or not all(final_cleanup.values()):
        return 1
    result["final_cleanup"] = final_cleanup
    print(json.dumps(result, sort_keys=True))
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
