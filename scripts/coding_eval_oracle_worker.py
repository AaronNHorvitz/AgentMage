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
import os
from pathlib import Path
import re
import subprocess
import stat
import sys
import tempfile
import unittest
from unittest import mock

MAX_MESSAGE = 64 * 1024
MAX_TRANSCRIPT = 2 * 1024 * 1024
MAX_CALLS = 128
POLICY_RESULT_TYPES = frozenset({"argument-result", "verification-result", "capture-result",
                                "bool", "bool-map", "path-result", "mode-result", "command-result"})


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
                elif config["result_type"] == "capture-result":
                    result = capture_result(module, *request["args"])
                elif config["result_type"] == "mode-result":
                    result = mode_result(function, *request["args"])
                elif config["result_type"] == "command-result":
                    result = command_result(function, module, *request["args"])
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
