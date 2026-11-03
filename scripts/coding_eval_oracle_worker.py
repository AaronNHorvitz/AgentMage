"""Separate JSON function execution from trusted frozen-test assertions.

Original evaluation adapter. Neither role is a product tool or model admission.
Only the oracle has hidden tests and a writable result directory.
"""
from __future__ import annotations

import contextlib
import importlib
import importlib.util
import json
from pathlib import Path
import sys
import unittest

MAX_MESSAGE = 64 * 1024
MAX_TRANSCRIPT = 2 * 1024 * 1024
MAX_CALLS = 128


class ProtocolError(ValueError):
    """Candidate data is never interpreted as code or a test result."""


def unique_object(pairs):
    result = {}
    for key, value in pairs:
        if key in result:
            raise ProtocolError("oracle.duplicate-key")
        result[key] = value
    return result


def reject_constant(value):
    raise ProtocolError("oracle.nonfinite-number")


def decode_message(line):
    if not line or len(line) > MAX_MESSAGE or not line.endswith(b"\n"):
        raise ProtocolError("oracle.invalid-frame")
    try:
        value = json.loads(line.decode("utf-8"), object_pairs_hook=unique_object,
                           parse_constant=reject_constant)
    except (ValueError, RecursionError):
        raise ProtocolError("oracle.invalid-json") from None
    return value, line


def read_message(stream):
    return decode_message(stream.readline(MAX_MESSAGE + 1))


def encode_message(value):
    try:
        data = (json.dumps(value, allow_nan=False, separators=(",", ":")) + "\n").encode()
    except (ValueError, TypeError, RecursionError):
        raise ProtocolError("oracle.invalid-json") from None
    if len(data) > MAX_MESSAGE:
        raise ProtocolError("oracle.invalid-frame")
    return data


class RemoteFunction:
    def __init__(self, reader, writer, transcript, result_type):
        self.reader, self.writer, self.transcript = reader, writer, transcript
        self.result_type = result_type
        self.calls = 0
        self.transcript_bytes = 0
        self.failed = False

    def record(self, data):
        remaining = MAX_TRANSCRIPT - self.transcript_bytes
        self.transcript.write(data[:remaining])
        self.transcript.flush()
        self.transcript_bytes += min(len(data), remaining)
        if len(data) > remaining:
            raise ProtocolError("oracle.transcript-limit")

    def __call__(self, *args):
        try:
            self.calls += 1
            if self.failed or self.calls > MAX_CALLS:
                raise ProtocolError("oracle.call-limit")
            request = encode_message({"seq": self.calls, "args": args})
            self.record(request)
            self.writer.write(request)
            self.writer.flush()
            raw = self.reader.readline(MAX_MESSAGE + 1)
            self.record(raw)
            reply, _ = decode_message(raw)
            if (not isinstance(reply, dict) or reply.keys() != {"seq", "result"}
                    or type(reply["seq"]) is not int or reply["seq"] != self.calls):
                raise ProtocolError("oracle.invalid-reply")
            result = reply["result"]
            valid = (type(result) is bool if self.result_type == "bool" else
                     isinstance(result, dict) and bool(result)
                     and all(type(value) is bool for value in result.values()))
            if not valid:
                raise ProtocolError("oracle.invalid-result")
            return result
        except (OSError, ValueError, RecursionError):
            self.failed = True
            raise ProtocolError("oracle.exchange-failed") from None


def candidate(config):
    reader, writer = sys.stdin.buffer, sys.stdout.buffer
    # Ordinary candidate prints are diagnostics. Deliberate protocol corruption
    # through original stdout is still untrusted input to the other process.
    with contextlib.redirect_stdout(sys.stderr):
        sys.path.insert(0, "/workspace")
        module = importlib.import_module(config["module"])
        function = getattr(module, config["function"])
        while True:
            # EOF after the oracle exits is normal; truncated messages are not.
            if not reader.peek(1):
                return
            request, _ = read_message(reader)
            if (not isinstance(request, dict) or request.keys() != {"seq", "args"}
                    or type(request["seq"]) is not int or not isinstance(request["args"], list)):
                raise ProtocolError("oracle.invalid-request")
            result = function(*request["args"])
            writer.write(encode_message({"seq": request["seq"], "result": result}))
            writer.flush()


def oracle(config):
    # This workspace contains the frozen base, never candidate source. Helpers,
    # fixture builders, imports and assertions all execute on the trusted side.
    sys.path.insert(0, "/workspace")
    module = importlib.import_module(config["module"])
    with Path("/output/wire.log").open("xb") as transcript:
        remote = RemoteFunction(sys.stdin.buffer, sys.stdout.buffer, transcript, config["result_type"])
        setattr(module, config["function"], remote)
        spec = importlib.util.spec_from_file_location("hidden_eval", "/adapter/test.py")
        hidden = importlib.util.module_from_spec(spec)
        spec.loader.exec_module(hidden)
        # Select complete original methods; hidden source is never rewritten.
        suite = unittest.TestSuite(unittest.defaultTestLoader.loadTestsFromName(name, hidden)
                                   for name in config["test_names"])
        result = unittest.TextTestRunner(stream=sys.stderr, verbosity=2).run(suite)
        counts = {"tests": result.testsRun, "failures": len(result.failures),
                  "errors": len(result.errors), "skipped": len(result.skipped),
                  "expected_failures": len(result.expectedFailures),
                  "unexpected_successes": len(result.unexpectedSuccesses)}
        passed = (result.wasSuccessful() and result.testsRun == config["expected_tests"]
                  and not result.skipped and not result.expectedFailures
                  and not remote.failed and remote.calls > 0)
        Path("/output/tests.json").write_text(json.dumps({
            "tests": counts, "calls": remote.calls, "protocol_failed": remote.failed,
        }, sort_keys=True) + "\n")
    return 0 if passed else 1


def main():
    config = json.loads(Path("/adapter/config.json").read_text())
    if sys.argv[1:] == ["candidate"]:
        candidate(config)
        return 0
    if sys.argv[1:] == ["oracle"]:
        return oracle(config)
    return 2


if __name__ == "__main__":
    raise SystemExit(main())
