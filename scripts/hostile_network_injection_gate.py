#!/usr/bin/env python3
"""Apply hostile network mutations without executing injected code."""

from __future__ import annotations

import json
import sys
from pathlib import Path
from typing import Any, Final


ROOT: Final = Path(__file__).resolve().parents[1]
if str(ROOT) not in sys.path:
    sys.path.insert(0, str(ROOT))

from scripts import strict_local_source_audit as audit  # noqa: E402


FIXTURE_PATH: Final = (
    ROOT / "fixtures/strict-local-hostile-surfaces/v1/mutations.json"
)
EXPECTED_IDS: Final = (
    "telemetry-upload",
    "crash-upload",
    "remote-font-or-asset",
    "marketplace-contact",
    "update-download",
    "ambient-proxy",
    "hostile-loopback-client",
    "hostile-loopback-runtime-dependency",
)
CASE_FIELDS: Final = {
    "source": {"content", "expected_failures", "id", "kind", "target"},
    "npm_runtime_dependency": {"expected_failures", "id", "kind", "package"},
}


class HostileInjectionGateError(ValueError):
    """Raised when the hostile-surface corpus or result is invalid."""


def load_fixture(path: Path = FIXTURE_PATH) -> dict[str, Any]:
    try:
        value = json.loads(path.read_text(encoding="utf-8"))
    except (OSError, UnicodeError, json.JSONDecodeError) as error:
        raise HostileInjectionGateError("hostile fixture is unavailable") from error
    if errors := validate_fixture(value):
        raise HostileInjectionGateError("; ".join(errors))
    return value


def validate_fixture(value: Any) -> list[str]:
    if not isinstance(value, dict) or set(value) != {"cases", "schema_version"}:
        return ["hostile fixture fields changed"]
    if value.get("schema_version") != 1 or not isinstance(value.get("cases"), list):
        return ["hostile fixture schema changed"]
    cases = value["cases"]
    if [case.get("id") for case in cases if isinstance(case, dict)] != list(EXPECTED_IDS):
        return ["hostile fixture identity closure changed"]
    failures: list[str] = []
    for case in cases:
        if not isinstance(case, dict):
            failures.append("hostile fixture case is invalid")
            continue
        kind = case.get("kind")
        if kind not in CASE_FIELDS or set(case) != CASE_FIELDS[kind]:
            failures.append(f"hostile fixture case fields changed: {case.get('id')}")
            continue
        expected = case.get("expected_failures")
        if (
            not isinstance(expected, list)
            or not expected
            or any(not isinstance(item, str) or not item for item in expected)
            or expected != sorted(set(expected))
        ):
            failures.append(f"hostile fixture detector closure changed: {case['id']}")
        if kind == "source" and (
            not isinstance(case.get("target"), str)
            or not case["target"]
            or not isinstance(case.get("content"), str)
            or not case["content"]
        ):
            failures.append(f"hostile source fixture is invalid: {case['id']}")
        if kind == "npm_runtime_dependency" and (
            not isinstance(case.get("package"), str) or not case["package"]
        ):
            failures.append(f"hostile dependency fixture is invalid: {case['id']}")
    return sorted(set(failures))


def evaluate_case(
    policy: dict[str, Any], sources: dict[str, str], case: dict[str, Any]
) -> list[str]:
    if case["kind"] == "source":
        changed = dict(sources)
        changed[case["target"]] = case["content"]
        return audit.scan_sources(policy, changed)
    package = case["package"]
    return sorted(
        audit.audit_npm_runtime_packages(policy, {package}, {package})
    )


def run_gate(fixture: dict[str, Any] | None = None) -> dict[str, Any]:
    selected = load_fixture() if fixture is None else fixture
    if errors := validate_fixture(selected):
        raise HostileInjectionGateError("; ".join(errors))
    policy = audit.load_policy()
    sources = audit.source_map(policy)
    if baseline := audit.audit(policy, sources):
        raise HostileInjectionGateError(
            f"strict-local baseline failed with {len(baseline)} finding(s)"
        )
    results: list[dict[str, Any]] = []
    for case in selected["cases"]:
        observed = evaluate_case(policy, sources, case)
        if observed != case["expected_failures"]:
            raise HostileInjectionGateError(
                f"hostile mutation detector mismatch: {case['id']}"
            )
        results.append(
            {
                "detector_count": len(observed),
                "id": case["id"],
                "status": "blocked-before-execution",
            }
        )
    return {
        "schema_version": 1,
        "baseline_status": "pass",
        "case_count": len(results),
        "injected_code_executed": False,
        "external_network_used": False,
        "results": results,
        "status": "pass-all-hostile-surfaces-blocked-before-execution",
    }


def expected_result() -> dict[str, Any]:
    expected_results = [
        {
            "detector_count": 3 if identifier == EXPECTED_IDS[-1] else (
                2 if identifier in {"remote-font-or-asset", "marketplace-contact"} else 1
            ),
            "id": identifier,
            "status": "blocked-before-execution",
        }
        for identifier in EXPECTED_IDS
    ]
    return {
        "schema_version": 1,
        "baseline_status": "pass",
        "case_count": len(EXPECTED_IDS),
        "injected_code_executed": False,
        "external_network_used": False,
        "results": expected_results,
        "status": "pass-all-hostile-surfaces-blocked-before-execution",
    }


def validate_result(value: Any) -> list[str]:
    if not isinstance(value, dict) or set(value) != set(expected_result()):
        return ["hostile gate result fields changed"]
    return [] if value == expected_result() else ["hostile gate result changed"]


def main() -> int:
    try:
        result = run_gate()
        if errors := validate_result(result):
            raise HostileInjectionGateError("; ".join(errors))
    except (OSError, HostileInjectionGateError, audit.StrictLocalSourceAuditError) as error:
        print(f"Hostile network injection gate failed: {error}", file=sys.stderr)
        return 1
    print("Hostile network injection gate blocked all 8 cases before execution")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
