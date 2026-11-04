"""Validate an independent function oracle on frozen development controls.

Candidate source and hidden assertions run in separate networkless namespaces.
Generated scoring stays disabled until all remaining admission requirements hold.
"""
from __future__ import annotations

import argparse
from contextlib import contextmanager
import json
import os
from pathlib import Path
import selectors
import signal
import subprocess
import sys
import tempfile
import time

if __package__:
    from . import coding_eval as evaluation
    from . import coding_eval_oracle_worker as worker
else:
    import coding_eval as evaluation
    import coding_eval_oracle_worker as worker

VERSION = "coding-independent-oracle-v19"
# Complete frozen methods for finite function and fixture observations. Paths
# are lexical values; a fixed ambiguity tag becomes a trusted local exception.
SUPPORTED = {
    "dev-005": {"function": "capture", "result_type": "capture-result"},
    "dev-006": {"function": "route_grant_arguments", "result_type": "argument-result"},
    "dev-007": {"function": "memory_arguments", "result_type": "argument-result"},
    "dev-009": {"function": "subprocess_runner", "result_type": "diagnostic-result"},
    "dev-010": {"function": "test_only_files", "result_type": "path-result"},
    "dev-012": {"function": "test_only_files", "result_type": "path-result"},
    "dev-015": {"function": "case_passed", "result_type": "bool"},
    "dev-017": {"function": "make_container_readable", "result_type": "mode-result"},
    "dev-018": {"function": "capture", "result_type": "capture-result"},
    "dev-019": {"function": "extract_deb", "result_type": "command-result"},
    "dev-020": {"function": "verify_case", "result_type": "verification-result"},
    "dev-021": {"function": "verify_case", "result_type": "verification-result"},
    "dev-022": {"function": "verify_case", "result_type": "verification-result", "observe_case_roots": True},
    "dev-024": {"function": "gpu_memory", "result_type": "resource-result"},
    "dev-026": {"function": "run_commands", "result_type": "deadline-result", "observe_commands": True},
    "dev-027": {"function": "extension_arguments", "result_type": "argument-result"},
    "dev-029": {"function": "suspend_resume_checks", "result_type": "bool-map"},
}
LOG_LIMIT = 2 * evaluation.MAX_FILE
# Measured historical outcomes, not model scores or altered hidden assertions.
# Subtests can report more failures than the number of selected methods.
CONTROL_PROFILES = {
    "dev-005": {"failures": 3, "base_calls": 3, "reference_calls": 3},
    "dev-006": {"failures": 0, "errors": 1, "base_calls": 1, "reference_calls": 19,
                "function_found": False},
    "dev-007": {"failures": 0, "errors": 1, "base_calls": 1, "reference_calls": 52,
                "function_found": False},
    "dev-009": {"failures": 1, "base_calls": 1, "reference_calls": 1},
    "dev-010": {"failures": 1, "base_calls": 1, "reference_calls": 4},
    "dev-012": {"failures": 10, "base_calls": 12, "reference_calls": 14},
    "dev-015": {"failures": 1, "base_calls": 3, "reference_calls": 4},
    "dev-017": {"failures": 0, "errors": 1, "base_calls": 1, "reference_calls": 1,
                "function_found": False, "candidate_exception": True},
    "dev-018": {"failures": 1, "base_calls": 1, "reference_calls": 5},
    "dev-019": {"failures": 1, "base_calls": 1, "reference_calls": 1},
    "dev-020": {"failures": 1, "base_calls": 2, "reference_calls": 3},
    "dev-021": {"failures": 0, "errors": 1, "base_calls": 1, "reference_calls": 6},
    "dev-022": {"failures": 0, "errors": 1, "base_calls": 2, "reference_calls": 4},
    "dev-024": {"failures": 0, "errors": 2, "base_calls": 2, "reference_calls": 6,
                "function_found": False, "candidate_exception": True},
    "dev-026": {"failures": 1, "base_calls": 3, "reference_calls": 3},
    "dev-027": {"failures": 0, "errors": 1, "base_calls": 1, "reference_calls": 37,
                "function_found": False},
    "dev-029": {"failures": 0, "base_calls": 1, "reference_calls": 6,
                "candidate_exception": True},
}


def coverage(task):
    """State what transported observations can and cannot establish."""
    kind = SUPPORTED[task["id"]]["result_type"]
    if kind == "deadline-result":
        return {"graded": ["collector-record-and-call-counts", "collector-call-order-and-arguments",
                           "collector-deadlines-and-capture-truthiness", "main-timeout-propagation",
                           "no-report-write-on-timeout"],
                "adapter_policy": ["bounded-encoded-response", "finite-collector-stubs",
                                   "bounded-tagged-observations"],
                "not_graded": ["real-command-execution", "record-content-and-digests",
                               "missing-executable-branch", "timeout-exception-fields",
                               "unselected-main-paths", "minimum-command-count",
                               "unselected-statement-clauses"]}
    if kind == "resource-result":
        return {"graded": ["selected-gpu-output-values-and-refusals", "gpu-return-container-type",
                           "sampled-pressure-return-truthiness", "sampled-error-and-peak-values"],
                "adapter_policy": ["bounded-encoded-response", "finite-resource-stubs",
                                   "bounded-tagged-observations"],
                "not_graded": ["real-gpu-observation", "subprocess-invocation-arguments",
                               "exception-messages", "guard-acquisition-and-cancellation",
                               "unselected-guard-state-and-transitions", "unselected-statement-clauses"]}
    if kind == "diagnostic-result":
        return {"graded": ["failed-child-diagnostic-text", "runtime-error-message-substring"],
                "adapter_policy": ["bounded-encoded-response", "closed-diagnostic-observation"],
                "not_graded": ["subprocess-invocation-arguments", "successful-return-value",
                               "output-tail-bound-and-newline", "exact-exception-type",
                               "unselected-statement-clauses"]}
    if kind == "mode-result":
        return {"graded": ["frozen-tree-permission-bits"],
                "adapter_policy": ["bounded-encoded-response", "finite-text-tree-fixture",
                                   "regular-inspectable-mode-observations"],
                "not_graded": ["file-content-and-tree-shape", "archive-integration",
                               "special-file-refusals", "unselected-statement-clauses"]}
    if kind == "command-result":
        return {"graded": ["helper-call-order-and-values", "helper-argument-container-types"],
                "adapter_policy": ["bounded-encoded-response", "bounded-helper-call-observations"],
                "not_graded": ["real-command-execution", "helper-error-propagation",
                               "unselected-statement-clauses"]}
    if kind == "capture-result":
        return {"graded": ["selected-private-path-refusals", "captured-output-redaction",
                           "selected-return-values"],
                "adapter_policy": ["bounded-encoded-response", "bounded-json-observations",
                                   "finite-capture-stub"],
                "not_graded": ["real-command-execution", "subprocess-invocation-arguments",
                               *(["home-alias-variant"] if task["id"] == "dev-018" else []),
                               "other-module-globals", "unselected-statement-clauses"]}
    if kind == "path-result":
        return {"graded": ["path-membership", "ambiguity-behavior"],
                "adapter_policy": ["bounded-encoded-response"],
                "not_graded": ["return-container-type", "return-element-type",
                               "unselected-statement-clauses"]}
    if kind == "verification-result":
        return {"graded": ["passed-truthiness", "selected-check-truthiness",
                           "frozen-fixture-variations",
                           *(["candidate-case-root-uniqueness"] if task["id"] == "dev-022" else [])],
                "adapter_policy": ["bounded-encoded-response", "bounded-json-observations",
                                   *(["bounded-case-root-strings"] if task["id"] == "dev-022" else [])],
                "not_graded": ["exact-boolean-result-type", "run-case-integration", "other-report-fields",
                               "unselected-cases", "unselected-statement-clauses"]}
    if kind == "argument-result":
        return {"graded": ["selected-parser-values", "argument-order-and-values",
                           "selected-refusal-types", "argument-return-container-type"],
                "adapter_policy": ["bounded-encoded-response", "bounded-namespace-values",
                                   "bounded-helper-values", "fixed-parser-exit-code"],
                "not_graded": ["exception-messages", "parser-diagnostics", "catalog-effects",
                               "parser-return-object-type",
                               "unselected-statement-clauses"]}
    return {"graded": ["selected-predicate-truthiness"],
            "adapter_policy": ["bounded-encoded-response", "bounded-json-observations"],
            "not_graded": ["exact-boolean-result-type", "unselected-statement-clauses"]}


def supported_task(task):
    if task["id"] not in SUPPORTED or task not in evaluation.collection():
        raise evaluation.EvaluationError("oracle.task-refused")
    return {**SUPPORTED[task["id"]], "module": task["source"][:-3].replace("/", ".")}


def sandbox(workspace, adapter, output, role, candidate_tmp=None):
    command = [
        "/usr/bin/bwrap", "--unshare-all", "--unshare-user", "--die-with-parent", "--new-session",
        "--disable-userns", "--clearenv", "--cap-drop", "ALL",
        "--ro-bind", "/usr", "/usr", "--symlink", "usr/bin", "/bin",
        "--symlink", "usr/lib", "/lib", "--symlink", "usr/lib64", "/lib64",
        "--proc", "/proc", "--dev", "/dev",
        "--ro-bind", str(workspace), "/workspace", "--ro-bind", str(adapter), "/adapter",
        "--setenv", "PATH", "/usr/bin", "--setenv", "HOME", "/tmp",
        "--setenv", "LANG", "C.UTF-8", "--chdir", "/workspace",
    ]
    if candidate_tmp is not None and role == "candidate":
        command.extend(["--bind", str(candidate_tmp), "/tmp"])
    else:
        command.extend(["--tmpfs", "/tmp"])
    if candidate_tmp is not None and role == "oracle":
        command.extend(["--ro-bind", str(candidate_tmp), "/candidate-tmp"])
    if role == "oracle":
        command.extend(["--bind", str(output), "/output"])
    elif role != "candidate":
        raise evaluation.EvaluationError("oracle.role-refused")
    return [*command, "--", "/usr/bin/python3", "-I", "-B", "/adapter/run.py", role]


def terminate(process):
    # Do not reap the leader until signalling its group; PID namespaces also end
    # descendants that escaped their original process group inside the worker.
    try:
        os.killpg(process.pid, signal.SIGKILL)
    except ProcessLookupError:
        pass
    process.wait()
    for stream in (process.stdin, process.stdout, process.stderr):
        if stream is not None:
            stream.close()


def launch(command, **options):
    """Keep host arguments and inherited environment out of namespace PID 1."""
    if command[0] != "/usr/bin/bwrap":
        # Non-bwrap commands are used only for host lifecycle fault tests.
        return subprocess.Popen(command, **options)
    boundary = command.index("--")
    with tempfile.TemporaryFile() as arguments:
        arguments.write(b"\0".join(os.fsencode(arg) for arg in command[1:boundary]) + b"\0")
        arguments.seek(0)
        # bwrap reads and closes this descriptor before starting the worker.
        # The temporary file has no directory entry; it closes here on every
        # launch outcome. Only this fd crosses exec alongside standard streams.
        # --args supplies options only; the fixed namespace-local executable
        # and its arguments must remain in argv after the option delimiter.
        # --clearenv only clears the worker; PID 1 can still expose bwrap's
        # initial environment through procfs. Neither launcher needs host vars.
        return subprocess.Popen([command[0], "--args", str(arguments.fileno()), *command[boundary:]],
                                env={}, pass_fds=(arguments.fileno(),), **options)


def exchange(candidate_command, oracle_command, output, timeout):
    """Connect pipes directly; the host accepts results only from the oracle mount."""
    processes = []
    exit_code = candidate_exit = None
    timed_out = output_limited = False
    try:
        with ((output / "candidate.log").open("xb") as candidate_log,
              (output / "oracle.log").open("xb") as oracle_log):
            candidate_process = launch(
                candidate_command, stdin=subprocess.PIPE, stdout=subprocess.PIPE,
                stderr=subprocess.PIPE, start_new_session=True, preexec_fn=evaluation.limits)
            processes.append(candidate_process)
            oracle_process = launch(
                oracle_command, stdin=candidate_process.stdout, stdout=candidate_process.stdin,
                stderr=subprocess.PIPE, start_new_session=True, preexec_fn=evaluation.limits)
            processes.append(oracle_process)
            candidate_process.stdin.close()
            candidate_process.stdout.close()
            totals = {candidate_log: 0, oracle_log: 0}
            deadline = time.monotonic() + timeout
            with selectors.DefaultSelector() as events:
                events.register(candidate_process.stderr, selectors.EVENT_READ, candidate_log)
                events.register(oracle_process.stderr, selectors.EVENT_READ, oracle_log)
                while True:
                    for key, _ in events.select(timeout=0.01):
                        data = os.read(key.fileobj.fileno(), 65536)
                        if not data:
                            events.unregister(key.fileobj)
                            key.fileobj.close()
                            continue
                        remaining = LOG_LIMIT - totals[key.data]
                        key.data.write(data[:remaining])
                        totals[key.data] += min(len(data), remaining)
                        if len(data) > remaining:
                            output_limited = True
                    status = os.waitid(os.P_PID, oracle_process.pid, os.WEXITED | os.WNOHANG | os.WNOWAIT)
                    candidate_status = os.waitid(os.P_PID, candidate_process.pid,
                                                 os.WEXITED | os.WNOHANG | os.WNOWAIT)
                    if status is not None and candidate_status is not None and not events.get_map():
                        exit_code = status.si_status if status.si_code == os.CLD_EXITED else -status.si_status
                        candidate_exit = (candidate_status.si_status if candidate_status.si_code == os.CLD_EXITED
                                          else -candidate_status.si_status)
                        break
                    if output_limited:
                        break
                    if time.monotonic() >= deadline:
                        timed_out = True
                        break
    finally:
        for process in reversed(processes):
            terminate(process)
    return exit_code, candidate_exit, timed_out, output_limited


def result_record(path, task):
    value = json.loads(evaluation.read_regular(path, 4096))
    fields = {"tests", "failures", "errors", "skipped", "expected_failures", "unexpected_successes"}
    if (not isinstance(value, dict) or value.keys() != {
            "tests", "calls", "protocol_failed", "adapter_policy_failed", "candidate_exception_failed",
            "candidate_ready", "function_found"}
            or not isinstance(value["tests"], dict) or value["tests"].keys() != fields
            or any(type(v) is not int or v < 0 for v in value["tests"].values())
            or type(value["calls"]) is not int or not 0 <= value["calls"] <= worker.MAX_CALLS
            or type(value["protocol_failed"]) is not bool
            or type(value["adapter_policy_failed"]) is not bool
            or type(value["candidate_exception_failed"]) is not bool
            or type(value["candidate_ready"]) is not bool):
        raise evaluation.EvaluationError("oracle.invalid-result-record")
    if value["candidate_ready"]:
        valid = (type(value["function_found"]) is bool and value["calls"] > 0
                 and (value["tests"]["tests"] == task["expected_tests"]
                      or ((SUPPORTED[task["id"]].get("observe_case_roots")
                           or SUPPORTED[task["id"]].get("observe_commands"))
                          and value["calls"] == 1 and not any(value["tests"].values())
                          and (value["protocol_failed"] or value["adapter_policy_failed"]
                               or value["candidate_exception_failed"]))))
    else:
        valid = (value["function_found"] is None and value["calls"] == 0
                 and value["protocol_failed"] and not value["adapter_policy_failed"]
                 and not value["candidate_exception_failed"]
                 and not any(value["tests"].values()))
    if not valid:
        raise evaluation.EvaluationError("oracle.invalid-result-record")
    return value


def repair_fixture_permissions(root):
    """Repair disposable directories through pinned Linux directory descriptors.

    Killing/reaping bwrap does not synchronously reap all namespace descendants.
    O_PATH|O_DIRECTORY|O_NOFOLLOW pins only real directories, even mode 0000;
    chmod through our own proc descriptor addresses that inode despite renames.
    No candidate-controlled path is resolved by chmod. Reopen deferred branches
    component by component from the pinned root, never across symlinks. A single
    child is visited directly, so deep chains need neither long paths nor a
    descriptor per ancestor. Concurrent removal/replacement may omit a branch;
    the existing cleanup still refuses publication if removal fails.
    """
    flags = os.O_PATH | os.O_DIRECTORY | os.O_NOFOLLOW | os.O_CLOEXEC

    def open_child(parent, name):
        try:
            return os.open(name, flags, dir_fd=parent)
        except (FileNotFoundError, NotADirectoryError):
            return None

    root_fd = os.open(root, flags)
    current = None
    try:
        current = os.dup(root_fd)
        components, pending = (), []
        while current is not None:
            os.chmod(f'/proc/self/fd/{current}', 0o700)
            # O_PATH cannot be scanned; open the same pinned inode for reading.
            readable = os.open('.', os.O_RDONLY | os.O_DIRECTORY | os.O_CLOEXEC, dir_fd=current)
            try:
                with os.scandir(readable) as entries:
                    children = [entry.name for entry in entries if entry.is_dir(follow_symlinks=False)]
            finally:
                os.close(readable)
            pending.extend(components + (name,) for name in children[1:])
            child = open_child(current, children[0]) if children else None
            os.close(current)
            current = child
            if current is not None:
                components += (children[0],)
                continue
            while pending and current is None:
                components = pending.pop()
                current = os.dup(root_fd)
                for name in components:
                    child = open_child(current, name)
                    os.close(current)
                    current = child
                    if current is None:
                        break
    finally:
        if current is not None:
            os.close(current)
        os.close(root_fd)


def remove_fixture_tree(root):
    """Remove repaired scratch with constant fds, even beyond rmtree's fd limit.

    Pin each child without following links. Ascend only to a parent whose inode
    matches the recorded pinned parent; a concurrent rename refuses cleanup.
    Unlink/rmdir are relative to that parent and never follow the removed name.
    The final root's parent is private host-owned case storage, not a worker bind.
    """
    flags = os.O_RDONLY | os.O_DIRECTORY | os.O_NOFOLLOW | os.O_CLOEXEC
    current = os.open(root, flags)
    parents = []
    try:
        while True:
            descend = None
            with os.scandir(current) as entries:
                for entry in entries:
                    if entry.is_dir(follow_symlinks=False):
                        descend = entry.name
                        break
                    os.unlink(entry.name, dir_fd=current)
            if descend is not None:
                child = os.open(descend, flags, dir_fd=current)
                parent = os.fstat(current)
                parents.append((descend, parent.st_dev, parent.st_ino))
                os.close(current)
                current = child
                continue
            if not parents:
                break
            name, device, inode = parents.pop()
            parent = os.open('..', flags, dir_fd=current)
            try:
                observed = os.fstat(parent)
                if (observed.st_dev, observed.st_ino) != (device, inode):
                    raise OSError('oracle.fixture-moved-during-cleanup')
                child = os.fstat(current)
                named = os.stat(name, dir_fd=parent, follow_symlinks=False)
                if (named.st_dev, named.st_ino) != (child.st_dev, child.st_ino):
                    raise OSError('oracle.fixture-moved-during-cleanup')
                os.rmdir(name, dir_fd=parent)
            except BaseException:
                os.close(parent)
                raise
            os.close(current)
            current = parent
    finally:
        os.close(current)
    root.rmdir()


@contextmanager
def candidate_temporary(root, enabled):
    """Repair writable-fixture modes before the enclosing case cleanup."""
    if not enabled:
        yield {}
        return
    directory = root / "candidate-tmp"
    directory.mkdir(mode=0o700)
    failed = False
    try:
        yield {"candidate_tmp": directory}
    except BaseException:
        failed = True
        raise
    finally:
        try:
            repair_fixture_permissions(directory)
            remove_fixture_tree(directory)
        except OSError:
            # Preserve interruption/setup errors. The enclosing case_workspace
            # still attempts removal and retains its recovery record on failure.
            if not failed:
                raise


def grade_case(task, files, output, timeout=evaluation.TIMEOUT):
    """Host-owned diagnostic entry; the public verifier admits frozen controls only."""
    config = supported_task(task)
    trusted_files = evaluation.projection(task)
    hidden = evaluation.pinned_blob(task["reference"], task["hidden_test"], task["hidden_sha256"])
    record = None
    with evaluation.case_workspace(output) as root, candidate_temporary(
            root, config["result_type"] == "mode-result") as fixture_options:
        trusted, candidate = root / "trusted", root / "candidate"
        oracle_adapter, candidate_adapter = root / "oracle-adapter", root / "candidate-adapter"
        results = root / "results"
        for directory in (trusted, candidate, oracle_adapter, candidate_adapter, results):
            directory.mkdir(mode=0o700)
        for workspace, content in ((trusted, trusted_files), (candidate, files)):
            for name, data in content.items():
                evaluation.write_file(workspace / name, data)
        for directory in (oracle_adapter, candidate_adapter):
            evaluation.write_file(directory / "run.py", Path(worker.__file__).read_bytes())
        evaluation.publish(candidate_adapter / "config.json", config)
        evaluation.publish(oracle_adapter / "config.json", {
            **config, "test_names": task["test_names"], "expected_tests": task["expected_tests"],
        })
        evaluation.write_file(oracle_adapter / "test.py", hidden)
        outcome = exchange(sandbox(candidate, candidate_adapter, results, "candidate", **fixture_options),
                           sandbox(trusted, oracle_adapter, results, "oracle", **fixture_options), output, timeout)
        try:
            record = result_record(results / "tests.json", task)
        except (OSError, ValueError):
            record = None
        transcript = evaluation.read_regular(results / "wire.log", worker.MAX_TRANSCRIPT)
    # Publishing counts/transcript follows namespace, process and tree cleanup.
    evaluation.write_file(output / "wire.log", transcript)
    if record is not None:
        evaluation.publish(output / "tests.json", record)
    return (*outcome, record)


def grade(task, files, output):
    supported_task(task)
    kind = evaluation.require_historical_candidate(task, files)
    evaluation.new_directory(output)
    started = time.monotonic()
    error = None
    input_error = None
    try:
        exit_code, candidate_exit, timed_out, output_limited, record = grade_case(task, files, output)
    except (OSError, ValueError, subprocess.SubprocessError) as exception:
        error = evaluation.refusal_detail(exception)
        if isinstance(exception, evaluation.InputIntegrityError):
            input_error = error
        exit_code, candidate_exit, timed_out, output_limited, record = None, None, False, False, None
    tests = record["tests"] if record else None
    passed = (error is None and exit_code == 0 and candidate_exit == 0 and not timed_out and not output_limited
              and record is not None and record["candidate_ready"] and record["function_found"]
              and not record["protocol_failed"]
              and not record["adapter_policy_failed"] and not record["candidate_exception_failed"]
              and all(v == 0 for k, v in tests.items() if k != "tests"))
    cause = ("none" if passed else "input_integrity" if input_error
             else "timeout" if timed_out else "output_limit" if output_limited
             else "infrastructure" if error or record is None or not record["candidate_ready"]
             else "protocol" if record["protocol_failed"]
             else "adapter_policy" if record["adapter_policy_failed"]
             else "candidate_exception" if record["candidate_exception_failed"] else "candidate_exit"
             if candidate_exit != 0 else "test_failure")
    report = {
        "version": VERSION, "task": task["id"], "passed": passed, "cause": cause,
        "exit_code": exit_code, "candidate_exit_code": candidate_exit,
        "tests": tests, "calls": record["calls"] if record else None,
        "protocol_failed": record["protocol_failed"] if record else None,
        "adapter_policy_failed": record["adapter_policy_failed"] if record else None,
        "candidate_exception_failed": record["candidate_exception_failed"] if record else None,
        "candidate_ready": record["candidate_ready"] if record else None,
        "function_found": record["function_found"] if record else None,
        "error": error, "input_error": input_error,
        "output_truncated": output_limited, "candidate_kind": kind,
        "hidden_sha256": task["hidden_sha256"], "test_names": task["test_names"],
        "candidate_sha256": evaluation.sha(files[task["source"]]),
        "task_sha256": evaluation.sha(json.dumps(task, sort_keys=True, separators=(",", ":")).encode()),
        "oracle_sha256": evaluation.sha(Path(__file__).read_bytes()),
        "worker_sha256": evaluation.sha(Path(worker.__file__).read_bytes()),
        "wall_seconds": round(time.monotonic() - started, 6),
        "model_attempts": 0, "model_score": None, "gpu_minutes": 0, "tokens": None,
        "score_scope": "independent-oracle-controls-only",
        # All launched diagnostic controls remain in the denominator, including
        # missing startup, infrastructure errors, timeout and protocol failure.
        # These counts are not model attempts or a generated-candidate score.
        "control_attempts": {"total": 1, "passed": int(passed), "failed": int(not passed)},
        "coverage": coverage(task),
        "aggregate_worker_quotas": False,
        "logs": {name: evaluation.sha(evaluation.read_regular(output / name, LOG_LIMIT))
                 for name in ("candidate.log", "oracle.log", "wire.log") if (output / name).exists()},
    }
    evaluation.publish(output / "report.json", report)
    return report


def expected_base(task, report):
    """An arbitrary failure is not evidence that a frozen regression works."""
    profile = CONTROL_PROFILES.get(task["id"])
    if profile is None:
        return False
    missing = profile.get("candidate_exception", False)
    tests = {"tests": task["expected_tests"], "failures": profile["failures"],
             "errors": profile.get("errors", 1 if missing else 0),
             "skipped": 0, "expected_failures": 0, "unexpected_successes": 0}
    return (not report["passed"] and report["candidate_kind"] == "frozen-base"
            and report["candidate_ready"] is True
            and report["function_found"] is profile.get("function_found", not missing)
            and report["cause"] == ("candidate_exception" if missing else "test_failure")
            and report["tests"] == tests and report["calls"] == profile["base_calls"]
            and report["protocol_failed"] is False
            and report["candidate_exception_failed"] is missing
            and report["adapter_policy_failed"] is False
            and report["candidate_exit_code"] == 0
            and report["exit_code"] == 1)


def verify(output):
    evaluation.new_directory(output)
    rows = []
    for task in evaluation.collection():
        if task["id"] not in SUPPORTED:
            continue
        files = evaluation.projection(task)
        base = grade(task, files, output / (task["id"] + "-base"))
        fixed = {**files, task["source"]: evaluation.pinned_blob(
            task["reference"], task["source"], task["reference_sha256"])}
        reference = grade(task, fixed, output / (task["id"] + "-reference"))
        rows.append({"task": task["id"], "base": base, "reference": reference,
                     "valid": (expected_base(task, base) and reference["passed"]
                               and reference["candidate_kind"] == "frozen-reference"
                               and reference["calls"] == CONTROL_PROFILES[task["id"]]["reference_calls"])})
    summary = {
        "version": VERSION, "kind": "independent-oracle-validation-only", "tasks": rows,
        "passed": len(rows) == len(SUPPORTED) and all(row["valid"] for row in rows),
        "collection_sha256": evaluation.sha(json.dumps(
            evaluation.collection(), sort_keys=True, separators=(",", ":")).encode()),
        "generated_scoring": False, "aggregate_worker_quotas": False, "model_score": None,
        "control_attempts": {
            key: sum(row[k]["control_attempts"][key] for row in rows for k in ("base", "reference"))
            for key in ("total", "passed", "failed")},
        "input_refused": any(row[k]["cause"] == "input_integrity" for row in rows
                             for k in ("base", "reference")),
    }
    evaluation.publish(output / "summary.json", summary)
    return summary


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("operation", choices=("verify",))
    parser.add_argument("--output", type=Path, required=True)
    args = parser.parse_args()
    try:
        result = verify(args.output)
        return 0 if result["passed"] else 2 if result["input_refused"] else 1
    except (OSError, ValueError, subprocess.SubprocessError) as error:
        print("oracle.refused " + evaluation.refusal_detail(error), file=sys.stderr)
        return 2


if __name__ == "__main__":
    for termination_signal in (signal.SIGTERM, signal.SIGHUP):
        signal.signal(termination_signal, evaluation.interrupted)
    raise SystemExit(main())
