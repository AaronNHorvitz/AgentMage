"""Validate an independent function oracle on two frozen development controls.

Candidate source and hidden assertions run in separate networkless namespaces.
Generated scoring stays disabled until all remaining admission requirements hold.
"""
from __future__ import annotations

import argparse
import json
import os
from pathlib import Path
import selectors
import signal
import subprocess
import sys
import time

if __package__:
    from . import coding_eval as evaluation
    from . import coding_eval_oracle_worker as worker
else:
    import coding_eval as evaluation
    import coding_eval_oracle_worker as worker

VERSION = "coding-independent-oracle-v1"
# These complete frozen methods need only JSON arguments/results for one pure
# function. No candidate mocks, filesystem views or exceptions are transported.
SUPPORTED = {
    "dev-015": {"function": "case_passed", "result_type": "bool"},
    "dev-029": {"function": "suspend_resume_checks", "result_type": "bool-map"},
}
LOG_LIMIT = 2 * evaluation.MAX_FILE


def supported_task(task):
    if task["id"] not in SUPPORTED or task not in evaluation.collection():
        raise evaluation.EvaluationError("oracle.task-refused")
    return {**SUPPORTED[task["id"]], "module": task["source"][:-3].replace("/", ".")}


def sandbox(workspace, adapter, output, role):
    command = [
        "/usr/bin/bwrap", "--unshare-all", "--unshare-user", "--die-with-parent", "--new-session",
        "--disable-userns", "--clearenv", "--cap-drop", "ALL",
        "--ro-bind", "/usr", "/usr", "--symlink", "usr/bin", "/bin",
        "--symlink", "usr/lib", "/lib", "--symlink", "usr/lib64", "/lib64",
        "--proc", "/proc", "--dev", "/dev", "--tmpfs", "/tmp",
        "--ro-bind", str(workspace), "/workspace", "--ro-bind", str(adapter), "/adapter",
        "--setenv", "PATH", "/usr/bin", "--setenv", "HOME", "/tmp",
        "--setenv", "LANG", "C.UTF-8", "--chdir", "/workspace",
    ]
    if role == "oracle":
        command.extend(["--bind", str(output), "/output"])
    elif role != "candidate":
        raise evaluation.EvaluationError("oracle.role-refused")
    return [*command, "/usr/bin/python3", "-I", "-B", "/adapter/run.py", role]


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


def exchange(candidate_command, oracle_command, output, timeout):
    """Connect pipes directly; the host accepts results only from the oracle mount."""
    processes = []
    exit_code = candidate_exit = None
    timed_out = output_limited = False
    try:
        with ((output / "candidate.log").open("xb") as candidate_log,
              (output / "oracle.log").open("xb") as oracle_log):
            candidate_process = subprocess.Popen(
                candidate_command, stdin=subprocess.PIPE, stdout=subprocess.PIPE,
                stderr=subprocess.PIPE, start_new_session=True, preexec_fn=evaluation.limits)
            processes.append(candidate_process)
            oracle_process = subprocess.Popen(
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
    if (not isinstance(value, dict) or value.keys() != {"tests", "calls", "protocol_failed"}
            or not isinstance(value["tests"], dict) or value["tests"].keys() != fields
            or any(type(v) is not int or v < 0 for v in value["tests"].values())
            or value["tests"]["tests"] != task["expected_tests"]
            or type(value["calls"]) is not int or not 0 < value["calls"] <= worker.MAX_CALLS
            or type(value["protocol_failed"]) is not bool):
        raise evaluation.EvaluationError("oracle.invalid-result-record")
    return value


def grade_case(task, files, output, timeout=evaluation.TIMEOUT):
    """Host-owned diagnostic entry; the public verifier admits frozen controls only."""
    config = supported_task(task)
    trusted_files = evaluation.projection(task)
    hidden = evaluation.pinned_blob(task["reference"], task["hidden_test"], task["hidden_sha256"])
    record = None
    with evaluation.case_workspace(output) as root:
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
        outcome = exchange(sandbox(candidate, candidate_adapter, results, "candidate"),
                           sandbox(trusted, oracle_adapter, results, "oracle"), output, timeout)
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
              and record is not None and not record["protocol_failed"]
              and all(v == 0 for k, v in tests.items() if k != "tests"))
    cause = ("none" if passed else "input_integrity" if input_error
             else "timeout" if timed_out else "output_limit" if output_limited
             else "infrastructure" if error or record is None
             else "protocol" if record["protocol_failed"] else "candidate_exit"
             if candidate_exit != 0 else "test_failure")
    report = {
        "version": VERSION, "task": task["id"], "passed": passed, "cause": cause,
        "exit_code": exit_code, "candidate_exit_code": candidate_exit,
        "tests": tests, "calls": record["calls"] if record else None,
        "protocol_failed": record["protocol_failed"] if record else None,
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
        "aggregate_worker_quotas": False,
        "logs": {name: evaluation.sha(evaluation.read_regular(output / name, LOG_LIMIT))
                 for name in ("candidate.log", "oracle.log", "wire.log") if (output / name).exists()},
    }
    evaluation.publish(output / "report.json", report)
    return report


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
                     "valid": base["cause"] in {"test_failure", "protocol"} and reference["passed"]})
    summary = {
        "version": VERSION, "kind": "independent-oracle-validation-only", "tasks": rows,
        "passed": len(rows) == len(SUPPORTED) and all(row["valid"] for row in rows),
        "collection_sha256": evaluation.sha(json.dumps(
            evaluation.collection(), sort_keys=True, separators=(",", ":")).encode()),
        "generated_scoring": False, "aggregate_worker_quotas": False, "model_score": None,
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
