"""Check the frozen development/held-out split and validate historical controls.

Held-out inputs are private operator data bound to a public digest. This command
does not admit generated candidates or produce model scores. Held-out control
validation requires an explicit row-closing invocation; it is never part of the
development fast gate.
"""
from __future__ import annotations

import argparse
import json
import os
from pathlib import Path
import re
import signal
import stat
import subprocess
import sys
import time

if __package__:
    from . import coding_eval as evaluation
    from . import coding_eval_oracle as oracle
else:
    import coding_eval as evaluation
    import coding_eval_oracle as oracle

VERSION = "coding-suite-v2"
DEVELOPMENT_SHA256 = "0c2a0624df93085540f61c4677260490a9810a0325efd6bcb75db0dce1461570"
HELD_OUT_SHA256 = "8141e9281c830fe510863403d6c1f137dcb661c2ba6067e68a94a578a13e5c3b"
HELD_OUT_COUNT = 10
HELD_OUT_METHODS = 52
TASK_FIELDS = frozenset({
    "id", "split", "base", "reference", "source", "hidden_test", "title", "statement",
    "test_names", "expected_tests", "hidden_sha256", "base_sha256", "reference_sha256",
    "projection_sha256",
})


def canonical_digest(value):
    return evaluation.sha(json.dumps(value, sort_keys=True, separators=(",", ":")).encode())


def refuse(code):
    raise evaluation.InputIntegrityError("suite." + code)


def development_tasks():
    tasks = evaluation.collection()
    if len(tasks) != 30 or canonical_digest(tasks) != DEVELOPMENT_SHA256:
        refuse("development-changed")
    return tasks


def unique_object(pairs):
    result = {}
    for key, value in pairs:
        if key in result:
            refuse("duplicate-key")
        result[key] = value
    return result


def resolved_location(path):
    resolved = Path(path).resolve()
    # Some runtimes leave a symlink loop unresolved in non-strict mode. Check
    # the nearest existing component explicitly, while allowing a new output
    # or an unavailable registered worktree. Only absence may be ignored.
    ancestor = resolved
    while True:
        try:
            ancestor.stat()
        except FileNotFoundError:
            if ancestor == ancestor.parent:
                raise
            ancestor = ancestor.parent
        else:
            return resolved


def editing_roots(workspace_roots=()):
    """Include registered worktrees even when they are currently unmounted."""
    raw = evaluation.git("worktree", "list", "--porcelain", "-z")
    paths = [Path(os.fsdecode(field[len(b"worktree "):]))
             for field in raw.split(b"\0") if field.startswith(b"worktree ")]
    if not paths or any(not path.is_absolute() for path in paths):
        refuse("worktree-discovery-refused")
    try:
        return {resolved_location(path) for path in (evaluation.ROOT, *paths, *workspace_roots)}
    except (OSError, RuntimeError):
        refuse("worktree-discovery-refused")


def private_location(path, workspace_roots=()):
    """The operator owns these paths; resolve aliases before opening or mkdir."""
    try:
        resolved = resolved_location(path)
    except (OSError, RuntimeError):
        refuse("private-location-refused")
    if any(resolved.is_relative_to(root) for root in editing_roots(workspace_roots)):
        refuse("private-location-exposed")


def private_bytes(path, workspace_roots=()):
    private_location(path, workspace_roots)
    descriptor = os.open(path, os.O_RDONLY | os.O_NOFOLLOW | os.O_NONBLOCK)
    with os.fdopen(descriptor, "rb") as stream:
        before = os.fstat(stream.fileno())
        if (not stat.S_ISREG(before.st_mode) or before.st_nlink != 1
                or before.st_uid != os.geteuid() or stat.S_IMODE(before.st_mode) & 0o077
                or before.st_size > evaluation.MAX_FILE):
            refuse("private-input-refused")
        data = stream.read(evaluation.MAX_FILE + 1)
        after = os.fstat(stream.fileno())
    fields = ("st_dev", "st_ino", "st_size", "st_mtime_ns", "st_ctime_ns")
    if (len(data) != before.st_size or len(data) > evaluation.MAX_FILE
            or any(getattr(before, field) != getattr(after, field) for field in fields)):
        refuse("private-input-changed")
    return data


def relative_file(value, prefix):
    return (type(value) is str and value.startswith(prefix + "/") and value.endswith(".py")
            and all(re.fullmatch(r"[A-Za-z_][A-Za-z0-9_]*", part) for part in value[:-3].split("/")))


def held_out_tasks(path, workspace_roots=()):
    raw = private_bytes(path, workspace_roots)
    if evaluation.sha(raw) != HELD_OUT_SHA256:
        refuse("held-out-changed")
    value = json.loads(raw, object_pairs_hook=unique_object)
    if (type(value) is not dict or value.keys() != {"version", "origin", "tasks"}
            or value["version"] != "coding-held-out-v1"
            or value["origin"] != "original-repository-history"
            or type(value["tasks"]) is not list or len(value["tasks"]) != HELD_OUT_COUNT):
        refuse("held-out-shape")
    tasks = value["tasks"]
    for index, task in enumerate(tasks, 1):
        if (type(task) is not dict or task.keys() != TASK_FIELDS
                or task["id"] != f"held-{index:03}" or task["split"] != "held-out"
                or not relative_file(task["source"], "scripts")
                or not relative_file(task["hidden_test"], "tests")
                or any(type(task[key]) is not str or re.fullmatch(pattern, task[key]) is None
                       for keys, pattern in ((('base', 'reference'), r"[0-9a-f]{40}"),
                                             (('hidden_sha256', 'base_sha256', 'reference_sha256',
                                               'projection_sha256'), r"[0-9a-f]{64}"))
                       for key in keys)
                or any(type(task[key]) is not str or not task[key].strip()
                       for key in ("title", "statement"))
                or task["base_sha256"] == task["reference_sha256"]):
            refuse("held-out-task-shape")
        # Reuse the original full-method selection validator; no new test code,
        # extraction, score-dependent filters or threshold overrides.
        evaluation.task_driver(task)
    if (len({(task["base"], task["source"]) for task in tasks}) != HELD_OUT_COUNT
            or len({task["reference"] for task in tasks}) != HELD_OUT_COUNT
            or sum(task["expected_tests"] for task in tasks) != HELD_OUT_METHODS):
        refuse("held-out-identity")
    return tasks


def is_ancestor(ancestor, descendant):
    result = subprocess.run(
        ["git", "--no-optional-locks", "merge-base", "--is-ancestor", ancestor, descendant],
        cwd=evaluation.ROOT, stdin=subprocess.DEVNULL, stdout=subprocess.DEVNULL,
        stderr=subprocess.DEVNULL, timeout=30,
    )
    if result.returncode not in (0, 1):
        refuse("ancestry-unavailable")
    return result.returncode == 0


def check_inputs(held_out, workspace_roots=()):
    """Check pins and split leakage without running any held-out assertion."""
    development = development_tasks()
    tasks = held_out_tasks(held_out, workspace_roots)
    development_sources = set()
    development_changes = {(task["base"], task["source"]) for task in development}
    development_revisions = {revision for task in development
                             for revision in (task["base"], task.get("hidden_revision", task["reference"]))}
    for task in development:
        files = evaluation.projection(task)
        development_sources.update((name, evaluation.sha(data)) for name, data in files.items())
        evaluation.pinned_blob(task["reference"], task["source"], task["reference_sha256"])
        evaluation.pinned_blob(task.get("hidden_revision", task["reference"]),
                               task["hidden_test"], task["hidden_sha256"])
        if "fixture" in task:
            evaluation.pinned_blob(task["hidden_revision"], task["fixture"], task["fixture_sha256"])
    for task in tasks:
        if ((task["base"], task["source"]) in development_changes
                or (task["source"], task["reference_sha256"]) in development_sources):
            refuse("split-leakage")
        if evaluation.git("rev-parse", task["reference"] + "^").decode().strip() != task["base"]:
            refuse("held-out-parent")
        if any(is_ancestor(task["reference"], revision) for revision in sorted(development_revisions)):
            refuse("split-ancestry-leakage")
        files = evaluation.projection(task)
        evaluation.pinned_blob(task["reference"], task["source"], task["reference_sha256"])
        evaluation.pinned_blob(task["reference"], task["hidden_test"], task["hidden_sha256"])
        if task["hidden_test"] in files or any(name.startswith("tests/") for name in files):
            refuse("hidden-input-exposed")
    return {"version": VERSION, "kind": "frozen-split-input-check", "passed": True,
            "development_tasks": len(development), "held_out_tasks": len(tasks),
            "development_sha256": DEVELOPMENT_SHA256, "held_out_sha256": HELD_OUT_SHA256,
            "held_out_methods": HELD_OUT_METHODS, "held_out_test_methods_run": 0,
            "generated_scoring": False, "model_score": None, "model_attempts": 0}


def held_out_control(held_out, task_id, kind, output, workspace_roots=()):
    """Accept only an id and control kind, never candidate bytes or task overrides."""
    private_location(output, workspace_roots)
    tasks = held_out_tasks(held_out, workspace_roots)
    task = next((item for item in tasks if item["id"] == task_id), None)
    if task is None or kind not in ("base", "reference"):
        refuse("control-refused")
    files = evaluation.projection(task)
    if kind == "reference":
        files[task["source"]] = evaluation.pinned_blob(
            task["reference"], task["source"], task["reference_sha256"])
    # The legacy process executes only these pinned original bytes. Arbitrary
    # generated code can forge in-process tests and remains forbidden here.
    evaluation.new_directory(output)
    started = time.monotonic()
    error = None
    input_error = None
    try:
        exit_code, timed_out, limited, tests = evaluation.grade_case(task, files, output)
    except (ValueError, OSError, subprocess.SubprocessError) as exception:
        if isinstance(exception, evaluation.InputIntegrityError):
            input_error = evaluation.refusal_detail(exception)
        else:
            error = evaluation.refusal_detail(exception)
        exit_code, timed_out, limited, tests = None, False, False, None
        (output / "tests.json").unlink(missing_ok=True)
    passed = (error is None and input_error is None and exit_code == 0
              and not timed_out and not limited and tests is not None
              and all(value == 0 for key, value in tests.items() if key != "tests"))
    cause = ("none" if passed else "input_integrity" if input_error else "timeout" if timed_out
             else "output_limit" if limited else "infrastructure"
             if error or tests is None else "test_failure")
    report = {"version": VERSION, "task": task_id, "split": "held-out", "passed": passed,
              "cause": cause, "exit_code": exit_code, "tests": tests,
              "candidate_kind": "frozen-" + kind, "candidate_sha256": evaluation.sha(files[task["source"]]),
              "task_sha256": canonical_digest(task), "held_out_sha256": HELD_OUT_SHA256,
              "suite_sha256": evaluation.sha(Path(__file__).read_bytes()),
              "evaluator_sha256": evaluation.sha(Path(evaluation.__file__).read_bytes()),
              "hidden_sha256": task["hidden_sha256"], "test_names": task["test_names"],
              "input_error": input_error, "infrastructure_error": error,
              "output_truncated": limited, "wall_seconds": round(time.monotonic() - started, 6),
              "model_score": None, "model_attempts": 0, "tokens": None, "gpu_minutes": 0,
              "score_scope": "frozen-held-out-controls-only",
              "logs": {name: evaluation.sha(evaluation.read_regular(output / name, 2 * evaluation.MAX_FILE))
                       for name in ("stdout.log", "stderr.log") if (output / name).exists()}}
    evaluation.publish(output / "report.json", report)
    return report


def controls_valid(base, reference):
    tests = base["tests"]
    return (not base["passed"] and base["cause"] == "test_failure" and base["exit_code"] == 1
            and tests is not None and tests["tests"] > 0 and tests["skipped"] == 0
            and tests["expected_failures"] == 0 and tests["unexpected_successes"] == 0
            and tests["failures"] + tests["errors"] > 0
            and reference["passed"] and reference["cause"] == "none")


def development_controls_valid(summary):
    tasks = development_tasks()
    expected = {task["id"]: task["expected_tests"] for task in tasks}
    rows = summary["tasks"]
    return (summary["passed"] and len(rows) == len(tasks)
            and {row["task"] for row in rows} == expected.keys()
            and all(controls_valid(row["base"], row["reference"])
                    and row["base"]["tests"]["tests"] == expected[row["task"]]
                    and row["reference"]["tests"]["tests"] == expected[row["task"]]
                    for row in rows))


def verify(held_out, output, purpose, workspace_roots=()):
    if purpose != "row-close":
        refuse("held-out-purpose-refused")
    private_location(output, workspace_roots)
    inputs = check_inputs(held_out, workspace_roots)
    evaluation.new_directory(output)
    started = time.monotonic()
    development = evaluation.verify(output / "development")
    development_valid = development_controls_valid(development)
    independent = oracle.verify(output / "independent-development")
    private_output = output / "held-out"
    evaluation.new_directory(private_output)
    rows = []
    for task in held_out_tasks(held_out, workspace_roots):
        base = held_out_control(held_out, task["id"], "base", private_output / (task["id"] + "-base"), workspace_roots)
        reference = held_out_control(held_out, task["id"], "reference", private_output / (task["id"] + "-reference"), workspace_roots)
        rows.append({"task": task["id"], "base": base, "reference": reference,
                     "valid": controls_valid(base, reference)})
    evaluation.publish(private_output / "controls.json", {"tasks": rows})
    # Aggregate metadata is safe to inspect without supplying task-level held-out
    # feedback to future harness/prompt tuning. Raw files stay private.
    summary = {**inputs, "kind": "complete-suite-historical-controls-only",
               "passed": development["passed"] and development_valid and independent["passed"] and all(row["valid"] for row in rows),
               "development_controls_valid": development["passed"] and development_valid,
               "independent_development_controls_valid": independent["passed"],
               "independent_development_tasks": len(independent["tasks"]),
               "held_out_controls_valid": all(row["valid"] for row in rows),
               "held_out_test_methods_run": sum(case["tests"]["tests"] if case["tests"] else 0
                                                for row in rows for case in (row["base"], row["reference"])),
               "control_attempts": 2 * (len(development["tasks"]) + len(independent["tasks"]) + len(rows)),
               "input_refused": (development["input_refused"] or independent["input_refused"]
                                 or any(case["cause"] == "input_integrity" for row in rows
                                        for case in (row["base"], row["reference"]))),
               "wall_seconds": round(time.monotonic() - started, 6),
               "aggregate_worker_quotas": False,
               "suite_sha256": evaluation.sha(Path(__file__).read_bytes()),
               "evaluator_sha256": evaluation.sha(Path(evaluation.__file__).read_bytes()),
               "oracle_sha256": evaluation.sha(Path(oracle.__file__).read_bytes()),
               "admission_blockers": ["remaining-independent-development-adapters", "held-out-independent-adapters",
                                      "aggregate-worker-quotas", "scratch-quota-and-cleanup-deadline",
                                      "host-mount-metadata", "coverage-and-known-race-reporting"],
               "logs": {name: evaluation.sha(evaluation.read_regular(output / name, 16 * evaluation.MAX_FILE))
                        for name in ("development/summary.json", "independent-development/summary.json",
                                     "held-out/controls.json")}}
    evaluation.publish(output / "summary.json", summary)
    return summary


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("operation", choices=("check", "verify"))
    parser.add_argument("--held-out", type=Path, required=True)
    parser.add_argument("--output", type=Path)
    parser.add_argument("--purpose", choices=("row-close",))
    parser.add_argument("--workspace-root", type=Path, action="append", default=[],
                        help="additional editing/model workspace to exclude from private storage; repeatable")
    args = parser.parse_args()
    if args.operation == "verify" and (args.output is None or args.purpose != "row-close"):
        parser.error("verify requires --output and --purpose row-close")
    try:
        result = (check_inputs(args.held_out, args.workspace_root) if args.operation == "check"
                  else verify(args.held_out, args.output, args.purpose, args.workspace_root))
        print(json.dumps(result, sort_keys=True))
        return 0 if result["passed"] else 2 if result["input_refused"] else 1
    except (OSError, ValueError, subprocess.SubprocessError) as error:
        print("suite.refused " + evaluation.refusal_detail(error), file=sys.stderr)
        return 2


if __name__ == "__main__":
    for termination_signal in (signal.SIGTERM, signal.SIGHUP):
        signal.signal(termination_signal, evaluation.interrupted)
    raise SystemExit(main())
