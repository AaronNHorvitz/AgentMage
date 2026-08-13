#!/usr/bin/env python3
"""Evidence-only authenticated peer for the one-session Docker guard."""

from __future__ import annotations

import hashlib
import json
import os
import socket
import struct
import time
from pathlib import Path


ROOT = Path("/run/agentmage-runtime-peer")


def sha256_file(path: Path) -> bytes:
    digest = hashlib.sha256()
    with path.open("rb") as stream:
        while block := stream.read(1024 * 1024):
            digest.update(block)
    return digest.digest()


def process_start() -> int:
    record = Path(f"/proc/{os.getpid()}/stat").read_text(encoding="ascii")
    return int(record[record.rfind(")") + 2 :].split()[19])


def wait_for(path: Path) -> None:
    deadline = time.monotonic() + 60
    while time.monotonic() < deadline:
        if path.exists():
            return
        time.sleep(0.1)
    raise RuntimeError("peer control timed out")


def main() -> int:
    ready = ROOT / "ready"
    go = ROOT / "go"
    secret_path = ROOT / "secret.bin"
    result_path = ROOT / "result.json"
    ready.write_text("ready\n", encoding="ascii")
    wait_for(go)
    secret = secret_path.read_bytes()
    if len(secret) != 32 or secret == bytes(32):
        return 2
    secret_path.unlink()
    go.unlink()
    peer = {
        "uid": os.getuid(),
        "pid": os.getpid(),
        "start": process_start(),
        "executable": sha256_file(Path(f"/proc/{os.getpid()}/exe")),
        "cgroup": hashlib.sha256(Path(f"/proc/{os.getpid()}/cgroup").read_bytes()).digest(),
    }
    connection = socket.socket(socket.AF_UNIX, socket.SOCK_STREAM)
    connection.settimeout(10)
    connection.connect("/run/agentmage-dmr/guard.sock")
    challenge_frame = connection.recv(34, socket.MSG_WAITALL)
    if len(challenge_frame) != 34 or challenge_frame[:2] != b"\x00\x01":
        return 3
    challenge = challenge_frame[2:]
    digest = hashlib.sha256()
    digest.update(b"agentmage-docker-guard-session-v1\0")
    digest.update(b"\x00\x01")
    digest.update(challenge)
    digest.update(secret)
    digest.update(struct.pack(">I", peer["uid"]))
    digest.update(struct.pack(">i", peer["pid"]))
    digest.update(struct.pack(">Q", peer["start"]))
    digest.update(peer["executable"])
    digest.update(peer["cgroup"])
    connection.sendall(b"\x00\x01" + challenge + digest.digest())
    rejected_request = b"GET / HTTP/1.1\r\nHost: 127.0.0.1:12434\r\n\r\n"
    connection.sendall(struct.pack(">I", len(rejected_request)) + rejected_request)
    try:
        while connection.recv(4096):
            pass
    except ConnectionResetError:
        pass
    connection.close()
    result_path.write_text(
        json.dumps(
            {
                "challenge_received": True,
                "authenticated_frame_delivered": True,
                "inference_request_sent": False,
            },
            sort_keys=True,
        )
        + "\n",
        encoding="ascii",
    )
    time.sleep(900)
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
