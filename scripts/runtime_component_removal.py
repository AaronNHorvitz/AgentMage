#!/usr/bin/env python3
"""Run and verify the closed Story 50.2 component-removal campaign."""

from __future__ import annotations

import argparse
import hashlib
import json
import os
import platform
import re
import shutil
import subprocess
import tempfile
from datetime import UTC, datetime
from pathlib import Path
from typing import Any

ROOT = Path(__file__).resolve().parents[1]
PROFILE = Path("fixtures/runtime-hardening/v1/component-removal-profile.json")
DEFAULT_OUTPUT = Path("artifacts/sprints/sprint-50/story-50.2-component-removal")
REPORT_NAME = "report.json"
HEX_64 = re.compile(r"^[0-9a-f]{64}$")
HEX_40 = re.compile(r"^[0-9a-f]{40}$")
TEST_RESULT = re.compile(
    r"test result: ok\. (\d+) passed; (\d+) failed; (\d+) ignored; "
    r"(\d+) measured; (\d+) filtered out"
)


class CampaignError(RuntimeError):
    """Raised when the closed campaign contract is violated."""


def _sha256_bytes(value: bytes) -> str:
    return hashlib.sha256(value).hexdigest()


def _sha256_file(path: Path) -> str:
    return _sha256_bytes(path.read_bytes())


def _run(
    argv: list[str],
    *,
    env: dict[str, str] | None = None,
    check: bool = True,
) -> subprocess.CompletedProcess[str]:
    result = subprocess.run(
        argv,
        cwd=ROOT,
        env=env,
        check=False,
        stdout=subprocess.PIPE,
        stderr=subprocess.STDOUT,
        text=True,
        timeout=1_800,
    )
    if check and result.returncode != 0:
        raise CampaignError(f"command failed ({result.returncode}): {argv!r}\n{result.stdout}")
    return result


def _git_bytes(revision: str, path: str) -> bytes:
    result = subprocess.run(
        ["git", "show", f"{revision}:{path}"],
        cwd=ROOT,
        check=False,
        stdout=subprocess.PIPE,
        stderr=subprocess.PIPE,
        timeout=30,
    )
    if result.returncode != 0:
        raise CampaignError(f"source path is absent from {revision}: {path}")
    return result.stdout


def _expected_argv(package: str, features: list[str], command: str) -> list[str]:
    argv = ["cargo", command, "-p", package, "--lib"]
    if command == "clippy":
        argv.append("--tests")
    argv.extend(["--locked", "--offline", "--no-default-features"])
    if features:
        argv.extend(["--features", ",".join(features)])
    if command == "clippy":
        argv.extend(["--", "-D", "warnings"])
    return argv


def validate_profile(profile: dict[str, Any]) -> None:
    """Validate the exact closed campaign profile."""
    if set(profile) != {
        "schema_version",
        "campaign_id",
        "disposition_on_success",
        "scenarios",
        "source_paths",
        "declared_limitations",
    }:
        raise CampaignError("profile fields are not closed")
    if profile["schema_version"] != 1:
        raise CampaignError("profile schema version is unsupported")
    if profile["campaign_id"] != "story-50.2-component-removal-v1":
        raise CampaignError("campaign identity changed")
    if profile["disposition_on_success"] != "LOCAL-SOURCE-PASS":
        raise CampaignError("campaign disposition changed")

    scenarios = profile["scenarios"]
    expected_ids = [
        "cli-removed",
        "native-chat-removed",
        "workflow-caller-removed",
        "runtime-projections-removed",
    ]
    if not isinstance(scenarios, list) or [item.get("id") for item in scenarios] != expected_ids:
        raise CampaignError("scenario identities or order changed")
    scenario_fields = {
        "id",
        "component",
        "package",
        "crate_root",
        "features",
        "removed_sources",
        "required_sources",
        "minimum_passed_tests",
        "expected_ignored_tests",
        "test_argv",
        "clippy_argv",
    }
    for scenario in scenarios:
        if set(scenario) != scenario_fields:
            raise CampaignError(f"scenario fields are not closed: {scenario.get('id')}")
        for key in ("features", "removed_sources", "required_sources"):
            values = scenario[key]
            if not isinstance(values, list) or values != sorted(set(values)):
                raise CampaignError(f"{scenario['id']} {key} must be unique and sorted")
        if not scenario["removed_sources"] or not scenario["required_sources"]:
            raise CampaignError(f"{scenario['id']} has an empty source boundary")
        if set(scenario["removed_sources"]) & set(scenario["required_sources"]):
            raise CampaignError(f"{scenario['id']} source boundaries overlap")
        if scenario["minimum_passed_tests"] <= 0 or scenario["expected_ignored_tests"] < 0:
            raise CampaignError(f"{scenario['id']} test bounds are invalid")
        if scenario["test_argv"] != _expected_argv(
            scenario["package"], scenario["features"], "test"
        ):
            raise CampaignError(f"{scenario['id']} test command changed")
        if scenario["clippy_argv"] != _expected_argv(
            scenario["package"], scenario["features"], "clippy"
        ):
            raise CampaignError(f"{scenario['id']} Clippy command changed")

    source_paths = profile["source_paths"]
    if not isinstance(source_paths, list) or source_paths != sorted(set(source_paths)):
        raise CampaignError("source paths must be unique and sorted")
    referenced = {
        value
        for scenario in scenarios
        for key in ("removed_sources", "required_sources")
        for value in scenario[key]
    }
    if not referenced.issubset(set(source_paths)):
        raise CampaignError("a scenario source is absent from the source registry")
    limitations = profile["declared_limitations"]
    if not isinstance(limitations, list) or len(limitations) != 4:
        raise CampaignError("exactly four campaign limitations are required")


def parse_test_results(output: str) -> dict[str, int]:
    """Aggregate only successful standard Rust test summaries."""
    matches = TEST_RESULT.findall(output)
    if not matches:
        raise CampaignError("Rust test summary is absent")
    totals = {
        "passed": 0,
        "failed": 0,
        "ignored": 0,
        "measured": 0,
        "filtered_out": 0,
    }
    for match in matches:
        for key, raw in zip(totals, match, strict=True):
            totals[key] += int(raw)
    if totals["failed"] != 0 or totals["measured"] != 0:
        raise CampaignError("test output contains failed or measured cases")
    return totals


def _dep_sources(target_dir: Path, scenario: dict[str, Any]) -> tuple[Path, list[str]]:
    crate_name = scenario["package"].replace("-", "_")
    candidates: list[tuple[Path, list[str]]] = []
    for path in (target_dir / "debug" / "deps").glob(f"{crate_name}-*.d"):
        text = path.read_text(encoding="utf-8", errors="strict")
        sources = sorted(
            {
                token.rstrip(":\\")
                for token in text.split()
                if token.rstrip(":\\").endswith((".rs", "Cargo.toml"))
            }
        )
        if scenario["crate_root"] in sources:
            candidates.append((path, sources))
    if not candidates:
        raise CampaignError(f"{scenario['id']} has no library dep-info")
    matching = [
        item
        for item in candidates
        if set(scenario["required_sources"]).issubset(set(item[1]))
    ]
    if len(matching) != 1:
        raise CampaignError(
            f"{scenario['id']} expected one complete dep-info manifest, found {len(matching)}"
        )
    return matching[0]


def source_manifest_failures(
    sources: list[str], scenario: dict[str, Any]
) -> list[str]:
    """Return deterministic source inclusion failures for one scenario."""
    source_set = set(sources)
    failures = [
        f"removed source compiled: {path}"
        for path in scenario["removed_sources"]
        if path in source_set
    ]
    failures.extend(
        f"required source missing: {path}"
        for path in scenario["required_sources"]
        if path not in source_set
    )
    return failures


def _source_hashes(revision: str, paths: list[str]) -> dict[str, str]:
    hashes: dict[str, str] = {}
    for path in paths:
        committed = _git_bytes(revision, path)
        current = (ROOT / path).read_bytes()
        if committed != current:
            raise CampaignError(f"working source differs from {revision}: {path}")
        hashes[path] = _sha256_bytes(committed)
    return hashes


def _clean_revision() -> tuple[str, str]:
    status = _run(["git", "status", "--porcelain", "--untracked-files=all"]).stdout
    if status:
        raise CampaignError("component-removal evidence requires a clean Git worktree")
    revision = _run(["git", "rev-parse", "HEAD"]).stdout.strip()
    branch = _run(["git", "branch", "--show-current"]).stdout.strip()
    if not HEX_40.fullmatch(revision) or not branch:
        raise CampaignError("Git source identity is unavailable")
    return revision, branch


def _write_text(path: Path, value: str) -> None:
    path.write_text(value, encoding="utf-8", newline="\n")


def run_campaign(profile: dict[str, Any], output: Path) -> dict[str, Any]:
    """Execute the exact offline removal matrix and atomically retain its report."""
    validate_profile(profile)
    revision, branch = _clean_revision()
    source_sha256 = _source_hashes(revision, profile["source_paths"])
    output = (ROOT / output).resolve()
    output.parent.mkdir(parents=True, exist_ok=True)
    staging = Path(tempfile.mkdtemp(prefix=".component-removal-", dir=output.parent))
    scenario_reports: list[dict[str, Any]] = []
    try:
        with tempfile.TemporaryDirectory(prefix="agentmage-component-removal-") as target_parent:
            for scenario in profile["scenarios"]:
                target_dir = Path(target_parent) / scenario["id"]
                env = os.environ.copy()
                env["CARGO_TARGET_DIR"] = str(target_dir)
                env["CARGO_NET_OFFLINE"] = "true"

                test_result = _run(scenario["test_argv"], env=env, check=False)
                test_log = staging / f"{scenario['id']}.test.log"
                _write_text(test_log, test_result.stdout)
                if test_result.returncode != 0:
                    raise CampaignError(f"{scenario['id']} tests failed")
                tests = parse_test_results(test_result.stdout)
                if tests["passed"] < scenario["minimum_passed_tests"]:
                    raise CampaignError(f"{scenario['id']} passed too few tests")
                if tests["ignored"] != scenario["expected_ignored_tests"]:
                    raise CampaignError(f"{scenario['id']} ignored-test count changed")

                dep_path, sources = _dep_sources(target_dir, scenario)
                failures = source_manifest_failures(sources, scenario)
                if failures:
                    raise CampaignError(f"{scenario['id']}: {'; '.join(failures)}")
                manifest_path = staging / f"{scenario['id']}.compiled-sources.txt"
                _write_text(manifest_path, "\n".join(sources) + "\n")

                clippy_result = _run(scenario["clippy_argv"], env=env, check=False)
                clippy_log = staging / f"{scenario['id']}.clippy.log"
                _write_text(clippy_log, clippy_result.stdout)
                if clippy_result.returncode != 0:
                    raise CampaignError(f"{scenario['id']} strict Clippy failed")

                scenario_reports.append(
                    {
                        "id": scenario["id"],
                        "component": scenario["component"],
                        "features": scenario["features"],
                        "test_argv": scenario["test_argv"],
                        "test_exit_code": test_result.returncode,
                        "test_log": test_log.name,
                        "test_log_sha256": _sha256_file(test_log),
                        "tests": tests,
                        "clippy_argv": scenario["clippy_argv"],
                        "clippy_exit_code": clippy_result.returncode,
                        "clippy_log": clippy_log.name,
                        "clippy_log_sha256": _sha256_file(clippy_log),
                        "compiled_sources": manifest_path.name,
                        "compiled_sources_sha256": _sha256_file(manifest_path),
                        "compiled_source_count": len(sources),
                        "dep_info_file": dep_path.name,
                        "removed_sources": scenario["removed_sources"],
                        "required_sources": scenario["required_sources"],
                        "removal_verified": True,
                    }
                )

        report = {
            "schema_version": 1,
            "campaign_id": profile["campaign_id"],
            "source_commit": revision,
            "source_branch": branch,
            "generated_at_utc": datetime.now(UTC).isoformat(),
            "environment": {
                "system": platform.system(),
                "release": platform.release(),
                "machine": platform.machine(),
                "rustc": _run(["rustc", "--version"]).stdout.strip(),
                "cargo": _run(["cargo", "--version"]).stdout.strip(),
                "network_mode": "Cargo offline; campaign commands use no shell",
            },
            "source_sha256": source_sha256,
            "scenarios": scenario_reports,
            "declared_limitations": profile["declared_limitations"],
            "disposition": profile["disposition_on_success"],
            "campaign_passed": True,
        }
        _write_text(staging / REPORT_NAME, json.dumps(report, indent=2, sort_keys=True) + "\n")
        failures = report_failures(report, profile, staging)
        if failures:
            raise CampaignError("report self-validation failed: " + "; ".join(failures))
        if output.exists():
            shutil.rmtree(output)
        staging.rename(output)
        return report
    except Exception:
        shutil.rmtree(staging, ignore_errors=True)
        raise


def report_failures(
    report: dict[str, Any], profile: dict[str, Any], artifact: Path
) -> list[str]:
    """Return currentness, integrity, and result failures for retained evidence."""
    failures: list[str] = []
    required = {
        "schema_version",
        "campaign_id",
        "source_commit",
        "source_branch",
        "generated_at_utc",
        "environment",
        "source_sha256",
        "scenarios",
        "declared_limitations",
        "disposition",
        "campaign_passed",
    }
    if set(report) != required:
        return ["report fields are not closed"]
    if report["schema_version"] != 1 or report["campaign_id"] != profile["campaign_id"]:
        failures.append("report identity mismatch")
    revision = report["source_commit"]
    if not isinstance(revision, str) or not HEX_40.fullmatch(revision):
        failures.append("invalid source commit")
    if report["disposition"] != profile["disposition_on_success"] or not report["campaign_passed"]:
        failures.append("campaign disposition is not passing")
    if report["declared_limitations"] != profile["declared_limitations"]:
        failures.append("declared limitations changed")
    if set(report["source_sha256"]) != set(profile["source_paths"]):
        failures.append("source hash registry mismatch")
    elif isinstance(revision, str) and HEX_40.fullmatch(revision):
        for path, expected in report["source_sha256"].items():
            if not isinstance(expected, str) or not HEX_64.fullmatch(expected):
                failures.append(f"invalid source hash: {path}")
                continue
            try:
                actual = _sha256_bytes(_git_bytes(revision, path))
            except CampaignError:
                failures.append(f"missing source blob: {path}")
                continue
            if actual != expected:
                failures.append(f"source hash mismatch: {path}")

    expected_scenarios = profile["scenarios"]
    scenarios = report["scenarios"]
    if [item.get("id") for item in scenarios] != [item["id"] for item in expected_scenarios]:
        failures.append("scenario registry mismatch")
        return failures
    for result, expected in zip(scenarios, expected_scenarios, strict=True):
        if result.get("features") != expected["features"]:
            failures.append(f"{expected['id']} feature mismatch")
        if result.get("test_argv") != expected["test_argv"]:
            failures.append(f"{expected['id']} test command mismatch")
        if result.get("clippy_argv") != expected["clippy_argv"]:
            failures.append(f"{expected['id']} Clippy command mismatch")
        tests = result.get("tests", {})
        if (
            result.get("test_exit_code") != 0
            or result.get("clippy_exit_code") != 0
            or tests.get("failed") != 0
            or tests.get("measured") != 0
            or tests.get("passed", -1) < expected["minimum_passed_tests"]
            or tests.get("ignored") != expected["expected_ignored_tests"]
            or not result.get("removal_verified")
        ):
            failures.append(f"{expected['id']} result mismatch")
        for path_key, hash_key in (
            ("test_log", "test_log_sha256"),
            ("clippy_log", "clippy_log_sha256"),
            ("compiled_sources", "compiled_sources_sha256"),
        ):
            path = artifact / str(result.get(path_key, ""))
            expected_hash = result.get(hash_key)
            if not path.is_file() or not isinstance(expected_hash, str):
                failures.append(f"{expected['id']} missing {path_key}")
            elif _sha256_file(path) != expected_hash:
                failures.append(f"{expected['id']} {path_key} hash mismatch")
        manifest = artifact / str(result.get("compiled_sources", ""))
        if manifest.is_file():
            sources = manifest.read_text(encoding="utf-8").splitlines()
            failures.extend(
                f"{expected['id']} {failure}"
                for failure in source_manifest_failures(sources, expected)
            )
            if len(sources) != result.get("compiled_source_count"):
                failures.append(f"{expected['id']} compiled source count mismatch")
    return failures


def verify_retained(profile: dict[str, Any], output: Path) -> dict[str, Any]:
    """Verify one retained campaign without executing Cargo."""
    validate_profile(profile)
    artifact = (ROOT / output).resolve()
    report = json.loads((artifact / REPORT_NAME).read_text(encoding="utf-8"))
    failures = report_failures(report, profile, artifact)
    if failures:
        raise CampaignError("retained evidence failed: " + "; ".join(failures))
    return report


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--verify", action="store_true", help="verify retained evidence only")
    parser.add_argument("--output", type=Path, default=DEFAULT_OUTPUT)
    args = parser.parse_args()
    profile = json.loads((ROOT / PROFILE).read_text(encoding="utf-8"))
    report = verify_retained(profile, args.output) if args.verify else run_campaign(profile, args.output)
    print(
        json.dumps(
            {
                "campaign_id": report["campaign_id"],
                "source_commit": report["source_commit"],
                "disposition": report["disposition"],
                "scenarios": len(report["scenarios"]),
            },
            sort_keys=True,
        )
    )
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
