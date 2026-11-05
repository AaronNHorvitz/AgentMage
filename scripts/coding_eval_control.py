"""Quota-backed execution of pinned historical controls, never model admission.

The trusted mount supervisor owns scratch and bounded logs. Task code runs in a
different user/PID/network namespace with no control connection or host paths.
This is confinement, not an independent oracle: tests still share a process
with historical candidate code and cannot grade arbitrary generated programs.
"""
from __future__ import annotations

import ctypes
import json
import os
from pathlib import Path
import selectors
import shutil
import socket
import subprocess
import time

if __package__:
    from . import coding_eval as evaluation
    from . import coding_eval_oracle as oracle
    from . import coding_eval_scratch as scratch
else:
    import coding_eval as evaluation
    import coding_eval_oracle as oracle
    import coding_eval_scratch as scratch


VERSION = "coding-historical-control-v1"
PYTHON_PROCESSES = 8
RUST_PROCESSES = 16
RUST_ADDRESS_BYTES = 768 * evaluation.MAX_FILE
RUST_TMP_BYTES = 768 * evaluation.MAX_FILE
RUST_TMP_INODES = 32768
RUST_RUNTIME_BYTES = 1536 * evaluation.MAX_FILE
RUST_RUNTIME_INODES = 32768


def limited_driver(driver, rust):
    """Hard limits belong to the worker namespace, before importing task code."""
    prefix = (
        "import resource as _coding_control_limits\n"
        f"_coding_control_limits.setrlimit(_coding_control_limits.RLIMIT_NPROC, "
        f"({RUST_PROCESSES if rust else PYTHON_PROCESSES},) * 2)\n"
    )
    if rust:
        prefix += (
            f"_coding_control_limits.setrlimit(_coding_control_limits.RLIMIT_AS, "
            f"({RUST_ADDRESS_BYTES},) * 2)\n"
        )
    anchor = b"from __future__ import annotations\n"
    if anchor in driver:
        return driver.replace(anchor, anchor + prefix.encode(), 1)
    return prefix.encode() + driver


def sandbox(root, rust):
    """All bind sources resolve inside the trusted anonymous-mount holder."""
    command = evaluation.sandbox_command(root / "workspace", root / "grader", root / "results")
    temporary = command.index("--tmpfs")
    command[temporary:temporary + 2] = ["--bind", str(root / "private-control"), "/tmp"]
    executable = command.index("/usr/bin/python3")
    command[executable:executable] = ["--remount-ro", "/dev", "--remount-ro", "/", "--"]
    if rust:
        executable = command.index("--remount-ro")
        command[executable:executable] = [
            "--ro-bind", "/runtime/toolchain", "/toolchain",
            "--bind", str(root / "private-control/cargo"), "/cargo",
            "--ro-bind", "/runtime/registry", "/cargo/registry",
            "--setenv", "CARGO_HOME", "/cargo", "--setenv", "CARGO_NET_OFFLINE", "true",
            "--setenv", "PATH", "/toolchain/bin:/usr/bin",
            "--setenv", "RUSTFLAGS", "-Ccodegen-units=1 -Clink-arg=-Wl,--threads=2",
        ]
    return command


def run_process(command, output, timeout, rust):
    """Trusted supervisor drains bounded logs and always kills/reaps its child."""
    process = None
    exit_code = None
    timed_out = limited = False
    try:
        with (output / "stdout.log").open("xb") as stdout, (output / "stderr.log").open("xb") as stderr:
            process = oracle.launch(command, stdin=subprocess.DEVNULL, stdout=subprocess.PIPE,
                                    stderr=subprocess.PIPE, start_new_session=True,
                                    preexec_fn=evaluation.coding_eval_rust.limits if rust else evaluation.limits)
            totals = {stdout: 0, stderr: 0}
            deadline = time.monotonic() + timeout
            with selectors.DefaultSelector() as events:
                events.register(process.stdout, selectors.EVENT_READ, stdout)
                events.register(process.stderr, selectors.EVENT_READ, stderr)
                while True:
                    for key, _ in events.select(timeout=0.01):
                        data = os.read(key.fileobj.fileno(), 65536)
                        if not data:
                            events.unregister(key.fileobj)
                            key.fileobj.close()
                            continue
                        remaining = 2 * evaluation.MAX_FILE - totals[key.data]
                        key.data.write(data[:remaining])
                        totals[key.data] += min(len(data), remaining)
                        limited |= len(data) > remaining
                    status = os.waitid(os.P_PID, process.pid, os.WEXITED | os.WNOHANG | os.WNOWAIT)
                    if status is not None and not events.get_map():
                        exit_code = status.si_status if status.si_code == os.CLD_EXITED else -status.si_status
                        break
                    if limited:
                        break
                    if time.monotonic() >= deadline:
                        timed_out = True
                        break
    finally:
        if process is not None:
            oracle.terminate(process)
    return exit_code, timed_out, limited


def run_request(request):
    """Fixed holder operation. No task or command text arrives over the socket."""
    if (type(request) is not dict or request.keys() != {"mode", "rust", "timeout"}
            or request["mode"] != "historical-control" or type(request["rust"]) is not bool
            or type(request["timeout"]) not in (int, float) or not 0 < request["timeout"] <= evaluation.TIMEOUT):
        raise scratch.ScratchError("control.request-refused")
    rust = request["rust"]
    root = Path("/scratch")
    temporary = root / "private-control"
    temporary.mkdir(mode=0o700)
    libc = ctypes.CDLL(None, use_errno=True)
    libc.mount.argtypes = [ctypes.c_char_p, ctypes.c_char_p, ctypes.c_char_p,
                           ctypes.c_ulong, ctypes.c_char_p]
    options = (f"size={RUST_TMP_BYTES},nr_inodes={RUST_TMP_INODES},mode=0700" if rust
               else f"size={scratch.PRIVATE_BYTES},nr_inodes={scratch.PRIVATE_INODES},mode=0700")
    if libc.mount(b"tmpfs", os.fsencode(temporary), b"tmpfs", 2 | 4, options.encode()) != 0:
        raise scratch.ScratchError("control.private-mount-refused")
    if rust:
        (temporary / "cargo").mkdir(mode=0o700)
        # A bind alias does not hide the backing filesystem's root path from
        # mountinfo. Copy only operator-provided, read-only build inputs onto
        # another quota-backed mount; workers see no host-backed Rust mounts.
        runtime_options = f"size={RUST_RUNTIME_BYTES},nr_inodes={RUST_RUNTIME_INODES},mode=0700"
        if libc.mount(b"tmpfs", b"/runtime", b"tmpfs", 2 | 4, runtime_options.encode()) != 0:
            raise scratch.ScratchError("control.runtime-mount-refused")
        for name in ("toolchain", "registry"):
            shutil.copytree(Path("/runtime-input") / name, Path("/runtime") / name, symlinks=True)
    if not rust:
        evaluation.initialize_grading_repository(root / "workspace")
    return run_process(sandbox(root, rust), Path("/logs"), request["timeout"], rust)


def run(mount, rust, timeout):
    """One request per holder; accept only the closed outcome tuple."""
    if (mount.used or type(rust) is not bool or type(timeout) not in (int, float)
            or not 0 < timeout <= evaluation.TIMEOUT):
        raise scratch.ScratchError("control.request-refused")
    mount.used = True
    mount.control.settimeout(timeout + scratch.SETUP_SECONDS)
    mount.control.send(json.dumps({"mode": "historical-control", "rust": rust, "timeout": timeout}).encode())
    data, ancillary, flags, _ = mount.control.recvmsg(4096)
    if ancillary or flags & (socket.MSG_TRUNC | socket.MSG_CTRUNC):
        raise scratch.ScratchError("control.outcome-refused")
    try:
        result = json.loads(data)
    except ValueError:
        raise scratch.ScratchError("control.outcome-refused") from None
    if (type(result) is not list or len(result) != 3
            or result[0] is not None and type(result[0]) is not int
            or any(type(value) is not bool for value in result[1:])):
        raise scratch.ScratchError("control.outcome-refused")
    return tuple(result)


def grade_case(task, files, output):
    try:
        return mounted_grade_case(task, files, output)
    except scratch.ScratchError as error:
        raise evaluation.EvaluationError(str(error)) from error


def mounted_grade_case(task, files, output):
    """Stage only verified controls; publication follows anonymous mount release."""
    rust = task.get("language") == "rust"
    driver = evaluation.task_driver(task)
    if rust:
        # One compilation job fits the bounded worker's process/thread budget.
        # The historical driver, source, corpus and test thread count stay
        # unchanged; only this trusted runtime copy narrows build concurrency.
        anchor = b'"--jobs", "2"'
        if driver.count(anchor) != 1:
            raise evaluation.EvaluationError("control.compiler-driver-refused")
        driver = driver.replace(anchor, b'"--jobs", "1"', 1)
    driver = limited_driver(driver, rust)
    hidden = evaluation.pinned_blob(task.get("hidden_revision", task["reference"]),
                                    task["hidden_test"], task["hidden_sha256"])
    # Verify every extra Rust input before starting the supervisor, preserving
    # prelaunch input-integrity refusal rather than misclassifying a failed test.
    fixture = (evaluation.pinned_blob(task["hidden_revision"], task["fixture"], task["fixture_sha256"])
               if rust else None)
    runtime = evaluation.coding_eval_rust.runtime_paths(evaluation) if rust else None
    tests = data = None
    with scratch.scratch_mount(output, runtime_paths=runtime) as mount:
        root = mount.root
        workspace, grader, results = (root / name for name in ("workspace", "grader", "results"))
        for directory in (workspace, grader, results):
            directory.mkdir(mode=0o700)
        for name, content in files.items():
            evaluation.write_file(workspace / name, content)
        if rust:
            evaluation.write_file(workspace / "kernel/contracts/tests/eval_hidden.rs", hidden)
            evaluation.write_file(workspace / task["fixture"], fixture)
        evaluation.write_file(grader / "test.py", hidden)
        evaluation.write_file(grader / "run.py", driver)
        outcome = run(mount, rust, evaluation.TIMEOUT)
        try:
            data = evaluation.read_regular(results / "tests.json", 4096)
            tests = json.loads(data)
            fields = {"tests", "failures", "errors", "skipped", "expected_failures", "unexpected_successes"}
            if (type(tests) is not dict or tests.keys() != fields
                    or any(type(value) is not int or value < 0 for value in tests.values())
                    or tests["tests"] != task["expected_tests"]):
                raise ValueError
        except (ValueError, OSError):
            tests = None
    if tests is not None:
        evaluation.write_file(output / "tests.json", data)
    return *outcome, tests


def policy(rust):
    processes = RUST_PROCESSES if rust else PYTHON_PROCESSES
    address = RUST_ADDRESS_BYTES if rust else 512 * evaluation.MAX_FILE
    return {"version": VERSION, "processes_per_namespace": processes,
            "address_bytes_per_process": address, "derived_address_bytes_per_namespace": processes * address,
            "cpu_seconds_per_process": 60, "wall_seconds": evaluation.TIMEOUT,
            "case_scratch_bytes": scratch.SCRATCH_BYTES, "case_scratch_inodes": scratch.SCRATCH_INODES,
            "private_tmp_bytes": RUST_TMP_BYTES if rust else scratch.PRIVATE_BYTES,
            "private_tmp_inodes": RUST_TMP_INODES if rust else scratch.PRIVATE_INODES,
            "runtime_snapshot_bytes": RUST_RUNTIME_BYTES if rust else 0,
            "runtime_snapshot_inodes": RUST_RUNTIME_INODES if rust else 0,
            "compiler_jobs": 1 if rust else None,
            "candidate_tree_cleanup": "anonymous-mount-release", "resident_memory_accounting": False,
            "aggregate_cpu_accounting": False, "generated_scoring": False}
