#!/usr/bin/env python3
"""Generate source-bound local evidence for Story 16.2."""

from __future__ import annotations

import hashlib
import json
import subprocess
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
EVIDENCE_DIR = ROOT / "artifacts/sprints/sprint-16/story-16.2"
LOG_PATH = EVIDENCE_DIR / "artifact-protocol-results.log"
REPORT_PATH = EVIDENCE_DIR / "artifact-protocol-report.json"

COMMANDS = (
    ("cargo", "test", "-p", "agentmage-capability-read-only", "artifact::tests", "--locked"),
    ("cargo", "test", "-p", "agentmage-host", "runtime_tools", "--locked"),
    ("cargo", "test", "-p", "agentmage-host", "runtime_parity_tests", "--locked"),
    (
        "cargo", "clippy", "-p", "agentmage-capability-read-only", "-p", "agentmage-host",
        "--all-targets", "--all-features", "--locked", "--", "-D", "warnings",
    ),
)

SOURCES = (
    "capabilities/read-only/src/artifact.rs",
    "capabilities/read-only/src/lib.rs",
    "shells/host/src/runtime_tools.rs",
    "shells/host/src/coding_tools.rs",
    "docs/architecture/native-source-artifact-tool-protocol.md",
    "scripts/story_16_2_artifact_protocol_evidence.py",
    "tests/test_story_16_2_artifact_protocol_evidence.py",
)

MARKERS = (
    "catalog_is_exact_closed_read_only_and_extensions_are_unregistered ... ok",
    "all_seven_fake_operations_are_deterministic_typed_and_receipted ... ok",
    "byte_line_page_sheet_cell_section_and_search_projections_are_exact ... ok",
    "every_tool_rejects_the_closed_schema_and_repeat_matrix_before_a_second_launch ... ok",
    "cancellation_timeout_and_crash_are_receipted_for_every_tool ... ok",
    "stale_restricted_unsupported_redacted_missing_and_range_fail_closed ... ok",
    "result_limits_truncate_or_return_large_reference_without_hidden_omission ... ok",
    "story_16_2_artifact_catalog_uses_common_registry_and_closed_validator ... ok",
    "story_50_2_read_only_and_coding_packets_are_equal_across_all_three_callers ... ok",
)


def digest(path: Path) -> str:
    return hashlib.sha256(path.read_bytes()).hexdigest()


def artifact(relative: str) -> dict[str, object]:
    path = ROOT / relative
    return {"path": relative, "byte_length": path.stat().st_size, "sha256": digest(path)}


def run() -> dict[str, object]:
    EVIDENCE_DIR.mkdir(parents=True, exist_ok=True)
    records: list[str] = []
    for command in COMMANDS:
        completed = subprocess.run(
            command,
            cwd=ROOT,
            check=False,
            stdout=subprocess.PIPE,
            stderr=subprocess.STDOUT,
            text=True,
        )
        records.append(f"$ {' '.join(command)}\n{completed.stdout}")
        if completed.returncode != 0:
            LOG_PATH.write_text("\n".join(records), encoding="utf-8")
            raise SystemExit(completed.returncode)
    LOG_PATH.write_text("\n".join(records), encoding="utf-8")
    log = LOG_PATH.read_text(encoding="utf-8")
    missing = [marker for marker in MARKERS if marker not in log]
    if missing:
        raise SystemExit(f"missing retained markers: {missing}")
    report: dict[str, object] = {
        "schema_version": 1,
        "record_type": "agentmage-story-16-2-artifact-protocol-evidence",
        "story_id": "16.2",
        "protocols": ["RV-03", "RV-04", "RV-08", "RV-16"],
        "generated_on": "2026-08-31",
        "status": "PASS_LOCAL_PROTOCOL_AND_FAKE_BACKEND",
        "commands": [" ".join(command) for command in COMMANDS],
        "required_markers": list(MARKERS),
        "artifacts": [artifact(relative) for relative in SOURCES]
        + [artifact(LOG_PATH.relative_to(ROOT).as_posix())],
        "catalog": [
            "artifact.list", "artifact.metadata", "artifact.read", "artifact.range",
            "artifact.sections", "artifact.search", "artifact.get_log_errors",
        ],
        "coordinate_kinds": ["byte", "line", "page", "sheet", "cell", "section"],
        "product_truth": {
            "closed_catalog_complete": True,
            "common_registry_complete": True,
            "fake_backend_complete": True,
            "deterministic_chat_cli_headless_correctness_complete": True,
            "terminal_receipt_per_launched_fake_attempt": True,
            "workspace_mutation_count": 0,
            "network_access_count": 0,
            "parser_launch_count": 0,
            "future_extension_registered_count": 0,
            "production_source_artifact_execution": False,
            "installed_worker_execution": False,
            "cross_platform_sandbox_campaign_complete": False,
            "release_claim": "none",
        },
        "limitations": [
            "the backend is explicitly synthetic and does not implement a source store or parser",
            "installed native worker and cross-platform sandbox execution belong to later stories",
            "the evidence makes no production extraction or release claim",
        ],
    }
    REPORT_PATH.write_text(json.dumps(report, indent=2, sort_keys=True) + "\n", encoding="utf-8")
    return report


if __name__ == "__main__":
    run()
