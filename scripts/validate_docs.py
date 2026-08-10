#!/usr/bin/env python3
"""Validate AgentMage documentation links, identities, and policy invariants."""

from __future__ import annotations

import re
import sys
from pathlib import Path
from urllib.parse import unquote


ROOT = Path(__file__).resolve().parents[1]
REQUIRED_FILES = (
    "README.md",
    "PRD.md",
    "IMPLEMENTATION-PLAN.md",
    "Agent-Scaffolding-Inventory.md",
    "TASKS.md",
    "SECURITY-REVIEW.md",
    "SECURITY.md",
    "MODEL-PROVENANCE-POLICY.md",
    "RUNTIME-BOUNDARIES.md",
    "architecture/language-build-matrix.json",
    "architecture/module-inventory.json",
    "architecture/dependency-rules.json",
    "architecture/build-contract.json",
    "architecture/clean-build-policy.json",
    "architecture/component-inventory-policy.json",
    "architecture/rollback-design.json",
    "architecture/signed-update-design.json",
    "architecture/dependency-classes.json",
    "architecture/optional-component-inventory.json",
    "architecture/artifact-scan-policy.json",
    "supply-chain/dependency-provenance.json",
    "supply-chain/cargo-external-catalog.json",
    "supply-chain/dependency-hashes.sha256",
    "supply-chain/sbom.cdx.json",
    "release/clean-build/Containerfile.linux",
    "artifacts/sprints/sprint-1/story-1.1/clean-build-report.json",
    "artifacts/sprints/sprint-1/story-1.1/security-evidence-map.json",
    "artifacts/sprints/sprint-2/story-2.1/fixture-generator-report.json",
    "artifacts/sprints/sprint-2/story-2.1/adversarial-fixture-report.json",
    "artifacts/sprints/sprint-2/story-2.1/adapter-mode-verification-report.json",
    "artifacts/sprints/sprint-2/story-2.1/corpus-reproducibility-report.json",
    "artifacts/sprints/sprint-2/story-2.1/document-fixture-report.json",
    "artifacts/sprints/sprint-2/story-2.1/expected-output-report.json",
    "artifacts/sprints/sprint-2/story-2.1/fake-adapter-report.json",
    "artifacts/sprints/sprint-2/story-2.1/fault-test-adapter-report.json",
    "artifacts/sprints/sprint-2/story-2.1/fixture-provenance-report.json",
    "artifacts/sprints/sprint-2/story-2.1/fixture-security-scan-report.json",
    "artifacts/sprints/sprint-2/story-2.1/input-class-fixture-matrix-report.json",
    "artifacts/sprints/sprint-2/story-2.1/path-fixture-report.json",
    "artifacts/sprints/sprint-2/story-2.1/platform-result-recorder-report.json",
    "artifacts/sprints/sprint-2/story-2.1/shared-acceptance-runner-report.json",
    "artifacts/sprints/sprint-2/story-2.1/summary-reconciliation-report.json",
    "artifacts/sprints/sprint-2/story-2.1/story-gate-report.json",
    "artifacts/sprints/sprint-2/story-2.1/summary-comparison.json",
    "artifacts/sprints/sprint-2/story-2.1/summary-comparison-public.pem",
    "artifacts/sprints/sprint-2/story-2.1/summary-comparison.sig",
    "artifacts/sprints/sprint-2/story-2.1/summary-comparison-signature.json",
    "artifacts/sprints/sprint-2/story-2.1/security-evidence-map.json",
    "artifacts/sprints/sprint-2/story-2.1/test-result-bundle-report.json",
    "artifacts/sprints/sprint-2/story-2.1/versioned-corpus-report.json",
    "artifacts/sprints/sprint-2/story-2.2/target-registry-report.json",
    "artifacts/sprints/sprint-2/story-2.2/toolchain-policy-report.json",
    "artifacts/sprints/sprint-2/story-2.2/fuzz-result-schema-report.json",
    "artifacts/sprints/sprint-2/story-2.2/gate-policy-report.json",
    "artifacts/sprints/sprint-2/story-2.2/seeded-failure-report.json",
    "artifacts/sprints/sprint-2/story-2.2/baseline-reconciliation-report.json",
    "artifacts/sprints/sprint-2/story-2.2/rv-15-control-map.json",
    "artifacts/sprints/sprint-2/story-2.2/reviewer-disposition.json",
    "artifacts/sprints/sprint-2/story-2.2/security-evidence-map.json",
    "artifacts/sprints/sprint-2/story-2.2/story-gate-report.json",
    "artifacts/sprints/sprint-2/sprint-gate-report.json",
    "artifacts/sprints/sprint-3/story-3.1/configuration-schema-report.json",
    "artifacts/sprints/sprint-3/story-3.1/profile-catalog-report.json",
    "artifacts/sprints/sprint-3/story-3.1/configuration-loader-report.json",
    "artifacts/sprints/sprint-3/story-3.1/configuration-authority-report.json",
    "artifacts/sprints/sprint-3/story-3.1/configuration-result-report.json",
    "artifacts/sprints/sprint-3/story-3.1/component-inventory-report.json",
    "artifacts/sprints/sprint-3/story-3.1/update-rollback-design-report.json",
    "artifacts/sprints/sprint-3/story-3.1/configuration-review-artifacts-report.json",
    "fuzzing/target-registry.json",
    "fuzzing/toolchain-policy.json",
    "fuzzing/story-gate-policy.json",
    "fuzzing/seeds/security-failures-v1.json",
    "fuzzing/regressions/FT-GRANT-001/7f84c8c9be1e625621626ae0cef24fb24783041fd38856ec4d255d2a2ee3541a.bin",
    "fuzzing/regressions/FT-IPC-001/b1432642907bf9ece49a1ea6cb8c54bebd9f1e5ca2df66bfd995cf6a7a6f73c9.bin",
    "fuzzing/regressions/FT-MANIFEST-001/6f93481fe2c111b1c44176bfa27e6461bf26668e0c6c31fab4a32fa8a4f663e5.bin",
    "fuzzing/regressions/FT-MODEL-OUTPUT-001/1e88bb017cece80e46f2c16182e132f91161d3481388f7ab9259f783fc269ccc.bin",
    "fuzzing/regressions/FT-PATH-001/fa08499e14d0113ba6794623f1badedcc8e9ae51cb5bafc7e14a5af1454bcfe7.bin",
    "fuzzing/regressions/FT-TEXT-001/e1a480b8acb07821ae78acd569b61b07eb629ab423ace44b6d4e7179fe1e9c43.bin",
    "fuzzing/dictionaries/common.dict",
    "fuzzing/dictionaries/structured.dict",
    "fuzzing/dictionaries/paths.dict",
    "fuzzing/dictionaries/git.dict",
    "fuzzing/dictionaries/sqlite.dict",
    "fuzzing/dictionaries/archives.dict",
    "fixtures/generator-profile.json",
    "fixtures/corpus-profile.json",
    "fixtures/corpus/v1/agentmage-synthetic-corpus-v1.zip",
    "fixtures/corpus/v1/manifest.json",
    "fixtures/corpus/v1/provenance-ledger.json",
    "fixtures/adversarial-fixture-profile.json",
    "fixtures/acceptance-runner-profile.json",
    "fixtures/document-fixture-profile.json",
    "fixtures/expected-output-profile.json",
    "fixtures/fake-adapter-contract.json",
    "fixtures/fault-test-adapter-profile.json",
    "fixtures/story-2.1/later-input-class-fixtures-v1.json",
    "fixtures/path-fixture-profile.json",
    "fixtures/platform-result-profile.json",
    "fixtures/test-result-bundle-profile.json",
    "schemas/testing/fixture-provenance-ledger.schema.json",
    "schemas/testing/platform-result.schema.json",
    "schemas/testing/fuzz-result.schema.json",
    "schemas/testing/examples/fuzz-result.valid.json",
    "schemas/configuration/common.schema.json",
    "schemas/configuration/core.schema.json",
    "schemas/configuration/platform.schema.json",
    "schemas/configuration/model.schema.json",
    "schemas/configuration/workspace.schema.json",
    "schemas/configuration/tool.schema.json",
    "schemas/configuration/permission.schema.json",
    "schemas/configuration/budget.schema.json",
    "schemas/configuration/logging.schema.json",
    "schemas/configuration/retention.schema.json",
    "schemas/configuration/skill.schema.json",
    "schemas/configuration/shell.schema.json",
    "schemas/configuration/agent-configuration.schema.json",
    "schemas/configuration/profile-catalog.schema.json",
    "schemas/configuration/configuration-result.schema.json",
    "schemas/configuration/configuration-diff.schema.json",
    "schemas/configuration/configuration-rollback-report.schema.json",
    "schemas/configuration/examples/agent-configuration.valid.json",
    "schemas/configuration/examples/configuration-session-result.valid.json",
    "schemas/configuration/examples/configuration-release-result.valid.json",
    "schemas/configuration/examples/configuration-diff.valid.json",
    "schemas/configuration/examples/configuration-rollback-report.valid.json",
    "fixtures/configuration/migration/v0.valid.json",
    "fixtures/configuration/migration/v1.expected.json",
    "fixtures/configuration/migration/v1.not-migratable.invalid.json",
    "fixtures/configuration/migration/v0.reserved-version.invalid.json",
    "fixtures/configuration/migration/v0.missing-section.invalid.json",
    "configuration/profiles/capability-deltas.json",
    "configuration/profiles/catalog.json",
    "configuration/profiles/development.json",
    "configuration/profiles/synthetic-test.json",
    "configuration/profiles/strict-local-read-only.json",
    "configuration/profiles/knowledge.json",
    "configuration/profiles/write.json",
    "configuration/profiles/coding.json",
    "configuration/profiles/later-network.json",
    "supply-chain/README.md",
    "requirements/additions-only-baseline.json",
    "requirements/conflict-policy.json",
    "requirements/normative-map.json",
    "requirements/policy-expectations.json",
    "requirements/registry.json",
    "requirements/security-references.json",
    "requirements/security-reference-baseline.json",
    "requirements/traceability-report.json",
    "scripts/verify_clean_traceability.py",
    "scripts/story_2_1_security_evidence.py",
    "scripts/story_2_1_gate.py",
    "scripts/later_input_fixture_matrix.py",
    "scripts/fuzz_target_registry.py",
    "scripts/fuzz_toolchain_policy.py",
    "scripts/fuzz_result_contract.py",
    "scripts/fuzz_story_gate.py",
    "scripts/seeded_fuzz_failures.py",
    "scripts/fuzz_baseline_reconciliation.py",
    "scripts/story_2_2_security_evidence.py",
    "scripts/story_2_2_gate.py",
    "scripts/sprint_2_gate.py",
    "scripts/configuration_profiles.py",
    "scripts/configuration_loader_evidence.py",
    "scripts/configuration_authority_evidence.py",
    "scripts/configuration_result_evidence.py",
    "scripts/configuration_review_artifacts.py",
    "scripts/component_inventory.py",
    "scripts/update_design.py",
    "tests/test_configuration_loader_evidence.py",
    "tests/test_configuration_authority_evidence.py",
    "tests/test_configuration_result_evidence.py",
    "tests/test_configuration_review_artifacts.py",
    "tests/test_component_inventory.py",
    "tests/test_update_design.py",
    "kernel/engine/src/configuration.rs",
    "scripts/sprint0_evidence.py",
    "scripts/story_0_2_evidence.py",
    "scripts/model_admission.py",
    "model-profiles/candidates/gemma-4-e4b/source-admission.json",
    "model-profiles/candidates/gemma-4-e4b/artifact-admission.json",
    "model-profiles/evaluation/corpus-v1.json",
    "scripts/model_corpus.py",
    "artifacts/sprints/sprint-0/story-0.1/evidence-manifest.json",
    "artifacts/sprints/sprint-0/story-0.1/summary.md",
    "artifacts/sprints/sprint-0/story-0.2/evidence-manifest.json",
    "artifacts/sprints/sprint-0/story-0.2/summary.md",
    "schemas/planning/common.schema.json",
    "schemas/planning/decision-record.schema.json",
    "schemas/planning/risk-register.schema.json",
    "schemas/planning/change-log.schema.json",
    "schemas/planning/release-manifest.schema.json",
    "schemas/planning/requirement-supersession.schema.json",
    "schemas/planning/examples/decision-record.valid.json",
    "schemas/planning/examples/risk-register.valid.json",
    "schemas/planning/examples/change-log.valid.json",
    "schemas/planning/examples/release-manifest.valid.json",
    "schemas/planning/examples/requirement-supersession.valid.json",
    "schemas/planning/templates/decision-record.template.json",
    "schemas/planning/templates/requirement-supersession.template.json",
    "LICENSE",
    "docs/decisions/0001-product-security-and-runtime-baseline.md",
    "docs/decisions/0002-public-security-reference-retention.md",
    "docs/decisions/0003-blocked-platform-lane-continuation.md",
    "docs/decisions/0004-language-and-build-system-architecture.md",
    "docs/decisions/0005-signed-manual-update-design.md",
    "docs/decisions/0006-update-rollback-design.md",
    "docs/architecture/dependency-direction.md",
)
CANONICAL_DOCS = (
    "README.md",
    "PRD.md",
    "IMPLEMENTATION-PLAN.md",
    "Agent-Scaffolding-Inventory.md",
    "TASKS.md",
)
DECISION_FILE = "docs/decisions/0001-product-security-and-runtime-baseline.md"
DECISION_BOUNDARIES = {
    "license": (
        re.compile(r"Apache(?: License)?[- ]2\.0", re.IGNORECASE),
    ),
    "model": (
        re.compile(r"Gemma 4 E4B"),
        re.compile(r"Gemma 4 12B Unified"),
    ),
    "runtime": (
        re.compile(r"llama\.cpp"),
        re.compile(r"Docker Model Runner"),
        re.compile(r"LocalModelRuntime"),
    ),
    "platform": (
        re.compile(r"Apple Silicon"),
        re.compile(r"Fedora"),
        re.compile(r"Ubuntu"),
    ),
    "interface": (
        re.compile(r"Visual Studio Code Chat"),
    ),
    "handoff": (
        re.compile(r"Codex"),
        re.compile(
            r"(?:only the user|user manually|user chooses|cannot invoke Codex|"
            r"without invoking Codex|Codex invocation)",
            re.IGNORECASE,
        ),
    ),
    "support": (
        re.compile(r"SECURITY\.md"),
        re.compile(r"support(?:ed|ing|s|able|-|\b)", re.IGNORECASE),
    ),
    "release": (
        re.compile(r"release gate", re.IGNORECASE),
        re.compile(r"\bblocked\b", re.IGNORECASE),
    ),
}
LINK = re.compile(r"(?<!!)\[[^\]]+\]\(([^)]+)\)")
IDENTIFIER = re.compile(r"\b(?:AM|AT|CR)-[A-Z0-9.-]+\b")
SR_IDENTIFIER = re.compile(r"\bSR-[A-Z]+-\d{3}\b")
RV_IDENTIFIER = re.compile(r"\bRV-(?:0[1-9]|1\d|2[0-2])\b")
PROHIBITED_CLAIM = re.compile(
    r"\b(?:federal|government|treasury|fedramp|fisma|fips|nist|sp\s*800)\b",
    re.IGNORECASE,
)
SECRET_SIGNATURES = {
    "private-key marker": re.compile(r"-----BEGIN [A-Z ]*PRIVATE KEY-----"),
    "AWS access key": re.compile(r"\bAKIA[0-9A-Z]{16}\b"),
    "GitHub token": re.compile(r"\bgh[pousr]_[A-Za-z0-9]{20,}\b"),
    "Slack token": re.compile(r"\bxox[baprs]-[A-Za-z0-9-]{10,}\b"),
    "OpenAI-style token": re.compile(r"\bsk-[A-Za-z0-9_-]{20,}\b"),
}


def markdown_files() -> list[Path]:
    return sorted(
        path
        for path in ROOT.rglob("*.md")
        if ".git" not in path.parts and "node_modules" not in path.parts
    )


def text_files() -> list[Path]:
    files: list[Path] = []
    for path in ROOT.rglob("*"):
        if not path.is_file() or ".git" in path.parts or "node_modules" in path.parts:
            continue
        try:
            path.read_text(encoding="utf-8")
        except UnicodeDecodeError:
            continue
        files.append(path)
    return sorted(files)


def read(relative: str) -> str:
    return (ROOT / relative).read_text(encoding="utf-8")


def check_required(failures: list[str]) -> None:
    for relative in REQUIRED_FILES:
        if not (ROOT / relative).is_file():
            failures.append(f"missing required file: {relative}")


def check_links(files: list[Path], failures: list[str]) -> None:
    for path in files:
        for raw_target in LINK.findall(path.read_text(encoding="utf-8")):
            target = raw_target.strip().split()[0].strip("<>")
            if target.startswith(("http://", "https://", "mailto:", "#")):
                continue
            file_part = unquote(target.split("#", 1)[0])
            if not file_part:
                continue
            destination = (path.parent / file_part).resolve()
            try:
                destination.relative_to(ROOT)
            except ValueError:
                failures.append(f"{path.relative_to(ROOT)}: link leaves repository: {target}")
                continue
            if not destination.exists():
                failures.append(f"{path.relative_to(ROOT)}: broken local link: {target}")


def check_sensitive(files: list[Path], failures: list[str]) -> None:
    for path in files:
        text = path.read_text(encoding="utf-8")
        relative = path.relative_to(ROOT)
        for label, pattern in SECRET_SIGNATURES.items():
            match = pattern.search(text)
            if match:
                failures.append(f"{relative}: possible {label} at character {match.start()}")


def check_claims(files: list[Path], failures: list[str]) -> None:
    for path in files:
        text = path.read_text(encoding="utf-8")
        claim = PROHIBITED_CLAIM.search(text)
        if claim:
            relative = path.relative_to(ROOT)
            failures.append(
                f"{relative}: prohibited deployment-specific claim at character {claim.start()}"
            )


def check_identifiers(files: list[Path], failures: list[str]) -> None:
    inventory = read("Agent-Scaffolding-Inventory.md")
    security = read("SECURITY-REVIEW.md")
    definition_list = re.findall(
        r"^\| `((?:AM|AT|CR)-[A-Z0-9.-]+)` \|", inventory, re.MULTILINE
    )
    definition_list.extend(
        re.findall(r"^\| `(SR-[A-Z]+-\d{3})` \|", security, re.MULTILINE)
    )
    definition_list.extend(
        re.findall(r"^### `(RV-(?:0[1-9]|1\d|2[0-2]))`", security, re.MULTILINE)
    )
    definitions = set(definition_list)

    duplicates = sorted(
        identifier for identifier in definitions if definition_list.count(identifier) > 1
    )
    for identifier in duplicates:
        failures.append(f"duplicate stable identifier definition: {identifier}")

    references: set[str] = set()
    for path in files:
        text = path.read_text(encoding="utf-8")
        references.update(IDENTIFIER.findall(text))
        references.update(SR_IDENTIFIER.findall(text))
        references.update(RV_IDENTIFIER.findall(text))

    for identifier in sorted(references - definitions):
        failures.append(f"unresolved stable identifier: {identifier}")

    expected_rv = {f"RV-{number:02d}" for number in range(1, 23)}
    missing_rv = expected_rv - set(RV_IDENTIFIER.findall(security))
    if missing_rv:
        failures.append(f"missing reviewer protocols: {', '.join(sorted(missing_rv))}")


def check_cross_document_contract(failures: list[str]) -> None:
    required_terms = (
        "Gemma 4 E4B",
        "Docker Model Runner",
        "llama.cpp",
        "Fedora",
        "Ubuntu",
        "Apple Silicon",
        "Visual Studio Code Chat",
    )
    required_links = (
        "MODEL-PROVENANCE-POLICY.md",
        "SECURITY.md",
        "RUNTIME-BOUNDARIES.md",
    )
    for relative in CANONICAL_DOCS:
        text = read(relative)
        for term in required_terms:
            if term not in text:
                failures.append(f"{relative}: missing cross-document term: {term}")
        for linked_file in required_links:
            if linked_file not in text:
                failures.append(f"{relative}: missing policy reference: {linked_file}")

    tasks = read("TASKS.md")
    sprint_count = len(re.findall(r"^### \[ \] Sprint \d+", tasks, re.MULTILINE))
    if sprint_count != 103:
        failures.append(f"TASKS.md: expected 103 sprint headings, found {sprint_count}")

    license_text = read("LICENSE")
    if "Apache License" not in license_text or "Version 2.0" not in license_text:
        failures.append("LICENSE: expected complete Apache License 2.0 text")


def check_accepted_decision_contract(
    failures: list[str],
    documents: dict[str, str] | None = None,
    decision: str | None = None,
) -> None:
    """Require every canonical document to preserve Decision 0001 boundaries."""
    canonical = documents or {relative: read(relative) for relative in CANONICAL_DOCS}
    decision_text = decision if decision is not None else read(DECISION_FILE)
    decision_markers = {
        "license": ("Apache License 2.0",),
        "model": ("Gemma 4 E4B", "Gemma 4 12B Unified"),
        "runtime": ("llama.cpp", "Docker Model Runner", "LocalModelRuntime"),
        "platform": ("Fedora", "Ubuntu"),
        "interface": ("native-Chat diagnostics",),
        "handoff": ("handoff disclosure warnings",),
        "support": ("SECURITY.md", "supported versions", "end of support"),
        "release": ("release sprint", "assembles evidence"),
    }

    if "| Status | Accepted |" not in decision_text:
        failures.append(f"{DECISION_FILE}: decision is not accepted")
    for boundary, markers in decision_markers.items():
        for marker in markers:
            if marker not in decision_text:
                failures.append(
                    f"{DECISION_FILE}: {boundary} boundary is missing decision marker"
                )

    for relative in CANONICAL_DOCS:
        text = canonical.get(relative)
        if text is None:
            failures.append(f"{relative}: missing from decision-consistency input")
            continue
        for boundary, patterns in DECISION_BOUNDARIES.items():
            for pattern in patterns:
                if pattern.search(text) is None:
                    failures.append(
                        f"{relative}: {boundary} boundary disagrees with Decision 0001"
                    )
                    break


def main() -> int:
    failures: list[str] = []
    files = markdown_files()
    project_text_files = text_files()
    check_required(failures)
    if failures:
        print("\n".join(failures), file=sys.stderr)
        return 1

    check_links(files, failures)
    check_sensitive(project_text_files, failures)
    check_claims(files, failures)
    check_identifiers(files, failures)
    check_cross_document_contract(failures)
    check_accepted_decision_contract(failures)

    if failures:
        print("Documentation validation failed:", file=sys.stderr)
        for failure in failures:
            print(f"- {failure}", file=sys.stderr)
        return 1

    print(f"Validated {len(files)} Markdown file(s) and all policy invariants.")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
