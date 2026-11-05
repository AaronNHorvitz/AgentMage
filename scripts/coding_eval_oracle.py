"""Validate an independent function oracle on frozen development controls.

Candidate source and hidden assertions run in separate networkless namespaces.
Generated scoring stays disabled until all remaining admission requirements hold.
"""
from __future__ import annotations

import argparse
from array import array
from contextlib import contextmanager
import ctypes
import json
import os
from pathlib import Path
import resource
import selectors
import signal
import stat
import subprocess
import sys
import tempfile
import time

if __package__:
    from . import coding_eval as evaluation
    from . import coding_eval_oracle_worker as worker
    from . import coding_eval_scratch as scratch
else:
    import coding_eval as evaluation
    import coding_eval_oracle_worker as worker
    import coding_eval_scratch as scratch

VERSION = "coding-independent-oracle-v30"
# Complete frozen methods for finite function and fixture observations. Paths
# are lexical values; a fixed ambiguity tag becomes a trusted local exception.
SUPPORTED = {
    "dev-001": {"function": "untracked_file_identities", "result_type": "source-result", "source_fixture": "identity"},
    "dev-002": {"function": "_model_request_diagnosis", "result_type": "source-result", "source_fixture": "diagnostics"},
    "dev-005": {"function": "capture", "result_type": "capture-result"},
    "dev-006": {"function": "route_grant_arguments", "result_type": "argument-result"},
    "dev-007": {"function": "memory_arguments", "result_type": "argument-result"},
    "dev-008": {"function": "doc_pack_arguments", "result_type": "fixture-result"},
    "dev-009": {"function": "subprocess_runner", "result_type": "diagnostic-result"},
    "dev-010": {"function": "test_only_files", "result_type": "path-result"},
    "dev-011": {"function": "retain", "result_type": "source-result", "source_fixture": "retention"},
    "dev-012": {"function": "test_only_files", "result_type": "path-result"},
    "dev-013": {"function": "confinement_diagnosis", "result_type": "native-result"},
    "dev-014": {"function": "complete_source_closure", "result_type": "source-result", "source_fixture": "closure"},
    "dev-015": {"function": "case_passed", "result_type": "bool"},
    "dev-016": {"function": "OpenAIAdapter", "result_type": "usage-result"},
    "dev-017": {"function": "make_container_readable", "result_type": "mode-result"},
    "dev-018": {"function": "capture", "result_type": "capture-result"},
    "dev-019": {"function": "extract_deb", "result_type": "command-result"},
    "dev-020": {"function": "verify_case", "result_type": "verification-result"},
    "dev-021": {"function": "verify_case", "result_type": "verification-result"},
    "dev-022": {"function": "verify_case", "result_type": "verification-result", "observe_case_roots": True},
    "dev-023": {"function": "implementation_identity", "result_type": "bool-map", "identity_fixture": True},
    "dev-024": {"function": "gpu_memory", "result_type": "resource-result"},
    "dev-025": {"function": "setup", "result_type": "fixture-result"},
    "dev-026": {"function": "run_commands", "result_type": "deadline-result", "observe_commands": True},
    "dev-027": {"function": "extension_arguments", "result_type": "argument-result"},
    "dev-028": {"function": "control", "result_type": "job-result"},
    "dev-029": {"function": "suspend_resume_checks", "result_type": "bool-map"},
}
LOG_LIMIT = 2 * evaluation.MAX_FILE
DIRECT_WALK_DEPTH = 256
PRIVATE_TMP_BYTES = 64 * evaluation.MAX_FILE
WORKER_PROCESSES = 8
# Install the hard process quota after namespace creation and before importing
# any task code. Lowering it on the host launcher would count unrelated work.
WORKER_LIMIT_PREFIX = (
    "import resource as _coding_quota_resource\n"
    f"_coding_quota_resource.setrlimit(_coding_quota_resource.RLIMIT_NPROC, ({WORKER_PROCESSES}, {WORKER_PROCESSES}))\n"
).encode("ascii")
CLEANUP_SECONDS = 5
CLEANUP_ADDRESS_BYTES = 256 * evaluation.MAX_FILE
# Measured historical outcomes, not model scores or altered hidden assertions.
# Subtests can report more failures than the number of selected methods.
CONTROL_PROFILES = {
    "dev-001": {"failures": 0, "errors": 3, "base_calls": 4, "reference_calls": 7,
                "function_found": False, "candidate_exception": True},
    "dev-002": {"failures": 0, "errors": 14, "base_calls": 14, "reference_calls": 53,
                "function_found": False, "candidate_exception": True},
    "dev-011": {"failures": 0, "errors": 2, "base_calls": 1, "reference_calls": 18},
    "dev-014": {"failures": 0, "errors": 27, "base_calls": 28, "reference_calls": 46,
                "function_found": False, "candidate_exception": True},
    "dev-005": {"failures": 3, "base_calls": 3, "reference_calls": 3},
    "dev-006": {"failures": 0, "errors": 1, "base_calls": 1, "reference_calls": 19,
                "function_found": False},
    "dev-007": {"failures": 0, "errors": 1, "base_calls": 1, "reference_calls": 52,
                "function_found": False},
    "dev-008": {"failures": 0, "errors": 2, "base_calls": 2, "reference_calls": 40,
                "function_found": False, "candidate_exception": True},
    "dev-009": {"failures": 1, "base_calls": 1, "reference_calls": 1},
    "dev-010": {"failures": 1, "base_calls": 1, "reference_calls": 4},
    "dev-012": {"failures": 10, "base_calls": 12, "reference_calls": 14},
    "dev-013": {"failures": 0, "errors": 8, "base_calls": 8, "reference_calls": 12,
                "function_found": False, "candidate_exception": True},
    "dev-015": {"failures": 1, "base_calls": 3, "reference_calls": 4},
    "dev-016": {"failures": 0, "errors": 2, "base_calls": 5, "reference_calls": 5,
                "candidate_exception": True, "function_found": True},
    "dev-017": {"failures": 0, "errors": 1, "base_calls": 1, "reference_calls": 1,
                "function_found": False, "candidate_exception": True},
    "dev-018": {"failures": 1, "base_calls": 1, "reference_calls": 5},
    "dev-019": {"failures": 1, "base_calls": 1, "reference_calls": 1},
    "dev-020": {"failures": 1, "base_calls": 2, "reference_calls": 3},
    "dev-021": {"failures": 0, "errors": 1, "base_calls": 1, "reference_calls": 6},
    "dev-022": {"failures": 0, "errors": 1, "base_calls": 2, "reference_calls": 4},
    "dev-023": {"failures": 0, "errors": 1, "base_calls": 1, "reference_calls": 1,
                "function_found": False, "candidate_exception": True},
    "dev-024": {"failures": 0, "errors": 2, "base_calls": 2, "reference_calls": 6,
                "function_found": False, "candidate_exception": True},
    "dev-025": {"failures": 0, "errors": 1, "base_calls": 1, "reference_calls": 3,
                "function_found": True, "candidate_exception": True},
    "dev-026": {"failures": 1, "base_calls": 3, "reference_calls": 3},
    "dev-027": {"failures": 0, "errors": 1, "base_calls": 1, "reference_calls": 37,
                "function_found": False},
    "dev-028": {"failures": 0, "errors": 1, "base_calls": 13, "reference_calls": 18},
    "dev-029": {"failures": 0, "base_calls": 1, "reference_calls": 6,
                "candidate_exception": True},
}


def coverage(task):
    """State what transported observations can and cannot establish."""
    kind = SUPPORTED[task["id"]]["result_type"]
    if kind == "source-result":
        return {"graded": (["selected-untracked-identities-and-refusals", "implementation-identity-fields",
                            "changed-untracked-content-and-ignored-output"] if task["id"] == "dev-001" else
                           ["requested-model-values-and-refusals", "diagnosis-schema-and-fixture-lifecycle",
                            "run-record-invariance"] if task["id"] == "dev-002" else
                           ["selected-retain-and-load-refusals", "retained-stage-fields-and-files",
                            "changed-source-and-private-field-checks"] if task["id"] == "dev-011" else
                           ["selected-source-closure-values-and-refusals", "schema-and-report-validation",
                            "synthetic-git-child-mode-and-document-closure"]),
                "adapter_policy": ["finite-typed-observations", "closed-exception-categories",
                                   "inert-mock-only-call-replay", "read-only-trusted-temporary-fixtures",
                                   "read-only-candidate-output-observations",
                                   "bounded-data-only-output-replay"],
                "not_graded": ["unselected-statement-clauses", "real-installed-worker",
                               "exception-subclass-identity", "effects-outside-declared-retain-target",
                               "concurrent-fixture-mutations",
                               *(["candidate-process-ownership-implementation", "candidate-setup-and-reservation-implementation",
                                  "model-choice-constant-membership-and-order"]
                                 if task["id"] == "dev-002" else
                                 ["unselected-harness-functions", "atomic-source-snapshot",
                                  "candidate-gpu-and-acceptance-helper-implementations"] if task["id"] == "dev-001" else
                                 ["unasserted-closure-return-fields"] if task["id"] == "dev-014" else
                                 ["attachments-and-effects-outside-retain-target"])]}
    if kind == "native-result":
        return {"graded": ["selected-native-path-metadata-refusals", "manager-observation-values",
                           "manager-call-timeout-output-and-environment", "no-probe-on-untrusted-systemctl"],
                "adapter_policy": ["finite-metadata-fixtures", "inert-manager-process-stub",
                                   "trusted-mock-only-call-replay", "bounded-json-observations"],
                "not_graded": ["real-native-path-or-user-manager-trust", "unasserted-call-arguments",
                               "unselected-statement-clauses"]}
    if kind == "fixture-result":
        return {"graded": (["selected-catalog-parser-values-and-refusals", "catalog-command-arguments",
                            "sample-manifest-seal-and-file-bytes", "selected-file-mode-and-target-refusals"]
                           if task["id"] == "dev-008" else
                           ["selected-fixture-file-presence", "generated-validation-nonzero-exit"]),
                "adapter_policy": ["read-only-candidate-fixture-observation", "closed-fixture-process-stubs",
                                   "bounded-observations", "candidate-only-validation-execution"],
                "not_graded": ["unselected-statement-clauses", "real-agent-execution",
                               "git-history-and-reservation-correctness", "diagnosis-correctness",
                               *(["catalog-command-presence-and-count"] if task["id"] == "dev-008" else
                                 ["validation-result-schema", "setup-return-value-and-directory-modes"])]}
    if task["id"] == "dev-023":
        return {"graded": ["identity-field-counts-and-lengths", "wrapper-path-value"],
                "adapter_policy": ["bounded-json-observations", "closed-binary-fixture"],
                "not_graded": ["actual-identity-digest-content", "git-invocation-flags",
                               "concurrent-file-mutation", "start-recording",
                               "unselected-statement-clauses"]}
    if kind == "usage-result":
        return {"graded": ["selected-token-count-values", "repeated-message-cache",
                           "transport-call-count-and-selected-probe-payload",
                           "context-error-token-count", "repeated-message-root-identity"],
                "adapter_policy": ["bounded-encoded-response", "finite-transport-stubs",
                                   "bounded-tagged-observations", "exclusive-inert-transport-mock",
                                   "trusted-mock-history-drift-refusal", "candidate-message-mutation-refusal",
                                   "sticky-usage-input-encoding-refusal"],
                "not_graded": ["real-model-or-network", "transport-status-filter-implementation",
                               "unasserted-transport-arguments", "invalid-count-and-error-refusals",
                               "non-usage-tokenization", "adapter-attributes-and-other-methods",
                               "nested-object-identity-and-equality-subclasses",
                               "candidate-message-mutation-roundtrip",
                               "cross-instance-post-response-mutations", "unselected-statement-clauses"]}
    if kind == "job-result":
        return {"graded": ["selected-parser-values", "export-selection-values-and-refusals",
                           "support-directory-values-and-refusals", "mocked-job-signal-order-and-values",
                           "unknown-control-and-no-live-refusals"],
                "adapter_policy": ["bounded-encoded-response", "finite-job-stubs",
                                   "bounded-tagged-observations", "finite-immediate-temporary-directory-fixtures",
                                   "expired-signal-mock-refusal"],
                "not_graded": ["real-signal-delivery", "process-identity-and-liveness",
                               "signal-enum-type", "support-bundle-start-propagation-and-approval",
                               "support-bundle-privacy-and-ownership", "exception-messages",
                               "unselected-statement-clauses"]}
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


def limited_source(source):
    """Prefix only the trusted worker copy; hidden/candidate bytes stay intact."""
    anchor = b"from __future__ import annotations\n"
    if anchor in source:
        return source.replace(anchor, anchor + WORKER_LIMIT_PREFIX, 1)
    return WORKER_LIMIT_PREFIX + source


def sandbox(workspace, adapter, output, role, candidate_tmp=None, private_tmp=None, trusted_tmp=None):
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
    elif private_tmp is not None:
        command.extend(["--bind", str(private_tmp), "/tmp"])
    else:
        command.extend(["--size", str(PRIVATE_TMP_BYTES), "--tmpfs", "/tmp"])
    if candidate_tmp is not None and role == "oracle":
        command.extend(["--ro-bind", str(candidate_tmp), "/candidate-tmp"])
    if trusted_tmp is not None and role == "candidate":
        command.extend(["--ro-bind", str(trusted_tmp), "/trusted-tmp"])
    if role == "oracle":
        command.extend(["--bind", str(output), "/output"])
    elif role != "candidate":
        raise evaluation.EvaluationError("oracle.role-refused")
    # Private scratch must not be bypassed by creating ordinary files in the
    # synthetic root or device mount. Remounts are non-recursive: /tmp and the
    # trusted result mount keep their explicitly selected access.
    command.extend(["--remount-ro", "/dev", "--remount-ro", "/"])
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


def exchange(candidate_command, oracle_command, output, timeout, *, mount=None, identity=False):
    """Connect pipes directly; the host accepts results only from the oracle mount."""
    if mount is not None:
        return mount.exchange(candidate_command, oracle_command, timeout, identity=identity)
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


def detached_fixture_walk(current, parent, name, staging_parent, *, remove):
    """Walk a deep disposable subtree through shallow, private staging names.

    Bind-mount '..' lookup can itself walk all ancestors in the kernel. Detach
    directories into a host-only sibling, verifying each moved inode before use.
    Each record stores one basename and parent index, never an ancestor path.
    Repair restores children before parents; removal deletes emptied parents.
    Worker-held descriptors can still mutate their own inodes, so every unsafe
    move or failed operation refuses cleanup rather than publishing counts.
    """
    flags = os.O_PATH | os.O_DIRECTORY | os.O_NOFOLLOW | os.O_CLOEXEC
    records, entries = [], []
    stage_fd = node = None
    stage = None

    def identity(info):
        return info.st_dev, info.st_ino

    def check(label, expected, directory):
        if identity(os.stat(label, dir_fd=directory, follow_symlinks=False)) != expected:
            raise OSError('oracle.fixture-moved-during-staging')

    try:
        original_info = os.fstat(current)
        if not stat.S_ISDIR(original_info.st_mode):
            raise OSError('oracle.fixture-not-directory')
        original = identity(original_info)
        check(name, original, parent)
        os.chmod(f'/proc/self/fd/{current}', 0o700)
        stage = staging_parent / ('oracle-fixture-stage-' + os.urandom(16).hex())
        stage.mkdir(mode=0o700)
        stage_fd = os.open(stage, os.O_RDONLY | os.O_DIRECTORY | os.O_NOFOLLOW | os.O_CLOEXEC)
        os.rename(name, '0', src_dir_fd=parent, dst_dir_fd=stage_fd)
        check('0', original, stage_fd)
        records.append((None, None, *original))
        index = 0
        while index < len(records):
            label = str(index)
            expected = records[index][2:]
            node = os.open(label, flags, dir_fd=stage_fd)
            if identity(os.fstat(node)) != expected:
                raise OSError('oracle.fixture-moved-during-staging')
            os.chmod(f'/proc/self/fd/{node}', 0o700)
            readable = os.open('.', os.O_RDONLY | os.O_DIRECTORY | os.O_CLOEXEC, dir_fd=node)
            try:
                with os.scandir(readable) as scan:
                    entries = list(scan)
            finally:
                os.close(readable)
            for entry in entries:
                if not entry.is_dir(follow_symlinks=False):
                    if remove:
                        os.unlink(entry.name, dir_fd=node)
                    continue
                try:
                    child = os.open(entry.name, flags, dir_fd=node)
                except (FileNotFoundError, NotADirectoryError):
                    # Concurrent replacement is never followed or chmodded.
                    # Any surviving entry makes removal fail closed below.
                    continue
                try:
                    child_identity = identity(os.fstat(child))
                    # Moving a directory changes its '..' entry and requires
                    # write/search permission on that pinned child as well.
                    os.chmod(f'/proc/self/fd/{child}', 0o700)
                    child_label = str(len(records))
                    os.rename(entry.name, child_label, src_dir_fd=node, dst_dir_fd=stage_fd)
                    check(child_label, child_identity, stage_fd)
                    records.append((index, entry.name, *child_identity))
                finally:
                    os.close(child)
            entries.clear()
            check(label, expected, stage_fd)
            if remove:
                os.rmdir(label, dir_fd=stage_fd)
            os.close(node)
            node = None
            index += 1
        if not remove:
            for index in range(len(records) - 1, 0, -1):
                parent_index, original_name, device, inode = records[index]
                node = os.open(str(parent_index), flags, dir_fd=stage_fd)
                if identity(os.fstat(node)) != records[parent_index][2:]:
                    raise OSError('oracle.fixture-moved-during-staging')
                check(str(index), (device, inode), stage_fd)
                os.rename(str(index), original_name, src_dir_fd=stage_fd, dst_dir_fd=node)
                os.close(node)
                node = None
            check('0', original, stage_fd)
            os.rename('0', name, src_dir_fd=stage_fd, dst_dir_fd=parent)
    finally:
        entries.clear()
        records.clear()
        if node is not None:
            os.close(node)
        if stage_fd is not None:
            os.close(stage_fd)
    stage.rmdir()


def repair_fixture_permissions(root):
    """Repair disposable directories through pinned Linux directory descriptors.

    Killing/reaping bwrap does not synchronously reap all namespace descendants.
    O_PATH|O_DIRECTORY|O_NOFOLLOW pins only real directories, even mode 0000;
    chmod through our own proc descriptor addresses that inode despite renames.
    No candidate-controlled path is resolved by chmod. A depth-first walk keeps
    sibling names and parent inode identities, never full component paths. Each
    directory is scanned once; ascent opens '..' and verifies its identity before
    using it for another child. Work and retained names are linear in the tree,
    with constant descriptors regardless of depth. Concurrent replacement may
    omit a branch; cleanup still refuses publication if removal fails.
    """
    flags = os.O_PATH | os.O_DIRECTORY | os.O_NOFOLLOW | os.O_CLOEXEC

    def open_child(parent, name):
        try:
            return os.open(name, flags, dir_fd=parent)
        except (FileNotFoundError, NotADirectoryError):
            return None

    # Packed buffers avoid retaining a graph of Python containers at each
    # level. Frames contain (device, inode, names_start, names_cursor).
    parents, names = array('Q'), bytearray()
    current = os.open(root, flags)
    try:
        while True:
            os.chmod(f'/proc/self/fd/{current}', 0o700)
            if len(parents) >= 4 * DIRECT_WALK_DEPTH:
                parent_fd = os.open('..', flags, dir_fd=current)
                try:
                    parent = os.fstat(parent_fd)
                    if (parent.st_dev, parent.st_ino) != (parents[-4], parents[-3]):
                        raise OSError('oracle.fixture-moved-during-repair')
                    detached_fixture_walk(current, parent_fd, child_name, root.parent, remove=False)
                finally:
                    os.close(parent_fd)
                # The staged walk repaired and restored this entire subtree.
                start = len(names)
            else:
                # O_PATH cannot be scanned; open the same inode for reading.
                readable = os.open('.', os.O_RDONLY | os.O_DIRECTORY | os.O_CLOEXEC, dir_fd=current)
                try:
                    start = len(names)
                    with os.scandir(readable) as entries:
                        for entry in entries:
                            if entry.is_dir(follow_symlinks=False):
                                # Filesystem names cannot contain NUL or '/'.
                                names.extend(os.fsencode(entry.name))
                                names.append(0)
                finally:
                    os.close(readable)
            parent = os.fstat(current)
            parents.extend((parent.st_dev, parent.st_ino, start, len(names)))
            # Consume each name once without copying ancestor paths or the
            # remaining siblings. Truncate buffers only when a frame finishes.
            while True:
                child = None
                start, cursor = parents[-2], parents[-1]
                while cursor > start and child is None:
                    end = cursor - 1
                    cursor = max(start, names.rfind(b'\0', start, end) + 1)
                    child_name = bytes(names[cursor:end])
                    child = open_child(current, child_name)
                    parents[-1] = cursor
                if child is not None:
                    os.close(current)
                    current = child
                    break
                del names[start:]
                del parents[-4:]
                if not parents:
                    return
                parent_fd = os.open('..', flags, dir_fd=current)
                try:
                    parent = os.fstat(parent_fd)
                    if (parent.st_dev, parent.st_ino) != (parents[-4], parents[-3]):
                        raise OSError('oracle.fixture-moved-during-repair')
                except BaseException:
                    os.close(parent_fd)
                    raise
                os.close(current)
                current = parent_fd
    finally:
        # Exception tracebacks retain locals. Release pending names before the
        # enclosing recovery/removal path runs, including after MemoryError.
        names.clear()
        del parents[:]
        os.close(current)


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
            if len(parents) >= DIRECT_WALK_DEPTH:
                name, device, inode = parents[-1]
                parent = os.open('..', flags, dir_fd=current)
                try:
                    observed = os.fstat(parent)
                    if (observed.st_dev, observed.st_ino) != (device, inode):
                        raise OSError('oracle.fixture-moved-during-cleanup')
                    detached_fixture_walk(current, parent, name, root.parent, remove=True)
                except BaseException:
                    os.close(parent)
                    raise
                os.close(current)
                current = parent
                parents.pop()
                continue
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
        parents.clear()
        os.close(current)
    root.rmdir()


def cleanup_child(root, parent_pid):
    """Trusted, single-process cleanup; no candidate code is imported here."""
    # A killed parent must not leave a private cleanup process running. Install
    # this before touching any tree, then close the fork/parent-death race.
    libc = ctypes.CDLL(None, use_errno=True)
    if libc.prctl(1, signal.SIGKILL, 0, 0, 0) != 0 or os.getppid() != parent_pid:
        os._exit(2)
    os.setsid()
    for number in (signal.SIGINT, signal.SIGTERM, signal.SIGHUP, signal.SIGALRM):
        signal.signal(number, signal.SIG_DFL)
    # Fork inherits the trusted host interpreter, but no host stream/lock may
    # keep a reservation alive. Exit uses _exit, without inherited destructors.
    maximum = resource.getrlimit(resource.RLIMIT_NOFILE)[1]
    os.closerange(0, int(maximum))
    resource.setrlimit(resource.RLIMIT_CORE, (0, 0))
    resource.setrlimit(resource.RLIMIT_AS, (CLEANUP_ADDRESS_BYTES, CLEANUP_ADDRESS_BYTES))
    resource.setrlimit(resource.RLIMIT_CPU, (CLEANUP_SECONDS, CLEANUP_SECONDS))
    resource.setrlimit(resource.RLIMIT_NOFILE, (64, 64))
    repair_fixture_permissions(root)
    remove_fixture_tree(root)


def bounded_cleanup(root, timeout=CLEANUP_SECONDS):
    """Publish no result until cleanup finishes under a parent-owned deadline.

    The unreaped child reserves its PID until the final group signal. The child
    never starts subprocesses; the parent also kills its PID if setup fails
    before setsid. A timeout or repair/removal error retains the private tree.
    """
    parent_pid = os.getpid()
    child = os.fork()
    if child == 0:
        try:
            cleanup_child(root, parent_pid)
        except BaseException:
            os._exit(2)
        os._exit(0)
    deadline = time.monotonic() + timeout
    try:
        while True:
            status = os.waitid(os.P_PID, child, os.WEXITED | os.WNOHANG | os.WNOWAIT)
            if status is not None:
                if status.si_code != os.CLD_EXITED or status.si_status != 0:
                    raise evaluation.EvaluationError("oracle.cleanup-failed")
                return
            if time.monotonic() >= deadline:
                raise evaluation.EvaluationError("oracle.cleanup-timeout")
            time.sleep(0.01)
    finally:
        # The child may have exited before setsid. Kill both identities while
        # its unreaped PID still belongs to us, then reap exactly once.
        for kill in (os.killpg, os.kill):
            try:
                kill(child, signal.SIGKILL)
            except ProcessLookupError:
                pass
        os.waitpid(child, 0)


@contextmanager
def case_workspace(output):
    """Keep interrupted/failed cleanup bounded, including the enclosing case.

    Staging stays inside an owner-only container, so recovery records also own
    detached deep-tree fragments. No unbounded rmtree fallback runs afterward.
    """
    container = Path(tempfile.mkdtemp(prefix="agentmage-eval-"))
    root = container / "case"
    failed = False
    try:
        root.mkdir(mode=0o700)
        yield root
    except BaseException:
        failed = True
        raise
    finally:
        try:
            bounded_cleanup(root)
            container.rmdir()
        except (OSError, evaluation.EvaluationError):
            try:
                evaluation.publish(output / "cleanup.json", {
                    "removed": False, "attempts": 1, "private_workspace": str(container),
                    "wall_limit_seconds": CLEANUP_SECONDS,
                })
            except (OSError, evaluation.EvaluationError):
                if not failed:
                    raise
            if not failed:
                raise


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
            bounded_cleanup(directory)
        except (OSError, evaluation.EvaluationError):
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
    # All task-visible bind sources live on one anonymous quota-backed mount.
    # Its lifetime replaces traversal of candidate-controlled shared scratch;
    # the outer host-only container retains bounded lifecycle fault handling.
    with case_workspace(output), scratch.scratch_mount(output) as mount:
        root = mount.root
        fixture_options = {}
        if config.get("source_fixture"):
            fixture_options["trusted_tmp"] = root / "private-oracle"
        if config["result_type"] in {"mode-result", "fixture-result"} or config.get("source_fixture") in {"identity", "diagnostics", "retention"}:
            directory = root / "candidate-tmp"
            directory.mkdir(mode=0o700)
            fixture_options["candidate_tmp"] = directory
        trusted, candidate = root / "trusted", root / "candidate"
        oracle_adapter, candidate_adapter = root / "oracle-adapter", root / "candidate-adapter"
        results = root / "results"
        for directory in (trusted, candidate, oracle_adapter, candidate_adapter, results):
            directory.mkdir(mode=0o700)
        for workspace, content in ((trusted, trusted_files), (candidate, files)):
            for name, data in content.items():
                evaluation.write_file(workspace / name, data)
        if config.get("identity_fixture"):
            # The complete frozen method substitutes this one binary path.
            # Keep the same lexical module path and a private grading Git tree
            # on each side; only the candidate's inert dictionary crosses back.
            # Git initialization runs in the mount owner's namespace, where
            # these directories have reachable cwd paths.
            evaluation.write_file(candidate / "identity_entry.py", b'''
from pathlib import Path
from scripts import coding_harness as subject
subject.binary = lambda name: Path("/usr/bin/true")
if callable(getattr(subject, "implementation_identity", None)):
    implementation_identity = subject.implementation_identity
''')
        for directory in (oracle_adapter, candidate_adapter):
            evaluation.write_file(directory / "run.py", limited_source(Path(worker.__file__).read_bytes()))
            evaluation.write_file(directory / "source_adapter.py",
                                  Path(__file__).with_name("coding_eval_source_oracle.py").read_bytes())
        evaluation.publish(candidate_adapter / "config.json", {
            **config, **({"module": "identity_entry"} if config.get("identity_fixture") else {}),
        })
        evaluation.publish(oracle_adapter / "config.json", {
            **config, "test_names": task.get("test_names"), "expected_tests": task["expected_tests"],
        })
        evaluation.write_file(oracle_adapter / "test.py", hidden)
        outcome = exchange(sandbox(candidate, candidate_adapter, results, "candidate",
                                   private_tmp=root / "private-candidate", **fixture_options),
                           sandbox(trusted, oracle_adapter, results, "oracle",
                                   private_tmp=root / "private-oracle", **fixture_options), output, timeout,
                           mount=mount, identity=bool(config.get("identity_fixture")
                                                    or config.get("source_fixture") == "identity"))
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
        with evaluation.coding_eval_resources.measurement() as resource_observation:
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
        "hidden_sha256": task["hidden_sha256"], "test_names": task.get("test_names"),
        "candidate_sha256": evaluation.sha(files[task["source"]]),
        "task_sha256": evaluation.sha(json.dumps(task, sort_keys=True, separators=(",", ":")).encode()),
        "oracle_sha256": evaluation.sha(Path(__file__).read_bytes()),
        "worker_sha256": evaluation.sha(Path(worker.__file__).read_bytes()),
        "source_adapter_sha256": evaluation.sha(Path(__file__).with_name("coding_eval_source_oracle.py").read_bytes()),
        "materialized_worker_sha256": evaluation.sha(limited_source(Path(worker.__file__).read_bytes())),
        "scratch_sha256": evaluation.sha(Path(scratch.__file__).read_bytes()),
        "evaluator_sha256": evaluation.sha(Path(evaluation.__file__).read_bytes()),
        "resource_adapter_sha256": evaluation.sha(Path(evaluation.coding_eval_resources.__file__).read_bytes()),
        "inherited_lane_resources": resource_observation,
        "wall_seconds": round(time.monotonic() - started, 6),
        "model_attempts": 0, "model_score": None, "gpu_minutes": 0, "tokens": None,
        "score_scope": "independent-oracle-controls-only",
        # All launched diagnostic controls remain in the denominator, including
        # missing startup, infrastructure errors, timeout and protocol failure.
        # These counts are not model attempts or a generated-candidate score.
        "control_attempts": {"total": 1, "passed": int(passed), "failed": int(not passed)},
        "coverage": coverage(task),
        "aggregate_worker_quotas": False,
        "worker_limits": {"processes_per_namespace": WORKER_PROCESSES,
                          "address_bytes_per_process": 512 * evaluation.MAX_FILE,
                          "derived_address_bytes_per_namespace": WORKER_PROCESSES * 512 * evaluation.MAX_FILE,
                          "cpu_seconds_per_process": 60,
                          "wall_seconds_per_exchange": evaluation.TIMEOUT,
                          "scope": "independent-python-workers-only"},
        "cleanup_limits": {"wall_seconds_per_tree": CLEANUP_SECONDS,
                           "address_bytes": CLEANUP_ADDRESS_BYTES,
                           "cpu_seconds_per_tree": CLEANUP_SECONDS,
                           "open_files": 64},
        "scratch_limits": {"private_tmp_bytes_per_worker": PRIVATE_TMP_BYTES,
                           "private_tmp_inodes_per_worker": scratch.PRIVATE_INODES,
                           "shared_mode_fixture_quota": True,
                           "case_mount_bytes": scratch.SCRATCH_BYTES,
                           "case_mount_inodes": scratch.SCRATCH_INODES,
                           "candidate_tree_cleanup": "anonymous-mount-release"},
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
