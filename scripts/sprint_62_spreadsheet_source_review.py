#!/usr/bin/env python3
"""Build the gate-owned Sprint 62 spreadsheet source-adapter review."""

from __future__ import annotations

import argparse
import hashlib
import json
import subprocess
import sys
from pathlib import Path
from typing import Any, Final

ROOT: Final = Path(__file__).resolve().parents[1]
REPORT: Final = ROOT / "artifacts/sprints/sprint-62/spreadsheet-source-review.json"
SOURCES: Final = (
    "kernel/contracts/src/structured_source.rs",
    "capabilities/knowledge/src/spreadsheet_ooxml.rs",
    "capabilities/knowledge/src/tabular.rs",
    "capabilities/knowledge/src/json_data.rs",
    "capabilities/knowledge/src/spreadsheet_source.rs",
    "shells/host/src/spreadsheet_source_artifact.rs",
)


def git_bytes(revision: str, path: str) -> bytes:
    result = subprocess.run(
        ["git", "show", f"{revision}:{path}"],
        cwd=ROOT,
        capture_output=True,
        check=False,
        timeout=30,
    )
    if result.returncode:
        raise ValueError(f"spreadsheet review source unavailable: {path}")
    return result.stdout


def has_all(text: str, tokens: tuple[str, ...]) -> bool:
    return all(token in text for token in tokens)


def expected(revision: str) -> dict[str, Any]:
    sources = {path: git_bytes(revision, path) for path in SOURCES}
    decoded = {path: data.decode("utf-8") for path, data in sources.items()}
    contract = decoded["kernel/contracts/src/structured_source.rs"]
    xlsx = decoded["capabilities/knowledge/src/spreadsheet_ooxml.rs"]
    csv = decoded["capabilities/knowledge/src/tabular.rs"]
    json_source = decoded["capabilities/knowledge/src/json_data.rs"]
    adapter = decoded["capabilities/knowledge/src/spreadsheet_source.rs"]
    service = decoded["shells/host/src/spreadsheet_source_artifact.rs"]
    checks = {
        "shared_trait_covers_xlsx_csv_json": has_all(
            contract,
            (
                "pub const XLSX_MEDIA_TYPE",
                "pub const CSV_MEDIA_TYPE",
                "pub const JSON_MEDIA_TYPE",
                "Xlsx,",
                "Csv,",
                "Json,",
            ),
        )
        and has_all(
            adapter,
            (
                "impl StructuredSourceExtractor for SpreadsheetStructuredSourceExtractor",
                "XLSX_MEDIA_TYPE => project_xlsx",
                "CSV_MEDIA_TYPE => project_csv",
                "JSON_MEDIA_TYPE => project_json",
            ),
        ),
        "canonical_structure_and_exact_provenance_are_emitted": has_all(
            adapter,
            (
                "StructuredSourceSectionKind::Document",
                "StructuredSourceSectionKind::Table",
                "StructuredSourceSectionKind::TableRow",
                "StructuredSourceSectionKind::TableCell",
                '"/workbook/sheets/sheet[{sheet_number}]/row[{}]/cell[{}]"',
                'format!("/csv/table[1]/row[{row_number}]/cell[{column}]")',
                "project_json_value",
            ),
        ),
        "cell_formula_cache_date_error_and_link_semantics_are_visible": has_all(
            adapter,
            (
                '"cached_result": cell.raw_value',
                '"cell_type": cell.kind',
                '"date_value": cell.date_value',
                '"displayed_value": cell.displayed_value',
                '"formula": cell.formula',
                "cell.hyperlink",
                "spreadsheet.formula-preserved-not-calculated",
            ),
        )
        and has_all(
            xlsx,
            (
                "SpreadsheetCellKind::Error",
                "SpreadsheetDateSystem::Excel1900",
                "SpreadsheetDateSystem::Excel1904",
                "followed: false",
            ),
        ),
        "hidden_and_active_content_policy_is_explicit_and_non_executing": has_all(
            adapter,
            (
                "spreadsheet.sheet.hidden-preserved",
                "spreadsheet.sheet.very-hidden-preserved",
                "spreadsheet.macro-content-not-executed",
                "spreadsheet.external-workbook-reference-not-resolved",
                "spreadsheet.dde-formula-not-executed",
                "spreadsheet.external-hyperlink-not-followed",
                "spreadsheet.embedded-object-not-opened",
                "spreadsheet.sheet-protection-preserved",
                "spreadsheet.sparse-range-not-expanded",
                "spreadsheet.feature-preserved-not-interpreted",
                "execution_performed: false",
                "network_access_performed: false",
                "filesystem_effect_performed: false",
            ),
        ),
        "parser_and_projection_resource_ceilings_fail_closed": has_all(
            xlsx,
            (
                "maximum_source_bytes",
                "maximum_entries",
                "maximum_entry_bytes",
                "maximum_total_uncompressed_bytes",
                "maximum_sheets",
                "maximum_cells",
                "maximum_shared_strings",
                "maximum_rows",
                "maximum_columns",
                "maximum_relationships",
                "maximum_string_bytes",
                "maximum_working_memory_bytes",
                "maximum_elapsed_milliseconds",
                "InspectionControl",
                "control.checkpoint()?",
                "control.account(expected_size)?",
            ),
        )
        and has_all(
            csv,
            (
                "maximum_source_bytes",
                "maximum_rows",
                "maximum_columns",
                "maximum_field_bytes",
            ),
        )
        and has_all(
            json_source,
            (
                "maximum_source_bytes",
                "maximum_depth",
                "maximum_nodes",
                "maximum_string_bytes",
            ),
        )
        and has_all(
            adapter,
            (
                "MAX_SECTIONS",
                "MAX_OUTPUT_BYTES",
                "StructuredSourceExtractionError::ResourceLimit",
                "StructuredSourceExtractionError::Cancelled",
                "if cancelled()",
                "inspect_xlsx_with_control",
            ),
        ),
        "focused_contract_tests_cover_formats_fidelity_and_failure": has_all(
            adapter,
            (
                "xlsx_projection_preserves_hidden_formula_cache_date_and_cell_provenance",
                "csv_projection_preserves_exact_rows_cells_and_inert_formulas",
                "json_projection_preserves_pointer_provenance_and_scalar_types",
                "cancellation_and_output_bounds_fail_closed",
                "rejects_digest_mismatch_and_unknown_media_type",
            ),
        )
        and "explicit_coordinate_memory_time_and_cancellation_ceilings_fail_closed"
        in xlsx,
        "golden_projection_binds_exact_values_types_and_provenance": has_all(
            adapter,
            (
                'assert_eq!(formula_value["address"], "B1")',
                'assert_eq!(formula_value["cell_type"], "formula")',
                'assert_eq!(formula_value["formula"], "SUM(A1,1)")',
                'assert_eq!(formula_value["cached_result"], "46001")',
                'assert_eq!(date_value["cell_type"], "date")',
                'assert_eq!(date_value["displayed_value"], "2025-12-09")',
                'assert_eq!(date_value["number_format_id"], 14)',
                'assert_eq!(sheet_value["state"], "very_hidden")',
                "assert_eq!(formula.provenance.table, Some(1))",
                "assert_eq!(formula.provenance.row, Some(1))",
                "assert_eq!(formula.provenance.cell, Some(2))",
                "assert_eq!(date.provenance.cell, Some(1))",
            ),
        ),
        "malicious_and_unsupported_content_remains_bounded_and_inert": has_all(
            xlsx,
            (
                "malformed_missing_oversized_and_macro_packages_fail_or_quarantine",
                "external_formula_and_dde_are_inert_and_block_safe_analysis",
                "SpreadsheetFindingKind::EmbeddedObject",
                "SpreadsheetFindingKind::ProtectedSheet",
                "SpreadsheetFindingKind::SparseRange",
                "SpreadsheetFindingKind::UnsupportedFeature",
                "filesystem_effect_performed: false",
                "network_access_performed: false",
                "execution_performed: false",
            ),
        )
        and has_all(
            adapter,
            (
                "spreadsheet.embedded-object-not-opened",
                "spreadsheet.sheet-protection-preserved",
                "spreadsheet.sparse-range-not-expanded",
                "spreadsheet.feature-preserved-not-interpreted",
                "cancellation_and_output_bounds_fail_closed",
            ),
        ),
        "hostile_and_fidelity_corpus_covers_required_local_families": has_all(
            xlsx,
            (
                "hostile_zip_xml_traversal_bomb_unicode_error_and_protection_corpus_is_closed",
                "SpreadsheetError::MalformedPackage",
                "SpreadsheetError::UnsafePackage",
                "SpreadsheetError::ResourceLimit",
                "SpreadsheetCellKind::Error",
                "excel_serial_conversion_preserves_both_date_systems_and_leap_bug",
                "external_formula_and_dde_are_inert_and_block_safe_analysis",
                "malformed_missing_oversized_and_macro_packages_fail_or_quarantine",
                "explicit_coordinate_memory_time_and_cancellation_ceilings_fail_closed",
            ),
        )
        and has_all(
            service,
            (
                "crash_receipt_and_cleanup_leave_no_retrievable_projection_or_effect_claim",
                "ArtifactExecutionSignal::Crashed",
                "result.receipt.parser_launched",
                "result.receipt.network_accessed",
                "result.receipt.workspace_mutated",
                "lexical_index",
                "is_empty()",
            ),
        ),
        "runtime_service_binds_manifests_sheets_ranges_search_context_and_native_tools": has_all(
            service,
            (
                "pub struct PreparedSpreadsheetSourceManifest",
                "pub struct PreparedSpreadsheetSheetSummary",
                "pub struct SpreadsheetSourceArtifactService",
                "pub struct PreparedSpreadsheetLexicalEntry",
                "pub fn prepared_manifest",
                "fn get_sheet",
                "pub fn lexical_index",
                "pub fn context_candidates",
                "ArtifactFragment::Sheet",
                "ArtifactFragment::Cell",
                "impl ArtifactBackend for SpreadsheetSourceArtifactService",
                "pub fn dispatch_spreadsheet_source_artifact",
                "dispatch_artifact(",
                "freshness_sha256",
                "manifest_sha256",
                "ArtifactExtensionError::Stale",
                "ArtifactExtensionError::LimitExceeded",
                "ArtifactClassification::Restricted",
                "admission_manifest_sheet_summary_and_cache_bind_exact_source",
                "exact_sheet_cell_search_and_context_use_one_fresh_projection",
                "stale_restricted_missing_and_bounded_sheet_requests_fail_closed",
            ),
        ),
        "runtime_lifecycle_retention_restart_and_exact_token_accounting_are_closed": has_all(
            service,
            (
                "pub enum PreparedSpreadsheetLifecycle",
                "pub enum PreparedSpreadsheetRetention",
                "PolicyPersisted",
                "pub fn admit_with_retention",
                "pub fn restore_persisted",
                "pub fn invalidate",
                "pub fn release",
                "pub fn delete",
                "extraction: Option<StructuredSourceExtraction>",
                "pub fn account_combined_context",
                "ExactSourceTokenCounter",
                "token_counter_sha256",
                "tokenizer_sha256",
                "SourcePreparationError::ResourceLimit",
                "persisted_restart_invalidation_release_delete_and_reattachment_are_exact",
                "combined_artifact_context_uses_exact_counter_and_one_projection_per_source",
            ),
        )
        and "source_bytes: Vec" not in service,
    }
    return {
        "schema_version": 1,
        "record_type": "agentmage-sprint-62-spreadsheet-source-review",
        "source_revision": revision,
        "reviewer_identity": "scripts/sprint_62_spreadsheet_source_review.py",
        "review_class": "gate-owned-automated-spreadsheet-source-contract-review",
        "independent_human_review_performed": False,
        "source_sha256": {
            path: hashlib.sha256(data).hexdigest() for path, data in sources.items()
        },
        "checks": checks,
        "status": "PASS_LOCAL_SPREADSHEET_SOURCE_REVIEW"
        if all(checks.values())
        else "FAIL",
        "limitations": [
            "This is gate-owned automated review, not a human-review claim.",
            "It proves authority-free parser projection and deterministic local tests only.",
            "Installed-client parity, live models, and physical-platform campaigns remain absent.",
            "It grants no model, interface, platform, integration, milestone, or release support.",
        ],
    }


def validate(value: Any) -> list[str]:
    if not isinstance(value, dict):
        return ["review is not an object"]
    revision = value.get("source_revision")
    if (
        not isinstance(revision, str)
        or len(revision) != 40
        or any(character not in "0123456789abcdef" for character in revision)
    ):
        return ["source revision invalid"]
    try:
        return [] if value == expected(revision) else ["review is stale, incomplete, or widened"]
    except (UnicodeDecodeError, ValueError) as error:
        return [str(error)]


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
    print("Sprint 62 spreadsheet source review passed")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
