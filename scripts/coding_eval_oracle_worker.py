"""Separate JSON function execution from trusted frozen-test assertions.

Original evaluation adapter. Neither role is a product tool or model admission.
Only the oracle has hidden tests and a writable result directory.
"""
from __future__ import annotations

import argparse
import contextlib
import importlib
import importlib.util
import json
import math
from pathlib import Path
import re
import sys
import tempfile
import unittest
from unittest import mock

MAX_MESSAGE = 64 * 1024
MAX_TRANSCRIPT = 2 * 1024 * 1024
MAX_CALLS = 128
POLICY_RESULT_TYPES = frozenset({"argument-result", "verification-result"})


class ProtocolError(ValueError):
    """Candidate data is never interpreted as code or a test result."""


class AdapterPolicyError(ValueError):
    """A candidate observation cannot be represented by this bounded adapter."""


class CandidateExecutionError(RuntimeError):
    """One fixed failure, without candidate exception objects or messages."""


def scalar_value(value):
    return (value is None or type(value) in (str, bool, int)
            or (type(value) is float and math.isfinite(value)))


def observation_value(value, depth=0):
    """Finite built-in JSON data only; frame and transcript limits still apply."""
    if scalar_value(value):
        return True
    if depth >= 16:
        return False
    if type(value) is list:
        return all(observation_value(item, depth + 1) for item in value)
    return (type(value) is dict and all(type(key) is str and observation_value(item, depth + 1)
                                       for key, item in value.items()))


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
        self.policy_failed = False
        self.exception_failed = False
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
            if self.exception_failed:
                raise CandidateExecutionError("oracle.candidate-exception")
            if self.policy_failed:
                raise AdapterPolicyError("oracle.adapter-policy")
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
            if isinstance(result, dict) and "adapter_policy" in result:
                if (result.keys() != {"adapter_policy"} or result["adapter_policy"] is not True
                        or self.result_type not in POLICY_RESULT_TYPES):
                    raise ProtocolError("oracle.undeclared-or-invalid-policy")
                raise AdapterPolicyError("oracle.adapter-policy")
            if isinstance(result, dict) and "candidate_exception" in result:
                if result.keys() != {"candidate_exception"} or result["candidate_exception"] is not True:
                    raise ProtocolError("oracle.invalid-candidate-exception")
                raise CandidateExecutionError("oracle.candidate-exception")
            if self.result_type == "bool":
                # Frozen predicates assert truthiness, not exact Boolean type.
                valid = scalar_value(result)
            elif self.result_type == "path-result":
                valid = (isinstance(result, dict) and (
                    (result.keys() == {"ambiguous"} and result["ambiguous"] is True)
                    or (result.keys() == {"paths"} and isinstance(result["paths"], list)
                        and all(type(path) is str and path in args[0] for path in result["paths"])
                        and len(result["paths"]) == len(set(result["paths"])))))
            elif self.result_type == "verification-result":
                if args[0] == {"operation": "case-roots"}:
                    valid = (isinstance(result, dict) and result.keys() == {"case_roots"}
                             and valid_case_roots(result["case_roots"]))
                else:
                    valid = (isinstance(result, dict) and (
                        (result.keys() == {"missing_case"} and result["missing_case"] is True)
                        or (result.keys() == {"passed", "checks"}
                            and observation_value(result["passed"]) and observation_value(result["checks"]))))
            elif self.result_type == "argument-result":
                valid = valid_argument_result(result, args[0])
            elif self.result_type == "bool-map":
                valid = (isinstance(result, dict)
                         and all(type(key) is str and observation_value(value)
                                 for key, value in result.items()))
            else:
                valid = False
            if not valid:
                raise ProtocolError("oracle.invalid-result")
            return result
        except CandidateExecutionError:
            self.exception_failed = True
            raise
        except AdapterPolicyError:
            self.policy_failed = True
            raise
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


def namespace_values(values):
    """Only scalar parser values and lexical Paths, never arbitrary objects."""
    if (not isinstance(values, dict) or len(values) > 128
            or any(not re.fullmatch(r"[a-z][a-z0-9_]{0,63}", key) for key in values)):
        raise ProtocolError("oracle.invalid-namespace")
    result = {}
    for key, value in values.items():
        if value is None or type(value) in (str, bool, int):
            result[key] = value
        elif isinstance(value, dict) and value.keys() == {"path"} and type(value["path"]) is str:
            result[key] = Path(value["path"])
        else:
            raise ProtocolError("oracle.invalid-namespace")
    return result


def pack_namespace(namespace):
    values = {key: {"path": str(value)} if isinstance(value, Path) else value
              for key, value in vars(namespace).items()}
    namespace_values(values)
    return values


def valid_argument_result(result, request):
    if not isinstance(result, dict):
        return False
    if result.keys() in ({"harness_error"}, {"parser_exit"}, {"missing_function"}):
        return next(iter(result.values())) is True
    if request.get("operation") == "arguments" and result.keys() == {"value"}:
        return observation_value(result["value"])
    if request.get("operation") == "parse" and result.keys() == {"namespace"}:
        namespace_values(result["namespace"])
        return True
    return (request.get("operation") == "arguments" and result.keys() == {"arguments", "container"}
            and type(result["container"]) is str and result["container"] in {"list", "tuple"}
            and isinstance(result["arguments"], list)
            and observation_value(result["arguments"]))


def argument_value(result, module):
    # No supplied exception class, message, traceback or exit status is used.
    if "harness_error" in result:
        raise module.HarnessError("oracle.argument-refused")
    if "parser_exit" in result:
        raise SystemExit(2)
    if "missing_function" in result:
        raise AttributeError("oracle.missing-function")
    return result


def argument_proxies(remote, module):
    class Parser:
        def parse_args(self, arguments):
            result = argument_value(remote({"operation": "parse", "argv": arguments}), module)
            return argparse.Namespace(**namespace_values(result["namespace"]))

    def arguments(namespace):
        result = argument_value(remote({"operation": "arguments",
                                        "namespace": pack_namespace(namespace)}), module)
        if "value" in result:
            return result["value"]
        return (result["arguments"] if result["container"] == "list"
                else tuple(result["arguments"]))
    return Parser, arguments


def argument_result(module, name, request):
    """Run the candidate's parser and helper; all returned observations are data."""
    if not isinstance(request, dict):
        raise ProtocolError("oracle.invalid-argument-request")
    operation = request.get("operation")
    try:
        if (operation == "parse" and request.keys() == {"operation", "argv"}
                and isinstance(request["argv"], list)
                and all(type(value) is str for value in request["argv"])):
            parsed = module.parser().parse_args(request["argv"])
            try:
                return {"namespace": pack_namespace(parsed)}
            except (ProtocolError, TypeError):
                return {"adapter_policy": True}
        if operation == "arguments" and request.keys() == {"operation", "namespace"}:
            values = namespace_values(request["namespace"])
            function = getattr(module, name, None)
            if not callable(function):
                return {"missing_function": True}
            result = function(argparse.Namespace(**values))
            if type(result) in (list, tuple) and observation_value(list(result)):
                return {"arguments": list(result), "container": "list" if type(result) is list else "tuple"}
            if observation_value(result):
                return {"value": result}
            return {"adapter_policy": True}
    except module.HarnessError:
        return {"harness_error": True}
    except SystemExit as error:
        if type(error.code) is int and error.code == 2:
            return {"parser_exit": True}
        return {"adapter_policy": True}
    raise ProtocolError("oracle.invalid-argument-request")


def verification_proxy(remote, module):
    """Snapshot trusted fixture inputs, never test code, mocks or shared paths."""
    def call(case, base, log_dir, exit_code):
        workspace = module.coding_harness.paths(base)[2]
        files = {name: (log_dir / name).read_text()
                 for name in ("result.json", "stdout.jsonl", "stderr.log")}
        calc = workspace / "src/calc.py"
        if calc.exists():
            files["calc.py"] = calc.read_text()
        # Only the multi-file frozen method stubs a validation subprocess. The
        # other selected methods have no command, and never start one here.
        validation = None
        if case == "multi-file":
            validation = module.subprocess.run(
                [sys.executable, "tests/run_validation.py"], cwd=workspace, check=False,
                stdout=module.subprocess.PIPE, stderr=module.subprocess.PIPE, text=True,
            ).returncode
        result = remote({"case": case, "exit_code": exit_code, "files": files,
                         "rows": module.rows(log_dir), "status": module.git_status(workspace),
                         "validation_returncode": validation})
        if "missing_case" in result:
            raise KeyError("oracle.missing-case")
        return result
    return call


def verification_result(function, module, request):
    """Rebuild finite fixture text in the candidate's own temporary filesystem."""
    if (not isinstance(request, dict) or request.keys() != {
            "case", "exit_code", "files", "rows", "status", "validation_returncode"}
            or type(request["case"]) is not str
            or request["case"] not in {"multi-file", "read-arguments-correction", "read-arguments-denied",
                                       "native-command-repair"}
            or type(request["exit_code"]) is not int or not isinstance(request["files"], dict)
            or set(request["files"]) not in (
                {"result.json", "stdout.jsonl", "stderr.log"},
                {"result.json", "stdout.jsonl", "stderr.log", "calc.py"})
            or any(type(text) is not str for text in request["files"].values())
            or not isinstance(request["rows"], list)
            or not isinstance(request["status"], list)
            or any(type(row) is not str for row in request["status"])
            or (request["validation_returncode"] is not None
                and type(request["validation_returncode"]) is not int)):
        raise ProtocolError("oracle.invalid-verification-input")
    with tempfile.TemporaryDirectory(prefix="oracle-fixture-") as directory:
        root = Path(directory)
        base, logs = root / "fixture", root / "logs"
        logs.mkdir()
        for name, text in request["files"].items():
            path = base / "disposable/worktree/src/calc.py" if name == "calc.py" else logs / name
            path.parent.mkdir(parents=True, exist_ok=True)
            path.write_text(text)
        completed = module.subprocess.CompletedProcess([], request["validation_returncode"])
        with (mock.patch.object(module, "rows", return_value=request["rows"]),
              mock.patch.object(module, "git_status", return_value=request["status"]),
              mock.patch.object(module.subprocess, "run", return_value=completed)):
            try:
                result = function(request["case"], base, logs, request["exit_code"])
            except KeyError:
                # Only a literal tag crosses the boundary, never exception data.
                return {"missing_case": True}
        passed, checks = result["passed"], result["checks"]
        if not observation_value(passed) or not observation_value(checks):
            return {"adapter_policy": True}
        return {"passed": passed, "checks": checks}


def valid_case_roots(roots):
    """Bounded strings for the frozen uniqueness assertion; never file access."""
    return (type(roots) is dict and len(roots) <= 128
            and all(type(key) is str and len(key) <= 256
                    and type(value) is str and len(value) <= 256
                    for key, value in roots.items()))


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
        while True:
            # EOF after the oracle exits is normal; truncated messages are not.
            if not reader.peek(1):
                return
            request, _ = read_message(reader)
            if (not isinstance(request, dict) or request.keys() != {"seq", "args"}
                    or type(request["seq"]) is not int or not isinstance(request["args"], list)):
                raise ProtocolError("oracle.invalid-request")
            try:
                function = (None if config["result_type"] == "argument-result"
                            else getattr(module, config["function"]))
                if config["result_type"] == "path-result":
                    result = path_result(function, module.AmbiguousReference, *request["args"])
                elif config["result_type"] == "verification-result":
                    if config.get("observe_case_roots") and request["args"] == [{"operation": "case-roots"}]:
                        roots = getattr(module, "CASE_ROOTS", None)
                        result = {"case_roots": roots} if valid_case_roots(roots) else {"adapter_policy": True}
                    else:
                        result = verification_result(function, module, *request["args"])
                elif config["result_type"] == "argument-result":
                    result = argument_result(module, config["function"], *request["args"])
                else:
                    result = function(*request["args"])
            except BaseException:
                # This tag carries no candidate-selected class, status or text.
                # A candidate can forge it only to fail its own attempt.
                result = {"candidate_exception": True}
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
            if config.get("observe_case_roots"):
                module.CASE_ROOTS = remote({"operation": "case-roots"})["case_roots"]
        except (ProtocolError, AdapterPolicyError, CandidateExecutionError):
            # Failed startup or a refused prerequisite observation runs no
            # assertions. Retain the diagnostic and failed attempt, never a score.
            pass
        else:
            if config["result_type"] == "path-result":
                proxy = path_proxy(remote, module.AmbiguousReference)
            elif config["result_type"] == "verification-result":
                proxy = verification_proxy(remote, module)
            elif config["result_type"] == "argument-result":
                parser, proxy = argument_proxies(remote, module)
                module.parser = parser
            else:
                proxy = remote
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
                  and not remote.failed and not remote.policy_failed and not remote.exception_failed
                  and remote.calls > 0)
        Path("/output/tests.json").write_text(json.dumps({
            "tests": counts, "calls": remote.calls, "protocol_failed": remote.failed,
            "adapter_policy_failed": remote.policy_failed,
            "candidate_exception_failed": remote.exception_failed,
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
