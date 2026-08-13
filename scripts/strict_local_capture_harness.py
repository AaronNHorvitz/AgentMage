#!/usr/bin/env python3
"""Run a fixed-purpose firewall and packet-capture acceptance fixture."""

from __future__ import annotations

import argparse
import errno
import hashlib
import json
import os
import re
import select
import socket
import subprocess
import sys
import threading
import time
from pathlib import Path
from typing import Any, Final


ROOT: Final = Path(__file__).resolve().parents[1]
IP: Final = "/usr/bin/ip"
NFT: Final = "/usr/bin/nft"
UNSHARE: Final = "/usr/bin/unshare"
ETH_P_ALL: Final = 0x0003
EGRESS_INTERFACE: Final = "am-egress0"
TEST_NET_ADDRESS: Final = "192.0.2.2/24"
TEST_NET_DNS: Final = "192.0.2.53"
FIXED_ENV: Final = {
    "HOME": "/nonexistent",
    "LANG": "C",
    "LC_ALL": "C",
    "PATH": "/usr/bin:/bin",
}
POLICY_SPEC: Final = {
    "family": "inet",
    "table": "agentmage_acceptance",
    "chain": "output",
    "hook": "output",
    "priority": 0,
    "policy": "drop",
    "rules": [
        {
            "interface": "lo",
            "protocol": "ipv4",
            "destination": "127.0.0.0/8",
            "action": "accept",
        },
        {
            "interface": "lo",
            "protocol": "ipv6",
            "destination": "::1/128",
            "action": "accept",
        },
        {"interface": "any", "protocol": "any", "action": "drop"},
    ],
}
RESULT_KEYS: Final = {
    "schema_version",
    "fixture",
    "namespace_sha256",
    "interfaces",
    "routes",
    "firewall_policy_sha256",
    "firewall_policy_exact",
    "loopback_allowed",
    "loopback_captured_frames",
    "loopback_captured_bytes",
    "synthetic_dns_attempted",
    "synthetic_dns_send_refused",
    "synthetic_egress_frames",
    "synthetic_egress_bytes",
    "firewall_drop_packets",
    "firewall_drop_bytes",
    "packet_payload_retained",
    "external_network_used",
}
RULESET: Final = """
table inet agentmage_acceptance {
    chain output {
        type filter hook output priority 0; policy drop;
        oifname "lo" ip daddr 127.0.0.0/8 counter accept
        oifname "lo" ip6 daddr ::1 counter accept
        counter drop
    }
}
"""


class CaptureHarnessError(RuntimeError):
    """Raised when the isolated acceptance fixture cannot prove its boundary."""


def canonical_sha256(value: Any) -> str:
    return hashlib.sha256(
        json.dumps(value, sort_keys=True, separators=(",", ":")).encode()
    ).hexdigest()


def namespace_identity() -> str:
    try:
        stat = os.stat("/proc/self/ns/net")
    except OSError as error:
        raise CaptureHarnessError("network namespace identity unavailable") from error
    return hashlib.sha256(f"{stat.st_dev}:{stat.st_ino}".encode()).hexdigest()


def run_checked(arguments: list[str], *, input_text: str | None = None) -> str:
    completed = subprocess.run(
        arguments,
        input=input_text,
        stdout=subprocess.PIPE,
        stderr=subprocess.PIPE,
        text=True,
        timeout=15,
        check=False,
        env=FIXED_ENV,
    )
    if completed.returncode != 0:
        raise CaptureHarnessError(f"isolated command failed: {Path(arguments[0]).name}")
    return completed.stdout


def configure_namespace() -> None:
    run_checked([IP, "link", "set", "lo", "up"])
    run_checked([IP, "link", "add", EGRESS_INTERFACE, "type", "dummy"])
    run_checked([IP, "addr", "add", TEST_NET_ADDRESS, "dev", EGRESS_INTERFACE])
    run_checked([IP, "link", "set", EGRESS_INTERFACE, "up"])
    run_checked([NFT, "--file", "-"], input_text=RULESET)


def interface_profile() -> list[dict[str, Any]]:
    try:
        links = json.loads(run_checked([IP, "-json", "link", "show"]))
    except json.JSONDecodeError as error:
        raise CaptureHarnessError("interface inventory is invalid") from error
    if not isinstance(links, list):
        raise CaptureHarnessError("interface inventory is invalid")
    return sorted(
        (
            {
                "name": item.get("ifname"),
                "up": "UP" in item.get("flags", []),
                "loopback": "LOOPBACK" in item.get("flags", []),
            }
            for item in links
        ),
        key=lambda item: str(item["name"]),
    )


def route_profile() -> list[dict[str, Any]]:
    try:
        routes = json.loads(
            run_checked([IP, "-json", "route", "show", "table", "main"])
        )
    except json.JSONDecodeError as error:
        raise CaptureHarnessError("route inventory is invalid") from error
    if not isinstance(routes, list):
        raise CaptureHarnessError("route inventory is invalid")
    return sorted(
        (
            {
                "destination": item.get("dst"),
                "device": item.get("dev"),
                "scope": item.get("scope"),
            }
            for item in routes
        ),
        key=lambda item: (str(item["destination"]), str(item["device"])),
    )


def strip_observation_fields(value: Any) -> Any:
    if isinstance(value, dict):
        return {
            key: strip_observation_fields(item)
            for key, item in value.items()
            if key not in {"bytes", "handle", "packets"}
        }
    if isinstance(value, list):
        return [strip_observation_fields(item) for item in value]
    return value


def normalized_ruleset() -> list[dict[str, Any]]:
    try:
        value = json.loads(
            run_checked([NFT, "--json", "list", "table", "inet", "agentmage_acceptance"])
        )
    except json.JSONDecodeError as error:
        raise CaptureHarnessError("firewall observation is invalid") from error
    items = value.get("nftables") if isinstance(value, dict) else None
    if not isinstance(items, list):
        raise CaptureHarnessError("firewall observation is invalid")
    return [
        strip_observation_fields(item)
        for item in items
        if isinstance(item, dict) and "metainfo" not in item
    ]


def expected_normalized_ruleset() -> list[dict[str, Any]]:
    return [
        {"table": {"family": "inet", "name": "agentmage_acceptance"}},
        {
            "chain": {
                "family": "inet",
                "table": "agentmage_acceptance",
                "name": "output",
                "type": "filter",
                "hook": "output",
                "prio": 0,
                "policy": "drop",
            }
        },
        {
            "rule": {
                "family": "inet",
                "table": "agentmage_acceptance",
                "chain": "output",
                "expr": [
                    {
                        "match": {
                            "op": "==",
                            "left": {"meta": {"key": "oifname"}},
                            "right": "lo",
                        }
                    },
                    {
                        "match": {
                            "op": "==",
                            "left": {"payload": {"protocol": "ip", "field": "daddr"}},
                            "right": {"prefix": {"addr": "127.0.0.0", "len": 8}},
                        }
                    },
                    {"counter": {}},
                    {"accept": None},
                ],
            }
        },
        {
            "rule": {
                "family": "inet",
                "table": "agentmage_acceptance",
                "chain": "output",
                "expr": [
                    {
                        "match": {
                            "op": "==",
                            "left": {"meta": {"key": "oifname"}},
                            "right": "lo",
                        }
                    },
                    {
                        "match": {
                            "op": "==",
                            "left": {"payload": {"protocol": "ip6", "field": "daddr"}},
                            "right": "::1",
                        }
                    },
                    {"counter": {}},
                    {"accept": None},
                ],
            }
        },
        {
            "rule": {
                "family": "inet",
                "table": "agentmage_acceptance",
                "chain": "output",
                "expr": [{"counter": {}}, {"drop": None}],
            }
        },
    ]


def drop_counter() -> dict[str, int]:
    try:
        value = json.loads(
            run_checked([NFT, "--json", "list", "chain", "inet", "agentmage_acceptance", "output"])
        )
    except json.JSONDecodeError as error:
        raise CaptureHarnessError("firewall counters are invalid") from error
    items = value.get("nftables") if isinstance(value, dict) else None
    if not isinstance(items, list):
        raise CaptureHarnessError("firewall counters are invalid")
    for item in items:
        rule = item.get("rule") if isinstance(item, dict) else None
        expressions = rule.get("expr") if isinstance(rule, dict) else None
        if not isinstance(expressions, list) or not expressions or "drop" not in expressions[-1]:
            continue
        counters = [part["counter"] for part in expressions if "counter" in part]
        if len(counters) != 1:
            break
        counter = counters[0]
        packets = counter.get("packets")
        size = counter.get("bytes")
        if isinstance(packets, int) and isinstance(size, int):
            return {"packets": packets, "bytes": size}
    raise CaptureHarnessError("firewall drop counter is unavailable")


def open_capture(interface: str) -> socket.socket:
    try:
        capture = socket.socket(socket.AF_PACKET, socket.SOCK_RAW, socket.htons(ETH_P_ALL))
        capture.bind((interface, 0))
        capture.setblocking(False)
    except OSError as error:
        raise CaptureHarnessError("packet capture could not start") from error
    return capture


def collect_capture(capture: socket.socket, duration: float = 0.2) -> dict[str, int]:
    frames = 0
    captured_bytes = 0
    deadline = time.monotonic() + duration
    while True:
        remaining = deadline - time.monotonic()
        if remaining <= 0:
            break
        readable, _, _ = select.select([capture], [], [], remaining)
        if not readable:
            break
        try:
            frame = capture.recv(65535)
        except BlockingIOError:
            continue
        frames += 1
        captured_bytes += len(frame)
    return {"frames": frames, "bytes": captured_bytes}


def discard_pending(capture: socket.socket) -> None:
    while True:
        try:
            capture.recv(65535)
        except BlockingIOError:
            return


def exercise_loopback(capture: socket.socket) -> dict[str, int]:
    listener = socket.socket(socket.AF_INET, socket.SOCK_STREAM)
    listener.settimeout(2)
    listener.bind(("127.0.0.1", 0))
    listener.listen(1)
    port = listener.getsockname()[1]
    failure: list[BaseException] = []

    def serve() -> None:
        try:
            connection, _ = listener.accept()
            with connection:
                if connection.recv(32) != b"agentmage-fixture":
                    raise CaptureHarnessError("local fixture payload changed")
                connection.sendall(b"accepted")
        except BaseException as error:
            failure.append(error)

    worker = threading.Thread(target=serve, daemon=True)
    worker.start()
    try:
        with socket.create_connection(("127.0.0.1", port), timeout=2) as client:
            client.sendall(b"agentmage-fixture")
            if client.recv(32) != b"accepted":
                raise CaptureHarnessError("local fixture response changed")
    finally:
        listener.close()
    worker.join(timeout=2)
    if worker.is_alive() or failure:
        raise CaptureHarnessError("local fixture did not complete")
    return collect_capture(capture)


def attempt_synthetic_dns_egress(capture: socket.socket) -> dict[str, int | bool]:
    discard_pending(capture)
    refused = False
    with socket.socket(socket.AF_INET, socket.SOCK_DGRAM) as client:
        try:
            client.sendto(b"agentmage-synthetic-dns", (TEST_NET_DNS, 53))
        except OSError as error:
            if error.errno not in {errno.EACCES, errno.EPERM}:
                raise CaptureHarnessError("synthetic egress failed unexpectedly") from error
            refused = True
    return {**collect_capture(capture), "send_refused": refused}


def validate_result(result: dict[str, Any]) -> list[str]:
    errors: list[str] = []
    if set(result) != RESULT_KEYS:
        errors.append("capture result fields changed")
    exact = {
        "schema_version": 1,
        "fixture": "isolated-user-network-namespace",
        "interfaces": [
            {"name": EGRESS_INTERFACE, "up": True, "loopback": False},
            {"name": "lo", "up": True, "loopback": True},
        ],
        "routes": [
            {
                "destination": "192.0.2.0/24",
                "device": EGRESS_INTERFACE,
                "scope": "link",
            }
        ],
        "firewall_policy_sha256": canonical_sha256(POLICY_SPEC),
        "firewall_policy_exact": True,
        "loopback_allowed": True,
        "synthetic_dns_attempted": True,
        "synthetic_dns_send_refused": True,
        "synthetic_egress_frames": 0,
        "synthetic_egress_bytes": 0,
        "packet_payload_retained": False,
        "external_network_used": False,
    }
    for key, value in exact.items():
        if result.get(key) != value:
            errors.append(f"capture result changed: {key}")
    for key in ("loopback_captured_frames", "loopback_captured_bytes"):
        value = result.get(key)
        if not isinstance(value, int) or isinstance(value, bool) or value < 1:
            errors.append(f"capture result invalid: {key}")
    for key in ("firewall_drop_packets", "firewall_drop_bytes"):
        value = result.get(key)
        if not isinstance(value, int) or isinstance(value, bool) or value < 1:
            errors.append(f"capture result invalid: {key}")
    namespace = result.get("namespace_sha256")
    if not isinstance(namespace, str) or re.fullmatch(r"[0-9a-f]{64}", namespace) is None:
        errors.append("capture result invalid: namespace_sha256")
    return errors


def run_guest() -> dict[str, Any]:
    if os.geteuid() != 0:
        raise CaptureHarnessError("isolated namespace capabilities are absent")
    configure_namespace()
    interfaces = interface_profile()
    routes = route_profile()
    observed_rules = normalized_ruleset()
    if observed_rules != expected_normalized_ruleset():
        raise CaptureHarnessError("firewall policy observation changed")
    with open_capture("lo") as loopback_capture, open_capture(
        EGRESS_INTERFACE
    ) as egress_capture:
        discard_pending(loopback_capture)
        loopback = exercise_loopback(loopback_capture)
        egress = attempt_synthetic_dns_egress(egress_capture)
    dropped = drop_counter()
    result = {
        "schema_version": 1,
        "fixture": "isolated-user-network-namespace",
        "namespace_sha256": namespace_identity(),
        "interfaces": interfaces,
        "routes": routes,
        "firewall_policy_sha256": canonical_sha256(POLICY_SPEC),
        "firewall_policy_exact": True,
        "loopback_allowed": True,
        "loopback_captured_frames": loopback["frames"],
        "loopback_captured_bytes": loopback["bytes"],
        "synthetic_dns_attempted": True,
        "synthetic_dns_send_refused": egress["send_refused"],
        "synthetic_egress_frames": egress["frames"],
        "synthetic_egress_bytes": egress["bytes"],
        "firewall_drop_packets": dropped["packets"],
        "firewall_drop_bytes": dropped["bytes"],
        "packet_payload_retained": False,
        "external_network_used": False,
    }
    if errors := validate_result(result):
        raise CaptureHarnessError("; ".join(errors))
    return result


def run_harness() -> dict[str, Any]:
    parent_namespace = namespace_identity()
    completed = subprocess.run(
        [
            UNSHARE,
            "--user",
            "--map-root-user",
            "--net",
            sys.executable,
            str(Path(__file__).resolve()),
            "--guest",
        ],
        cwd=ROOT,
        stdout=subprocess.PIPE,
        stderr=subprocess.PIPE,
        text=True,
        timeout=30,
        check=False,
        env=FIXED_ENV,
    )
    if completed.returncode != 0:
        raise CaptureHarnessError("isolated capture guest failed")
    try:
        result = json.loads(completed.stdout)
    except json.JSONDecodeError as error:
        raise CaptureHarnessError("isolated capture result is invalid") from error
    if not isinstance(result, dict) or result.get("namespace_sha256") == parent_namespace:
        raise CaptureHarnessError("network namespace was not isolated")
    if errors := validate_result(result):
        raise CaptureHarnessError("; ".join(errors))
    return result


def main() -> int:
    parser = argparse.ArgumentParser()
    group = parser.add_mutually_exclusive_group(required=True)
    group.add_argument("--run", action="store_true")
    group.add_argument("--guest", action="store_true", help=argparse.SUPPRESS)
    arguments = parser.parse_args()
    if arguments.guest:
        print(json.dumps(run_guest(), sort_keys=True, separators=(",", ":")))
        return 0
    run_harness()
    print("Strict-local isolated firewall and packet-capture harness passed")
    return 0


if __name__ == "__main__":
    try:
        raise SystemExit(main())
    except CaptureHarnessError as error:
        print(f"Capture harness failed: {error}", file=sys.stderr)
        raise SystemExit(1)
