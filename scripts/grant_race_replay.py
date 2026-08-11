#!/usr/bin/env python3
"""Build and verify grant race, result, and replay evidence."""

from __future__ import annotations

import argparse
import hashlib
import json
import os
import re
import subprocess
import sys
import tempfile
from pathlib import Path
from typing import Any


ROOT = Path(__file__).resolve().parents[1]
REPORT_PATH = ROOT / "artifacts/sprints/sprint-5/story-5.1/grant-race-replay-report.json"
MARKER = "AGENTMAGE_GRANT_RACE_REPLAY="
SCENARIOS = ("success", "denial", "timeout", "crash", "uncertain")
EXPECTED = {
    "success": {
        "result_outcome": "succeeded",
        "atomic_consumption_count": 1,
        "race_denial_scopes": ["grant"],
        "worker_start_count": 1,
        "simulated_effect_count": 1,
        "post_attempt_uncertain_transition_count": 0,
        "terminal_status": "consumed",
        "terminal_revision": 2,
        "terminal_use_count": 1,
    },
    "denial": {
        "result_outcome": "denied",
        "atomic_consumption_count": 0,
        "race_denial_scopes": ["argument", "grant"],
        "worker_start_count": 0,
        "simulated_effect_count": 0,
        "post_attempt_uncertain_transition_count": 0,
        "terminal_status": "invalidated",
        "terminal_revision": 2,
        "terminal_use_count": 0,
    },
    "timeout": {
        "result_outcome": "timed_out",
        "atomic_consumption_count": 1,
        "race_denial_scopes": ["grant"],
        "worker_start_count": 1,
        "simulated_effect_count": 0,
        "post_attempt_uncertain_transition_count": 1,
        "terminal_status": "uncertain",
        "terminal_revision": 3,
        "terminal_use_count": 1,
    },
    "crash": {
        "result_outcome": "failed",
        "atomic_consumption_count": 1,
        "race_denial_scopes": ["grant"],
        "worker_start_count": 1,
        "simulated_effect_count": 1,
        "post_attempt_uncertain_transition_count": 1,
        "terminal_status": "uncertain",
        "terminal_revision": 3,
        "terminal_use_count": 1,
    },
    "uncertain": {
        "result_outcome": "uncertain",
        "atomic_consumption_count": 1,
        "race_denial_scopes": ["grant"],
        "worker_start_count": 1,
        "simulated_effect_count": 1,
        "post_attempt_uncertain_transition_count": 1,
        "terminal_status": "uncertain",
        "terminal_revision": 3,
        "terminal_use_count": 1,
    },
}
SOURCE_PATHS = (
    "kernel/contracts/src/grant.rs",
    "kernel/engine/src/grants.rs",
    "kernel/engine/src/policy.rs",
    "kernel/engine/tests/grant_race_replay.rs",
    "scripts/grant_race_replay.py",
    "tests/test_grant_race_replay.py",
)


class GrantRaceReplayError(ValueError):
    """Raised when race/replay evidence is incomplete or inconsistent."""


def sha256_bytes(value: bytes) -> str:
    return hashlib.sha256(value).hexdigest()


def canonical_json(value: Any) -> bytes:
    return (json.dumps(value, indent=2, sort_keys=True) + "\n").encode("utf-8")


def write_atomic(path: Path, content: bytes) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    descriptor, temporary_name = tempfile.mkstemp(
        prefix=".agentmage-grant-race-", dir=path.parent
    )
    temporary = Path(temporary_name)
    try:
        with os.fdopen(descriptor, "wb") as handle:
            handle.write(content)
            handle.flush()
            os.fsync(handle.fileno())
        temporary.chmod(0o644)
        os.replace(temporary, path)
    except BaseException:
        temporary.unlink(missing_ok=True)
        raise


def run_traces(root: Path = ROOT) -> list[dict[str, Any]]:
    environment = {**os.environ, "AGENTMAGE_EMIT_GRANT_RACE_REPLAY": "1"}
    completed = subprocess.run(
        [
            "cargo",
            "test",
            "--locked",
            "--offline",
            "-p",
            "agentmage-kernel-engine",
            "--test",
            "grant_race_replay",
            "race_and_replay_preserve_exactly_once_admission_and_effect_bounds",
            "--",
            "--exact",
            "--nocapture",
            "--test-threads=1",
        ],
        cwd=root,
        check=False,
        stdin=subprocess.DEVNULL,
        stdout=subprocess.PIPE,
        stderr=subprocess.STDOUT,
        timeout=120,
        env=environment,
    )
    if completed.returncode != 0:
        raise GrantRaceReplayError("typed race/replay verifier failed")
    if len(completed.stdout) > 1024 * 1024:
        raise GrantRaceReplayError("typed race/replay output is oversized")
    decoded = completed.stdout.decode("utf-8", "strict")
    payloads = [line.split(MARKER, 1)[1] for line in decoded.splitlines() if MARKER in line]
    if len(payloads) != 1:
        raise GrantRaceReplayError("typed race/replay marker count is invalid")
    try:
        traces = json.loads(payloads[0])
    except json.JSONDecodeError as error:
        raise GrantRaceReplayError("typed race/replay trace is malformed") from error
    if not isinstance(traces, list):
        raise GrantRaceReplayError("typed race/replay trace must be an array")
    return traces


def validate_traces(traces: list[dict[str, Any]]) -> dict[str, Any]:
    if tuple(trace.get("scenario") for trace in traces) != SCENARIOS:
        raise GrantRaceReplayError("race/replay scenario closure or order changed")
    common_fields = {
        "schema_version",
        "scenario",
        "result_outcome",
        "racing_consumer_count",
        "atomic_consumption_count",
        "race_denial_scopes",
        "worker_start_count",
        "simulated_effect_count",
        "post_attempt_uncertain_transition_count",
        "terminal_status",
        "terminal_revision",
        "terminal_use_count",
        "replay_allowed",
        "replay_denial_scope",
        "effect_count_after_replay",
    }
    for trace in traces:
        scenario = trace["scenario"]
        expected = EXPECTED[scenario]
        if (
            set(trace) != common_fields
            or trace["schema_version"] != 1
            or trace["racing_consumer_count"] != 2
            or trace["replay_allowed"] is not False
            or trace["replay_denial_scope"] != "grant"
            or trace["effect_count_after_replay"] != trace["simulated_effect_count"]
            or any(trace[field] != value for field, value in expected.items())
        ):
            raise GrantRaceReplayError(f"race/replay expectation changed: {scenario}")
    return {
        "scenario_count": len(traces),
        "racing_consumer_count_per_scenario": 2,
        "attempt_scenarios_with_one_consumption": 4,
        "policy_denial_scenarios_with_zero_consumption": 1,
        "maximum_worker_start_count": max(trace["worker_start_count"] for trace in traces),
        "maximum_effect_count": max(trace["simulated_effect_count"] for trace in traces),
        "replay_success_count": sum(trace["replay_allowed"] for trace in traces),
        "uncertain_terminal_scenario_count": sum(
            trace["terminal_status"] == "uncertain" for trace in traces
        ),
    }


def git_revision(root: Path = ROOT) -> str:
    completed = subprocess.run(
        ["git", "rev-parse", "HEAD"],
        cwd=root,
        check=False,
        stdout=subprocess.PIPE,
        stderr=subprocess.DEVNULL,
        timeout=10,
    )
    revision = completed.stdout.decode("ascii", "strict").strip()
    if completed.returncode != 0 or re.fullmatch(r"[0-9a-f]{40}", revision) is None:
        raise GrantRaceReplayError("source revision is unavailable")
    return revision


def git_file(revision: str, relative: str, root: Path = ROOT) -> bytes:
    completed = subprocess.run(
        ["git", "show", f"{revision}:{relative}"],
        cwd=root,
        check=False,
        stdout=subprocess.PIPE,
        stderr=subprocess.DEVNULL,
        timeout=10,
    )
    if completed.returncode != 0:
        raise GrantRaceReplayError(f"source is absent at revision: {relative}")
    return completed.stdout


def build_report(reference_revision: str, root: Path = ROOT) -> dict[str, Any]:
    traces = run_traces(root)
    coverage = validate_traces(traces)
    ancestor = subprocess.run(
        ["git", "merge-base", "--is-ancestor", reference_revision, "HEAD"],
        cwd=root,
        check=False,
        stdout=subprocess.DEVNULL,
        stderr=subprocess.DEVNULL,
        timeout=10,
    )
    if ancestor.returncode != 0:
        raise GrantRaceReplayError("reference revision is not an ancestor of HEAD")
    sources = []
    for relative in SOURCE_PATHS:
        committed = git_file(reference_revision, relative, root)
        current = (root / relative).read_bytes()
        if committed != current:
            raise GrantRaceReplayError(f"source differs from reference revision: {relative}")
        sources.append({"path": relative, "sha256": sha256_bytes(committed)})
    return {
        "schema_version": 1,
        "task_id": "5.1.3.2",
        "artifact_id": "grant-race-replay-verification",
        "status": "pass-shared-linux-reference",
        "reference_revision": reference_revision,
        "sources": sources,
        "coverage": coverage,
        "traces": traces,
        "verification": {
            "two_consumer_race": "pass",
            "single_atomic_admission": "pass",
            "policy_denial_without_admission": "pass",
            "terminal_replay_denial": "pass",
            "no_second_worker_start": "pass",
            "no_second_effect": "pass",
            "unreconciled_result_terminalization": "pass",
        },
        "platform_status": {
            "shared_contracts": "verified-local",
            "linux_reference": "verified-local",
            "macos": "blocked-macos",
            "macos_implementation_claim": "none",
        },
        "limitations": [
            "Worker starts and effects are deterministic test probes, not a production executor.",
            "Timeout, crash, and uncertain probes conservatively use the uncertain lifecycle.",
            "No durable cross-process transaction or effect reconciliation is claimed.",
            "No macOS implementation or execution evidence is claimed.",
        ],
    }


def write_report(root: Path = ROOT) -> None:
    write_atomic(REPORT_PATH, canonical_json(build_report(git_revision(root), root)))


def check_report(root: Path = ROOT) -> None:
    try:
        actual = json.loads(REPORT_PATH.read_text(encoding="utf-8"))
        revision = actual["reference_revision"]
    except (OSError, KeyError, TypeError, json.JSONDecodeError) as error:
        raise GrantRaceReplayError(f"cannot read race/replay report: {error}") from error
    if not isinstance(revision, str) or actual != build_report(revision, root):
        raise GrantRaceReplayError("grant race/replay report is stale or malformed")


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--write", action="store_true")
    parser.add_argument("--traces-only", action="store_true")
    args = parser.parse_args()
    try:
        if args.write and args.traces_only:
            raise GrantRaceReplayError("select only one operation")
        if args.write:
            write_report()
        elif args.traces_only:
            validate_traces(run_traces())
        else:
            check_report()
    except (OSError, GrantRaceReplayError, subprocess.SubprocessError) as error:
        print(f"Grant race/replay validation failed: {error}", file=sys.stderr)
        return 1
    print("Grant race and replay verification validated")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
