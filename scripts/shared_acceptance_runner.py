#!/usr/bin/env python3
"""Run deterministic platform-neutral acceptance checks over synthetic artifacts."""

from __future__ import annotations

import argparse
import copy
import hashlib
import io
import json
import os
import re
import sys
import tempfile
import zipfile
from dataclasses import dataclass
from pathlib import Path
from typing import Any, Callable


ROOT = Path(__file__).resolve().parents[1]
if str(ROOT) not in sys.path:
    sys.path.insert(0, str(ROOT))

from fixtures.fake_adapters import (  # noqa: E402
    FakeMode,
    FakeModel,
    FakeStatus,
    ScenarioPlan,
    baseline_trace,
)
from scripts import document_fixture_generator as documents  # noqa: E402
from scripts import expected_output_manifests as goldens  # noqa: E402
from scripts import platform_result_recorder as platform_records  # noqa: E402
from scripts import test_result_bundle as result_bundles  # noqa: E402
from scripts import versioned_corpus  # noqa: E402


PROFILE_PATH = ROOT / "fixtures" / "acceptance-runner-profile.json"
REPORT_PATH = (
    ROOT
    / "artifacts"
    / "sprints"
    / "sprint-2"
    / "story-2.1"
    / "shared-acceptance-runner-report.json"
)
EXPECTED_CASES = (
    (
        "acceptance.corpus-integrity",
        "corpus-integrity",
        "agentmage-versioned-synthetic-corpus-v1",
        10,
    ),
    (
        "acceptance.golden-integrity",
        "golden-integrity",
        "agentmage-expected-output-manifests-v1",
        11,
    ),
    (
        "acceptance.fake-adapter-closure",
        "fake-adapter-closure",
        "agentmage-fake-adapters-v1",
        12,
    ),
    (
        "acceptance.path-declaration-safety",
        "path-declaration-safety",
        "agentmage-path-fixtures-v1",
        13,
    ),
    (
        "acceptance.document-structure-safety",
        "document-structure-safety",
        "agentmage-document-fixtures-v1",
        14,
    ),
    (
        "acceptance.platform-redaction",
        "platform-redaction",
        "agentmage-platform-result-recorder-v1",
        15,
    ),
    (
        "acceptance.result-reconciliation",
        "result-reconciliation",
        "agentmage-test-result-bundle-v1",
        16,
    ),
)
RUN_STATUSES = ("pass", "fail", "skipped", "error", "cancelled")
EXPECTED_EXECUTION_CONTRACT = {
    "external_commands": False,
    "network": False,
    "real_user_data": False,
    "wall_clock_timing": False,
    "writes_outside_explicit_output": False,
}
IDENTIFIER = re.compile(r"^[a-z0-9][a-z0-9._-]{0,127}$")
TIMESTAMP = re.compile(r"^\d{4}-\d{2}-\d{2}T\d{2}:\d{2}:\d{2}Z$")
SENSITIVE_TEXT = re.compile(
    r"(?:AM_SYNTHETIC_(?:SECRET|CANARY)_[A-Z0-9_]+|/(?:home|Users)/[^\s]+)",
    re.IGNORECASE,
)


@dataclass(frozen=True)
class CheckObservation:
    passed: bool
    diagnostic: str
    metrics: dict[str, str | int | bool]
    skipped: bool = False


class CancellationToken:
    def __init__(self) -> None:
        self._cancelled = False

    def cancel(self) -> None:
        self._cancelled = True

    @property
    def cancelled(self) -> bool:
        return self._cancelled


Handler = Callable[[Path], CheckObservation]


def read_json(path: Path) -> Any:
    return json.loads(path.read_text(encoding="utf-8"))


def canonical_json(value: Any) -> bytes:
    return (json.dumps(value, indent=2, sort_keys=True) + "\n").encode("utf-8")


def sha256_bytes(value: bytes) -> str:
    return hashlib.sha256(value).hexdigest()


def validate_profile(profile: Any) -> list[str]:
    if not isinstance(profile, dict):
        return ["shared acceptance runner profile must be an object"]
    failures: list[str] = []
    if (
        profile.get("schema_version") != 1
        or profile.get("profile_id") != "agentmage-shared-acceptance-runner-v1"
        or profile.get("status") != "synthetic-acceptance-runner-contract"
        or profile.get("run_id") != "agentmage-sprint-2-shared-acceptance-v1"
    ):
        failures.append("shared acceptance runner profile identity is invalid")
    if not isinstance(profile.get("generated_at"), str) or not TIMESTAMP.fullmatch(
        profile["generated_at"]
    ):
        failures.append("shared acceptance runner timestamp is invalid")
    cases = profile.get("case_order")
    observed_cases = tuple(
        (
            case.get("test_id"),
            case.get("check_id"),
            case.get("fixture_id"),
            case.get("logical_duration_ms"),
        )
        for case in cases
        if isinstance(case, dict)
    ) if isinstance(cases, list) else ()
    if observed_cases != EXPECTED_CASES:
        failures.append("shared acceptance runner case closure or order drifted")
    if profile.get("expected_status") != "pass":
        failures.append("shared acceptance runner canonical expected status drifted")
    if profile.get("output_policy") != {
        "max_retained_bytes_per_case": 256,
        "raw_output_retained": False,
        "synthetic_canaries_redacted": True,
    }:
        failures.append("shared acceptance runner output policy drifted")
    if profile.get("execution_contract") != EXPECTED_EXECUTION_CONTRACT:
        failures.append("shared acceptance runner execution contract was weakened")
    if profile.get("macos_execution_status") != "blocked-macos":
        failures.append("shared acceptance runner lost blocked macOS status")
    return failures


def safe_metrics(metrics: Any) -> bool:
    if not isinstance(metrics, dict):
        return False
    for key, value in metrics.items():
        if not isinstance(key, str) or not IDENTIFIER.fullmatch(key):
            return False
        if isinstance(value, bool) or isinstance(value, int):
            continue
        if not isinstance(value, str) or len(value.encode("utf-8")) > 128:
            return False
        if SENSITIVE_TEXT.search(value):
            return False
    return True


def sanitize_diagnostic(value: str, root: Path) -> str:
    sanitized = value.replace(str(root), "<REPO_ROOT>")
    sanitized = SENSITIVE_TEXT.sub("<REDACTED>", sanitized)
    return sanitized.replace("\r\n", "\n")


def bound_diagnostic(value: str, limit: int, root: Path) -> dict[str, Any]:
    raw = value.encode("utf-8")
    sanitized = sanitize_diagnostic(value, root).encode("utf-8")
    retained = sanitized[:limit]
    while True:
        try:
            text = retained.decode("utf-8")
            break
        except UnicodeDecodeError as error:
            retained = retained[: error.start]
    return {
        "diagnostic_bytes": len(raw),
        "diagnostic_sha256": sha256_bytes(raw),
        "retained_bytes": len(retained),
        "retained_diagnostic": text,
        "retained_sha256": sha256_bytes(retained),
        "truncated": len(sanitized) > len(retained),
    }


def corpus_integrity(root: Path) -> CheckObservation:
    failures = versioned_corpus.check_checked_corpus(root)
    manifest = read_json(root / "fixtures/corpus/v1/manifest.json")
    return CheckObservation(
        passed=not failures,
        diagnostic="corpus archive and manifest validated" if not failures else "; ".join(failures),
        metrics={
            "entry_count": manifest["archive"]["entry_count"],
            "family_count": len(manifest["family_counts"]),
        },
    )


def golden_integrity(root: Path) -> CheckObservation:
    profile = read_json(root / "fixtures/expected-output-profile.json")
    failures = goldens.validate_profile(profile) + goldens.validate_manifest_records(profile, root)
    records = goldens.build_manifest_records(profile, root)
    states = {record["expected_output"]["evidence"]["state"] for record in records}
    if states != {"Observed", "Derived", "Inferred", "Unknown/Blocked"}:
        failures.append("golden evidence-state closure is incomplete")
    return CheckObservation(
        passed=not failures,
        diagnostic="golden manifests and receipt chain validated" if not failures else "; ".join(failures),
        metrics={"golden_count": len(records), "evidence_state_count": len(states)},
    )


def fake_adapter_closure(_root: Path) -> CheckObservation:
    expected_status = {
        FakeMode.SUCCESS: FakeStatus.SUCCEEDED,
        FakeMode.DENIAL: FakeStatus.DENIED,
        FakeMode.MALFORMED: FakeStatus.MALFORMED,
        FakeMode.CANCELLATION: FakeStatus.CANCELLED,
        FakeMode.TIMEOUT: FakeStatus.TIMED_OUT,
        FakeMode.CRASH_BEFORE: FakeStatus.CRASHED,
        FakeMode.CRASH_AFTER: FakeStatus.CRASHED,
        FakeMode.UNCERTAIN: FakeStatus.UNCERTAIN,
    }
    failures = []
    for mode, expected in expected_status.items():
        outcome = FakeModel(ScenarioPlan({"fake-model.generate": [mode]})).generate(
            "synthetic acceptance prompt"
        )
        if outcome.status is not expected:
            failures.append(f"fake mode mismatch: {mode.value}")
    trace = baseline_trace()
    if trace["event_count"] != 13 or trace["raw_secret_retained"]:
        failures.append("fake adapter baseline trace drifted")
    return CheckObservation(
        passed=not failures,
        diagnostic="fake adapters cover all typed modes" if not failures else "; ".join(failures),
        metrics={"mode_count": len(expected_status), "baseline_event_count": trace["event_count"]},
    )


def path_declaration_safety(root: Path) -> CheckObservation:
    archive_content = (root / "fixtures/corpus/v1/agentmage-synthetic-corpus-v1.zip").read_bytes()
    failures = []
    with zipfile.ZipFile(io.BytesIO(archive_content)) as archive:
        declarations = json.loads(archive.read("paths/symlink-declarations.json"))
        names = set(archive.namelist())
        for item in declarations["symlinks"]:
            if not item.get("materialize_only_in_temporary_sandbox"):
                failures.append("path declaration permits direct materialization")
            if f"paths/{item['path']}" in names:
                failures.append("path declaration was stored as an archive entry")
        for info in archive.infolist():
            if (info.external_attr >> 16) & 0o170000 == 0o120000:
                failures.append("corpus archive contains a symlink")
    return CheckObservation(
        passed=not failures,
        diagnostic="path escapes remain inert declarations" if not failures else "; ".join(failures),
        metrics={"symlink_declaration_count": len(declarations["symlinks"])},
    )


def document_structure_safety(root: Path) -> CheckObservation:
    profile = read_json(root / "fixtures/document-fixture-profile.json")
    failures = documents.validate_profile(profile)
    specs = documents.materialize_specs(profile)
    for fixture in specs.values():
        if fixture.family in {"word", "spreadsheets", "presentations"}:
            with zipfile.ZipFile(io.BytesIO(fixture.content)) as archive:
                if archive.testzip() is not None:
                    failures.append(f"corrupt office fixture: {fixture.family}")
                for name in archive.namelist():
                    content = archive.read(name).lower()
                    if b"targetmode=\"external\"" in content or b"<f>" in content:
                        failures.append(f"active office content: {fixture.family}")
        if fixture.family == "portable-document-format" and any(
            token in fixture.content for token in (b"/JavaScript", b"/Launch", b"/EmbeddedFile")
        ):
            failures.append("active PDF content")
        if fixture.family == "notebooks":
            notebook = json.loads(fixture.content)
            if any(cell["cell_type"] == "code" for cell in notebook["cells"]):
                failures.append("notebook fixture contains code")
    return CheckObservation(
        passed=not failures,
        diagnostic="document structures are valid and inert" if not failures else "; ".join(failures),
        metrics={"document_family_count": len(specs)},
    )


def platform_redaction(root: Path) -> CheckObservation:
    profile = read_json(root / "fixtures/platform-result-profile.json")
    failures = platform_records.validate_profile(profile)
    records = [
        platform_records.record_platform_result(
            run["environment"],
            run["result"],
            run["captured_at"],
            profile["allowlist_version"],
        )
        for run in profile["synthetic_runs"]
    ]
    for record in records:
        failures.extend(platform_records.validate_record(record))
        if platform_records.nested_keys(record) & set(platform_records.EXPECTED_FORBIDDEN_FIELDS):
            failures.append("platform record contains forbidden fields")
    return CheckObservation(
        passed=not failures,
        diagnostic="platform records retain only allowlisted identity" if not failures else "; ".join(failures),
        metrics={
            "platform_record_count": len(records),
            "allowed_field_count": len(platform_records.EXPECTED_ENVIRONMENT_FIELDS),
        },
    )


def result_reconciliation(root: Path) -> CheckObservation:
    profile = read_json(root / "fixtures/test-result-bundle-profile.json")
    bundle = result_bundles.build_bundle(profile, root)
    failures = result_bundles.validate_bundle(bundle, profile, root)
    summary = bundle["summary"]
    if summary["non_pass_test_count"] != 5 or summary["overall_status"] != "fail":
        failures.append("synthetic non-pass results were omitted or converted")
    return CheckObservation(
        passed=not failures,
        diagnostic="raw results and non-pass summary reconcile" if not failures else "; ".join(failures),
        metrics={
            "test_count": summary["test_count"],
            "non_pass_test_count": summary["non_pass_test_count"],
            "retry_attempt_count": summary["retry_attempt_count"],
        },
    )


def default_handlers() -> dict[str, Handler]:
    return {
        "corpus-integrity": corpus_integrity,
        "golden-integrity": golden_integrity,
        "fake-adapter-closure": fake_adapter_closure,
        "path-declaration-safety": path_declaration_safety,
        "document-structure-safety": document_structure_safety,
        "platform-redaction": platform_redaction,
        "result-reconciliation": result_reconciliation,
    }


def summary_for(results: list[dict[str, Any]]) -> dict[str, Any]:
    status_counts = {
        status: sum(result["status"] == status for result in results)
        for status in RUN_STATUSES
    }
    expectation_failures = sum(not result["expectation_met"] for result in results)
    return {
        "case_count": len(results),
        "expectation_failure_count": expectation_failures,
        "overall_status": "pass" if expectation_failures == 0 else "fail",
        "status_counts": status_counts,
        "total_logical_duration_ms": sum(result["logical_duration_ms"] for result in results),
    }


def run_acceptance(
    profile: dict[str, Any],
    root: Path = ROOT,
    handlers: dict[str, Handler] | None = None,
    token: CancellationToken | None = None,
) -> dict[str, Any]:
    failures = validate_profile(profile)
    if failures:
        raise ValueError("; ".join(failures))
    registered = default_handlers() if handlers is None else handlers
    expected_checks = {case[1] for case in EXPECTED_CASES}
    if set(registered) != expected_checks:
        raise ValueError("shared acceptance handler registration does not match the plan")
    cancellation = token or CancellationToken()
    limit = profile["output_policy"]["max_retained_bytes_per_case"]
    results = []
    for case in profile["case_order"]:
        status = "cancelled"
        diagnostic = "acceptance run cancelled before case execution"
        metrics: dict[str, str | int | bool] = {}
        if not cancellation.cancelled:
            try:
                observation = registered[case["check_id"]](root)
                if not isinstance(observation, CheckObservation) or not safe_metrics(
                    observation.metrics
                ):
                    raise ValueError("acceptance handler returned an invalid observation")
                status = (
                    "skipped"
                    if observation.skipped
                    else "pass"
                    if observation.passed
                    else "fail"
                )
                diagnostic = observation.diagnostic
                metrics = dict(sorted(observation.metrics.items()))
            except Exception as error:  # The result must retain handler failures.
                status = "error"
                diagnostic = f"{type(error).__name__}: {error}"
                metrics = {}
        result = {
            "test_id": case["test_id"],
            "check_id": case["check_id"],
            "fixture_id": case["fixture_id"],
            "status": status,
            "expected_status": profile["expected_status"],
            "expectation_met": status == profile["expected_status"],
            "logical_duration_ms": case["logical_duration_ms"],
            "timing_kind": "synthetic-logical-not-wall-clock",
            "diagnostic": bound_diagnostic(diagnostic, limit, root),
            "metrics": metrics,
        }
        result["result_sha256"] = sha256_bytes(canonical_json(result))
        results.append(result)
    corpus_manifest = read_json(root / "fixtures/corpus/v1/manifest.json")
    run = {
        "schema_version": 1,
        "run_type": "shared-synthetic-acceptance",
        "run_id": profile["run_id"],
        "generated_at": profile["generated_at"],
        "runner_profile": {
            "id": profile["profile_id"],
            "sha256": sha256_bytes((root / PROFILE_PATH.relative_to(ROOT)).read_bytes()),
        },
        "corpus": {
            "archive_sha256": corpus_manifest["archive"]["sha256"],
            "manifest_sha256": corpus_manifest["manifest_sha256"],
            "version": corpus_manifest["corpus_version"],
        },
        "results": results,
        "summary": summary_for(results),
        "execution_contract": profile["execution_contract"],
        "raw_output_retained": False,
        "macos_execution_status": "blocked-macos",
        "macos_support_claim": "none",
    }
    run["run_sha256"] = sha256_bytes(canonical_json(run))
    return run


def validate_run(run: Any, profile: dict[str, Any]) -> list[str]:
    if not isinstance(run, dict):
        return ["shared acceptance run must be an object"]
    failures: list[str] = []
    expected_fields = {
        "schema_version",
        "run_type",
        "run_id",
        "generated_at",
        "runner_profile",
        "corpus",
        "results",
        "summary",
        "execution_contract",
        "raw_output_retained",
        "macos_execution_status",
        "macos_support_claim",
        "run_sha256",
    }
    if set(run) != expected_fields:
        return ["shared acceptance run fields do not match the contract"]
    unhashed = copy.deepcopy(run)
    recorded_hash = unhashed.pop("run_sha256")
    if not isinstance(recorded_hash, str) or sha256_bytes(canonical_json(unhashed)) != recorded_hash:
        failures.append("shared acceptance run hash is invalid")
    expected_ids = [case["test_id"] for case in profile["case_order"]]
    results = run.get("results", [])
    if [result.get("test_id") for result in results] != expected_ids:
        failures.append("shared acceptance result closure or order drifted")
    limit = profile["output_policy"]["max_retained_bytes_per_case"]
    for result in results:
        unhashed_result = copy.deepcopy(result)
        recorded_result_hash = unhashed_result.pop("result_sha256", None)
        if sha256_bytes(canonical_json(unhashed_result)) != recorded_result_hash:
            failures.append(f"shared acceptance result hash is invalid: {result.get('test_id')}")
        if result.get("status") not in RUN_STATUSES:
            failures.append(f"shared acceptance status is invalid: {result.get('test_id')}")
        if result.get("expectation_met") != (
            result.get("status") == result.get("expected_status")
        ):
            failures.append(
                f"shared acceptance expectation flag is invalid: {result.get('test_id')}"
            )
        diagnostic = result.get("diagnostic", {})
        retained = diagnostic.get("retained_diagnostic", "")
        if len(retained.encode("utf-8")) > limit or SENSITIVE_TEXT.search(retained):
            failures.append(f"shared acceptance diagnostic is unsafe: {result.get('test_id')}")
        if (
            diagnostic.get("retained_bytes") != len(retained.encode("utf-8"))
            or diagnostic.get("retained_sha256")
            != sha256_bytes(retained.encode("utf-8"))
        ):
            failures.append(
                f"shared acceptance diagnostic identity is invalid: {result.get('test_id')}"
            )
        if "raw_diagnostic" in diagnostic:
            failures.append(f"shared acceptance retained raw diagnostic: {result.get('test_id')}")
    if run.get("summary") != summary_for(results):
        failures.append("shared acceptance summary does not reconcile with results")
    if run.get("execution_contract") != EXPECTED_EXECUTION_CONTRACT:
        failures.append("shared acceptance execution contract was weakened")
    if run.get("raw_output_retained") is not False:
        failures.append("shared acceptance run retained raw output")
    if run.get("macos_execution_status") != "blocked-macos" or run.get(
        "macos_support_claim"
    ) != "none":
        failures.append("shared acceptance run made an invalid macOS claim")
    return failures


def write_run(profile: dict[str, Any], output: Path, root: Path = ROOT) -> None:
    if output.exists():
        raise FileExistsError("shared acceptance output already exists")
    output.parent.mkdir(parents=True, exist_ok=True)
    run = run_acceptance(profile, root)
    descriptor, temporary_name = tempfile.mkstemp(prefix=".agentmage-acceptance-", dir=output.parent)
    temporary = Path(temporary_name)
    try:
        with os.fdopen(descriptor, "wb") as handle:
            handle.write(canonical_json(run))
            handle.flush()
            os.fsync(handle.fileno())
        temporary.chmod(0o644)
        os.replace(temporary, output)
    except BaseException:
        temporary.unlink(missing_ok=True)
        raise


def build_report(root: Path = ROOT) -> dict[str, Any]:
    profile_path = root / PROFILE_PATH.relative_to(ROOT)
    profile = read_json(profile_path)
    run = run_acceptance(profile, root)
    failures = validate_run(run, profile)
    if failures:
        raise ValueError("; ".join(failures))
    if run["summary"]["overall_status"] != "pass":
        raise ValueError("canonical shared acceptance run did not pass")
    return {
        "schema_version": 1,
        "task_id": "2.1.2.2",
        "status": "pass",
        "profile": run["runner_profile"],
        "run": {
            "case_count": run["summary"]["case_count"],
            "corpus": run["corpus"],
            "run_id": run["run_id"],
            "run_sha256": run["run_sha256"],
            "status_counts": run["summary"]["status_counts"],
            "total_logical_duration_ms": run["summary"]["total_logical_duration_ms"],
        },
        "registered_checks": [case["check_id"] for case in profile["case_order"]],
        "raw_output_retained": False,
        "run_record_persisted": False,
        "execution_contract": EXPECTED_EXECUTION_CONTRACT,
        "product_acceptance_claim": "none",
        "macos_execution_status": "blocked-macos",
        "macos_support_claim": "none",
    }


def validate_report(report: Any, root: Path = ROOT) -> list[str]:
    if not isinstance(report, dict):
        return ["shared acceptance runner report must be an object"]
    failures: list[str] = []
    if report.get("schema_version") != 1 or report.get("task_id") != "2.1.2.2":
        failures.append("shared acceptance runner report identity is invalid")
    if report.get("status") != "pass":
        failures.append("shared acceptance runner report did not pass")
    if report.get("raw_output_retained") is not False or report.get(
        "run_record_persisted"
    ) is not False:
        failures.append("shared acceptance runner report retained raw output or host run data")
    if report.get("product_acceptance_claim") != "none":
        failures.append("shared acceptance runner made a product acceptance claim")
    if report.get("macos_execution_status") != "blocked-macos" or report.get(
        "macos_support_claim"
    ) != "none":
        failures.append("shared acceptance runner report made an invalid macOS claim")
    if report != build_report(root):
        failures.append("shared acceptance runner report is stale or non-deterministic")
    return failures


def write_report(root: Path = ROOT) -> None:
    output = root / REPORT_PATH.relative_to(ROOT)
    output.parent.mkdir(parents=True, exist_ok=True)
    output.write_bytes(canonical_json(build_report(root)))


def check_report(root: Path = ROOT) -> list[str]:
    try:
        report = read_json(root / REPORT_PATH.relative_to(ROOT))
    except (OSError, json.JSONDecodeError) as error:
        return [f"cannot read shared acceptance runner report: {error}"]
    return validate_report(report, root)


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--output", type=Path)
    parser.add_argument("--write-report", action="store_true")
    args = parser.parse_args()
    try:
        profile = read_json(PROFILE_PATH)
        if args.output is not None:
            write_run(profile, args.output)
        if args.write_report:
            write_report()
        failures = check_report()
    except (OSError, ValueError) as error:
        print(f"shared acceptance runner failed: {error}", file=sys.stderr)
        return 1
    if failures:
        for failure in failures:
            print(f"shared acceptance runner failed: {failure}", file=sys.stderr)
        return 1
    if args.output is None:
        print("shared synthetic acceptance runner validated")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
