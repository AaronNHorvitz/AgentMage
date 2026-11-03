#!/usr/bin/env python3
# SPDX-License-Identifier: Apache-2.0
"""Record a two-turn chat through a dedicated, already running local API.

This is a connection measurement, not a coding evaluation or an installed CLI.
It starts no model, runs no tools, uses no proxy, and never prints model output.
"""

from __future__ import annotations

import argparse
import hashlib
import http.client
import ipaddress
import json
import os
from pathlib import Path
import re
import stat
import time


MAX_RECORD_BYTES = 8 * 1024 * 1024
MAX_LINE_BYTES = 1024 * 1024
PROMPTS = (
    "Name a color in one word.",
    "What color did you just name? Reply with only the same word.",
)


class SmokeError(Exception):
    """Fixed local diagnostics; upstream text and credentials are never echoed."""


def private_text(path: Path, limit: int) -> str:
    fd = os.open(path, os.O_RDONLY | os.O_NOFOLLOW | os.O_NONBLOCK)
    with os.fdopen(fd, "rb") as source:
        info = os.fstat(source.fileno())
        if (not stat.S_ISREG(info.st_mode) or info.st_uid != os.getuid()
                or info.st_nlink != 1 or info.st_mode & 0o077 or info.st_size > limit):
            raise SmokeError("Expected a private owned regular credential file")
        data = source.read(limit + 1)
        if len(data) > limit:
            raise SmokeError("Credential exceeds its byte limit")
        return data.decode("ascii")


def api_address(value: str) -> tuple[str, int]:
    try:
        host, port = value.rsplit(":", 1)
        host = host.removeprefix("[").removesuffix("]")
        address = ipaddress.ip_address(host)
        number = int(port)
        if not address.is_loopback or not 1 <= number <= 65535:
            raise ValueError
        return str(address), number
    except ValueError:
        raise SmokeError("API address must be a numeric loopback address and port") from None


def request(address: tuple[str, int], token: str, method: str, route: str,
            body: object = None) -> tuple[http.client.HTTPConnection, http.client.HTTPResponse]:
    connection = http.client.HTTPConnection(*address, timeout=130)
    payload = b"" if body is None else json.dumps(body).encode("utf-8")
    headers = {"Authorization": "Bearer " + token, "Content-Length": str(len(payload)),
               "Connection": "close"}
    if body is not None:
        headers["Content-Type"] = "application/json"
    try:
        connection.request(method, route, payload, headers)
        response = connection.getresponse()
        if response.status != 200:
            raise SmokeError("Local API refused the measurement request")
        return connection, response
    except BaseException:
        connection.close()
        raise


def json_request(address: tuple[str, int], token: str, method: str, route: str,
                 body: object = None) -> object:
    connection, response = request(address, token, method, route, body)
    try:
        if response.getheader("Content-Type") != "application/json":
            raise SmokeError("Unexpected API response media type")
        data = response.read(MAX_RECORD_BYTES + 1)
        if len(data) > MAX_RECORD_BYTES:
            raise SmokeError("API response exceeds its byte limit")
        return json.loads(data)
    finally:
        connection.close()


def consume_stream(response: http.client.HTTPResponse, target, turn: int,
                   started: float) -> tuple[str, dict]:
    if response.getheader("Content-Type") != "application/x-ndjson":
        raise SmokeError("Unexpected chat media type")
    text = []
    first = None
    terminal = False
    total = 0
    count = 0
    while True:
        if time.monotonic() - started > 130:
            raise SmokeError("Chat measurement deadline exceeded")
        line = response.readline(MAX_LINE_BYTES + 1)
        if not line:
            break
        total += len(line)
        count += 1
        if len(line) > MAX_LINE_BYTES or total > MAX_RECORD_BYTES or count > 16384:
            raise SmokeError("Chat measurement exceeds its record limits")
        if not line.endswith(b"\n") or terminal:
            raise SmokeError("Truncated event or data after chat completion")
        event = json.loads(line)
        elapsed = time.monotonic() - started
        target.write(json.dumps({"turn": turn, "seconds": elapsed, "event": event}) + "\n")
        target.flush()
        if isinstance(event, dict) and event.get("type") == "text" and set(event) == {"type", "text"}:
            if not isinstance(event["text"], str) or not event["text"]:
                raise SmokeError("Invalid text event")
            if first is None:
                first = elapsed
            text.append(event["text"])
        elif event == {"type": "complete", "finish": "stop"}:
            terminal = True
        else:
            raise SmokeError("Chat failed, was truncated, or returned an unsupported event")
    answer = "".join(text)
    if not terminal or not answer.strip():
        raise SmokeError("No complete nonempty model answer")
    return answer, {"first_text_seconds": first, "wall_seconds": time.monotonic() - started,
                    "events": count, "text_bytes": len(answer.encode("utf-8")), "finish": "stop"}


def private_output(path: Path):
    fd = os.open(path, os.O_WRONLY | os.O_CREAT | os.O_EXCL | os.O_NOFOLLOW, 0o600)
    return os.fdopen(fd, "w", encoding="utf-8")


def save(path: Path, value: object) -> None:
    with private_output(path) as target:
        json.dump(value, target, indent=2)
        target.write("\n")


def measure(address: tuple[str, int], auth: Path, endpoint: str, model: str,
            output: Path, key_file: Path | None = None) -> dict:
    # A new output directory prevents overwriting records or following a leaf symlink.
    output.mkdir(mode=0o700)
    report = {"schema_version": 1, "purpose": "connection-only", "result": "FAIL",
              "prompts": list(PROMPTS), "turns": [], "tokens": None,
              "token_note": "The API does not expose upstream token usage.",
              "settings_revoked": False}
    control = client = None
    choice_attempted = False
    try:
        control = private_text(auth / "control.token", 64)
        client = private_text(auth / "client.token", 64)
        if not all(re.fullmatch(r"[0-9a-f]{64}", token) for token in (control, client)):
            raise SmokeError("Invalid API token format")
        key = None if key_file is None else private_text(key_file, 4096)
        status = json_request(address, client, "GET", "/v1/status")
        if status.get("version") != 1 or status.get("configured") or status.get("busy"):
            raise SmokeError("Use a dedicated unconfigured version 1 API instance")
        choice_attempted = True
        json_request(address, control, "PUT", "/v1/settings",
                     {"endpoint": endpoint, "key": key, "approve_conversations": True})
        models = json_request(address, client, "GET", "/v1/models")
        if model not in models.get("models", []):
            raise SmokeError("Requested model is absent from the endpoint listing")
        report["model"] = model
        messages = []
        with private_output(output / "events.ndjson") as events:
            for number, prompt in enumerate(PROMPTS, 1):
                messages.append({"role": "user", "content": prompt})
                started = time.monotonic()
                connection, response = request(address, client, "POST", "/v1/chat",
                                               {"model": model, "messages": messages,
                                                "max_tokens": 1024})
                try:
                    answer, timing = consume_stream(response, events, number, started)
                finally:
                    connection.close()
                messages.append({"role": "assistant", "content": answer})
                report["turns"].append(timing)
        report["result"] = "PASS"
    except (SmokeError, OSError, ValueError, http.client.HTTPException):
        # No upstream body, key, token, path or arbitrary exception text in diagnostics.
        report["error"] = "Connection measurement failed; retained events may be partial."
    finally:
        if choice_attempted:
            try:
                json_request(address, client, "POST", "/v1/cancel")
                until = time.monotonic() + 5
                while json_request(address, client, "GET", "/v1/status").get("busy"):
                    if time.monotonic() >= until:
                        raise SmokeError("Operation did not stop")
                    time.sleep(0.05)
                result = json_request(address, control, "DELETE", "/v1/settings")
                report["settings_revoked"] = result.get("configured") is False
            except (SmokeError, OSError, ValueError, http.client.HTTPException):
                report["settings_revoked"] = False
            if not report["settings_revoked"]:
                report["result"] = "FAIL"
        event_file = output / "events.ndjson"
        if event_file.exists():
            report["events_sha256"] = hashlib.sha256(event_file.read_bytes()).hexdigest()
        save(output / "report.json", report)
    return report


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--api-address", required=True)
    parser.add_argument("--auth-directory", required=True, type=Path)
    parser.add_argument("--endpoint", required=True)
    parser.add_argument("--model", required=True)
    parser.add_argument("--output-directory", required=True, type=Path)
    parser.add_argument("--endpoint-key-file", type=Path)
    parser.add_argument("--approve-conversations", required=True, action="store_true",
                        help="Explicitly authorize the two synthetic prompts to this endpoint")
    args = parser.parse_args()
    os.umask(0o077)
    try:
        report = measure(api_address(args.api_address), args.auth_directory, args.endpoint,
                         args.model, args.output_directory, args.endpoint_key_file)
    except (SmokeError, OSError, ValueError):
        print("CHAT CONNECTION: FAIL (check private inputs and use a new output directory)")
        return 1
    print("CHAT CONNECTION: " + report["result"] + " (connection only; no coding score)")
    return 0 if report["result"] == "PASS" else 1


if __name__ == "__main__":
    raise SystemExit(main())
