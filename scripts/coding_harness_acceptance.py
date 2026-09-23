#!/usr/bin/env python3
"""Run the closed actual-process coding-development acceptance matrix.

Every case uses a fresh private synthetic repository and the packaged debug CLI/host pair. The
script retains raw JSONL/stderr per case and emits one recomputable summary. It does not qualify a
model or activate production coding.
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


EXPECTED = {
    "patch-test-revision": ("failed-test-repair", "repair", True, False, 0, "SUCCESS"),
    "new-file": ("new-file", "new-file", True, False, 0, "SUCCESS"),
    "multi-file": ("multi-file", "multi-file", True, False, 0, "SUCCESS"),
    "rollback": ("rollback", "stable", True, False, 0, "SUCCESS"),
    "no-op": ("no-op", "repair", True, False, 0, "NO_OP"),
    "denial": ("failed-test-repair", "repair", False, False, 4, "DECLINED"),
    "stale-approval": ("failed-test-repair", "repair", True, True, 5, None),
    "overflow": ("overflow", "repair", True, False, 7, "EXHAUSTED"),
    "false-completion": ("false-completion", "repair", True, False, 8, "FAILED"),
    "protocol-correction": ("protocol-correction", "repair", True, False, 0, "SUCCESS"),
    "arguments-correction": ("arguments-correction", "repair", True, False, 0, "SUCCESS"),
    "repeated-protocol-rejection": ("repeated-protocol-rejection", "repair", True, False, 7, "EXHAUSTED"),
    "native-command-failure": ("native-command-failure", "repair", True, False, 8, "FAILED"),
    "read-arguments-correction": ("read-arguments-correction", "repair", True, False, 0, "SUCCESS"),
    "read-arguments-denied": ("read-arguments-denied", "repair", True, False, 8, "FAILED"),
}
CASE_ROOTS = {case: f"c{index:02d}" for index, case in enumerate(EXPECTED, start=1)}
CASE_ROOTS["cancel"] = "c10"
CASE_ROOTS.update({
    "invalid-activation": "c11",
    "replayed-approval": "c12",
    "expired-cursor": "c13",
    "approval-cancel-race": "c14",
    "artifact-integrity": "c15",
    "rollback-conflict": "c16",
    "protocol-correction": "c17",
    "arguments-correction": "c18",
    "repeated-protocol-rejection": "c19",
    "native-command-failure": "c20",
    "read-arguments-correction": "c21",
    "read-arguments-denied": "c22",
})


def sha256(path: Path) -> str:
    digest = hashlib.sha256()
    with path.open("rb") as stream:
        for block in iter(lambda: stream.read(1024 * 1024), b""):
            digest.update(block)
    return digest.hexdigest()


def rows(log_dir: Path) -> list[dict]:
    with (log_dir / "stdout.jsonl").open(encoding="utf-8") as stream:
        return [json.loads(line) for line in stream]


def git_status(workspace: Path) -> list[str]:
    result = subprocess.run(
        ["git", "status", "--porcelain=v1", "--untracked-files=all"],
        cwd=workspace,
        check=True,
        text=True,
        stdout=subprocess.PIPE,
        env={**os.environ, "GIT_CONFIG_NOSYSTEM": "1"},
    )
    return result.stdout.splitlines()


def expected_status(case: str) -> list[str]:
    if case in {"patch-test-revision", "protocol-correction", "arguments-correction", "read-arguments-correction"}:
        return [" M src/calc.py"]
    if case == "new-file":
        return ["?? src/calc.py"]
    if case == "multi-file":
        return [" M src/calc.py", " M src/subtract.py"]
    return []


def verify_case(case: str, base: Path, log_dir: Path, exit_code: int) -> dict:
    scenario, _, _, stale, expected_exit, terminal = EXPECTED[case]
    workspace = coding_harness.paths(base)[2]
    observed_rows = rows(log_dir)
    observed_outcome = observed_rows[-1] if observed_rows and terminal is not None else {}
    status = git_status(workspace)
    checks = {
        "exit": exit_code == expected_exit,
        "filesystem": status == expected_status(case),
        "terminal": terminal is None or observed_outcome.get("state") == terminal,
        "scenario": json.loads((log_dir / "result.json").read_text())["scenario"] == scenario,
    }
    if case in {"protocol-correction", "arguments-correction", "repeated-protocol-rejection",
                "read-arguments-correction", "read-arguments-denied"}:
        rejections = [row for row in observed_rows if (
            row.get("kind", {}).get("event") == "model_failed" and
            row["kind"].get("failure_code") == "runtime.model.protocol_rejected") or (
            row.get("kind", {}).get("event") == "tool_rejected" and
            row["kind"].get("reason") == "arguments_invalid")]
        checks["exact-rejection-count"] = len(rejections) == (2 if case == "repeated-protocol-rejection" else 1)
        rejected_turns = {row["turn_id"] for row in rejections}
        checks["no-rejection-authority-or-effects"] = not any(
            row.get("turn_id") in rejected_turns and row.get("kind", {}).get("event") in {
                "permission_requested", "permission_decided", "tool_started", "tool_completed",
            } for row in observed_rows)
        tools = [row["kind"]["tool_call_id"] for row in observed_rows
                 if row.get("kind", {}).get("event") == "tool_completed"]
        terminal_rejection = case in {"repeated-protocol-rejection", "read-arguments-denied"}
        checks["bounded-tools"] = tools == ([] if terminal_rejection else [
            "scripted-validation-failing", "scripted-inspect-preimage", "scripted-repair-patch",
            "scripted-validation-passing", "scripted-git-diff", "scripted-git-status",
        ])
        checks["bounded-turns"] = observed_outcome.get("turn_count") == (
            1 if case == "read-arguments-denied" else 2 if case == "repeated-protocol-rejection" else 8)
        if case == "read-arguments-correction":
            checks["retained-muse-read-arguments"] = any(
                row.get("kind", {}).get("event") == "tool_requested"
                and row["kind"].get("tool_call_id") == "scripted-invalid-read-paths"
                and row["kind"].get("arguments_sha256") ==
                "aa960a422a134655e7cc9eeebc922e74d9d10883d3fdc1e5721693c9544b5028"
                for row in observed_rows)
        if case == "read-arguments-denied":
            checks["terminal-rejection-code"] = observed_outcome.get("unresolved_codes") == ["runtime.proposal.invalid"]
        if not terminal_rejection:
            checks["verified-artifacts"] = any(row.get("type") == "runtime_artifact_verified" for row in observed_rows)
            checks["exact-repair"] = (workspace / "src/calc.py").read_text() == (
                "def add(left, right):\n    return left + right\n")
    if case == "native-command-failure":
        tools = [row["kind"] for row in observed_rows
                 if row.get("kind", {}).get("event") in {"tool_completed", "tool_failed"}]
        checks["exact-tool-order"] = len(tools) == 1 and (
            tools[0].get("tool_call_id") == "scripted-native-ordered-command"
            and tools[0].get("event") == "tool_failed"
            and tools[0].get("failure_code") == "runtime.tool.failed"
            and bool(tools[0].get("receipt_id")))
        command_turns = {row["turn_id"] for row in observed_rows
                         if row.get("kind", {}).get("event") == "tool_failed"
                         and row["kind"].get("tool_call_id") == "scripted-native-ordered-command"}
        command_artifacts = {row["kind"]["artifact_id"] for row in observed_rows
                             if row.get("turn_id") in command_turns
                             and row.get("kind", {}).get("event") == "artifact_created"}
        failed_output = (json.dumps({
            "schema_version": 1, "status": "assertion_failed", "passed": 0,
            "failed": 1, "skipped": 0, "duration_ms": 1, "failed_names": ["fixture::add"],
            "artifact_ids": [], "retry_count": 0, "initial_failure_sha256": None,
        }, sort_keys=True) + "\n").encode()
        checks["generic-command-failure-output-verified"] = any(
            row.get("type") == "runtime_artifact_verified"
            and row.get("artifact_id") in command_artifacts
            and row.get("payload_sha256") == hashlib.sha256(failed_output).hexdigest()
            and row.get("byte_size") == len(failed_output) for row in observed_rows)
        checks["bounded-turns"] = observed_outcome.get("turn_count") == 1
        checks["no-validation-evidence"] = observed_outcome.get("evidence") == []
        checks["exact-failure-code"] = observed_outcome.get("unresolved_codes") == ["runtime.tool.failed"]
        checks["retained-muse-argument-bytes"] = any(
            row.get("kind", {}).get("event") == "tool_requested"
            and row["kind"].get("tool_call_id") == "scripted-native-ordered-command"
            and row["kind"].get("arguments_sha256") ==
            "87565c772c7ae8b4c39bfe07ac130422afadac4bb90b44e3ceaccc73a0aad922"
            for row in observed_rows)
        checks["verified-artifacts"] = any(row.get("type") == "runtime_artifact_verified" for row in observed_rows)
        checks["no-repair-after-generic-failure"] = (workspace / "src/calc.py").read_text() == (
            "def broken_add(left, right):\n    return left + right\n")
    if case == "patch-test-revision":
        checks["native-read-worker"] = any(
            row.get("kind", {}).get("event") == "tool_completed"
            and row["kind"].get("tool_call_id") == "scripted-inspect-preimage"
            for row in observed_rows
        )
        checks["revision"] = (workspace / "src/calc.py").read_text() == (
            "def add(left, right):\n    return left + right\n"
        )
    elif case == "new-file":
        checks["private-create"] = (workspace / "src/calc.py").stat().st_mode & 0o777 == 0o600
    elif case == "multi-file":
        validation = subprocess.run(
            [sys.executable, "tests/run_validation.py"], cwd=workspace, check=False,
            stdout=subprocess.PIPE, stderr=subprocess.PIPE, text=True,
        )
        checks["complete-validation"] = validation.returncode == 0
        checks["recorded-nine-tool-regression"] = observed_outcome.get("tool_call_count") == 9
        checks["mandatory-records-beyond-old-count-limit"] = sum(
            row.get("type") == "runtime_artifact_verified" for row in observed_rows
        ) > 33
    elif case == "rollback":
        checks["exact-preimage-restored"] = (workspace / "src/calc.py").read_text() == (
            "def add(left, right):\n    return left + right\n"
        )
    elif stale:
        stderr = (log_dir / "stderr.log").read_text(encoding="utf-8")
        checks["stale-rejected"] = "host.runtime.approval_denied\n" in stderr
        checks["no-outcome"] = not any("state" in row for row in observed_rows)
    passed = all(checks.values())
    return {
        "case": case,
        "scenario": scenario,
        "exit_code": exit_code,
        "terminal": terminal,
        "event_count": len(observed_rows) - (1 if "state" in observed_outcome else 0),
        "worktree_status": status,
        "checks": checks,
        "passed": passed,
        "stdout_sha256": sha256(log_dir / "stdout.jsonl"),
        "stderr_sha256": sha256(log_dir / "stderr.log"),
    }


def run_case(case: str, work_root: Path, log_root: Path) -> dict:
    scenario, fixture, approve, stale, _, _ = EXPECTED[case]
    base = work_root / CASE_ROOTS[case]
    log_dir = log_root / case
    coding_harness.setup(base, fixture)
    objective = f"Actual-process acceptance case: {case}."
    exit_code = coding_harness.start(
        base, scenario, objective, approve, stale, log_dir, record_session=case == "multi-file",
    )
    return verify_case(case, base, log_dir, exit_code)


def run_cancel(work_root: Path, log_root: Path) -> dict:
    case = "cancel"
    base = work_root / CASE_ROOTS[case]
    log_dir = log_root / case
    coding_harness.setup(base, "repair")
    command = [
        sys.executable, str(coding_harness.ROOT / "scripts/coding_harness.py"), "start",
        "--root", str(base), "--scenario", "slow-cancel", "--objective",
        "Cancel the actual blocked model call.", "--approve-this-run",
        "--slow-subscriber-probe", "--log-dir", str(log_dir),
    ]
    process = subprocess.Popen(command, cwd=coding_harness.ROOT)
    deadline = time.monotonic() + 30
    while time.monotonic() < deadline:
        try:
            status = coding_harness.diagnose(base)
        except (FileNotFoundError, coding_harness.HarnessError):
            status = {}
        if status.get("lifecycle") == "running":
            coding_harness.stop(base)
            break
        if process.poll() is not None:
            raise coding_harness.HarnessError("coding.acceptance.cancel-ended-before-ready")
        time.sleep(0.05)
    else:
        process.terminate()
        raise coding_harness.HarnessError("coding.acceptance.cancel-start-timeout")
    exit_code = process.wait(timeout=30)
    observed_rows = rows(log_dir)
    outcome = observed_rows[-1]
    status = git_status(coding_harness.paths(base)[2])
    events = [row.get("kind", {}).get("event") for row in observed_rows[:-1]]
    stderr = (log_dir / "stderr.log").read_text(encoding="utf-8")
    checks = {
        "exit": exit_code == 6,
        "terminal": outcome.get("state") == "CANCELLED",
        "requested": "cancellation_requested" in events,
        "observed": "cancellation_observed" in events,
        "filesystem": status == [],
        "slow-subscriber-disconnected": "coding.live.slow-subscriber-disconnected" in stderr,
    }
    return {
        "case": case,
        "scenario": "slow-cancel",
        "exit_code": exit_code,
        "terminal": outcome.get("state"),
        "event_count": len(observed_rows) - 1,
        "worktree_status": status,
        "checks": checks,
        "passed": all(checks.values()),
        "stdout_sha256": sha256(log_dir / "stdout.jsonl"),
        "stderr_sha256": sha256(log_dir / "stderr.log"),
    }


def run_invalid_activation(work_root: Path, log_root: Path) -> dict:
    case = "invalid-activation"
    base = work_root / CASE_ROOTS[case]
    log_dir = log_root / case
    coding_harness.setup(base, "repair")
    marker = coding_harness.paths(base)[2] / coding_harness.MARKER
    marker.chmod(0o644)
    exit_code = coding_harness.start(
        base, "no-op", "Reject the invalid development activation.", True, False, log_dir,
    )
    stderr = (log_dir / "stderr.log").read_text(encoding="utf-8")
    checks = {
        "exit": exit_code == 3,
        "activation-rejected": "coding.development.client.activation-denied\n" in stderr,
        "filesystem": git_status(coding_harness.paths(base)[2]) == [],
        "no-state-key": not (coding_harness.paths(base)[0] / "operational-store-development-v1.key").exists(),
    }
    return {
        "case": case,
        "scenario": "no-op",
        "exit_code": exit_code,
        "terminal": None,
        "event_count": 0,
        "worktree_status": git_status(coding_harness.paths(base)[2]),
        "checks": checks,
        "passed": all(checks.values()),
        "stdout_sha256": sha256(log_dir / "stdout.jsonl"),
        "stderr_sha256": sha256(log_dir / "stderr.log"),
    }


def run_transport_probe(case: str, work_root: Path, log_root: Path) -> dict:
    base = work_root / CASE_ROOTS[case]
    log_dir = log_root / case
    coding_harness.setup(base, "repair")
    replay = case == "replayed-approval"
    expired = case == "expired-cursor"
    exit_code = coding_harness.start(
        base,
        "failed-test-repair",
        f"Reject the actual {case} probe.",
        True,
        False,
        log_dir,
        replay_approval_probe=replay,
        expired_cursor_probe=expired,
    )
    stderr = (log_dir / "stderr.log").read_text(encoding="utf-8")
    status = git_status(coding_harness.paths(base)[2])
    expected_code = (
        "host.runtime.approval_denied\n" if replay
        else "host.runtime.event_cursor_expired\n"
    )
    checks = {
        "exit": exit_code == 5,
        "exact-refusal": expected_code in stderr,
        "filesystem": status == ([] if replay else [" M src/calc.py"]),
        "no-outcome": not any("state" in row for row in rows(log_dir)),
    }
    return {
        "case": case,
        "scenario": "failed-test-repair",
        "exit_code": exit_code,
        "terminal": None,
        "event_count": len(rows(log_dir)),
        "worktree_status": status,
        "checks": checks,
        "passed": all(checks.values()),
        "stdout_sha256": sha256(log_dir / "stdout.jsonl"),
        "stderr_sha256": sha256(log_dir / "stderr.log"),
    }


def run_approval_cancel_race(work_root: Path, log_root: Path) -> dict:
    case = "approval-cancel-race"
    base = work_root / CASE_ROOTS[case]
    log_dir = log_root / case
    coding_harness.setup(base, "repair")
    command = [
        sys.executable, str(coding_harness.ROOT / "scripts/coding_harness.py"), "start",
        "--root", str(base), "--scenario", "failed-test-repair", "--objective",
        "Cancel while the exact approval is displayed.", "--approve-this-run",
        "--approval-delay-ms", "5000", "--log-dir", str(log_dir),
    ]
    process = subprocess.Popen(command, cwd=coding_harness.ROOT)
    deadline = time.monotonic() + 30
    while time.monotonic() < deadline:
        if (log_dir / "stderr.log").is_file() and "preauthorized_for_this_run " in (
            log_dir / "stderr.log"
        ).read_text(encoding="utf-8"):
            coding_harness.stop(base)
            break
        if process.poll() is not None:
            raise coding_harness.HarnessError("coding.acceptance.approval-ended-before-cancel")
        time.sleep(0.05)
    else:
        process.terminate()
        raise coding_harness.HarnessError("coding.acceptance.approval-cancel-timeout")
    exit_code = process.wait(timeout=30)
    observed_rows = rows(log_dir)
    events = [row.get("kind", {}).get("event") for row in observed_rows[:-1]]
    status = git_status(coding_harness.paths(base)[2])
    checks = {
        "exit": exit_code == 6,
        "terminal": observed_rows[-1].get("state") == "CANCELLED",
        "requested": "cancellation_requested" in events,
        "observed": "cancellation_observed" in events,
        "no-permission-decision": "permission_decided" not in events,
        "filesystem": status == [],
    }
    return {
        "case": case,
        "scenario": "failed-test-repair",
        "exit_code": exit_code,
        "terminal": observed_rows[-1].get("state"),
        "event_count": len(observed_rows) - 1,
        "worktree_status": status,
        "checks": checks,
        "passed": all(checks.values()),
        "stdout_sha256": sha256(log_dir / "stdout.jsonl"),
        "stderr_sha256": sha256(log_dir / "stderr.log"),
    }


def run_artifact_integrity_probe(work_root: Path, log_root: Path) -> dict:
    case = "artifact-integrity"
    base = work_root / CASE_ROOTS[case]
    log_dir = log_root / case
    coding_harness.setup(base, "repair")
    exit_code = coding_harness.start(
        base,
        "no-op",
        "Reject a substituted verified-artifact page.",
        True,
        False,
        log_dir,
        artifact_integrity_probe=True,
    )
    stderr = (log_dir / "stderr.log").read_text(encoding="utf-8")
    status = git_status(coding_harness.paths(base)[2])
    checks = {
        "exit": exit_code == 5,
        "exact-refusal": "cli.runtime.evidence_denied\n" in stderr,
        "filesystem": status == [],
        "no-outcome": not any("state" in row for row in rows(log_dir)),
    }
    return {
        "case": case,
        "scenario": "no-op",
        "exit_code": exit_code,
        "terminal": None,
        "event_count": len(rows(log_dir)),
        "worktree_status": status,
        "checks": checks,
        "passed": all(checks.values()),
        "stdout_sha256": sha256(log_dir / "stdout.jsonl"),
        "stderr_sha256": sha256(log_dir / "stderr.log"),
    }


def run_rollback_conflict(work_root: Path, log_root: Path) -> dict:
    case = "rollback-conflict"
    base = work_root / CASE_ROOTS[case]
    log_dir = log_root / case
    coding_harness.setup(base, "stable")
    command = [
        sys.executable, str(coding_harness.ROOT / "scripts/coding_harness.py"), "start",
        "--root", str(base), "--scenario", "rollback", "--objective",
        "Refuse rollback if a concurrent edit changes the exact postimage.",
        "--approve-this-run", "--approval-delay-ms", "10000", "--log-dir", str(log_dir),
    ]
    process = subprocess.Popen(
        command,
        cwd=coding_harness.ROOT,
        stdout=subprocess.PIPE,
        stderr=subprocess.PIPE,
        text=True,
    )
    deadline = time.monotonic() + 60
    rollback_challenge = "tool=agentmage.code.rollback@"
    while time.monotonic() < deadline:
        if (log_dir / "stderr.log").is_file() and rollback_challenge in (
            log_dir / "stderr.log"
        ).read_text(encoding="utf-8"):
            source = coding_harness.paths(base)[2] / "src/calc.py"
            if source.read_text(encoding="utf-8") != (
                "def temporary_add(left, right):\n    return left + right\n"
            ):
                process.terminate()
                raise coding_harness.HarnessError(
                    "coding.acceptance.rollback-conflict-preimage-denied"
                )
            descriptor = os.open(source, os.O_WRONLY | os.O_TRUNC | os.O_NOFOLLOW)
            try:
                os.write(
                    descriptor,
                    b"def human_concurrent_add(left, right):\n    return left + right + 1\n",
                )
            finally:
                os.close(descriptor)
            break
        if process.poll() is not None:
            raise coding_harness.HarnessError(
                "coding.acceptance.rollback-ended-before-conflict"
            )
        time.sleep(0.02)
    else:
        process.terminate()
        raise coding_harness.HarnessError("coding.acceptance.rollback-conflict-timeout")
    stdout, stderr = process.communicate(timeout=30)
    if stderr:
        raise coding_harness.HarnessError("coding.acceptance.rollback-runner-stderr")
    exit_code = process.returncode
    workspace = coding_harness.paths(base)[2]
    retained = (workspace / "src/calc.py").read_text(encoding="utf-8")
    runtime_stderr = (log_dir / "stderr.log").read_text(encoding="utf-8")
    status = git_status(workspace)
    checks = {
        "exit": exit_code == 5,
        "runner-result": '"exit_code": 5' in stdout,
        "rollback-challenged": rollback_challenge in runtime_stderr,
        "exact-refusal": "host.runtime.failed\n" in runtime_stderr,
        "human-edit-retained": retained == (
            "def human_concurrent_add(left, right):\n    return left + right + 1\n"
        ),
        "filesystem": status == [" M src/calc.py"],
        "no-outcome": not any("state" in row for row in rows(log_dir)),
    }
    return {
        "case": case,
        "scenario": "rollback",
        "exit_code": exit_code,
        "terminal": None,
        "event_count": len(rows(log_dir)),
        "worktree_status": status,
        "checks": checks,
        "passed": all(checks.values()),
        "stdout_sha256": sha256(log_dir / "stdout.jsonl"),
        "stderr_sha256": sha256(log_dir / "stderr.log"),
    }


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--work-root", type=Path, required=True)
    parser.add_argument("--log-root", type=Path, required=True)
    arguments = parser.parse_args()
    work_root = arguments.work_root.resolve(strict=False)
    log_root = arguments.log_root.resolve(strict=False)
    for target in (work_root, log_root):
        if target.exists() or target.is_symlink():
            raise coding_harness.HarnessError("coding.acceptance.target-exists")
        coding_harness.private_directory(target)
    results = [run_case(case, work_root, log_root) for case in EXPECTED]
    results.insert(6, run_cancel(work_root, log_root))
    results.extend([
        run_invalid_activation(work_root, log_root),
        run_transport_probe("replayed-approval", work_root, log_root),
        run_transport_probe("expired-cursor", work_root, log_root),
        run_approval_cancel_race(work_root, log_root),
        run_artifact_integrity_probe(work_root, log_root),
        run_rollback_conflict(work_root, log_root),
    ])
    agentmage = coding_harness.binary("agentmage")
    host = coding_harness.binary("agentmage-host")
    report = {
        "schema_version": 1,
        "qualification": "executable-scripted-only",
        "profile_id": coding_harness.PROFILE,
        "agentmage_sha256": sha256(agentmage),
        "host_sha256": sha256(host),
        "case_count": len(results),
        "passed": all(result["passed"] for result in results),
        "cases": results,
    }
    coding_harness.private_file(log_root / "acceptance-report.json", json.dumps(report, sort_keys=True) + "\n")
    print(json.dumps(report, sort_keys=True))
    return 0 if report["passed"] else 1


if __name__ == "__main__":
    try:
        raise SystemExit(main())
    except (coding_harness.HarnessError, OSError, subprocess.SubprocessError, json.JSONDecodeError) as error:
        print(str(error), file=sys.stderr)
        raise SystemExit(1) from error
