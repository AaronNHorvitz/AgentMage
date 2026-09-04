#!/usr/bin/env python3
"""Build the truthful Sprint 62 foundational-runtime milestone evaluation."""

from __future__ import annotations

import argparse
import hashlib
import json
import subprocess
import sys
from pathlib import Path
from typing import Any, Final

ROOT: Final = Path(__file__).resolve().parents[1]
REPORT: Final = ROOT / "artifacts/sprints/sprint-62/foundational-runtime-evaluation.json"
SOURCES: Final = (
    "architecture/status-model.json",
    "configuration/profiles/catalog.json",
    "configuration/profiles/development.json",
    "configuration/profiles/synthetic-test.json",
    "configuration/profiles/strict-local-read-only.json",
    "configuration/profiles/knowledge.json",
    "configuration/profiles/write.json",
    "configuration/profiles/coding.json",
    "configuration/profiles/later-network.json",
    "artifacts/sprints/sprint-22/story-22.5/vertical-slice-report.json",
    "artifacts/sprints/sprint-58/local-evidence-report.json",
    "artifacts/sprints/sprint-60/local-evidence-report.json",
    "artifacts/sprints/sprint-62/local-evidence-report.json",
    "artifacts/sprints/sprint-62/spreadsheet-source-review.json",
    "scripts/sprint_62_foundational_runtime_evaluation.py",
)

MODEL_BLOCKER: Final = (
    "BLOCKED_EXTERNAL(platform=approved local inference host; artifact=two exact eligible model "
    "profile admission bundles and successful Chat, CLI, and headless text-log, DOCX, PDF, and "
    "XLSX workflow records; action=authorized model owner admits two exact profiles, supplies any "
    "required model artifacts or API credentials, runs the untouched matrix, and transfers the "
    "results; credential=model artifact source and/or API credentials; payment=model artifact, API, "
    "or hardware if applicable); substitution_set=empty"
)
OCR_BLOCKER: Final = (
    "BLOCKED_EXTERNAL(platform=native OCR-enabled Fedora and Ubuntu hosts; artifact=scanned-PDF/OCR "
    "Chat, CLI, and headless workflow records with exact engine and language-pack identities; "
    "action=platform owner provisions the admitted OCR runtime, runs the untouched workflow matrix, "
    "and transfers the results; credential=platform access; payment=OCR runtime or hardware if "
    "applicable); substitution_set=empty"
)
PLATFORM_BLOCKER: Final = (
    "BLOCKED_EXTERNAL(platform=Windows 11 x64 KVM guest and physical supported MacBook; "
    "artifact=foundational-runtime parser and client workflow campaign with exact environment, "
    "resource, receipt, recovery, performance, and security results; action=platform owners run the "
    "untouched campaign and transfer the results; credential=Windows image source and physical Mac "
    "access; payment=Windows license or hardware if required); substitution_set=empty"
)
INSTALL_BLOCKER: Final = (
    "blocked: host change required — run the foundational-runtime Chat, CLI, and headless workflow "
    "matrix from trusted installed AgentMage launchers outside the development shell; "
    "substitution_set=empty"
)
BLOCKERS: Final = (MODEL_BLOCKER, OCR_BLOCKER, PLATFORM_BLOCKER, INSTALL_BLOCKER)


def git_bytes(revision: str, path: str) -> bytes:
    result = subprocess.run(
        ["git", "show", f"{revision}:{path}"],
        cwd=ROOT,
        capture_output=True,
        check=False,
        timeout=30,
    )
    if result.returncode:
        raise ValueError(f"foundational-runtime source unavailable: {path}")
    return result.stdout


def load(revision: str, path: str) -> Any:
    return json.loads(git_bytes(revision, path))


def expected(revision: str) -> dict[str, Any]:
    values = {path: git_bytes(revision, path) for path in SOURCES}
    status = json.loads(values["architecture/status-model.json"])
    catalog = json.loads(values["configuration/profiles/catalog.json"])
    profiles = [
        json.loads(values[entry["configuration_path"]]) for entry in catalog["profiles"]
    ]
    spreadsheet = json.loads(
        values["artifacts/sprints/sprint-62/spreadsheet-source-review.json"]
    )
    sprint_58 = json.loads(values["artifacts/sprints/sprint-58/local-evidence-report.json"])
    sprint_60 = json.loads(values["artifacts/sprints/sprint-60/local-evidence-report.json"])
    sprint_62 = json.loads(values["artifacts/sprints/sprint-62/local-evidence-report.json"])
    integrated = status["current_product"]["integrated_workflow"]
    enabled = [profile["model"]["model_profile_id"] for profile in profiles if profile["model"]["enabled"]]
    local_checks = {
        "spreadsheet_parser_and_three_client_adapter_contracts_pass": spreadsheet["status"]
        == "PASS_LOCAL_SPREADSHEET_SOURCE_REVIEW",
        "deterministic_fake_model_repository_workflow_is_bound": integrated["id"]
        == "story-22.5-deterministic-repository-analysis",
        "word_local_contract_passes_but_product_workflow_is_absent": sprint_58["summary"]
        ["local_sprint_58_contract_passed"]
        and not sprint_58["summary"]["word_workflow_integrated"],
        "pdf_local_contract_passes_but_product_and_ocr_workflows_are_absent": sprint_60
        ["summary"]["local_sprint_60_contract_passed"]
        and not sprint_60["summary"]["pdf_extraction_product_integrated"]
        and not sprint_60["summary"]["native_ocr_complete"],
        "spreadsheet_legacy_local_contract_passes_with_platform_blocks": sprint_62["summary"]
        ["local_sprint_62_contract_passed"]
        and not sprint_62["summary"]["cross_platform_acceptance_passed"],
        "no_model_profile_is_enabled_or_product_registered": not enabled
        and not any(entry["product_registration"] for entry in catalog["profiles"]),
        "no_platform_or_release_support_is_claimed": not status["current_product"]
        ["supported_platforms"]
        and not status["current_product"]["released_packages"],
    }
    campaign_records = {
        name: {
            "state": "absent_blocking",
            "reason": "no qualifying two-profile installed foundational-runtime campaign exists",
        }
        for name in (
            "source_and_context_manifests",
            "native_tool_results",
            "preflights",
            "attempts",
            "receipts",
            "verifications",
            "recovery_decisions",
            "diagnoses",
            "performance_thresholds",
            "security_results",
        )
    }
    return {
        "schema_version": 1,
        "record_type": "agentmage-sprint-62-foundational-runtime-evaluation",
        "source_revision": revision,
        "source_sha256": {
            path: hashlib.sha256(value).hexdigest() for path, value in values.items()
        },
        "local_checks": local_checks,
        "enabled_model_profile_ids": enabled,
        "qualifying_installed_campaign_count": 0,
        "campaign_record_reconciliation": campaign_records,
        "hidden_non_pass_state_count": 0,
        "rows": {
            "62.2.4.1": "BLOCKED_EXTERNAL",
            "62.2.4.2": "PASS_RECONCILED_WITH_VISIBLE_BLOCKERS",
            "62.2.4.3": "PASS_RECORDED_BLOCKED_MILESTONE",
            "M-FOUNDATIONAL-RUNTIME": "BLOCKED",
        },
        "blockers": list(BLOCKERS),
        "limitations": [
            "Local parser and source-level three-client contracts are not installed-product workflows.",
            "The one integrated fake-model repository workflow is not a parser workflow or a second eligible model profile.",
            "Legacy XLS, encrypted-workbook, native Office reopen, accessibility, independent-review, and deferred-manual-fuzz evidence remain absent.",
            "No OCR, live-model, Windows, macOS, installed-launcher, milestone, release, or support pass is inferred.",
        ],
        "status": "BLOCKED_M_FOUNDATIONAL_RUNTIME",
    }


def validate(value: Any) -> list[str]:
    if not isinstance(value, dict):
        return ["evaluation is not an object"]
    revision = value.get("source_revision")
    if (
        not isinstance(revision, str)
        or len(revision) != 40
        or any(character not in "0123456789abcdef" for character in revision)
    ):
        return ["source revision invalid"]
    try:
        expected_value = expected(revision)
    except (KeyError, TypeError, ValueError, json.JSONDecodeError) as error:
        return [str(error)]
    return [] if value == expected_value else ["evaluation is stale, incomplete, or widened"]


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--write", action="store_true")
    parser.add_argument("--source-revision", default="HEAD")
    arguments = parser.parse_args()
    revision = subprocess.check_output(
        ["git", "rev-parse", arguments.source_revision], cwd=ROOT, text=True
    ).strip()
    if arguments.write:
        REPORT.parent.mkdir(parents=True, exist_ok=True)
        REPORT.write_text(
            json.dumps(expected(revision), indent=2, sort_keys=True) + "\n",
            encoding="utf-8",
        )
    try:
        value = json.loads(REPORT.read_text(encoding="utf-8"))
    except (OSError, json.JSONDecodeError) as error:
        print(error, file=sys.stderr)
        return 1
    failures = validate(value)
    if failures:
        print("\n".join(failures), file=sys.stderr)
        return 1
    print("Sprint 62 foundational-runtime milestone remains truthfully BLOCKED")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
