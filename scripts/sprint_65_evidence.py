#!/usr/bin/env python3
"""Build and validate truthful locally executable Sprint 65 evidence."""

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
    "capabilities/knowledge/src/image_workflows.rs", "capabilities/knowledge/src/lib.rs",
    "schemas/runtime/image-inspection.schema.json",
    "schemas/runtime/image-redaction-receipt.schema.json",
    "schemas/runtime/image-visual-comparison.schema.json",
    "schemas/runtime/examples/image-inspection.valid.json",
    "schemas/runtime/examples/image-redaction-receipt.valid.json",
    "schemas/runtime/examples/image-visual-comparison.valid.json",
    "scripts/validate_planning_schemas.mjs", "tests/test_planning_schemas.mjs",
    "supply-chain/dependency-hashes.sha256", "supply-chain/dependency-provenance.json",
    "supply-chain/sbom.cdx.json", "docs/architecture/presentation-workflows.md",
    "docs/architecture/image-redaction-and-visual-verification.md",
    "docs/guides/image-redaction-and-visual-review.md",
    "docs/verification/sprint-65-image-corpus.json",
    "docs/verification/sprint-65-image-dependency-manifest.json",
    "docs/verification/sprint-65-local-results.md",
    "scripts/image_workflow_contract.py", "tests/test_image_workflow_contract.py",
    "scripts/sprint_evidence_recorder.py", "scripts/sprint_65_evidence.py",
    "tests/test_sprint_65_evidence.py",
)
COMMANDS: Final = (
    ("image-workflow-unit", ("cargo", "test", "-p", "agentmage-capability-knowledge", "image_workflows::tests", "--lib", "--locked")),
    ("image-runtime-schemas", ("node", "--test", "tests/test_planning_schemas.mjs")),
    ("image-review-corpus", ("python3", "-m", "unittest", "tests.test_image_workflow_contract")),
    ("image-strict-clippy", ("cargo", "clippy", "-p", "agentmage-capability-knowledge", "--all-targets", "--locked", "--", "-D", "warnings")),
    ("image-format", ("cargo", "fmt", "--all", "--", "--check")),
    ("supply-chain-currentness", ("python3", "scripts/supply_chain.py")),
    ("product-ci-contract", ("python3", "scripts/product_ci.py", "--check")),
    ("evidence-tests", ("python3", "-m", "unittest", "tests.test_sprint_65_evidence")),
)
FOCUSED_COMMANDS: Final = tuple(item[0] for item in COMMANDS[:3])
SECURITY_REQUIREMENTS: Final = (
    "SR-DAT-002", "SR-DAT-003", "SR-SUP-008", "SR-SUP-009",
    "SR-TST-002", "SR-TST-004", "SR-CIV-006", "SR-CIV-007", "SR-CIV-008", "SR-CIV-009",
)
IMPLEMENTED: Final = {
    "runtime_schema_count": 3, "image_rust_fixture_count": 6,
    "review_corpus_case_count": 99, "bounded_bmp_rgba_decoder": True,
    "png_metadata_inspection": True, "slide_object_image_provenance": True,
    "approved_vision_profile_view_proposal": True, "decoded_pixel_redaction": True,
    "deterministic_bmp_full_regeneration": True, "redaction_pixel_residue_scan": True,
    "ancillary_metadata_removal": True, "four_artifact_visual_diff": True,
    "strict_local_provider_denial": True, "exact_disclosure_approval_receipt": True,
    "network_access_capability": False, "filesystem_mutation_capability": False,
    "content_execution_capability": False, "native_screenshot_capture_admitted": False,
    "native_renderer_admitted": False, "provider_execution_admitted": False,
    "cross_platform_acceptance": False, "accessibility_acceptance": False,
    "independent_review": False, "manual_fuzzing": False,
}
BLOCKERS: Final = (
    {"code": "UPSTREAM-SPRINT-64-BLOCKED", "owner": "65.1"},
    {"code": "NATIVE-SCREENSHOT-VIEWER-ABSENT", "owner": "65.1.1.2"},
    {"code": "PNG-JPEG-PIXEL-DECODER-ABSENT", "owner": "65.1.3.2"},
    {"code": "PROVIDER-IMAGE-MODEL-EXECUTION-ABSENT", "owner": "65.1.1.5"},
    {"code": "NATIVE-DOCUMENT-SLIDE-UI-RENDERER-ABSENT", "owner": "65.1.3.4"},
    {"code": "FEDORA-NATIVE-VISUAL-EVIDENCE-ABSENT", "owner": "65.1.3.4"},
    {"code": "UBUNTU-NATIVE-VISUAL-EVIDENCE-ABSENT", "owner": "65.1.3.4"},
    {"code": "WINDOWS-NATIVE-VISUAL-EVIDENCE-ABSENT", "owner": "65.1.3.4"},
    {"code": "MACOS-NATIVE-VISUAL-EVIDENCE-ABSENT", "owner": "65.1.3.4"},
    {"code": "STEGANOGRAPHY-CAMPAIGN-ABSENT", "owner": "65.1.3.3"},
    {"code": "ACCESSIBILITY-ACCEPTANCE-ABSENT", "owner": "65.1.3.5"},
    {"code": "INDEPENDENT-BOUNDARY-REVIEW-ABSENT", "owner": "65.1.3.5"},
    {"code": "MANUAL-FUZZING-DEFERRED", "owner": "SR-TST-004"},
)
VERIFICATION: Final = {
    "focused_local_contracts": True, "focused_blocking_skip_count": 0,
    "review_corpus_case_count": 99, "accepted_network_effect_count": 0,
    "accepted_execution_effect_count": 0, "accepted_filesystem_effect_count": 0,
    "bmp_round_trip_verified": True, "image_provenance_verified": True,
    "redaction_pixel_and_metadata_scan_verified": True, "visual_diff_semantics_verified": True,
    "strict_local_provider_denial_verified": True, "approval_receipt_binding_verified": True,
    "native_screenshot_capture_admitted": False, "native_renderer_admitted": False,
    "native_visual_platform_count": 0, "provider_execution_admitted": False,
    "cross_platform_acceptance": False, "accessibility_acceptance": False,
    "independent_review": False, "manual_fuzzing": False, "sprint_gate_closed": False,
}
SUMMARY: Final = {
    "local_sprint_65_contract_passed": True, "sprint_status": "BLOCKED",
    "upstream_sprint_64_closed": False, "native_screenshot_view_complete": False,
    "native_render_complete": False, "provider_generation_complete": False,
    "cross_platform_acceptance_passed": False, "accessibility_acceptance_passed": False,
    "independent_review_present": False, "manual_fuzzing_complete": False,
    "release_approval": False,
}
DEFINITION: Final = SprintEvidenceDefinition(
    sprint=65, root=ROOT, output="artifacts/sprints/sprint-65/local-evidence-report.json",
    source_paths=SOURCE_PATHS, commands=COMMANDS, focused_commands=FOCUSED_COMMANDS,
    rust_focused_commands=frozenset(FOCUSED_COMMANDS[:1]),
    security_requirement_ids=SECURITY_REQUIREMENTS, implemented_contracts=IMPLEMENTED,
    verification_evidence=VERIFICATION, blockers=BLOCKERS, summary=SUMMARY,
)


if __name__ == "__main__":
    raise SystemExit(main(DEFINITION))
