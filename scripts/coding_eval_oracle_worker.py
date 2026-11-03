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
        self.candidate_ready = False
        self.function_found = None

    def start(self):
        """Require the worker to import its candidate before running assertions."""
        try:
            raw = self.reader.readline(MAX_MESSAGE + 1)
            self.record(raw)
            frame, _ = decode_message(raw)
            if (self.failed or self.candidate_ready or not isinstance(frame, dict)
                    or frame.keys() != {"ready", "function_found"}
                    or frame["ready"] is not True or type(frame["function_found"]) is not bool):
                raise ProtocolError("oracle.invalid-startup")
            self.candidate_ready = True
            self.function_found = frame["function_found"]
        except (OSError, ValueError, RecursionError):
            self.failed = True
            raise ProtocolError("oracle.startup-failed") from None

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
            if self.result_type == "bool":
                valid = type(result) is bool
            elif self.result_type == "path-result":
                valid = (isinstance(result, dict) and (
                    (result.keys() == {"ambiguous"} and result["ambiguous"] is True)
                    or (result.keys() == {"paths"} and isinstance(result["paths"], list)
                        and all(type(path) is str and path in args[0] for path in result["paths"])
                        and len(result["paths"]) == len(set(result["paths"])))))
            else:
                valid = (isinstance(result, dict) and bool(result)
                         and all(type(value) is bool for value in result.values()))
            if not valid:
                raise ProtocolError("oracle.invalid-result")
            return result
        except (OSError, ValueError, RecursionError):
            self.failed = True
            raise ProtocolError("oracle.exchange-failed") from None


def path_proxy(remote, ambiguity):
    """Rebuild only original Path values and one fixed trusted exception type."""
    def call(sources):
        result = remote({str(path): text for path, text in sources.items()})
        if "ambiguous" in result:
            # Exception names, arguments, classes and tracebacks never cross the
            # pipe. This local class is supplied by the frozen base module.
            raise ambiguity("oracle.remote-ambiguity")
        return frozenset(Path(path) for path in result["paths"])
    return call


def path_result(function, ambiguity, sources):
    try:
        result = function({Path(path): text for path, text in sources.items()})
    except ambiguity:
        return {"ambiguous": True}
    return {"paths": sorted(str(path) for path in result)}


def candidate(config):
    reader, writer = sys.stdin.buffer, sys.stdout.buffer
    # Ordinary candidate prints are diagnostics. Deliberate protocol corruption
    # through original stdout is still untrusted input to the other process.
    with contextlib.redirect_stdout(sys.stderr):
        sys.path.insert(0, "/workspace")
        module = importlib.import_module(config["module"])
        # An absent function is a frozen task outcome; a failed import or
        # namespace launch is not. Send startup only after the import succeeds.
        writer.write(encode_message({"ready": True, "function_found":
                                     callable(getattr(module, config["function"], None))}))
        writer.flush()
        function = getattr(module, config["function"])
        while True:
            # EOF after the oracle exits is normal; truncated messages are not.
            if not reader.peek(1):
                return
            request, _ = read_message(reader)
            if (not isinstance(request, dict) or request.keys() != {"seq", "args"}
                    or type(request["seq"]) is not int or not isinstance(request["args"], list)):
                raise ProtocolError("oracle.invalid-request")
            if config["result_type"] == "path-result":
                result = path_result(function, module.AmbiguousReference, *request["args"])
            else:
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
        result = unittest.TestResult()
        try:
            remote.start()
        except ProtocolError:
            # No assertion runs when the candidate did not reach its function
            # lookup. Still retain a trusted diagnostic record, never a score.
            pass
        else:
            proxy = (path_proxy(remote, module.AmbiguousReference)
                     if config["result_type"] == "path-result" else remote)
            setattr(module, config["function"], proxy)
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
            "candidate_ready": remote.candidate_ready, "function_found": remote.function_found,
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
