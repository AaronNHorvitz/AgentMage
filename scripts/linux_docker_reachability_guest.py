#!/usr/bin/env python3
"""Run hostile-position reachability probes inside one disposable KVM guest."""

from __future__ import annotations

import json
import os
import subprocess
import sys
import time
from pathlib import Path
from typing import Any

import linux_docker_kvm_guest as topology


PEER_ROOT = Path("/run/agentmage-runtime-peer")
PEER_SCRIPT = Path("/home/agentmage/linux_docker_guard_peer.py")
CONNECT_CODE = (
    "import socket; s=socket.socket(); s.settimeout(2); "
    "r=s.connect_ex((%r,12434)); s.close(); "
    "print('denied' if r else 'connected')"
)


def denied(arguments: list[str]) -> bool:
    completed = topology.run(arguments, check=False, timeout=30)
    return completed.returncode == 0 and completed.stdout.strip() == b"denied"


def guest_lan_address() -> str:
    records = json.loads(topology.text(["ip", "-json", "address", "show"]))
    for interface in records:
        if interface.get("ifname") == "lo":
            continue
        for address in interface.get("addr_info", []):
            if address.get("family") == "inet" and address.get("scope") == "global":
                return str(address["local"])
    raise topology.GuestEvidenceError("guest LAN address is unavailable")


def raw_reachability_probes() -> list[dict[str, Any]]:
    local = CONNECT_CODE % "127.0.0.1"
    probes = [
        ("host", ["python3", "-c", local]),
        (
            "same-user",
            [
                "setpriv",
                "--reuid=10001",
                "--regid=10001",
                "--clear-groups",
                "python3",
                "-c",
                local,
            ],
        ),
        (
            "vscode-extension",
            [
                "setpriv",
                "--reuid=10001",
                "--regid=10001",
                "--clear-groups",
                "env",
                "AGENTMAGE_EVIDENCE_ROLE=vscode-extension",
                "python3",
                "-c",
                local,
            ],
        ),
        (
            "tool-worker",
            [
                "unshare",
                "--net",
                "setpriv",
                "--reuid=10001",
                "--regid=10001",
                "--clear-groups",
                "python3",
                "-c",
                local,
            ],
        ),
        ("separate-namespace", ["unshare", "--net", "python3", "-c", local]),
        ("lan", ["python3", "-c", CONNECT_CODE % guest_lan_address()]),
        (
            "ordinary-container",
            [
                "docker",
                "run",
                "--rm",
                "--network=bridge",
                "--read-only",
                "--cap-drop=ALL",
                "--security-opt=no-new-privileges",
                "--entrypoint=/usr/bin/python3",
                f"docker.io/docker/model-runner@{topology.RUNNER_DIGEST}",
                "-c",
                local,
            ],
        ),
    ]
    results = []
    for position, command in probes:
        if not denied(command):
            raise topology.GuestEvidenceError(f"raw reachability probe failed: {position}")
        results.append(
            {"position": position, "raw_tcp_connected": False, "status": "denied"}
        )
    return results


def start_peer(secret: bytes) -> tuple[int, dict[str, Any]]:
    PEER_ROOT.mkdir(mode=0o700)
    os.chown(PEER_ROOT, topology.RUNTIME_UID, topology.RUNTIME_GID)
    secret_path = PEER_ROOT / "secret.bin"
    secret_path.write_bytes(secret)
    os.chown(secret_path, topology.RUNTIME_UID, topology.RUNTIME_GID)
    secret_path.chmod(0o400)
    topology.run(
        [
            "systemd-run",
            "--quiet",
            "--unit=agentmage-runtime-peer",
            "--property=User=agentmage",
            "--property=Group=agentmage",
            "--property=NoNewPrivileges=yes",
            "--property=PrivateTmp=yes",
            "/usr/bin/python3",
            str(PEER_SCRIPT),
        ]
    )
    topology.wait_for(lambda: (PEER_ROOT / "ready").is_file(), "guard peer readiness")
    pid = 0

    def peer_ready() -> bool:
        nonlocal pid
        try:
            candidate = topology.unit_pid(topology.RUNTIME_UNIT)
            record = topology.process_record(candidate)
            executable = os.readlink(f"/proc/{candidate}/exe")
        except (OSError, ValueError, topology.GuestEvidenceError):
            return False
        if (
            record["uid"] != topology.RUNTIME_UID
            or record["gid"] != topology.RUNTIME_GID
            or executable != os.path.realpath("/usr/bin/python3")
        ):
            return False
        pid = candidate
        return True

    topology.wait_for(peer_ready, "guard peer final identity")
    return pid, topology.process_record(pid) | {"pid": pid}


def authenticated_guard_control(runner_pid: int) -> dict[str, Any]:
    secret = os.urandom(32)
    peer_pid, peer = start_peer(secret)
    guard_pid = topology.start_guard(runner_pid, peer, secret)
    go = PEER_ROOT / "go"
    go.write_text("go\n", encoding="ascii")
    os.chown(go, topology.RUNTIME_UID, topology.RUNTIME_GID)
    deadline = time.monotonic() + 60
    while time.monotonic() < deadline and not (PEER_ROOT / "result.json").is_file():
        peer_state = topology.text(
            ["systemctl", "show", "--property=ActiveState", "--value", topology.RUNTIME_UNIT]
        )
        guard_state = topology.text(
            ["systemctl", "show", "--property=ActiveState", "--value", topology.GUARD_UNIT]
        )
        if peer_state in {"failed", "inactive"} or guard_state == "failed":
            unit = topology.RUNTIME_UNIT if peer_state in {"failed", "inactive"} else topology.GUARD_UNIT
            lines = topology.text(
                ["journalctl", "--unit", unit, "--output=cat", "--no-pager", "--lines=20"]
            ).splitlines()
            detail = lines[-1] if lines else "transient unit exited"
            raise topology.GuestEvidenceError(f"guard control unit failed: {detail}")
        time.sleep(0.1)
    if not (PEER_ROOT / "result.json").is_file():
        raise topology.GuestEvidenceError("guard control result timed out")
    topology.wait_for(
        lambda: topology.run(
            ["systemctl", "is-active", topology.GUARD_UNIT], check=False
        ).stdout.strip()
        != b"active",
        "guard policy refusal",
    )
    result = json.loads((PEER_ROOT / "result.json").read_text(encoding="ascii"))
    journal = topology.text(
        ["journalctl", "--unit", topology.GUARD_UNIT, "--output=cat", "--no-pager"]
    ).splitlines()
    if "docker-guard.service.request-policy" not in journal or result != {
        "authenticated_frame_delivered": True,
        "challenge_received": True,
        "inference_request_sent": False,
    }:
        raise topology.GuestEvidenceError("authenticated guard control did not terminate safely")
    return {
        "path": "authenticated-runtime-peer-to-production-guard",
        "peer_pid": peer_pid,
        "guard_pid": guard_pid,
        "challenge_authenticated": True,
        "terminal_guard_status": "docker-guard.service.request-policy",
        "raw_endpoint_forwarded": False,
        "inference_request_sent": False,
    }


def collect(revision: str) -> dict[str, Any]:
    daemon = topology.configure_direct_daemon()
    container_id, runner_pid = topology.start_runner()
    probes = raw_reachability_probes()
    control = authenticated_guard_control(runner_pid)
    return {
        "source_revision": revision,
        "distribution": topology.distribution_id(),
        "daemon_configuration": daemon,
        "runner_container_id": container_id,
        "raw_endpoint": "private-namespace-only",
        "probes": probes,
        "positive_control": control,
        "inference_performed": False,
    }


def cleanup() -> dict[str, bool]:
    record = topology.cleanup()
    subprocess.run(
        ["systemctl", "stop", topology.RUNTIME_UNIT],
        stdout=subprocess.DEVNULL,
        stderr=subprocess.DEVNULL,
        check=False,
    )
    if PEER_ROOT.exists():
        for child in PEER_ROOT.iterdir():
            child.unlink()
        PEER_ROOT.rmdir()
    record["peer_material_absent"] = not PEER_ROOT.exists()
    return record


def main() -> int:
    if os.geteuid() != 0 or len(sys.argv) != 2 or len(sys.argv[1]) != 40:
        return 2
    result: dict[str, Any] | None = None
    failure: BaseException | None = None
    try:
        result = collect(sys.argv[1])
    except BaseException as error:
        failure = error
    cleanup_record = cleanup()
    if failure is not None:
        print(f"reachability evidence failed: {failure}", file=sys.stderr)
        return 1
    if result is None or not all(cleanup_record.values()):
        return 1
    result["cleanup"] = cleanup_record
    print(json.dumps(result, sort_keys=True))
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
