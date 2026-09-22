#!/usr/bin/env python3
"""Exercise the bounded pinned-Linux daily-use coding campaign.

The campaign uses only fresh synthetic repositories. Candidate-model inputs are retained results
from the separate sequential live-binary campaign; this script verifies them without rerunning a
GPU model or treating failure handling as model qualification.
"""

from __future__ import annotations

import argparse
import hashlib
import json
import os
from pathlib import Path
import subprocess
import sys
import time

try:
    from scripts import coding_harness
except ModuleNotFoundError:
    import coding_harness


SETUP_RUNS = 3
FOLLOW_UPS = 25
CONTENTION_SECONDS = 5
CANCELLATION_SECONDS = 30
SOAK_SECONDS = 15 * 60


def sha256(path: Path) -> str:
    digest = hashlib.sha256()
    with path.open("rb") as stream:
        for block in iter(lambda: stream.read(1024 * 1024), b""):
            digest.update(block)
    return digest.hexdigest()


def rows(log_dir: Path) -> list[dict]:
    path = log_dir / "stdout.jsonl"
    if not path.is_file():
        return []
    with path.open(encoding="utf-8") as stream:
        return [json.loads(line) for line in stream]


def git_status(base: Path) -> list[str]:
    workspace = coding_harness.paths(base)[2]
    result = subprocess.run(
        ["git", "status", "--porcelain=v1", "--untracked-files=all"],
        cwd=workspace,
        check=True,
        capture_output=True,
        text=True,
        env={**os.environ, "GIT_CONFIG_NOSYSTEM": "1"},
    )
    return result.stdout.splitlines()


def terminals(observed: list[dict]) -> list[dict]:
    return [row for row in observed if "state" in row and "run_id" in row]


def setup_campaign(work_root: Path, log_root: Path) -> dict:
    results = []
    for index in range(1, SETUP_RUNS + 1):
        base = work_root / f"setup-{index}"
        log_dir = log_root / f"setup-{index}"
        setup = coding_harness.setup(base, "stable")
        diagnosed = coding_harness.diagnose(base)
        exit_code = coding_harness.start(
            base,
            "no-op",
            f"Fresh setup acceptance {index}.",
            True,
            False,
            log_dir,
        )
        outcome = terminals(rows(log_dir))
        checks = {
            "activation": setup["activation"] is True and diagnosed["activation"] is True,
            "private": setup["disposable_private"] is True and setup["state_private"] is True,
            "ready": diagnosed["lifecycle"] == "ready",
            "run": exit_code == 0 and len(outcome) == 1 and outcome[0]["state"] == "NO_OP",
            "clean": git_status(base) == [],
        }
        results.append({
            "index": index,
            "checks": checks,
            "passed": all(checks.values()),
            "stdout_sha256": sha256(log_dir / "stdout.jsonl"),
            "stderr_sha256": sha256(log_dir / "stderr.log"),
        })
    return {
        "case": "repeatable-setup",
        "threshold": SETUP_RUNS,
        "runs": results,
        "passed": len(results) == SETUP_RUNS and all(result["passed"] for result in results),
    }


def pressure_case(kind: str, work_root: Path, log_root: Path) -> dict:
    base = work_root / kind
    log_dir = log_root / kind
    coding_harness.setup(base, "stable")
    exit_code = coding_harness.start(
        base,
        kind,
        f"Predeclared {kind} daily-use acceptance.",
        True,
        False,
        log_dir,
    )
    observed = rows(log_dir)
    outcome = terminals(observed)
    stderr = (log_dir / "stderr.log").read_text(encoding="utf-8")
    if kind == "disk-pressure":
        checks = {
            "exit": exit_code == 5,
            "before-model-tool-effect": not any(
                row.get("kind", {}).get("event") in {
                    "model_requested", "tool_requested", "tool_started", "tool_completed"
                }
                for row in observed
            ),
            "no-terminal-success": not outcome,
            "runtime-refusal": "host.runtime.failed\n" in stderr,
            "clean": git_status(base) == [],
        }
    else:
        terminal = outcome[-1] if outcome else {}
        checks = {
            "exit": exit_code == 7,
            "terminal": terminal.get("state") == "EXHAUSTED",
            "explicit-budget": terminal.get("unresolved_codes") == ["runtime.budget.exhausted"],
            "no-partial-output": terminal.get("output") is None,
            "clean": git_status(base) == [],
        }
    return {
        "case": kind,
        "budget_bytes": 1_024,
        "exit_code": exit_code,
        "event_count": len(observed) - len(outcome),
        "checks": checks,
        "passed": all(checks.values()),
        "stdout_sha256": sha256(log_dir / "stdout.jsonl"),
        "stderr_sha256": sha256(log_dir / "stderr.log"),
    }


def contention_case(work_root: Path, log_root: Path) -> dict:
    base = work_root / "contention"
    log_dir = log_root / "contention"
    coding_harness.setup(base, "repair")
    command = [
        sys.executable,
        str(coding_harness.ROOT / "scripts/coding_harness.py"),
        "start",
        "--root", str(base),
        "--scenario", "slow-cancel",
        "--objective", "Hold the exact worktree writer lease for contention acceptance.",
        "--approve-this-run",
        "--log-dir", str(log_dir),
    ]
    process = subprocess.Popen(
        command,
        cwd=coding_harness.ROOT,
        stdout=subprocess.PIPE,
        stderr=subprocess.PIPE,
        text=True,
    )
    ready_deadline = time.monotonic() + 30
    while time.monotonic() < ready_deadline:
        if process.poll() is not None:
            raise coding_harness.HarnessError("coding.daily.contention-owner-ended")
        try:
            if coding_harness.diagnose(base).get("lifecycle") == "running":
                break
        except (FileNotFoundError, coding_harness.HarnessError):
            pass
        time.sleep(0.05)
    else:
        process.terminate()
        raise coding_harness.HarnessError("coding.daily.contention-owner-timeout")

    refused_at = time.monotonic()
    refusal = None
    try:
        coding_harness.start(
            base,
            "no-op",
            "This competing owner must be refused.",
            True,
            False,
            None,
        )
    except coding_harness.HarnessError as error:
        refusal = str(error)
    refusal_seconds = time.monotonic() - refused_at

    cancelled_at = time.monotonic()
    coding_harness.stop(base)
    stdout, stderr = process.communicate(timeout=CANCELLATION_SECONDS)
    cancellation_seconds = time.monotonic() - cancelled_at
    observed = rows(log_dir)
    outcome = terminals(observed)
    events = [row.get("kind", {}).get("event") for row in observed]
    checks = {
        "second-owner-refused": refusal == "coding.harness.start-state-denied",
        "refusal-threshold": refusal_seconds <= CONTENTION_SECONDS,
        "exit": process.returncode == 6,
        "terminal": len(outcome) == 1 and outcome[0]["state"] == "CANCELLED",
        "cancellation-threshold": cancellation_seconds <= CANCELLATION_SECONDS,
        "cancellation-observed": "cancellation_requested" in events
        and "cancellation_observed" in events,
        "runner-stderr": stderr == "",
        "runner-result": '"exit_code": 6' in stdout,
        "clean": git_status(base) == [],
    }
    return {
        "case": "worktree-contention",
        "refusal_seconds": refusal_seconds,
        "refusal_threshold_seconds": CONTENTION_SECONDS,
        "cancellation_seconds": cancellation_seconds,
        "cancellation_threshold_seconds": CANCELLATION_SECONDS,
        "checks": checks,
        "passed": all(checks.values()),
        "stdout_sha256": sha256(log_dir / "stdout.jsonl"),
        "stderr_sha256": sha256(log_dir / "stderr.log"),
    }


def soak_case(work_root: Path, log_root: Path) -> dict:
    base = work_root / "long-session-soak"
    log_dir = log_root / "long-session-soak"
    coding_harness.setup(base, "stable")
    follow_ups = tuple(f"Bounded follow-up {index}." for index in range(1, FOLLOW_UPS + 1))
    started = time.monotonic()
    exit_code = coding_harness.start(
        base,
        "no-op",
        "Initial bounded long-session soak run.",
        True,
        False,
        log_dir,
        record_session=True,
        follow_ups=follow_ups,
    )
    elapsed = time.monotonic() - started
    observed = rows(log_dir)
    outcome = terminals(observed)
    run_ids = {item["run_id"] for item in outcome}
    session_ids = {item["session_id"] for item in outcome}
    verified = [row for row in observed if row.get("type") == "runtime_artifact_verified"]
    expected_runs = FOLLOW_UPS + 1
    checks = {
        "exit": exit_code == 0,
        "terminal-count": len(outcome) == expected_runs,
        "all-no-op": all(item["state"] == "NO_OP" for item in outcome),
        "distinct-run-identities": len(run_ids) == expected_runs,
        "one-session": len(session_ids) == 1,
        "verified-artifacts": len(verified) >= 2 * expected_runs,
        "elapsed-threshold": elapsed <= SOAK_SECONDS,
        "clean": git_status(base) == [],
    }
    return {
        "case": "long-session-soak",
        "initial_runs": 1,
        "follow_up_runs": FOLLOW_UPS,
        "elapsed_seconds": elapsed,
        "threshold_seconds": SOAK_SECONDS,
        "verified_artifact_count": len(verified),
        "checks": checks,
        "passed": all(checks.values()),
        "stdout_sha256": sha256(log_dir / "stdout.jsonl"),
        "stderr_sha256": sha256(log_dir / "stderr.log"),
    }


def candidate_failure(model: str, base: Path, log_dir: Path) -> dict:
    result = json.loads((log_dir / "result.json").read_text(encoding="utf-8"))
    observed = rows(log_dir)
    outcome = terminals(observed)
    events = [row.get("kind", {}).get("event") for row in observed]
    rejection_root = coding_harness.paths(base)[0] / "candidate-rejections" / model
    responses = sorted(rejection_root.glob("*.response.bin"))
    metadata = sorted(rejection_root.glob("*.metadata.json"))
    stderr = (log_dir / "stderr.log").read_text(encoding="utf-8")
    checks = {
        "exact-model": result.get("model") == model,
        "exit": result.get("exit_code") == 8,
        "terminal": len(outcome) == 1 and outcome[0]["state"] == "FAILED",
        "first-call-failed": len(outcome) == 1
        and outcome[0].get("model_call_count") == 1
        and outcome[0].get("tool_call_count") == 0,
        "model-failure-event": "model_failed" in events,
        "rejection-retained": len(responses) == 1 and len(metadata) == 1,
        "codec-explicit": f"model.{model}-codec.final-channel-invalid" in stderr,
        "guard-clean": result.get("candidate_resources", {}).get("resource_guard_error") is None,
        "clean": git_status(base) == [],
    }
    return {
        "model": model,
        "disposition": "not-qualified",
        "checks": checks,
        "passed_failure_handling": all(checks.values()),
        "response_sha256": sha256(responses[0]) if len(responses) == 1 else None,
        "response_bytes": responses[0].stat().st_size if len(responses) == 1 else None,
        "metadata_sha256": sha256(metadata[0]) if len(metadata) == 1 else None,
        "stdout_sha256": sha256(log_dir / "stdout.jsonl"),
        "stderr_sha256": sha256(log_dir / "stderr.log"),
        "resources": result.get("candidate_resources"),
    }


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--work-root", type=Path, required=True)
    parser.add_argument("--log-root", type=Path, required=True)
    parser.add_argument("--muse-root", type=Path, required=True)
    parser.add_argument("--muse-log-dir", type=Path, required=True)
    parser.add_argument("--gpt-oss-root", type=Path, required=True)
    parser.add_argument("--gpt-oss-log-dir", type=Path, required=True)
    arguments = parser.parse_args()
    work_root = arguments.work_root.resolve(strict=False)
    log_root = arguments.log_root.resolve(strict=False)
    for target in (work_root, log_root):
        if target.exists() or target.is_symlink():
            raise coding_harness.HarnessError("coding.daily.target-exists")
        coding_harness.private_directory(target)

    results = [
        setup_campaign(work_root, log_root),
        pressure_case("disk-pressure", work_root, log_root),
        pressure_case("output-pressure", work_root, log_root),
        contention_case(work_root, log_root),
        soak_case(work_root, log_root),
    ]
    candidate_results = [
        candidate_failure("muse", arguments.muse_root.resolve(strict=True),
                          arguments.muse_log_dir.resolve(strict=True)),
        candidate_failure("gpt-oss", arguments.gpt_oss_root.resolve(strict=True),
                          arguments.gpt_oss_log_dir.resolve(strict=True)),
    ]
    report = {
        "schema_version": 1,
        "qualification": "executable-scripted-daily-use-local-verification",
        "profile_id": coding_harness.PROFILE,
        "threshold_decision": "Decision 0065",
        "supported_executable_languages": ["Python"],
        "supported_registered_commands": ["fixture.python-validation@1.0.0"],
        "unsupported_claims": [
            "qualified-coding-model",
            "independent-review",
            "M-HARNESS-DAILY",
            "supported-platform",
            "release",
        ],
        "agentmage_sha256": sha256(coding_harness.binary("agentmage")),
        "host_sha256": sha256(coding_harness.binary("agentmage-host")),
        "local_cases": results,
        "candidate_failure_handling": candidate_results,
        "local_cases_passed": all(result["passed"] for result in results),
        "candidate_failure_handling_passed": all(
            result["passed_failure_handling"] for result in candidate_results
        ),
    }
    report["passed"] = (
        report["local_cases_passed"] and report["candidate_failure_handling_passed"]
    )
    coding_harness.private_file(
        log_root / "daily-acceptance-report.json",
        json.dumps(report, sort_keys=True) + "\n",
    )
    print(json.dumps(report, sort_keys=True))
    return 0 if report["passed"] else 1


if __name__ == "__main__":
    try:
        raise SystemExit(main())
    except (
        coding_harness.HarnessError,
        OSError,
        subprocess.SubprocessError,
        json.JSONDecodeError,
    ) as error:
        print(str(error), file=sys.stderr)
        raise SystemExit(1) from error
