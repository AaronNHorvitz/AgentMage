#!/usr/bin/env python3
"""Build the gate-owned Sprint 48 thin-client boundary review."""

from __future__ import annotations

import argparse
import hashlib
import json
import subprocess
import sys
from pathlib import Path
from typing import Any, Final

ROOT: Final = Path(__file__).resolve().parents[1]
REPORT: Final = ROOT / "artifacts/sprints/sprint-48/source-boundary-review.json"
REQUIREMENTS: Final = [
    "SR-PLT-005", "SR-PLT-006", "SR-ACC-001", "SR-ACC-007",
    "SR-OPS-001", "SR-TST-001", "SR-TST-004",
]
SOURCES: Final = (
    "shells/host/src/headless.rs",
    "shells/host/src/cli.rs",
    "shells/host/src/bin/agent.rs",
    "docs/verification/sprint-48-headless-corpus.json",
    "docs/verification/sprint-48-local-results.md",
    "artifacts/sprints/sprint-48/local-evidence-report.json",
    "scripts/sprint_48_evidence.py",
)


def git_bytes(revision: str, path: str) -> bytes:
    result = subprocess.run(
        ["git", "show", f"{revision}:{path}"], cwd=ROOT, capture_output=True,
        check=False, timeout=30,
    )
    if result.returncode:
        raise ValueError(f"review source unavailable: {path}")
    return result.stdout


def expected(revision: str) -> dict[str, Any]:
    sources = {path: git_bytes(revision, path) for path in SOURCES}
    local = json.loads(sources["artifacts/sprints/sprint-48/local-evidence-report.json"])
    implemented = local["implemented_contracts"]
    verification = local["verification_evidence"]
    summary = local["summary"]
    checks = {
        "security_requirements_are_mapped": local.get("security_requirement_ids") == REQUIREMENTS,
        "closed_protocol_and_rendering_are_bound": all(
            implemented.get(key) is True for key in (
                "closed_thin_client_protocol", "closed_command_taxonomy",
                "shared_status_projection", "stable_exit_codes",
                "deterministic_shell_completion", "bounded_request_and_output",
                "closed_request_and_event_schemas",
            )
        ),
        "authority_and_transport_boundary_are_bound": all(
            implemented.get(key) is True for key in (
                "exact_predeclared_headless_grants", "headless_interactive_approval_denied",
                "transport_only_clients", "surface_independent_kernel_identity",
                "hash_chained_event_stream", "replay_resume_and_cancellation_contracts",
                "executable_cli_fail_closed_fixture",
            )
        ),
        "five_surface_and_adversarial_results_are_bound": (
            verification.get("client_surface_count") == 5
            and verification.get("command_family_count") == 4
            and verification.get("stable_exit_code_count") == 9
            and verification.get("runtime_record_count") == 23
            and verification.get("thin_client_schema_count") == 2
            and verification.get("adversarial_corpus_case_count") == 40
            and verification.get("unauthorized_effect_acceptance_count") == 0
        ),
        "local_contract_is_bound": summary.get("local_sprint_48_contract_passed") is True,
        "missing_product_proof_remains_false": all(
            verification.get(key) is False for key in (
                "production_authenticated_cli_transport", "production_command_coordinators",
                "native_disconnect_process_tree_campaign", "native_cross_platform_acceptance",
                "trusted_package_execution", "independent_review", "manual_fuzzing",
            )
        ),
    }
    return {
        "schema_version": 1,
        "record_type": "agentmage-sprint-48-source-boundary-review",
        "source_revision": revision,
        "reviewer_identity": "scripts/sprint_48_boundary_review.py",
        "review_class": "gate-owned-automated-thin-client-boundary-review",
        "independent_human_review_performed": False,
        "security_requirement_ids": REQUIREMENTS,
        "source_sha256": {
            path: hashlib.sha256(value).hexdigest() for path, value in sources.items()
        },
        "checks": checks,
        "status": "PASS_LOCAL_BOUNDARY_REVIEW" if all(checks.values()) else "FAIL",
        "limitations": [
            "This is gate-owned automated review, not a human-review claim.",
            "Authenticated product transport and production coordinators remain absent.",
            "Native disconnect, descendant cleanup, and platform parity remain blocked.",
            "No model, installed interface, platform support, or release is enabled.",
        ],
    }


def validate(value: Any) -> list[str]:
    if not isinstance(value, dict):
        return ["review is not an object"]
    failures: list[str] = []
    if value.get("reviewer_identity") != "scripts/sprint_48_boundary_review.py":
        failures.append("reviewer identity drift")
    if value.get("independent_human_review_performed") is not False:
        failures.append("human review overclaim")
    if value.get("security_requirement_ids") != REQUIREMENTS:
        failures.append("security mapping drift")
    checks = value.get("checks")
    if not isinstance(checks, dict) or not checks or any(item is not True for item in checks.values()):
        failures.append("review check failed or suppressed")
    if value.get("status") != "PASS_LOCAL_BOUNDARY_REVIEW":
        failures.append("review status is not pass")
    revision = str(value.get("source_revision", ""))
    if len(revision) != 40 or any(character not in "0123456789abcdef" for character in revision):
        return failures + ["source revision invalid"]
    try:
        if value != expected(revision):
            failures.append("review is stale, incomplete, reordered, or widened")
    except (ValueError, json.JSONDecodeError) as error:
        failures.append(str(error))
    return failures


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--write", action="store_true")
    parser.add_argument("--source-revision", default="HEAD")
    arguments = parser.parse_args()
    revision = subprocess.run(
        ["git", "rev-parse", arguments.source_revision], cwd=ROOT, check=True,
        capture_output=True, text=True,
    ).stdout.strip()
    if arguments.write:
        REPORT.parent.mkdir(parents=True, exist_ok=True)
        REPORT.write_text(json.dumps(expected(revision), indent=2, sort_keys=True) + "\n")
    try:
        value = json.loads(REPORT.read_text())
    except (OSError, json.JSONDecodeError) as error:
        print(error, file=sys.stderr)
        return 1
    failures = validate(value)
    if failures:
        print("\n".join(failures), file=sys.stderr)
        return 1
    print("Sprint 48 gate-owned boundary review passed")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
