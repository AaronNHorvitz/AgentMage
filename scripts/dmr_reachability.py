#!/usr/bin/env python3
"""Run and validate the Story 0.3 guarded Docker Model Runner reachability probe."""

from __future__ import annotations

import argparse
import ctypes
import hashlib
import ipaddress
import json
import os
import secrets
import select
import shutil
import socket
import subprocess
import sys
import threading
import time
from pathlib import Path
from typing import Final


ROOT: Final = Path(__file__).resolve().parents[1]
DMR_IMAGE: Final = (
    "docker.io/docker/model-runner@"
    "sha256:bd94095bbc1ddc4266c3a88f582a92562c6b63eceb175572c9a60045663727c9"
)
DMR_IMAGE_DIGEST: Final = (
    "sha256:bd94095bbc1ddc4266c3a88f582a92562c6b63eceb175572c9a60045663727c9"
)
PROBE_IMAGE: Final = (
    "docker.io/library/python@"
    "sha256:ff71127c215572121f1991bacf17f39ec5fcfd2de1f1c01a595835495bb9adfc"
)
PROBE_IMAGE_DIGEST: Final = (
    "sha256:ff71127c215572121f1991bacf17f39ec5fcfd2de1f1c01a595835495bb9adfc"
)
ROOTFS_CONTAINER: Final = "agentmage-reachability-rootfs"
RESULT_SCHEMA_VERSION: Final = 1
PR_SET_DUMPABLE: Final = 4
HTTP_REQUEST: Final = b"GET /models HTTP/1.1\r\nHost: localhost\r\nConnection: close\r\n\r\n"


class DMRReachabilityError(ValueError):
    """Raised when the guarded reachability probe cannot complete safely."""


def canonical_json(value: object) -> bytes:
    return (json.dumps(value, indent=2, sort_keys=True) + "\n").encode("utf-8")


def sha256_file(path: Path) -> str:
    digest = hashlib.sha256()
    with path.open("rb") as handle:
        for block in iter(lambda: handle.read(1024 * 1024), b""):
            digest.update(block)
    return digest.hexdigest()


def source_revision() -> str:
    result = subprocess.run(
        ["git", "rev-parse", "HEAD"],
        cwd=ROOT,
        check=True,
        capture_output=True,
        text=True,
    )
    return result.stdout.strip()


def revision_runner_sha256(revision: str) -> str:
    result = subprocess.run(
        ["git", "show", f"{revision}:scripts/dmr_reachability.py"],
        cwd=ROOT,
        check=False,
        capture_output=True,
    )
    if result.returncode != 0:
        raise DMRReachabilityError("runner is absent from its declared source revision")
    return hashlib.sha256(result.stdout).hexdigest()


def set_non_dumpable() -> None:
    libc = ctypes.CDLL(None, use_errno=True)
    if libc.prctl(PR_SET_DUMPABLE, 0, 0, 0, 0) != 0:
        error = ctypes.get_errno()
        raise DMRReachabilityError(f"cannot disable process dumpability: errno {error}")


def unix_http(socket_path: Path, token: str | None = None) -> tuple[int, bytes]:
    request = b"GET /models HTTP/1.1\r\nHost: localhost\r\nConnection: close\r\n"
    if token is not None:
        request += f"Authorization: Bearer {token}\r\n".encode("ascii")
    request += b"\r\n"
    client = socket.socket(socket.AF_UNIX, socket.SOCK_STREAM)
    client.settimeout(3)
    try:
        client.connect(str(socket_path))
        client.sendall(request)
        chunks: list[bytes] = []
        while True:
            block = client.recv(65536)
            if not block:
                break
            chunks.append(block)
    finally:
        client.close()
    response = b"".join(chunks)
    first_line = response.split(b"\r\n", 1)[0]
    parts = first_line.split()
    if len(parts) < 2 or not parts[1].isdigit():
        raise DMRReachabilityError("guard returned a malformed HTTP response")
    separator = response.find(b"\r\n\r\n")
    return int(parts[1]), response[separator + 4 :] if separator >= 0 else b""


def raw_http(socket_path: Path) -> bytes:
    client = socket.socket(socket.AF_UNIX, socket.SOCK_STREAM)
    client.settimeout(3)
    try:
        client.connect(str(socket_path))
        client.sendall(HTTP_REQUEST)
        chunks: list[bytes] = []
        while True:
            block = client.recv(65536)
            if not block:
                break
            chunks.append(block)
    finally:
        client.close()
    return b"".join(chunks)


def drain_stream(stream: object, lines: list[str]) -> None:
    if not hasattr(stream, "readline"):
        return
    for line in iter(stream.readline, ""):
        lines.append(line)


def wait_for_raw_socket(path: Path, process: subprocess.Popen[str]) -> None:
    deadline = time.monotonic() + 30
    while time.monotonic() < deadline:
        if process.poll() is not None:
            raise DMRReachabilityError("DMR exited before creating its private socket")
        if path.is_socket():
            response = raw_http(path)
            if response.startswith(b"HTTP/1.1 200"):
                return
        time.sleep(0.05)
    raise DMRReachabilityError("DMR did not become ready within 30 seconds")


def network_snapshot() -> dict[str, object]:
    interfaces: list[dict[str, object]] = []
    for line in Path("/proc/net/dev").read_text(encoding="utf-8").splitlines()[2:]:
        name, values = line.split(":", 1)
        fields = values.split()
        interfaces.append(
            {
                "name": name.strip(),
                "rx_bytes": int(fields[0]),
                "tx_bytes": int(fields[8]),
            }
        )
    routes = Path("/proc/net/route").read_text(encoding="utf-8").splitlines()[1:]
    listeners = 0
    for name in ("tcp", "tcp6"):
        path = Path("/proc/net") / name
        if not path.exists():
            continue
        listeners += sum(
            line.split()[3] == "0A"
            for line in path.read_text(encoding="utf-8").splitlines()[1:]
        )
    return {
        "interfaces": interfaces,
        "route_count": len(routes),
        "tcp_listener_count": listeners,
    }


class GuardProxy:
    def __init__(self, public_socket: Path, raw_socket: Path, token: str) -> None:
        self.public_socket = public_socket
        self.raw_socket = raw_socket
        self.token = token
        self.stop_event = threading.Event()
        self.server = socket.socket(socket.AF_UNIX, socket.SOCK_STREAM)
        self.server.settimeout(0.2)
        self.server.bind(str(public_socket))
        os.chmod(public_socket, 0o600)
        self.server.listen(8)
        self.thread = threading.Thread(target=self.serve, daemon=True)

    def start(self) -> None:
        self.thread.start()

    def close(self) -> None:
        self.stop_event.set()
        self.thread.join(timeout=3)
        self.server.close()
        self.public_socket.unlink(missing_ok=True)

    def serve(self) -> None:
        while not self.stop_event.is_set():
            try:
                client, _ = self.server.accept()
            except TimeoutError:
                continue
            except OSError:
                return
            with client:
                client.settimeout(2)
                try:
                    request = client.recv(65536)
                    expected = f"Authorization: Bearer {self.token}\r\n".encode("ascii")
                    if expected not in request:
                        client.sendall(
                            b"HTTP/1.1 401 Unauthorized\r\nContent-Length: 0\r\n"
                            b"Connection: close\r\n\r\n"
                        )
                        continue
                    response = raw_http(self.raw_socket)
                    client.sendall(response)
                except (OSError, DMRReachabilityError):
                    try:
                        client.sendall(
                            b"HTTP/1.1 502 Bad Gateway\r\nContent-Length: 0\r\n"
                            b"Connection: close\r\n\r\n"
                        )
                    except OSError:
                        pass


def dmr_preexec(rootfs: Path) -> None:
    os.chroot(rootfs)
    os.chdir("/app")
    os.setgroups([])
    os.setgid(995)
    os.setuid(995)
    set_non_dumpable()


def supervisor(rootfs: Path, public_dir: Path) -> int:
    set_non_dumpable()
    subprocess.run(
        ["mount", "-t", "tmpfs", "-o", "mode=0700,nosuid,nodev", "tmpfs", "/tmp"],
        check=True,
    )
    private_tmp = Path("/tmp/dmr-runtime")
    private_tmp.mkdir(mode=0o700)
    os.chown(private_tmp, 995, 995)
    subprocess.run(["mount", "--bind", str(private_tmp), str(rootfs / "tmp")], check=True)
    private_models = rootfs / "models"
    subprocess.run(
        [
            "mount",
            "-t",
            "tmpfs",
            "-o",
            "mode=0700,nosuid,nodev",
            "tmpfs",
            str(private_models),
        ],
        check=True,
    )
    os.chown(private_models, 995, 995)
    subprocess.run(
        ["mount", "-t", "tmpfs", "-o", "mode=0755,nosuid", "tmpfs", str(rootfs / "dev")],
        check=True,
    )
    for device in ("null", "random", "urandom", "zero"):
        target = rootfs / "dev" / device
        target.touch()
        subprocess.run(["mount", "--bind", f"/dev/{device}", str(target)], check=True)
    subprocess.run(["mount", "--rbind", "/proc", str(rootfs / "proc")], check=True)
    raw_socket = private_tmp / "model-runner.sock"
    public_socket = public_dir / "guard.sock"
    token = secrets.token_hex(32)
    logs: list[str] = []
    environment = {
        "HOME": "/home/modelrunner",
        "PATH": "/usr/local/sbin:/usr/local/bin:/usr/sbin:/usr/bin:/sbin:/bin",
        "LLAMA_SERVER_PATH": "/app",
        "MODELS_PATH": "/models",
        "MODEL_RUNNER_PORT": "",
        "MODEL_RUNNER_SOCK": "/tmp/model-runner.sock",
    }
    process = subprocess.Popen(
        ["/app/model-runner"],
        executable="/app/model-runner",
        env=environment,
        stdout=subprocess.PIPE,
        stderr=subprocess.STDOUT,
        text=True,
        preexec_fn=lambda: dmr_preexec(rootfs),
    )
    assert process.stdout is not None
    drain = threading.Thread(target=drain_stream, args=(process.stdout, logs), daemon=True)
    drain.start()
    proxy: GuardProxy | None = None
    try:
        wait_for_raw_socket(raw_socket, process)
        proxy = GuardProxy(public_socket, raw_socket, token)
        proxy.start()
        snapshot_before = network_snapshot()
        print(
            json.dumps(
                {
                    "event": "READY",
                    "token": token,
                    "supervisor_pid": os.getpid(),
                    "dmr_pid": process.pid,
                    "snapshot_before": snapshot_before,
                },
                sort_keys=True,
            ),
            flush=True,
        )
        if sys.stdin.readline().strip() != "STOP":
            raise DMRReachabilityError("supervisor did not receive its stop instruction")
        snapshot_after = network_snapshot()
        (public_dir / "runtime-state.json").write_bytes(
            canonical_json(
                {
                    "schema_version": 1,
                    "private_network_before": snapshot_before,
                    "private_network_after": snapshot_after,
                    "raw_socket_location": "private_tmpfs_only",
                    "guard_socket_mode": oct(public_socket.stat().st_mode & 0o777),
                    "supervisor_dumpable": False,
                    "dmr_dumpable": False,
                }
            )
        )
    finally:
        if proxy is not None:
            proxy.close()
        process.terminate()
        try:
            process.wait(timeout=10)
        except subprocess.TimeoutExpired:
            process.kill()
            process.wait(timeout=5)
        drain.join(timeout=3)
        (public_dir / "dmr-server.log").write_text("".join(logs), encoding="utf-8")
    return 0


def namespace_entry(container: str, public_dir: Path) -> int:
    mount = subprocess.run(
        ["podman", "mount", container],
        check=True,
        capture_output=True,
        text=True,
    ).stdout.strip()
    try:
        return subprocess.run(
            [
                "unshare",
                "--mount",
                "--net",
                "--fork",
                "--kill-child",
                sys.executable,
                str(Path(__file__).resolve()),
                "_supervisor",
                "--rootfs",
                mount,
                "--public-dir",
                str(public_dir),
            ],
            check=False,
        ).returncode
    finally:
        subprocess.run(["podman", "unmount", container], check=False, capture_output=True)


def classify_failure(output: str) -> str:
    known = (
        "PermissionError",
        "FileNotFoundError",
        "ConnectionRefusedError",
        "TimeoutError",
        "HTTP_401",
    )
    return next((name for name in known if name in output), "OTHER_FAILURE")


def run_probe_container(
    name: str,
    public_dir: Path,
    *,
    separate_mapping: bool = False,
) -> tuple[int, str]:
    uid = os.getuid()
    gid = os.getgid()
    mapping = ["--uidmap", "0:1025:1024", "--gidmap", "0:1025:1024"] if separate_mapping else ["--userns=keep-id"]
    script = (
        "import socket; p='/probe/guard.sock'; s=socket.socket(socket.AF_UNIX); "
        "s.settimeout(2); s.connect(p); "
        "s.sendall(b'GET /models HTTP/1.1\\r\\nHost: localhost\\r\\nConnection: close\\r\\n\\r\\n'); "
        "data=s.recv(4096); print('HTTP_401' if b' 401 ' in data else data[:80].decode(errors='replace'))"
    )
    command = [
        "podman",
        "run",
        "--rm",
        "--name",
        name,
        *mapping,
        "--user",
        f"{uid}:{gid}" if not separate_mapping else "0:0",
        "--network",
        "none",
        "--cap-drop=all",
        "--security-opt=no-new-privileges",
        "--security-opt=label=disable",
        "--volume",
        f"{public_dir}:/probe:ro,z",
        PROBE_IMAGE,
        "python3",
        "-c",
        script,
    ]
    result = subprocess.run(command, check=False, capture_output=True, text=True)
    return result.returncode, result.stdout + result.stderr


def private_host_address() -> str:
    result = subprocess.run(
        ["ip", "-j", "address", "show", "scope", "global"],
        check=True,
        capture_output=True,
        text=True,
    )
    for interface in json.loads(result.stdout):
        for address in interface.get("addr_info", []):
            value = address.get("local")
            if address.get("family") == "inet" and value and ipaddress.ip_address(value).is_private:
                return str(value)
    raise DMRReachabilityError("no private non-loopback IPv4 address is available")


def run_lan_probe(address: str) -> tuple[int, str]:
    script = (
        "import socket; s=socket.socket(); s.settimeout(2); "
        f"\ntry: s.connect(('{address}',12434)); print('CONNECTED')"
        "\nexcept Exception as e: print(type(e).__name__)"
    )
    result = subprocess.run(
        [
            "podman",
            "run",
            "--rm",
            "--name",
            "agentmage-reachability-lan",
            "--network",
            "pasta",
            "--cap-drop=all",
            "--security-opt=no-new-privileges",
            PROBE_IMAGE,
            "python3",
            "-c",
            script,
        ],
        check=False,
        capture_output=True,
        text=True,
    )
    return result.returncode, result.stdout + result.stderr


def ensure_tools() -> None:
    for command in ("bwrap", "ip", "mount", "podman", "unshare"):
        if shutil.which(command) is None:
            raise DMRReachabilityError(f"required probe command is unavailable: {command}")


def cleanup() -> None:
    subprocess.run(
        ["podman", "rm", "--force", ROOTFS_CONTAINER],
        check=False,
        capture_output=True,
    )


def run_live(output: Path) -> int:
    ensure_tools()
    revision = source_revision()
    if sha256_file(Path(__file__).resolve()) != revision_runner_sha256(revision):
        raise DMRReachabilityError(
            "live reachability runner differs from HEAD; commit it before collecting evidence"
        )
    if output.exists():
        raise DMRReachabilityError(f"refusing to overwrite reachability result: {output}")
    output.mkdir(parents=True, mode=0o700)
    os.chmod(output, 0o700)
    cleanup()
    create = subprocess.run(
        [
            "podman",
            "create",
            "--name",
            ROOTFS_CONTAINER,
            "--network",
            "none",
            "--entrypoint",
            "/bin/sh",
            DMR_IMAGE,
            "-c",
            "exit 0",
        ],
        check=False,
        capture_output=True,
        text=True,
    )
    if create.returncode != 0:
        raise DMRReachabilityError("cannot create exact DMR root-filesystem container")
    set_non_dumpable()
    command = [
        "podman",
        "unshare",
        sys.executable,
        str(Path(__file__).resolve()),
        "_namespace",
        "--container",
        ROOTFS_CONTAINER,
        "--public-dir",
        str(output),
    ]
    process = subprocess.Popen(
        command,
        stdin=subprocess.PIPE,
        stdout=subprocess.PIPE,
        stderr=subprocess.PIPE,
        text=True,
    )
    assert process.stdin is not None and process.stdout is not None and process.stderr is not None
    try:
        ready_streams, _, _ = select.select([process.stdout], [], [], 40)
        if not ready_streams:
            raise DMRReachabilityError("private DMR supervisor did not become ready")
        line = process.stdout.readline()
        try:
            ready = json.loads(line)
        except json.JSONDecodeError as error:
            raise DMRReachabilityError("private DMR supervisor returned malformed readiness data") from error
        if ready.get("event") != "READY":
            raise DMRReachabilityError("private DMR supervisor did not report READY")
        token = str(ready["token"])
        supervisor_pid = int(ready["supervisor_pid"])
        public_socket = output / "guard.sock"

        guarded_status, guarded_body = unix_http(public_socket, token)
        same_user_status, _ = unix_http(public_socket)
        proc_root_denied = False
        try:
            os.stat(Path("/proc") / str(supervisor_pid) / "root" / "tmp" / "model-runner.sock")
        except (FileNotFoundError, PermissionError):
            proc_root_denied = True

        rootfs_path = subprocess.run(
            ["podman", "inspect", ROOTFS_CONTAINER, "--format", "{{.GraphDriver.Data.MergedDir}}"],
            check=True,
            capture_output=True,
            text=True,
        ).stdout.strip()
        raw_visible = (Path(rootfs_path) / "tmp" / "model-runner.sock").exists()
        podman_exec = subprocess.run(
            ["podman", "exec", ROOTFS_CONTAINER, "/usr/bin/true"],
            check=False,
            capture_output=True,
            text=True,
        )
        tool_status, tool_output = run_probe_container(
            "agentmage-reachability-tool", output
        )
        namespace_status, namespace_output = run_probe_container(
            "agentmage-reachability-namespace", output, separate_mapping=True
        )
        lan_status, lan_output = run_lan_probe(private_host_address())
        process.stdin.write("STOP\n")
        process.stdin.flush()
        return_code = process.wait(timeout=30)
        stderr = process.stderr.read()
        runtime = read_json_file(output / "runtime-state.json")
        no_private_network = all(
            snapshot["route_count"] == 0
            and snapshot["tcp_listener_count"] == 0
            and [item["name"] for item in snapshot["interfaces"]] == ["lo"]
            for snapshot in (
                runtime["private_network_before"],
                runtime["private_network_after"],
            )
        )
        probes = {
            "guarded_kernel_adapter": {
                "expected": "HTTP_200",
                "observed": f"HTTP_{guarded_status}",
                "model_inventory_is_json_array": guarded_body.lstrip().startswith(b"["),
                "passed": guarded_status == 200 and guarded_body.lstrip().startswith(b"["),
            },
            "unrelated_same_user_process": {
                "expected": "HTTP_401_AND_RAW_NAMESPACE_HIDDEN",
                "observed": f"HTTP_{same_user_status}",
                "proc_root_access_denied": proc_root_denied,
                "raw_socket_visible_outside_private_mount": raw_visible,
                "rootless_podman_exec_succeeded": podman_exec.returncode == 0,
                "passed": (
                    same_user_status == 401
                    and proc_root_denied
                    and not raw_visible
                    and podman_exec.returncode != 0
                ),
            },
            "tool_container": {
                "expected": "HTTP_401",
                "observed": classify_failure(tool_output),
                "process_exit": tool_status,
                "passed": tool_status == 0 and "HTTP_401" in tool_output,
            },
            "separate_user_and_network_namespace": {
                "expected": "HTTP_401_OR_FILESYSTEM_DENIAL",
                "observed": classify_failure(namespace_output),
                "process_exit": namespace_status,
                "passed": (
                    (namespace_status == 0 and "HTTP_401" in namespace_output)
                    or classify_failure(namespace_output) in {"PermissionError", "FileNotFoundError"}
                ),
            },
            "emulated_lan_peer": {
                "expected": "CONNECTION_REFUSED",
                "observed": classify_failure(lan_output),
                "process_exit": lan_status,
                "target": "private_non_loopback_host_address:12434",
                "physical_peer_used": False,
                "passed": lan_status == 0 and "ConnectionRefusedError" in lan_output,
            },
        }
        result = {
            "schema_version": RESULT_SCHEMA_VERSION,
            "record_type": "dmr_guarded_reachability_probe",
            "source_revision": revision,
            "runner_sha256": sha256_file(Path(__file__).resolve()),
            "data_classification": "public_synthetic_metadata_only",
            "dmr_image_digest": DMR_IMAGE_DIGEST,
            "probe_image_digest": PROBE_IMAGE_DIGEST,
            "deployment": {
                "mode": "exact_image_rootfs_binary_in_private_user_mount_network_namespace",
                "raw_transport": "private_tmpfs_unix_domain_socket",
                "guard_transport": "mode_0600_authenticated_unix_domain_socket",
                "fresh_in_memory_token": True,
                "supervisor_and_dmr_dumpable": False,
                "production_support_claim": False,
            },
            "runtime": runtime,
            "probes": probes,
            "private_network_isolated": no_private_network,
            "supervisor_exit": return_code,
            "supervisor_stderr_empty": not stderr.strip(),
            "status": (
                "PASS"
                if all(item["passed"] for item in probes.values())
                and no_private_network
                and return_code == 0
                and not stderr.strip()
                else "FAIL"
            ),
            "limitations": [
                "The LAN probe used a separate rootless network namespace against the host's private non-loopback address, not a second physical workstation.",
                "The exact DMR binary and filesystem came from the pinned image, but this feasibility guard did not run the binary as an OCI workload.",
                "The guard is evaluation-only Python and is not the future signed AgentMage kernel adapter.",
                "The rejected model profiles remained unloaded; the guarded request exercised the DMR model-inventory API only.",
            ],
        }
        (output / "results.json").write_bytes(canonical_json(result))
        return 0 if result["status"] == "PASS" else 2
    finally:
        if process.poll() is None:
            try:
                process.stdin.write("STOP\n")
                process.stdin.flush()
                process.wait(timeout=10)
            except (BrokenPipeError, subprocess.TimeoutExpired):
                process.kill()
                process.wait(timeout=5)
        cleanup()


def read_json_file(path: Path) -> dict[str, object]:
    value = json.loads(path.read_text(encoding="utf-8"))
    if not isinstance(value, dict):
        raise DMRReachabilityError(f"expected JSON object: {path}")
    return value


def validate_result(
    result: dict[str, object], *, check_revision: bool = True
) -> list[str]:
    failures: list[str] = []
    expected_fields = {
        "schema_version",
        "record_type",
        "source_revision",
        "runner_sha256",
        "data_classification",
        "dmr_image_digest",
        "probe_image_digest",
        "deployment",
        "runtime",
        "probes",
        "private_network_isolated",
        "supervisor_exit",
        "supervisor_stderr_empty",
        "status",
        "limitations",
    }
    if set(result) != expected_fields:
        return ["reachability result fields do not match the schema"]
    if result["schema_version"] != RESULT_SCHEMA_VERSION or result["record_type"] != "dmr_guarded_reachability_probe":
        failures.append("unsupported reachability result schema")
    if result["dmr_image_digest"] != DMR_IMAGE_DIGEST or result["probe_image_digest"] != PROBE_IMAGE_DIGEST:
        failures.append("reachability image identity changed")
    if check_revision:
        try:
            if result["runner_sha256"] != revision_runner_sha256(str(result["source_revision"])):
                failures.append("reachability runner hash does not match its source revision")
        except DMRReachabilityError as error:
            failures.append(str(error))
    probes = result["probes"]
    expected_probes = {
        "guarded_kernel_adapter",
        "unrelated_same_user_process",
        "tool_container",
        "separate_user_and_network_namespace",
        "emulated_lan_peer",
    }
    if not isinstance(probes, dict) or set(probes) != expected_probes:
        failures.append("reachability probe matrix is incomplete")
    elif not all(isinstance(item, dict) and item.get("passed") is True for item in probes.values()):
        failures.append("one or more reachability probes failed")
    deployment = result["deployment"]
    if not isinstance(deployment, dict) or (
        deployment.get("mode")
        != "exact_image_rootfs_binary_in_private_user_mount_network_namespace"
        or deployment.get("raw_transport") != "private_tmpfs_unix_domain_socket"
        or deployment.get("guard_transport")
        != "mode_0600_authenticated_unix_domain_socket"
        or deployment.get("fresh_in_memory_token") is not True
        or deployment.get("supervisor_and_dmr_dumpable") is not False
        or deployment.get("production_support_claim") is not False
    ):
        failures.append("reachability deployment boundary is invalid or overstated")
    if result["private_network_isolated"] is not True:
        failures.append("private DMR namespace is not network isolated")
    runtime = result["runtime"]
    if not isinstance(runtime, dict):
        failures.append("reachability runtime evidence is missing")
    else:
        for phase in ("private_network_before", "private_network_after"):
            snapshot = runtime.get(phase, {})
            if not isinstance(snapshot, dict) or (
                snapshot.get("route_count") != 0
                or snapshot.get("tcp_listener_count") != 0
                or snapshot.get("interfaces")
                != [{"name": "lo", "rx_bytes": 0, "tx_bytes": 0}]
            ):
                failures.append(f"reachability {phase} is not isolated and idle")
        if runtime.get("raw_socket_location") != "private_tmpfs_only":
            failures.append("raw DMR socket location changed")
        if runtime.get("guard_socket_mode") != "0o600":
            failures.append("guard socket mode changed")
    limitations = result["limitations"]
    if not isinstance(limitations, list) or len(limitations) < 4:
        failures.append("reachability limitations are incomplete")
    if result["supervisor_exit"] != 0 or result["supervisor_stderr_empty"] is not True:
        failures.append("private DMR supervisor did not exit cleanly")
    expected_status = "PASS" if not failures else "FAIL"
    if result["status"] != expected_status:
        failures.append("reachability result status does not reconcile")
    return failures


def main(argv: list[str] | None = None) -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    subparsers = parser.add_subparsers(dest="command", required=True)
    live = subparsers.add_parser("live")
    live.add_argument("--output", type=Path, required=True)
    verify = subparsers.add_parser("verify")
    verify.add_argument("--result", type=Path, required=True)
    namespace = subparsers.add_parser("_namespace")
    namespace.add_argument("--container", required=True)
    namespace.add_argument("--public-dir", type=Path, required=True)
    private = subparsers.add_parser("_supervisor")
    private.add_argument("--rootfs", type=Path, required=True)
    private.add_argument("--public-dir", type=Path, required=True)
    args = parser.parse_args(argv)
    try:
        if args.command == "live":
            return run_live(args.output.resolve())
        if args.command == "verify":
            failures = validate_result(read_json_file(args.result))
            if failures:
                for failure in failures:
                    print(f"- {failure}", file=sys.stderr)
                return 1
            print(f"Validated guarded DMR reachability result: {args.result}")
            return 0
        if args.command == "_namespace":
            return namespace_entry(args.container, args.public_dir.resolve())
        if args.command == "_supervisor":
            return supervisor(args.rootfs.resolve(), args.public_dir.resolve())
        raise DMRReachabilityError(f"unsupported command: {args.command}")
    except (OSError, KeyError, TypeError, ValueError, subprocess.SubprocessError, DMRReachabilityError) as error:
        print(f"DMR reachability error: {error}", file=sys.stderr)
        return 1


if __name__ == "__main__":
    raise SystemExit(main())
