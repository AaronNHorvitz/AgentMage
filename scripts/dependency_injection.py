#!/usr/bin/env python3
"""Inject and verify every prohibited Story 1.1 compile dependency edge."""

from __future__ import annotations

import argparse
import copy
import json
import sys
from pathlib import Path
from typing import Any

try:
    from scripts.dependency_rules import (
        load_rules,
        prohibited_compile_edges,
        validate_rules,
    )
    from scripts.module_inventory import load_inventory
except ModuleNotFoundError:
    from dependency_rules import load_rules, prohibited_compile_edges, validate_rules
    from module_inventory import load_inventory


ROOT = Path(__file__).resolve().parents[1]
REPORT_PATH = (
    ROOT
    / "artifacts"
    / "sprints"
    / "sprint-1"
    / "story-1.1"
    / "dependency-injection-report.json"
)


def _policy(rules: dict[str, Any], module_id: str) -> dict[str, Any]:
    return next(item for item in rules["module_rules"] if item["id"] == module_id)


def build_report() -> dict[str, Any]:
    rules = load_rules()
    inventory = load_inventory()
    cases = []
    for index, (source, target) in enumerate(prohibited_compile_edges(), start=1):
        mutated = copy.deepcopy(rules)
        _policy(mutated, source)["declared_imports"].append(target)
        diagnostics = validate_rules(mutated, inventory)
        expected = f"prohibited import edge: {source} -> {target}; outside its allowlist"
        cases.append(
            {
                "case_id": f"S-001-UT01-{index:03d}",
                "source": source,
                "target": target,
                "expected_diagnostic": expected,
                "observed_diagnostics": diagnostics,
                "status": "pass" if expected in diagnostics else "fail",
            }
        )
    return {
        "schema_version": 1,
        "test_id": "S-001-UT01",
        "status": "pass" if all(item["status"] == "pass" for item in cases) else "fail",
        "scope": "platform-neutral-static-architecture-check",
        "macos_support_claim": "none",
        "case_count": len(cases),
        "cases": cases,
    }


def validate_report(report: Any) -> list[str]:
    failures: list[str] = []
    expected = build_report()
    if not isinstance(report, dict):
        return ["dependency injection report must be an object"]
    if report.get("schema_version") != 1 or report.get("test_id") != "S-001-UT01":
        failures.append("dependency injection report identity is invalid")
    if report.get("scope") != "platform-neutral-static-architecture-check":
        failures.append("dependency injection scope is invalid")
    if report.get("macos_support_claim") != "none":
        failures.append("static dependency tests cannot make a macOS support claim")
    if report.get("case_count") != len(prohibited_compile_edges()):
        failures.append("dependency injection case count is incomplete")
    if report.get("status") != "pass":
        failures.append("dependency injection report is not passing")
    for case in report.get("cases", []):
        if case.get("status") != "pass":
            failures.append(f"dependency injection case failed: {case.get('case_id')}")
        if case.get("expected_diagnostic") not in case.get("observed_diagnostics", []):
            failures.append(
                f"dependency injection diagnostic is imprecise: {case.get('case_id')}"
            )
    if report != expected:
        failures.append("dependency injection report is stale or non-deterministic")
    return failures


def write_report() -> None:
    REPORT_PATH.parent.mkdir(parents=True, exist_ok=True)
    REPORT_PATH.write_text(
        json.dumps(build_report(), indent=2, sort_keys=True) + "\n",
        encoding="utf-8",
    )


def check_report() -> list[str]:
    try:
        report = json.loads(REPORT_PATH.read_text(encoding="utf-8"))
    except (OSError, json.JSONDecodeError) as error:
        return [f"cannot read dependency injection report: {error}"]
    return validate_report(report)


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--write", action="store_true")
    args = parser.parse_args()
    if args.write:
        write_report()
    failures = check_report()
    if failures:
        for failure in failures:
            print(f"dependency injection validation failed: {failure}", file=sys.stderr)
        return 1
    print(f"validated {len(prohibited_compile_edges())} prohibited dependency edges")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
