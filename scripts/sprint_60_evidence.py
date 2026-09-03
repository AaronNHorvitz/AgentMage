#!/usr/bin/env python3
"""Build and validate truthful locally executable Sprint 60 evidence."""

from __future__ import annotations

import argparse
import hashlib
import json
import platform
import re
import shutil
import subprocess
from pathlib import Path
from typing import Any, Final


ROOT: Final = Path(__file__).resolve().parents[1]
OUTPUT: Final = ROOT / "artifacts/sprints/sprint-60/local-evidence-report.json"
REVISION: Final = re.compile(r"^[0-9a-f]{40}$")
SHA256: Final = re.compile(r"^[0-9a-f]{64}$")
IGNORED_TESTS: Final = re.compile(
    rb"test result: (?:ok|FAILED)\. \d+ passed; \d+ failed; (\d+) ignored;"
)
SOURCE_PATHS: Final = (
    "Cargo.toml",
    "Cargo.lock",
    "capabilities/knowledge/Cargo.toml",
    "capabilities/knowledge/src/pdf_extraction.rs",
    "capabilities/knowledge/src/lib.rs",
    "kernel/contracts/src/structured_source.rs",
    "schemas/runtime/pdf-extraction-result.schema.json",
    "schemas/runtime/examples/pdf-extraction-result.valid.json",
    "schemas/runtime/pdf-ocr-projection.schema.json",
    "schemas/runtime/examples/pdf-ocr-projection.valid.json",
    "scripts/validate_planning_schemas.mjs",
    "tests/test_planning_schemas.mjs",
    "supply-chain/cargo-external-catalog.json",
    "supply-chain/dependency-hashes.sha256",
    "supply-chain/dependency-provenance.json",
    "supply-chain/sbom.cdx.json",
    "docs/verification/sprint-60-pdf-extraction-corpus.json",
    "docs/verification/sprint-60-pdf-dependency-manifest.json",
    "scripts/pdf_extraction_contract.py",
    "tests/test_pdf_extraction_contract.py",
    "docs/architecture/pdf-extraction-and-page-citations.md",
    "docs/guides/pdf-extraction-review.md",
    "docs/verification/sprint-60-local-results.md",
    "scripts/sprint_60_evidence.py",
    "tests/test_sprint_60_evidence.py",
)
COMMANDS: Final = (
    (
        "pdf-extraction-unit",
        (
            "cargo",
            "test",
            "-p",
            "agentmage-capability-knowledge",
            "pdf_extraction::tests",
            "--lib",
            "--locked",
        ),
    ),
    ("pdf-runtime-schemas", ("node", "--test", "tests/test_planning_schemas.mjs")),
    (
        "pdf-review-corpus",
        ("python3", "-m", "unittest", "tests.test_pdf_extraction_contract"),
    ),
    (
        "pdf-strict-clippy",
        (
            "cargo",
            "clippy",
            "-p",
            "agentmage-capability-knowledge",
            "--all-targets",
            "--locked",
            "--",
            "-D",
            "warnings",
        ),
    ),
    ("pdf-format", ("cargo", "fmt", "--all", "--", "--check")),
    ("supply-chain", ("python3", "scripts/supply_chain.py")),
    ("documentation-gate", ("npm", "run", "docs:check")),
    ("product-ci-contract", ("python3", "scripts/product_ci.py", "--check")),
    (
        "evidence-tests",
        ("python3", "-m", "unittest", "tests.test_sprint_60_evidence"),
    ),
)
FOCUSED_COMMANDS: Final = tuple(item[0] for item in COMMANDS[:3])
RUST_FOCUSED_COMMANDS: Final = {"pdf-extraction-unit"}
SECURITY_REQUIREMENTS: Final = [
    "SR-DAT-002",
    "SR-DAT-003",
    "SR-SUP-008",
    "SR-SUP-009",
    "SR-TST-002",
    "SR-TST-004",
    "SR-CIV-006",
    "SR-CIV-007",
    "SR-CIV-008",
    "SR-CIV-009",
]
IMPLEMENTED: Final = {
    "admitted_pdf_parser_count": 1,
    "pdf_runtime_schema_count": 2,
    "executable_rust_fixture_count": 9,
    "review_corpus_case_count": 82,
    "bounded_strict_in_memory_extraction": True,
    "deterministic_page_object_identity": True,
    "exact_page_citations": True,
    "scanned_empty_encrypted_malformed_limit_detection": True,
    "ocr_observation_admission_boundary": True,
    "ocr_uncertainty_preserved": True,
    "shared_structured_source_adapter": True,
    "structured_span_provenance": True,
    "reading_order_uncertainty_visible": True,
    "text_density_observation": True,
    "active_or_external_content_quarantined": True,
    "unsupported_structure_preservation": True,
    "network_access_capability": False,
    "execution_capability": False,
    "filesystem_mutation_capability": False,
    "ocr_package_admitted": False,
    "renderer_admitted": False,
    "generator_admitted": False,
    "redaction_verification": False,
    "product_integration": False,
    "cross_platform_acceptance": False,
    "accessibility_acceptance": False,
    "independent_native_boundary_review": False,
    "manual_fuzzing": False,
}
BLOCKERS: Final = [
    {"code": "UPSTREAM-SPRINT-59-BLOCKED", "owner": "60.1"},
    {"code": "PDF-OCR-PACKAGE-MODEL-NOT-ADMITTED", "owner": "60.1.1.4"},
    {"code": "PDF-OCR-FAILURE-CANCELLATION-CAMPAIGN-ABSENT", "owner": "60.1.3.1"},
    {"code": "PDF-GENERATOR-NOT-ADMITTED", "owner": "60.1.1.1"},
    {"code": "PDF-RENDERER-NOT-ADMITTED", "owner": "60.1.1.1"},
    {"code": "PDF-REDACTION-RESIDUE-EVIDENCE-ABSENT", "owner": "60.1.3.4"},
    {"code": "FEDORA-NATIVE-PDF-EVIDENCE-ABSENT", "owner": "60.1.3.4"},
    {"code": "UBUNTU-NATIVE-PDF-EVIDENCE-ABSENT", "owner": "60.1.3.4"},
    {"code": "WINDOWS-NATIVE-PDF-EVIDENCE-ABSENT", "owner": "60.1.3.4"},
    {"code": "MACOS-RETAINED-PDF-EVIDENCE-ABSENT", "owner": "60.1.3.4"},
    {"code": "ACCESSIBILITY-ACCEPTANCE-ABSENT", "owner": "60.1.3.4"},
    {"code": "INDEPENDENT-NATIVE-BOUNDARY-REVIEW-ABSENT", "owner": "60.1.3.4"},
    {"code": "MANUAL-FUZZING-DEFERRED", "owner": "SR-TST-004"},
]


def digest(value: bytes) -> str:
    return hashlib.sha256(value).hexdigest()


def git_file(revision: str, path: str) -> bytes:
    result = subprocess.run(
        ["git", "show", f"{revision}:{path}"],
        cwd=ROOT,
        check=False,
        capture_output=True,
        timeout=30,
    )
    if result.returncode != 0:
        raise ValueError(f"committed Sprint 60 source is absent: {path}")
    return result.stdout


def version(executable: str, *arguments: str) -> str:
    resolved = shutil.which(executable)
    if resolved is None:
        return "unavailable"
    result = subprocess.run(
        (resolved, *arguments),
        cwd=ROOT,
        check=False,
        capture_output=True,
        text=True,
        timeout=30,
    )
    output = (result.stdout + result.stderr).strip().splitlines()
    return output[0][:256] if result.returncode == 0 and output else "unavailable"


def environment_manifest() -> dict[str, str]:
    return {
        "system": platform.system(),
        "release": platform.release(),
        "machine": platform.machine(),
        "python": platform.python_version(),
        "rustc": version("rustc", "--version"),
        "cargo": version("cargo", "--version"),
        "node": version("node", "--version"),
        "npm": version("npm", "--version"),
    }


def run_commands() -> list[dict[str, Any]]:
    records = []
    for identifier, argv in COMMANDS:
        executable = shutil.which(argv[0])
        if executable is None:
            code, output = 127, b"executable-unavailable"
        else:
            result = subprocess.run(
                (executable, *argv[1:]),
                cwd=ROOT,
                check=False,
                capture_output=True,
                timeout=1800,
            )
            code, output = result.returncode, result.stdout + result.stderr
        blocking_skip_count = None
        if identifier in RUST_FOCUSED_COMMANDS:
            matches = IGNORED_TESTS.findall(output)
            blocking_skip_count = sum(int(value) for value in matches) if matches else -1
        elif identifier in FOCUSED_COMMANDS:
            blocking_skip_count = 0 if code == 0 else None
        records.append(
            {
                "id": identifier,
                "argv": list(argv),
                "exit_code": code,
                "output_sha256": digest(output),
                "blocking_skip_count": blocking_skip_count,
            }
        )
    return records


def expected_verification(local_pass: bool = True) -> dict[str, Any]:
    return {
        "focused_local_contracts": local_pass,
        "focused_blocking_skip_count": 0 if local_pass else None,
        "review_corpus_case_count": 82 if local_pass else None,
        "accepted_network_effect_count": 0 if local_pass else None,
        "accepted_execution_effect_count": 0 if local_pass else None,
        "accepted_filesystem_effect_count": 0 if local_pass else None,
        "ocr_package_admitted": False,
        "native_ocr_platform_count": 0,
        "renderer_admitted": False,
        "native_render_platform_count": 0,
        "generator_admitted": False,
        "redaction_verification": False,
        "product_integration": False,
        "cross_platform_acceptance": False,
        "accessibility_acceptance": False,
        "independent_native_boundary_review": False,
        "manual_fuzzing": False,
        "sprint_gate_closed": False,
    }


def expected_summary(local_pass: bool = True) -> dict[str, Any]:
    return {
        "local_sprint_60_contract_passed": local_pass,
        "sprint_status": "BLOCKED",
        "upstream_sprint_59_closed": False,
        "pdf_extraction_product_integrated": False,
        "external_effects_enabled": False,
        "cross_platform_acceptance_passed": False,
        "native_ocr_complete": False,
        "native_render_complete": False,
        "redaction_verification_complete": False,
        "independent_review_present": False,
        "manual_fuzzing_complete": False,
        "release_approval": False,
    }


def build_report(revision: str, commands: list[dict[str, Any]]) -> dict[str, Any]:
    focused = [item for item in commands if item["id"] in FOCUSED_COMMANDS]
    local_pass = (
        all(item["exit_code"] == 0 for item in commands)
        and len(focused) == len(FOCUSED_COMMANDS)
        and all(item.get("blocking_skip_count") == 0 for item in focused)
    )
    return {
        "schema_version": 1,
        "record_type": "sprint_60_local_evidence",
        "source_revision": revision,
        "source_sha256": {path: digest(git_file(revision, path)) for path in SOURCE_PATHS},
        "environment": environment_manifest(),
        "commands": commands,
        "security_requirement_ids": list(SECURITY_REQUIREMENTS),
        "implemented_contracts": dict(IMPLEMENTED),
        "verification_evidence": expected_verification(local_pass),
        "blockers": [dict(blocker) for blocker in BLOCKERS],
        "summary": expected_summary(local_pass),
    }


def revision_is_ancestor(revision: str) -> bool:
    result = subprocess.run(
        ["git", "merge-base", "--is-ancestor", revision, "HEAD"],
        cwd=ROOT,
        check=False,
        capture_output=True,
        timeout=30,
    )
    return result.returncode == 0


def validate_report(report: dict[str, Any], verify_ancestry: bool = True) -> list[str]:
    failures: list[str] = []
    revision = str(report.get("source_revision", ""))
    if not REVISION.fullmatch(revision):
        failures.append("source revision invalid")
    if report.get("schema_version") != 1 or report.get("record_type") != "sprint_60_local_evidence":
        failures.append("report identity invalid")
    for actual, expected, name in (
        (report.get("security_requirement_ids"), SECURITY_REQUIREMENTS, "security mapping drift"),
        (report.get("implemented_contracts"), IMPLEMENTED, "implemented contract drift"),
        (report.get("blockers"), BLOCKERS, "blocker drift"),
    ):
        if actual != expected:
            failures.append(name)
    commands = report.get("commands", [])
    if [item.get("id") for item in commands] != [item[0] for item in COMMANDS]:
        failures.append("command inventory drift")
    if len(commands) != len(COMMANDS) or any(
        item.get("argv") != list(expected[1])
        or item.get("exit_code") != 0
        or not SHA256.fullmatch(str(item.get("output_sha256", "")))
        for item, expected in zip(commands, COMMANDS, strict=False)
    ):
        failures.append("command result invalid")
    for identifier in FOCUSED_COMMANDS:
        focused = next((item for item in commands if item.get("id") == identifier), None)
        if focused is None or focused.get("blocking_skip_count") != 0:
            failures.append(f"focused skipped, suppressed, or unavailable check: {identifier}")
    if report.get("verification_evidence") != expected_verification():
        failures.append("verification claim drift")
    if report.get("summary") != expected_summary():
        failures.append("summary or release claim drift")
    source = report.get("source_sha256", {})
    if list(source) != list(SOURCE_PATHS):
        failures.append("source inventory drift")
    elif REVISION.fullmatch(revision):
        for path in SOURCE_PATHS:
            try:
                expected = digest(git_file(revision, path))
            except ValueError as error:
                failures.append(str(error))
                continue
            if source.get(path) != expected:
                failures.append(f"source digest drift: {path}")
    if verify_ancestry and REVISION.fullmatch(revision) and not revision_is_ancestor(revision):
        failures.append("report source revision is not an ancestor of current HEAD")
    encoded = json.dumps(report, sort_keys=True).lower()
    for prohibited in (
        "credential_value",
        "secret_value",
        "private_key",
        "raw_output",
        "raw_result",
        "prompt_text",
        "raw_document",
        "remote_url",
        "repository_path",
        "access_token",
    ):
        if prohibited in encoded:
            failures.append(f"prohibited evidence field: {prohibited}")
    return failures


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--write", action="store_true")
    parser.add_argument("--source-revision")
    args = parser.parse_args()
    if args.write:
        revision = args.source_revision or subprocess.run(
            ["git", "rev-parse", "HEAD"],
            cwd=ROOT,
            check=True,
            capture_output=True,
            text=True,
            timeout=30,
        ).stdout.strip()
        report = build_report(revision, run_commands())
        failures = validate_report(report, verify_ancestry=False)
        if failures:
            for failure in failures:
                print(f"error: {failure}")
            return 1
        OUTPUT.parent.mkdir(parents=True, exist_ok=True)
        OUTPUT.write_text(json.dumps(report, indent=2) + "\n", encoding="utf-8")
        print(f"wrote {OUTPUT.relative_to(ROOT)}")
        return 0
    if not OUTPUT.is_file():
        print(f"error: missing {OUTPUT.relative_to(ROOT)}")
        return 1
    failures = validate_report(json.loads(OUTPUT.read_text(encoding="utf-8")))
    if failures:
        for failure in failures:
            print(f"error: {failure}")
        return 1
    print("Sprint 60 local evidence validated")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
