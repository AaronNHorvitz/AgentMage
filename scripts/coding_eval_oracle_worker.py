"""Separate JSON function execution from trusted frozen-test assertions.

Original evaluation adapter. Neither role is a product tool or model admission.
Only the oracle has hidden tests and a writable result directory.
"""
from __future__ import annotations

import argparse
import contextlib
import importlib
import importlib.util
import io
import json
import math
import os
from pathlib import Path
import re
import signal
import subprocess
import stat
import sys
import tempfile
from types import SimpleNamespace
import unittest
from unittest import mock

MAX_MESSAGE = 64 * 1024
MAX_TRANSCRIPT = 2 * 1024 * 1024
MAX_CALLS = 128
POLICY_RESULT_TYPES = frozenset({"argument-result", "verification-result", "capture-result",
                                "bool", "bool-map", "path-result", "mode-result", "command-result",
                                "diagnostic-result", "resource-result", "deadline-result", "job-result", "usage-result"})


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
    except (ValueError, TypeError, RecursionError, MemoryError):
        raise ProtocolError("oracle.invalid-json") from None
    if len(data) > MAX_MESSAGE:
        raise ProtocolError("oracle.invalid-frame")
    return data


def predicate_result(value, result_type):
    """Keep candidate keys in data, outside the adapter's fixed tag namespace."""
    valid = (observation_value(value) if result_type == "bool" else
             result_type == "bool-map" and type(value) is dict
             and all(type(key) is str and observation_value(item) for key, item in value.items()))
    return {"value": value} if valid else {"adapter_policy": True}


def response_message(sequence, result):
    """A bounded adapter refusal is distinct from a malformed wire frame."""
    try:
        return encode_message({"seq": sequence, "result": result})
    except ProtocolError:
        return encode_message({"seq": sequence, "result": {"adapter_policy": True}})


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

    def finish(self):
        """End requests, then require EOF with no unsolicited candidate bytes.

        A closed end marker lets the candidate exit even if its namespace init
        still holds the request pipe. The host deadline bounds held-open pipes.
        """
        try:
            end = encode_message({"done": True})
            self.record(end)
            try:
                self.writer.write(end)
                self.writer.flush()
            except BrokenPipeError:
                # A worker may have exited after its last reply. Still require
                # clean EOF; the host separately checks its actual exit code.
                pass
            try:
                self.writer.close()
            except BrokenPipeError:
                pass
            trailing = self.reader.read(1)
            if trailing:
                self.record(trailing)
                raise ProtocolError("oracle.trailing-data")
        except (OSError, ValueError):
            self.failed = True
            raise ProtocolError("oracle.finish-failed") from None

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
                valid = (isinstance(result, dict) and result.keys() == {"value"}
                         and observation_value(result["value"]))
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
            elif self.result_type == "capture-result":
                valid = (type(result) is dict and result.keys() == {"value", "container"}
                         and type(result["container"]) is str
                         and result["container"] in {"value", "tuple"}
                         and observation_value(result["value"])
                         and (result["container"] != "tuple" or type(result["value"]) is list))
            elif self.result_type == "mode-result":
                valid = (type(result) is dict and result.keys() == {"observed"}
                         and result["observed"] is True)
            elif self.result_type == "command-result":
                unpack_calls(result)
                valid = True
            elif self.result_type == "diagnostic-result":
                valid = valid_diagnostic_result(result)
            elif self.result_type == "usage-result":
                unpack_usage_result(result)
                valid = True
            elif self.result_type == "resource-result":
                valid = type(result) is dict and result.keys() == {"harness_error"} and result["harness_error"] is True
                if type(result) is dict and result.keys() == {"value"}:
                    unpack_call_value(result["value"])
                    valid = True
            elif self.result_type == "deadline-result":
                if args[0] == {"operation": "commands"}:
                    valid = (type(result) is dict and result.keys() == {"commands"}
                             and valid_collector_commands(unpack_call_value(result["commands"])))
                else:
                    valid = valid_deadline_result(result)
            elif self.result_type == "job-result":
                if args[0].get("operation") == "parse":
                    if type(result) is dict and result.keys() == {"job_namespace"}:
                        values = unpack_call_value(result["job_namespace"])
                        valid = (type(values) is dict and len(values) <= 128
                                 and all(re.fullmatch(r"[a-z][a-z0-9_]{0,63}", key) for key in values))
                    else:
                        valid = valid_argument_result(result, args[0]) and "namespace" not in result
                else:
                    valid = valid_job_result(result)
            elif self.result_type == "bool-map":
                valid = (isinstance(result, dict) and result.keys() == {"value"}
                         and type(result["value"]) is dict
                         and all(type(key) is str and observation_value(value)
                                 for key, value in result["value"].items()))
            else:
                valid = False
            if not valid:
                raise ProtocolError("oracle.invalid-result")
            return result["value"] if self.result_type in {"bool", "bool-map"} else result
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


def valid_job_result(result):
    if (type(result) is not dict or result.keys() != {"value", "calls", "exception"}
            or type(result["exception"]) is not str
            or result["exception"] not in {"none", "harness", "key", "argument-type"}):
        return False
    unpack_calls({"calls": result["calls"]})
    value = unpack_call_value(result["value"])
    return result["exception"] == "none" or value is None


def job_proxies(remote, module):
    class Parser:
        def parse_args(self, arguments):
            result = argument_value(remote({"operation": "parse", "argv": arguments}), module)
            return argparse.Namespace(**unpack_call_value(result["job_namespace"]))

    def call(operation, value, **options):
        result = remote({"operation": operation, "value": pack_call_value(value), **options})
        # Validate the entire response before replay. Only the original test's
        # mock is a replay target; no candidate data can invoke a live signal.
        calls = unpack_calls({"calls": result["calls"]})
        if operation in {"stop", "control"} and options["record"]:
            if not isinstance(module.signal_running_cli, mock.Mock):
                remote.failed = True
                raise ProtocolError("oracle.unsupported-signal-fixture")
            for args, kwargs in calls:
                module.signal_running_cli(*args, **kwargs)
        elif calls and operation not in {"stop", "control"}:
            remote.failed = True
            raise ProtocolError("oracle.unexpected-job-calls")
        if result["exception"] == "harness":
            raise module.HarnessError("oracle.job-refused")
        if result["exception"] == "key":
            raise KeyError("oracle.job-refused")
        if result["exception"] == "argument-type":
            raise argparse.ArgumentTypeError("oracle.job-refused")
        return unpack_call_value(result["value"])

    def stop(base):
        return call("stop", base, record=isinstance(module.signal_running_cli, mock.Mock))

    def control(base, command):
        return call("control", (base, command), record=isinstance(module.signal_running_cli, mock.Mock))

    def export(value):
        return call("export", value)

    def bundle(path):
        return call("bundle", path, directory=path.is_dir())
    return Parser, stop, control, export, bundle


def job_result(module, request, signal_state=None):
    """Refuse observations lost through signal mocks retained across requests."""
    if signal_state is None:
        signal_state = {"stale": False}
    if signal_state["stale"]:
        return {"adapter_policy": True}
    result = job_observation(module, request, signal_state)
    # Check every operation, including parser and expected-refusal returns.
    # Candidate code may catch an exception or clear its mock's call history;
    # neither clears this worker-owned state.
    return {"adapter_policy": True} if signal_state["stale"] else result


def job_observation(module, request, signal_state):
    """Finite mocked dispatch and candidate-owned empty-directory refusal."""
    if type(request) is not dict or type(request.get("operation")) is not str:
        raise ProtocolError("oracle.invalid-job-request")
    operation = request["operation"]
    if operation == "parse":
        if (request.keys() != {"operation", "argv"} or type(request["argv"]) is not list
                or any(type(value) is not str for value in request["argv"])):
            raise ProtocolError("oracle.invalid-job-request")
        try:
            parsed = module.parser().parse_args(request["argv"])
            return {"job_namespace": pack_call_value(vars(parsed))}
        except AdapterPolicyError:
            return {"adapter_policy": True}
        except module.HarnessError:
            return {"harness_error": True}
        except SystemExit as error:
            return {"parser_exit": True} if type(error.code) is int and error.code == 2 else {"adapter_policy": True}
    fields = {"operation", "value"}
    if operation in {"stop", "control"}:
        fields.add("record")
    elif operation == "bundle":
        fields.add("directory")
    elif operation != "export":
        raise ProtocolError("oracle.invalid-job-request")
    if request.keys() != fields:
        raise ProtocolError("oracle.invalid-job-request")
    value = unpack_call_value(request["value"])
    path_type = type(Path())
    if operation in {"stop", "control"}:
        if (type(request["record"]) is not bool
                or (operation == "stop" and type(value) is not path_type)
                or (operation == "control" and not (
                    type(value) is tuple and len(value) == 2
                    and type(value[0]) is path_type and type(value[1]) is str))):
            raise ProtocolError("oracle.invalid-job-request")
    elif operation == "bundle":
        if type(request["directory"]) is not bool or type(value) is not path_type:
            raise ProtocolError("oracle.invalid-job-request")
        if request["directory"]:
            # Preserve the fixture's exact lexical path in the private tmpfs.
            # The frozen method supplies one ordinary immediate /tmp child.
            if re.fullmatch(r"/tmp/[A-Za-z0-9_-]+", str(value)) is None:
                return {"adapter_policy": True}
            value.mkdir(mode=0o700, exist_ok=True)
        elif value.is_dir():
            return {"adapter_policy": True}
    elif type(value) is not str:
        raise ProtocolError("oracle.invalid-job-request")
    recording = operation in {"stop", "control"} and request["record"]
    if operation in {"stop", "control"} and not recording:
        base = value if operation == "stop" else value[0]
        # Recreate only the frozen empty-directory fixture in private tmpfs.
        # Leave the candidate's helper intact: its refusal is under test.
        # Other lexical paths (the unknown-control case) stay unmodified.
        if re.fullmatch(r"/tmp/[A-Za-z0-9_-]+", str(base)) is not None:
            base.mkdir(mode=0o700, exist_ok=True)
    observed = mock.Mock(return_value=None)
    active = True

    def record_signal(base, number):
        if not active:
            signal_state["stale"] = True
            return
        # Match the original side-effect signature. A swallowed wrong-arity
        # call never reached its body and contributes no signal observation.
        observed(base, number)

    exception, returned = "none", None
    replacement = (mock.patch.object(module, "signal_running_cli", side_effect=record_signal)
                   if recording else contextlib.nullcontext())
    try:
        with replacement:
            try:
                if operation == "control":
                    returned = module.control(*value)
                else:
                    name = {"stop": "stop", "export": "export_selection", "bundle": "support_bundle_directory"}[operation]
                    returned = getattr(module, name)(value)
            except module.HarnessError:
                exception = "harness"
            except KeyError:
                exception = "key"
            except argparse.ArgumentTypeError:
                exception = "argument-type"
    finally:
        active = False
    try:
        calls = []
        for call in observed.call_args_list:
            # Frozen assertions compare signal values, not enum identity. Only
            # this finite enum is normalized; arbitrary subclasses are refused.
            args = tuple(int(v) if type(v) is signal.Signals else v for v in call.args)
            kwargs = {k: int(v) if type(v) is signal.Signals else v for k, v in call.kwargs.items()}
            calls.append({"args": pack_call_value(args), "kwargs": pack_call_value(kwargs)})
            if len(calls) > 128:
                raise AdapterPolicyError("oracle.job-call-limit")
        # The frozen method never asserts wrapper return values.
        value = None if operation in {"stop", "control"} else returned
        return {"value": pack_call_value(value), "calls": calls, "exception": exception}
    except AdapterPolicyError:
        return {"adapter_policy": True}


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


def capture_proxies(remote, module):
    """Transfer only fixture text and closed command lists, never mock objects."""
    def unpack(result):
        return tuple(result["value"]) if result["container"] == "tuple" else result["value"]

    def validate_raw(text):
        return unpack(remote({"operation": "validate", "text": text}))

    def capture():
        fixture = module.subprocess.run.return_value
        if (type(fixture) is subprocess.CompletedProcess
                and type(fixture.args) is list and fixture.args == [] and fixture.stderr is None):
            stub_kind = "completed-process"
        elif isinstance(fixture, mock.MagicMock):
            stub_kind = "mock"
        else:
            remote.failed = True
            raise ProtocolError("oracle.unsupported-capture-stub")
        return unpack(remote({"operation": "capture", "stdout": fixture.stdout,
                              "returncode": fixture.returncode,
                              "stub_kind": stub_kind,
                              "commands": [list(command) for command in module.COMMANDS]}))
    return validate_raw, capture


def capture_result(module, request):
    """Recreate the original finite stub in the candidate; no command runs."""
    if type(request) is not dict:
        raise ProtocolError("oracle.invalid-capture-request")
    if (request.keys() == {"operation", "text"} and request["operation"] == "validate"
            and type(request["text"]) is str):
        value = module.validate_raw(request["text"])
    elif (request.keys() == {"operation", "stdout", "returncode", "commands", "stub_kind"}
          and request["operation"] == "capture" and type(request["stdout"]) is str
          and type(request["returncode"]) is int and type(request["commands"]) is list
          and type(request["stub_kind"]) is str and request["stub_kind"] in {"completed-process", "mock"}
          and len(request["commands"]) <= 128
          and all(type(command) is list and 0 < len(command) <= 128
                  and all(type(arg) is str for arg in command) for command in request["commands"])):
        with (mock.patch.object(module, "COMMANDS", tuple(tuple(c) for c in request["commands"])),
              mock.patch.object(module.subprocess, "run") as run):
            if request["stub_kind"] == "completed-process":
                run.return_value = subprocess.CompletedProcess([], request["returncode"], stdout=request["stdout"])
            else:
                run.return_value.stdout = request["stdout"]
                run.return_value.returncode = request["returncode"]
            value = module.capture()
    else:
        raise ProtocolError("oracle.invalid-capture-request")
    if type(value) is tuple and observation_value(list(value)):
        return {"value": list(value), "container": "tuple"}
    if observation_value(value):
        return {"value": value, "container": "value"}
    return {"adapter_policy": True}


def pack_call_value(value, depth=0):
    """A closed data codec preserves list/tuple/Path equality in mock calls."""
    if depth >= 16:
        raise AdapterPolicyError("oracle.call-depth")
    if scalar_value(value):
        return {"kind": "scalar", "value": value}
    if type(value) is type(Path()):
        return {"kind": "path", "value": str(value)}
    if type(value) in (list, tuple):
        return {"kind": "list" if type(value) is list else "tuple",
                "value": [pack_call_value(item, depth + 1) for item in value]}
    if type(value) is dict and all(type(key) is str for key in value):
        return {"kind": "dict", "value": {key: pack_call_value(item, depth + 1)
                                          for key, item in value.items()}}
    raise AdapterPolicyError("oracle.call-value")


def unpack_call_value(value, depth=0):
    if (depth >= 16 or type(value) is not dict or value.keys() != {"kind", "value"}
            or type(value["kind"]) is not str):
        raise ProtocolError("oracle.invalid-call-value")
    kind, data = value["kind"], value["value"]
    if kind == "scalar" and scalar_value(data):
        return data
    if kind == "path" and type(data) is str:
        return Path(data)
    if kind in {"list", "tuple"} and type(data) is list:
        items = [unpack_call_value(item, depth + 1) for item in data]
        return items if kind == "list" else tuple(items)
    if kind == "dict" and type(data) is dict and all(type(key) is str for key in data):
        return {key: unpack_call_value(item, depth + 1) for key, item in data.items()}
    raise ProtocolError("oracle.invalid-call-value")


def unpack_calls(result):
    if (type(result) is not dict or result.keys() != {"calls"}
            or type(result["calls"]) is not list or len(result["calls"]) > 128):
        raise ProtocolError("oracle.invalid-calls")
    calls = []
    for call in result["calls"]:
        if type(call) is not dict or call.keys() != {"args", "kwargs"}:
            raise ProtocolError("oracle.invalid-call")
        args, kwargs = unpack_call_value(call["args"]), unpack_call_value(call["kwargs"])
        if type(args) is not tuple or type(kwargs) is not dict:
            raise ProtocolError("oracle.invalid-call-arguments")
        calls.append((args, kwargs))
    return calls


def pack_usage_value(value, depth=0):
    """The usage transport fixture also needs inert bytes and frozenset values."""
    if depth >= 16:
        raise AdapterPolicyError("oracle.usage-depth")
    if type(value) is bytes:
        return {"kind": "bytes", "value": value.hex()}
    if type(value) is frozenset and all(type(item) is int for item in value):
        return {"kind": "integer-frozenset", "value": sorted(value)}
    if type(value) in (list, tuple):
        return {"kind": "list" if type(value) is list else "tuple",
                "value": [pack_usage_value(item, depth + 1) for item in value]}
    if type(value) is dict and all(type(key) is str for key in value):
        return {"kind": "dict", "value": {key: pack_usage_value(item, depth + 1)
                                          for key, item in value.items()}}
    return pack_call_value(value, depth)


def unpack_usage_value(value, depth=0):
    if (depth >= 16 or type(value) is not dict or value.keys() != {"kind", "value"}
            or type(value["kind"]) is not str):
        raise ProtocolError("oracle.invalid-usage-value")
    kind, data = value["kind"], value["value"]
    if kind == "bytes" and type(data) is str and re.fullmatch(r"(?:[0-9a-f]{2})*", data):
        return bytes.fromhex(data)
    if (kind == "integer-frozenset" and type(data) is list
            and all(type(item) is int for item in data) and data == sorted(set(data))):
        return frozenset(data)
    if kind in {"list", "tuple"} and type(data) is list:
        items = [unpack_usage_value(item, depth + 1) for item in data]
        return items if kind == "list" else tuple(items)
    if kind == "dict" and type(data) is dict and all(type(key) is str for key in data):
        return {key: unpack_usage_value(item, depth + 1) for key, item in data.items()}
    return unpack_call_value(value, depth)


def unpack_usage_result(result):
    if (type(result) is not dict or result.keys() != {"value", "calls"}
            or type(result["calls"]) is not list or len(result["calls"]) > 128):
        raise ProtocolError("oracle.invalid-usage-result")
    value = unpack_usage_value(result["value"])
    calls = []
    for call in result["calls"]:
        if type(call) is not dict or call.keys() != {"args", "kwargs"}:
            raise ProtocolError("oracle.invalid-usage-call")
        args, kwargs = unpack_usage_value(call["args"]), unpack_usage_value(call["kwargs"])
        if type(args) is not tuple or type(kwargs) is not dict:
            raise ProtocolError("oracle.invalid-usage-arguments")
        calls.append((args, kwargs))
    return value, calls


def usage_proxy(remote, module):
    """Keep message-root identity; replay only into an exclusively owned inert mock."""
    owners = []

    def refuse():
        remote.policy_failed = True
        raise AdapterPolicyError("oracle.usage-replay-policy")

    class Adapter:
        def __init__(self, *args, **kwargs):
            self.identifier = remote.calls + 1
            self.last_fixture = object()
            self.transport = module.unix_http_request
            self.message_roots = []
            self.invoke("create", args, kwargs)

        def replay_target(self):
            if remote.policy_failed:
                refuse()
            transport = module.unix_http_request
            if not isinstance(transport, mock.MagicMock):
                remote.failed = True
                raise ProtocolError("oracle.usage-transport-not-mocked")
            if (transport is not self.transport or transport.side_effect is not None
                    or transport._mock_wraps is not None):
                refuse()
            for owned_transport, owner in owners:
                if transport is owned_transport:
                    if owner is not self:
                        refuse()
                    return transport
            # Resetting an unowned mock would erase observations made elsewhere.
            if transport.call_args_list or len(owners) >= MAX_CALLS:
                refuse()
            owners.append((transport, self))
            return transport

        def invoke(self, operation, args, kwargs):
            transport = self.replay_target()
            fixture = transport.return_value
            if fixture is self.last_fixture:
                stub = {"unchanged": True}
            elif isinstance(fixture, mock.MagicMock):
                stub = {"default": True}
            elif (type(fixture) is tuple and len(fixture) == 3
                  and type(fixture[0]) is int and type(fixture[1]) is bytes
                  and type(fixture[2]) in (int, float) and math.isfinite(fixture[2])):
                stub = {"fixture": pack_usage_value(fixture)}
            else:
                remote.failed = True
                raise ProtocolError("oracle.unsupported-usage-fixture")
            self.last_fixture = fixture
            handle = None
            if operation == "tokenize":
                messages = args[0] if args else kwargs.get("messages")
                if type(messages) is list:
                    for index, root in enumerate(self.message_roots, 1):
                        if root is messages:
                            handle = index
                            break
                    if handle is None:
                        if len(self.message_roots) >= MAX_CALLS:
                            refuse()
                        self.message_roots.append(messages)
                        handle = len(self.message_roots)
            value, calls = unpack_usage_result(remote({
                "operation": operation, "instance": self.identifier, "stub": stub,
                "message_handle": handle,
                "args": pack_usage_value(args), "kwargs": pack_usage_value(kwargs)}))
            # Check again before invoking the mock; even a trusted fixture may
            # have changed configuration while the response was in flight.
            self.replay_target()
            transport.reset_mock()
            for call_args, call_kwargs in calls:
                transport(*call_args, **call_kwargs)
            return value

        def tokenize_messages(self, *args, **kwargs):
            return self.invoke("tokenize", args, kwargs)
    return Adapter


def usage_result(module, instances, request):
    if (type(request) is not dict
            or request.keys() != {"operation", "instance", "stub", "args", "kwargs", "message_handle"}
            or type(request["operation"]) is not str or request["operation"] not in {"create", "tokenize"}
            or type(request["instance"]) is not int or not 1 <= request["instance"] <= MAX_CALLS
            or type(request["stub"]) is not dict):
        raise ProtocolError("oracle.invalid-usage-request")
    args, kwargs = unpack_usage_value(request["args"]), unpack_usage_value(request["kwargs"])
    if type(args) is not tuple or type(kwargs) is not dict:
        raise ProtocolError("oracle.invalid-usage-arguments")
    handle = request["message_handle"]
    messages = args[0] if args else kwargs.get("messages")
    if request["operation"] == "tokenize" and type(messages) is list:
        if type(handle) is not int or not 1 <= handle <= MAX_CALLS:
            raise ProtocolError("oracle.invalid-message-handle")
    elif handle is not None:
        raise ProtocolError("oracle.invalid-message-handle")
    stub = request["stub"]
    if (stub.keys() in ({"default"}, {"unchanged"})
            and next(iter(stub.values())) is True):
        fixture = None
    elif stub.keys() == {"fixture"}:
        fixture = unpack_usage_value(stub["fixture"])
        if not (type(fixture) is tuple and len(fixture) == 3
                and type(fixture[0]) is int and type(fixture[1]) is bytes
                and type(fixture[2]) in (int, float) and math.isfinite(fixture[2])):
            raise ProtocolError("oracle.invalid-usage-stub")
    else:
        raise ProtocolError("oracle.invalid-usage-stub")
    identifier = request["instance"]
    if request["operation"] == "create":
        if identifier in instances or len(instances) >= MAX_CALLS:
            raise ProtocolError("oracle.invalid-usage-instance")
        if stub.keys() == {"unchanged"}:
            raise ProtocolError("oracle.invalid-usage-stub")
        transport = mock.MagicMock()
        roots = {}
    else:
        if identifier not in instances:
            raise ProtocolError("oracle.invalid-usage-instance")
        _, transport, roots = instances[identifier]
        if handle is not None:
            if handle not in roots:
                roots[handle] = messages
            else:
                # Only the message-list root has stable identity. Nested
                # values come from the latest trusted snapshot.
                roots[handle][:] = messages
            if args:
                args = (roots[handle], *args[1:])
            else:
                kwargs["messages"] = roots[handle]
    if fixture is not None:
        transport.return_value = fixture
    with mock.patch.object(module, "unix_http_request", transport):
        if request["operation"] == "create":
            instances[identifier] = (module.OpenAIAdapter(*args, **kwargs), transport, roots)
            value = None
        else:
            value = instances[identifier][0].tokenize_messages(*args, **kwargs)
    try:
        calls = transport.call_args_list
        if len(calls) > 128:
            raise AdapterPolicyError("oracle.usage-call-count")
        return {"value": pack_usage_value(value), "calls": [
            {"args": pack_usage_value(call.args), "kwargs": pack_usage_value(call.kwargs)} for call in calls]}
    except AdapterPolicyError:
        return {"adapter_policy": True}


def command_proxy(remote, module):
    def call(package, root):
        # The complete original test installs this mock. Never replay into the
        # real helper, and validate every call before touching even that mock.
        if not isinstance(module.command, mock.MagicMock):
            remote.failed = True
            raise ProtocolError("oracle.command-fixture-not-mocked")
        calls = unpack_calls(remote({"package": str(package), "root": str(root)}))
        for args, kwargs in calls:
            module.command(*args, **kwargs)
    return call


def command_result(function, module, request):
    if (type(request) is not dict or request.keys() != {"package", "root"}
            or any(type(value) is not str for value in request.values())):
        raise ProtocolError("oracle.invalid-command-request")
    with mock.patch.object(module, "command") as stub:
        function(Path(request["package"]), Path(request["root"]))
    try:
        if len(stub.call_args_list) > 128:
            raise AdapterPolicyError("oracle.call-count")
        return {"calls": [{"args": pack_call_value(call.args), "kwargs": pack_call_value(call.kwargs)}
                          for call in stub.call_args_list]}
    except AdapterPolicyError:
        return {"adapter_policy": True}


def valid_mode_request(request):
    """Only the closed regular-text fixture can be rebuilt in candidate /tmp."""
    if (type(request) is not dict or request.keys() != {"root", "entries"}
            or type(request["root"]) is not str
            or not re.fullmatch(r"/tmp/[A-Za-z0-9_-]{1,128}", request["root"])
            or type(request["entries"]) is not dict or not 1 <= len(request["entries"]) <= 128
            or any(type(entry) is not dict for entry in request["entries"].values())):
        return False
    entries = request["entries"]
    for name, entry in entries.items():
        if (type(name) is not str or (name != "." and not re.fullmatch(
                r"[A-Za-z0-9_-]+(?:[.][A-Za-z0-9_-]+)*(?:/[A-Za-z0-9_-]+(?:[.][A-Za-z0-9_-]+)*)*", name))
                or type(entry) is not dict or type(entry.get("mode")) is not int
                or not 0 <= entry["mode"] <= 0o777):
            return False
        if entry.keys() == {"kind", "mode"} and entry["kind"] == "directory":
            pass
        elif (entry.keys() == {"kind", "mode", "text"} and entry["kind"] == "file"
              and type(entry["text"]) is str):
            pass
        else:
            return False
        if name != "." and entries.get(str(Path(name).parent), {}).get("kind") != "directory":
            return False
    return entries.get(".", {}).get("kind") == "directory"


def fixture_modes(mount, root_name, entries):
    """Inspect actual effects from the trusted side, never candidate mode claims.

    Descriptor-relative traversal refuses every symlink, including ancestors;
    candidate renames cannot redirect inspection outside its disposable mount.
    """
    modes = {}
    for name, entry in entries.items():
        parts = (root_name, *Path(name).parts)
        descriptor = os.open(mount, os.O_PATH | os.O_DIRECTORY | os.O_NOFOLLOW)
        try:
            for part in parts[:-1]:
                child = os.open(part, os.O_PATH | os.O_DIRECTORY | os.O_NOFOLLOW, dir_fd=descriptor)
                os.close(descriptor)
                descriptor = child
            observed = os.stat(parts[-1], dir_fd=descriptor, follow_symlinks=False)
            if not (stat.S_ISDIR(observed.st_mode) if entry["kind"] == "directory"
                    else stat.S_ISREG(observed.st_mode)):
                raise AdapterPolicyError("oracle.unsupported-mode-observation")
            modes[name] = observed.st_mode & 0o777
        finally:
            os.close(descriptor)
    return modes


def mode_proxy(remote, observation_root=Path("/candidate-tmp")):
    def call(root):
        entries = {}
        for path in (root, *sorted(root.rglob("*"))):
            if path.is_symlink() or not (path.is_dir() or path.is_file()):
                remote.failed = True
                raise ProtocolError("oracle.unsupported-mode-fixture")
            entry = {"kind": "directory" if path.is_dir() else "file",
                     "mode": path.stat().st_mode & 0o777}
            if entry["kind"] == "file":
                entry["text"] = path.read_text(encoding="utf-8")
            entries[str(path.relative_to(root))] = entry
        request = {"root": str(root), "entries": entries}
        if not valid_mode_request(request):
            remote.failed = True
            raise ProtocolError("oracle.unsupported-mode-fixture")
        remote(request)
        try:
            modes = fixture_modes(observation_root, root.name, entries)
        except (OSError, AdapterPolicyError):
            remote.policy_failed = True
            raise AdapterPolicyError("oracle.unsupported-mode-observation") from None
        # Children first: a deliberately wrong parent mode must reach the
        # original assertions without preventing replay of the other bits.
        for name in sorted(modes, key=lambda name: len(Path(name).parts), reverse=True):
            (root / name).chmod(modes[name])
    return call


def mode_result(function, request):
    if not valid_mode_request(request):
        raise ProtocolError("oracle.invalid-mode-request")
    root, entries = Path(request["root"]), request["entries"]
    # The oracle and candidate have private /tmp mounts. Preserve lexical paths
    # while rebuilding bytes; no path from a candidate response is ever opened.
    for name in sorted(entries, key=lambda name: len(Path(name).parts)):
        path, entry = root / name, entries[name]
        if entry["kind"] == "directory":
            path.mkdir()
        else:
            path.write_text(entry["text"], encoding="utf-8")
    for name in sorted(entries, key=lambda name: len(Path(name).parts), reverse=True):
        (root / name).chmod(entries[name]["mode"])
    function(root)
    # Only acknowledge completion. The trusted role independently stats the
    # shared disposable /tmp through its read-only mount before replaying bits.
    return {"observed": True}


def valid_diagnostic_result(result):
    """Only text observations and one fixed exception category, never a class."""
    return (type(result) is dict and result.keys() == {"stderr", "outcome"}
            and type(result["stderr"]) is str and type(result["outcome"]) is dict
            and (result["outcome"] == {"kind": "return"}
                 or (result["outcome"].keys() == {"kind", "message"}
                     and result["outcome"]["kind"] == "runtime-error"
                     and type(result["outcome"]["message"]) is str)))


def diagnostic_proxy(remote):
    def call(command, root):
        result = remote({"command": list(command), "root": str(root)})
        # The original test captures this text locally. It remains inert data;
        # no candidate-supplied exception name, object or traceback is loaded.
        sys.stderr.write(result["stderr"])
        if result["outcome"]["kind"] == "runtime-error":
            raise RuntimeError(result["outcome"]["message"])
        # The frozen method only asserts the failure path. Successful return
        # values are intentionally ungraded, rather than fabricated by a codec.
    return call


def diagnostic_result(function, request):
    if (type(request) is not dict or request.keys() != {"command", "root"}
            or type(request["root"]) is not str or type(request["command"]) is not list
            or not 1 <= len(request["command"]) <= 128
            or any(type(arg) is not str for arg in request["command"])):
        raise ProtocolError("oracle.invalid-diagnostic-request")
    stderr = io.StringIO()
    with contextlib.redirect_stderr(stderr):
        try:
            function(tuple(request["command"]), Path(request["root"]))
        except RuntimeError as error:
            outcome = {"kind": "runtime-error", "message": str(error)}
        else:
            outcome = {"kind": "return"}
    return {"stderr": stderr.getvalue(), "outcome": outcome}


def resource_proxies(remote, module):
    """Transport finite mock observations; the original tests keep assertions."""
    instances = 0

    def observe(request):
        result = remote(request)
        if "harness_error" in result:
            raise module.HarnessError("oracle.resource-refused")
        return unpack_call_value(result["value"])

    def gpu_memory():
        stub = module.subprocess.run
        if not isinstance(stub, mock.MagicMock) or type(stub.return_value) is not subprocess.CompletedProcess:
            remote.failed = True
            raise ProtocolError("oracle.resource-fixture-not-mocked")
        completed = stub.return_value
        return observe({"operation": "gpu", "stdout": completed.stdout,
                        "stderr": completed.stderr, "returncode": completed.returncode})

    class Guard:
        def __init__(self, descriptor, profile, resources, used, free):
            nonlocal instances
            instances += 1
            self.instance = instances
            observe({"operation": "guard-init", "instance": self.instance,
                     "arguments": pack_call_value((descriptor, profile, resources, used, free))})

        def sample(self):
            if not isinstance(module.gpu_memory, mock.MagicMock):
                remote.failed = True
                raise ProtocolError("oracle.resource-fixture-not-mocked")
            result = observe({"operation": "guard-sample", "instance": self.instance,
                              "observation": pack_call_value(module.gpu_memory())})
            if type(result) is not dict or result.keys() != {"returned", "error", "peak"}:
                remote.failed = True
                raise ProtocolError("oracle.invalid-resource-state")
            self.error, self.peak_total_gpu_used_mib_sampled = result["error"], result["peak"]
            return result["returned"]
    return gpu_memory, Guard


def resource_result(module, guards, request):
    """No device, lock, cancellation or host resource is accessed by the adapter."""
    if type(request) is not dict or type(request.get("operation")) is not str:
        raise ProtocolError("oracle.invalid-resource-request")
    operation = request["operation"]
    try:
        if (operation == "gpu" and request.keys() == {"operation", "stdout", "stderr", "returncode"}
                and type(request["stdout"]) is str and type(request["stderr"]) is str
                and type(request["returncode"]) is int):
            completed = subprocess.CompletedProcess([], request["returncode"],
                                                    stdout=request["stdout"], stderr=request["stderr"])
            with mock.patch.object(module.subprocess, "run", return_value=completed):
                value = module.gpu_memory()
        elif (operation in {"guard-init", "guard-sample"}
              and type(request.get("instance")) is int and 1 <= request["instance"] <= 128):
            instance = request["instance"]
            if operation == "guard-init" and request.keys() == {"operation", "instance", "arguments"}:
                arguments = unpack_call_value(request["arguments"])
                if (type(arguments) is not tuple or len(arguments) != 5
                        or type(arguments[0]) is not int or arguments[0] != -1
                        or type(arguments[1]) is not dict
                        or arguments[1].keys() != {"maximum_total_vram_used_mib"}
                        or type(arguments[1]["maximum_total_vram_used_mib"]) is not int
                        or type(arguments[2]) is not dict or arguments[2].keys() != {"memory.peak"}
                        or type(arguments[2]["memory.peak"]) is not str
                        or any(type(v) is not int for v in arguments[3:]) or instance in guards):
                    raise ProtocolError("oracle.invalid-resource-request")
                guards[instance] = module.CandidateResourceGuard(*arguments)
                value = None
            elif (operation == "guard-sample" and instance in guards
                  and request.keys() == {"operation", "instance", "observation"}):
                observation = unpack_call_value(request["observation"])
                if type(observation) is not tuple or len(observation) != 2 or any(type(v) is not int for v in observation):
                    raise ProtocolError("oracle.invalid-resource-request")
                guard = guards[instance]
                with mock.patch.object(module, "gpu_memory", return_value=observation):
                    returned = guard.sample()
                value = {"returned": returned, "error": guard.error,
                         "peak": guard.peak_total_gpu_used_mib_sampled}
            else:
                raise ProtocolError("oracle.invalid-resource-request")
        else:
            raise ProtocolError("oracle.invalid-resource-request")
    except ProtocolError:
        raise
    except module.HarnessError:
        # Reconstruct one fixed trusted category; never transfer exception data.
        return {"harness_error": True}
    try:
        return {"value": pack_call_value(value)}
    except AdapterPolicyError:
        return {"adapter_policy": True}


def valid_collector_commands(commands):
    return (type(commands) in (list, tuple) and len(commands) <= 128
            and all(type(item) in (list, tuple) and len(item) == 2
                    and type(item[0]) is str and type(item[1]) in (list, tuple)
                    and 1 <= len(item[1]) <= 128 and all(type(v) is str for v in item[1])
                    for item in commands))


def valid_deadline_result(result):
    if (type(result) is not dict or result.keys() != {"value", "calls", "timeout"}
            or type(result["timeout"]) is not bool):
        return False
    value = unpack_call_value(result["value"])
    unpack_calls({"calls": result["calls"]})
    return not result["timeout"] or value is None


def deadline_proxies(remote, module):
    """Replay data only into original mocks, never into a real host function."""
    def replay(request, stub):
        if not isinstance(stub, mock.MagicMock):
            remote.failed = True
            raise ProtocolError("oracle.collector-fixture-not-mocked")
        result = remote(request)
        calls = unpack_calls({"calls": result["calls"]})
        # Validate every observation before replay, including the return value.
        value = unpack_call_value(result["value"])
        for args, kwargs in calls:
            stub(*args, **kwargs)
        if result["timeout"]:
            # Only this trusted standard-library class; no remote exception text.
            raise subprocess.TimeoutExpired(("oracle", "collection"), 2700)
        return value

    def run_commands():
        run, which = module.subprocess.run, module.shutil.which
        if (not isinstance(run, mock.MagicMock) or not isinstance(which, mock.MagicMock)
                or type(run.return_value) is not subprocess.CompletedProcess):
            remote.failed = True
            raise ProtocolError("oracle.collector-fixture-not-mocked")
        stub = run.return_value
        if type(stub.stdout) is not bytes or type(stub.stderr) is not bytes:
            remote.failed = True
            raise ProtocolError("oracle.collector-fixture-not-bytes")
        return replay({"operation": "run", "stub": {
            "args": pack_call_value(stub.args), "returncode": stub.returncode,
            "stdout_hex": stub.stdout.hex(), "stderr_hex": stub.stderr.hex()},
            "resolved": {argv[0]: which(argv[0]) for _, argv in module.COMMANDS}}, run)

    def main():
        parse, run, output = module.parse_args, module.run_commands, module.OUTPUT
        if (not isinstance(parse, mock.MagicMock) or not isinstance(run, mock.MagicMock)
                or not isinstance(output, mock.MagicMock)
                or type(parse.return_value) is not SimpleNamespace
                or type(run.side_effect) is not subprocess.TimeoutExpired
                or run.side_effect.output is not None or run.side_effect.stderr is not None):
            remote.failed = True
            raise ProtocolError("oracle.collector-fixture-not-mocked")
        return replay({"operation": "main", "arguments": pack_call_value(vars(parse.return_value)),
                       "timeout_command": pack_call_value(run.side_effect.cmd),
                       "timeout_seconds": run.side_effect.timeout}, output.write_text)
    return run_commands, main


def deadline_result(module, request):
    """Run the two finite collector fixtures; no compiler or documentation gate."""
    if type(request) is not dict or type(request.get("operation")) is not str:
        raise ProtocolError("oracle.invalid-collector-request")
    operation = request["operation"]
    if operation == "commands" and request.keys() == {"operation"}:
        commands = getattr(module, "COMMANDS", None)
        if not valid_collector_commands(commands):
            return {"adapter_policy": True}
        return {"commands": pack_call_value(commands)}
    if (operation == "run" and request.keys() == {"operation", "stub", "resolved"}
            and type(request["stub"]) is dict
            and request["stub"].keys() == {"args", "returncode", "stdout_hex", "stderr_hex"}
            and type(request["stub"]["returncode"]) is int
            and all(type(request["stub"][k]) is str
                    and re.fullmatch(r"(?:[0-9a-f]{2})*", request["stub"][k]) is not None
                    for k in ("stdout_hex", "stderr_hex"))
            and type(request["resolved"]) is dict and len(request["resolved"]) <= 128
            and all(type(k) is str and (v is None or type(v) is str)
                    for k, v in request["resolved"].items())):
        stub = request["stub"]
        completed = subprocess.CompletedProcess(unpack_call_value(stub["args"]), stub["returncode"],
                                               stdout=bytes.fromhex(stub["stdout_hex"]),
                                               stderr=bytes.fromhex(stub["stderr_hex"]))

        def resolve(name):
            if type(name) is not str or name not in request["resolved"]:
                raise AdapterPolicyError("oracle.collector-resolution-outside-fixture")
            return request["resolved"][name]

        stack = contextlib.ExitStack()
        with stack:
            stack.enter_context(mock.patch.object(module.shutil, "which", side_effect=resolve))
            observed = stack.enter_context(mock.patch.object(module.subprocess, "run", return_value=completed))
            function = module.run_commands
            return deadline_observation(function, observed)
    if operation == "main" and request.keys() == {"operation", "arguments", "timeout_command", "timeout_seconds"}:
        arguments = unpack_call_value(request["arguments"])
        command = unpack_call_value(request["timeout_command"])
        if (type(arguments) is not dict or arguments.keys() != {"write", "source_revision"}
                or type(arguments["write"]) is not bool or arguments["write"] is not True
                or type(arguments["source_revision"]) is not str
                or re.fullmatch(r"[0-9a-f]{40}", arguments["source_revision"]) is None
                or type(command) is not tuple or not command or any(type(v) is not str for v in command)
                or type(request["timeout_seconds"]) is not int or request["timeout_seconds"] <= 0):
            raise ProtocolError("oracle.invalid-collector-request")
        with (mock.patch.object(module, "parse_args", return_value=SimpleNamespace(**arguments)),
              mock.patch.object(module, "run_commands", side_effect=subprocess.TimeoutExpired(
                  command, request["timeout_seconds"])), mock.patch.object(module, "OUTPUT") as output):
            return deadline_observation(module.main, output.write_text)
    raise ProtocolError("oracle.invalid-collector-request")


def deadline_observation(function, observed):
    timed_out, value = False, None
    try:
        try:
            value = function()
        except subprocess.TimeoutExpired:
            timed_out = True
        calls = [{"args": pack_call_value(call.args), "kwargs": pack_call_value(call.kwargs)}
                 for call in observed.call_args_list]
        if len(calls) > 128:
            raise AdapterPolicyError("oracle.collector-call-limit")
        return {"value": pack_call_value(value), "calls": calls, "timeout": timed_out}
    except AdapterPolicyError:
        return {"adapter_policy": True}


def candidate(config):
    reader = sys.stdin.buffer
    # Keep the wire off standard output before importing any candidate module.
    # os.dup creates a non-inheritable fd; exec children inherit diagnostics on
    # fd 1, never this writer. This is output hygiene, not isolation from hostile
    # Python in the same interpreter: it can still discover our descriptors.
    descriptor = os.dup(1)
    with os.fdopen(descriptor, "wb") as writer, contextlib.redirect_stdout(sys.stderr):
        os.dup2(2, 1)
        sys.path.insert(0, "/workspace")
        module = importlib.import_module(config["module"])
        # An absent function is a frozen task outcome; a failed import or
        # namespace launch is not. Send startup only after the import succeeds.
        writer.write(encode_message({"ready": True, "function_found":
                                     callable(getattr(module, config["function"], None))}))
        writer.flush()
        guards = {}
        usage_instances = {}
        job_signal_state = {"stale": False}
        while True:
            # EOF after the oracle exits is normal; truncated messages are not.
            if not reader.peek(1):
                return
            request, _ = read_message(reader)
            if type(request) is dict and request.keys() == {"done"} and request["done"] is True:
                return
            if (not isinstance(request, dict) or request.keys() != {"seq", "args"}
                    or type(request["seq"]) is not int or not isinstance(request["args"], list)):
                raise ProtocolError("oracle.invalid-request")
            try:
                function = (None if config["result_type"] in {"argument-result", "resource-result", "deadline-result", "job-result", "usage-result"}
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
                elif config["result_type"] == "capture-result":
                    result = capture_result(module, *request["args"])
                elif config["result_type"] == "mode-result":
                    result = mode_result(function, *request["args"])
                elif config["result_type"] == "command-result":
                    result = command_result(function, module, *request["args"])
                elif config["result_type"] == "diagnostic-result":
                    result = diagnostic_result(function, *request["args"])
                elif config["result_type"] == "usage-result":
                    result = usage_result(module, usage_instances, *request["args"])
                elif config["result_type"] == "resource-result":
                    result = resource_result(module, guards, *request["args"])
                elif config["result_type"] == "deadline-result":
                    result = deadline_result(module, *request["args"])
                elif config["result_type"] == "job-result":
                    result = job_result(module, *request["args"], signal_state=job_signal_state)
                elif config["result_type"] in {"bool", "bool-map"}:
                    result = predicate_result(function(*request["args"]), config["result_type"])
                else:
                    raise ProtocolError("oracle.unknown-result-type")
            except ProtocolError:
                # Malformed adapter requests are not candidate exceptions.
                raise
            except BaseException:
                # This tag carries no candidate-selected class, status or text.
                # A candidate can forge it only to fail its own attempt.
                result = {"candidate_exception": True}
            writer.write(response_message(request["seq"], result))
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
            if config.get("observe_commands"):
                module.COMMANDS = unpack_call_value(remote({"operation": "commands"})["commands"])
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
            elif config["result_type"] == "capture-result":
                module.validate_raw, proxy = capture_proxies(remote, module)
            elif config["result_type"] == "mode-result":
                proxy = mode_proxy(remote)
            elif config["result_type"] == "command-result":
                proxy = command_proxy(remote, module)
            elif config["result_type"] == "diagnostic-result":
                proxy = diagnostic_proxy(remote)
            elif config["result_type"] == "usage-result":
                proxy = usage_proxy(remote, module)
            elif config["result_type"] == "resource-result":
                proxy, module.CandidateResourceGuard = resource_proxies(remote, module)
            elif config["result_type"] == "deadline-result":
                proxy, module.main = deadline_proxies(remote, module)
            elif config["result_type"] == "job-result":
                module.parser, module.stop, proxy, module.export_selection, module.support_bundle_directory = job_proxies(remote, module)
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
        try:
            remote.finish()
        except ProtocolError:
            # Even passing assertions cannot hide extra frames, including a
            # forged first reply followed by the actual function observation.
            pass
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
