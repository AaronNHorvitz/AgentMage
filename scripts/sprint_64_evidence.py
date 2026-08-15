#!/usr/bin/env python3
"""Build and validate truthful locally executable Sprint 64 evidence."""

from __future__ import annotations

from pathlib import Path
from typing import Final

try:
    from scripts.sprint_evidence_recorder import SprintEvidenceDefinition, main
except ModuleNotFoundError:
    from sprint_evidence_recorder import SprintEvidenceDefinition, main


ROOT: Final = Path(__file__).resolve().parents[1]
SOURCE_PATHS: Final = (
    "Cargo.toml", "Cargo.lock", "capabilities/knowledge/Cargo.toml",
    "capabilities/knowledge/src/presentation_ooxml.rs",
    "capabilities/knowledge/src/presentation_generation.rs",
    "capabilities/knowledge/src/lib.rs",
    "schemas/runtime/presentation-inspection.schema.json",
    "schemas/runtime/generated-presentation.schema.json",
    "schemas/runtime/edited-presentation.schema.json",
    "schemas/runtime/examples/presentation-inspection.valid.json",
    "schemas/runtime/examples/generated-presentation.valid.json",
    "schemas/runtime/examples/edited-presentation.valid.json",
    "scripts/validate_planning_schemas.mjs", "tests/test_planning_schemas.mjs",
    "supply-chain/dependency-hashes.sha256", "supply-chain/dependency-provenance.json",
    "supply-chain/sbom.cdx.json",
    "docs/architecture/presentation-workflows.md",
    "docs/guides/presentation-review.md",
    "docs/verification/sprint-64-presentation-corpus.json",
    "docs/verification/sprint-64-presentation-dependency-manifest.json",
    "docs/verification/sprint-64-local-results.md",
    "scripts/presentation_contract.py", "tests/test_presentation_contract.py",
    "scripts/sprint_evidence_recorder.py", "scripts/sprint_64_evidence.py",
    "tests/test_sprint_64_evidence.py",
)
COMMANDS: Final = (
    ("presentation-inspection-unit", ("cargo", "test", "-p", "agentmage-capability-knowledge", "presentation_ooxml::tests", "--lib", "--locked")),
    ("presentation-generation-unit", ("cargo", "test", "-p", "agentmage-capability-knowledge", "presentation_generation::tests", "--lib", "--locked")),
    ("presentation-runtime-schemas", ("node", "--test", "tests/test_planning_schemas.mjs")),
    ("presentation-review-corpus", ("python3", "-m", "unittest", "tests.test_presentation_contract")),
    ("presentation-strict-clippy", ("cargo", "clippy", "-p", "agentmage-capability-knowledge", "--all-targets", "--locked", "--", "-D", "warnings")),
    ("presentation-format", ("cargo", "fmt", "--all", "--", "--check")),
    ("supply-chain", ("python3", "scripts/supply_chain.py")),
    ("documentation-gate", ("npm", "run", "docs:check")),
    ("product-ci-contract", ("python3", "scripts/product_ci.py", "--check")),
    ("evidence-tests", ("python3", "-m", "unittest", "tests.test_sprint_64_evidence")),
)
FOCUSED_COMMANDS: Final = tuple(item[0] for item in COMMANDS[:4])
SECURITY_REQUIREMENTS: Final = (
    "SR-DAT-002", "SR-DAT-003", "SR-SUP-008", "SR-SUP-009",
    "SR-TST-002", "SR-TST-004", "SR-CIV-006", "SR-CIV-007", "SR-CIV-008", "SR-CIV-009",
)
IMPLEMENTED: Final = {
    "runtime_schema_count": 3, "presentation_rust_fixture_count": 7,
    "review_corpus_case_count": 57, "bounded_direct_pptx_inspection": True,
    "slide_object_and_note_provenance": True, "link_and_image_owner_provenance": True,
    "active_content_quarantine": True, "deterministic_pptx_generation": True,
    "fixed_metadata_and_package_golden": True, "generated_pptx_direct_reopen": True,
    "full_regeneration_slide_editing": True, "exact_structural_slide_previews": True,
    "deterministic_table_chart_plot_diagram_sources": True,
    "network_access_capability": False, "filesystem_mutation_capability": False,
    "content_execution_capability": False, "native_renderer_admitted": False,
    "pixel_golden_admitted": False, "cross_platform_acceptance": False,
    "accessibility_acceptance": False, "independent_review": False, "manual_fuzzing": False,
}
BLOCKERS: Final = (
    {"code": "UPSTREAM-SPRINT-63-BLOCKED", "owner": "64.1"},
    {"code": "NATIVE-PRESENTATION-RENDERER-ABSENT", "owner": "64.1.3.4"},
    {"code": "PIXEL-GOLDEN-AND-RENDER-DIFF-ABSENT", "owner": "64.1.3.4"},
    {"code": "FEDORA-NATIVE-VISUAL-EVIDENCE-ABSENT", "owner": "64.1.3.4"},
    {"code": "UBUNTU-NATIVE-VISUAL-EVIDENCE-ABSENT", "owner": "64.1.3.4"},
    {"code": "WINDOWS-NATIVE-VISUAL-EVIDENCE-ABSENT", "owner": "64.1.3.4"},
    {"code": "MACOS-NATIVE-VISUAL-EVIDENCE-ABSENT", "owner": "64.1.3.4"},
    {"code": "ACCESSIBILITY-ACCEPTANCE-ABSENT", "owner": "64.1.3.4"},
    {"code": "INDEPENDENT-BOUNDARY-REVIEW-ABSENT", "owner": "64.1.3.4"},
    {"code": "MANUAL-FUZZING-DEFERRED", "owner": "SR-TST-004"},
)
VERIFICATION: Final = {
    "focused_local_contracts": True, "focused_blocking_skip_count": 0,
    "review_corpus_case_count": 57, "accepted_network_effect_count": 0,
    "accepted_execution_effect_count": 0, "accepted_filesystem_effect_count": 0,
    "package_golden_verified": True, "direct_reopen_verified": True,
    "relationship_owner_provenance_verified": True, "runtime_semantics_verified": True,
    "structural_preview_verified": True, "native_renderer_admitted": False,
    "native_visual_platform_count": 0, "pixel_diff_complete": False,
    "cross_platform_acceptance": False, "accessibility_acceptance": False,
    "independent_review": False, "manual_fuzzing": False, "sprint_gate_closed": False,
}
SUMMARY: Final = {
    "local_sprint_64_contract_passed": True, "sprint_status": "BLOCKED",
    "upstream_sprint_63_closed": False, "native_render_complete": False,
    "pixel_diff_complete": False, "cross_platform_acceptance_passed": False,
    "accessibility_acceptance_passed": False, "independent_review_present": False,
    "manual_fuzzing_complete": False, "release_approval": False,
}
DEFINITION: Final = SprintEvidenceDefinition(
    sprint=64, root=ROOT, output="artifacts/sprints/sprint-64/local-evidence-report.json",
    source_paths=SOURCE_PATHS, commands=COMMANDS, focused_commands=FOCUSED_COMMANDS,
    rust_focused_commands=frozenset(FOCUSED_COMMANDS[:2]),
    security_requirement_ids=SECURITY_REQUIREMENTS, implemented_contracts=IMPLEMENTED,
    verification_evidence=VERIFICATION, blockers=BLOCKERS, summary=SUMMARY,
)


if __name__ == "__main__":
    raise SystemExit(main(DEFINITION))
