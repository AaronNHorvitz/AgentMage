#!/usr/bin/env python3
"""Build and validate Story 2.1 security evidence and signed comparison."""

from __future__ import annotations

import argparse
import hashlib
import json
import os
import subprocess
import sys
import tempfile
from pathlib import Path, PurePosixPath
from typing import Any


ROOT = Path(__file__).resolve().parents[1]
if str(ROOT) not in sys.path:
    sys.path.insert(0, str(ROOT))

from scripts.fixture_security_scan import (  # noqa: E402
    FINAL_EVIDENCE_ENVELOPE,
    FINAL_EVIDENCE_VERIFIER,
    ScanMetrics,
    scan_blob,
)

EVIDENCE_ROOT = ROOT / "artifacts/sprints/sprint-2/story-2.1"
SUMMARY_REPORT_PATH = EVIDENCE_ROOT / "summary-reconciliation-report.json"
COMPARISON_PATH = EVIDENCE_ROOT / "summary-comparison.json"
PUBLIC_KEY_PATH = EVIDENCE_ROOT / "summary-comparison-public.pem"
SIGNATURE_PATH = EVIDENCE_ROOT / "summary-comparison.sig"
SIGNATURE_RECORD_PATH = EVIDENCE_ROOT / "summary-comparison-signature.json"
MAP_PATH = EVIDENCE_ROOT / "security-evidence-map.json"
EXPECTED_REQUIREMENTS = (
    "SR-TST-001",
    "SR-TST-002",
    "SR-TST-003",
    "SR-TST-004",
    "SR-TST-005",
    "SR-TST-006",
    "SR-TST-010",
    "SR-DAT-002",
    "SR-OPS-003",
    "SR-AI-005",
)
EVIDENCE_PATHS = (
    "fixtures/adversarial-fixture-profile.json",
    "fixtures/corpus-profile.json",
    "fixtures/corpus/v1/agentmage-synthetic-corpus-v1.zip",
    "fixtures/corpus/v1/manifest.json",
    "fixtures/corpus/v1/provenance-ledger.json",
    "fixtures/expected-output-profile.json",
    "fixtures/fake-adapter-contract.json",
    "fixtures/fault-test-adapter-profile.json",
    "fixtures/story-2.1/later-input-class-fixtures-v1.json",
    "artifacts/sprints/sprint-2/story-2.1/adapter-mode-verification-report.json",
    "artifacts/sprints/sprint-2/story-2.1/corpus-reproducibility-report.json",
    "artifacts/sprints/sprint-2/story-2.1/fault-test-adapter-report.json",
    "artifacts/sprints/sprint-2/story-2.1/fixture-security-scan-report.json",
    "artifacts/sprints/sprint-2/story-2.1/input-class-fixture-matrix-report.json",
    "artifacts/sprints/sprint-2/story-2.1/summary-reconciliation-report.json",
    "artifacts/sprints/sprint-2/story-2.1/story-gate-report.json",
    "artifacts/sprints/sprint-2/story-2.1/summary-comparison.json",
    "artifacts/sprints/sprint-2/story-2.1/summary-comparison-public.pem",
    "artifacts/sprints/sprint-2/story-2.1/summary-comparison.sig",
    "artifacts/sprints/sprint-2/story-2.1/summary-comparison-signature.json",
)
MAPPINGS = {
    "SR-TST-001": {
        "story_contribution": "partial-story-evidence",
        "evidence": [
            "artifacts/sprints/sprint-2/story-2.1/corpus-reproducibility-report.json",
            "artifacts/sprints/sprint-2/story-2.1/adapter-mode-verification-report.json",
            "artifacts/sprints/sprint-2/story-2.1/summary-reconciliation-report.json",
            "artifacts/sprints/sprint-2/story-2.1/story-gate-report.json",
        ],
        "demonstrated": "Deterministic unit, integration, adversarial, and recovery foundations execute over synthetic data with requirement-labeled evidence.",
        "remaining": "Product end-to-end, release-platform, property, and later boundary suites remain incomplete.",
    },
    "SR-TST-002": {
        "story_contribution": "registered-next-story",
        "evidence": [
            "fixtures/adversarial-fixture-profile.json",
            "artifacts/sprints/sprint-2/story-2.1/fault-test-adapter-report.json",
        ],
        "demonstrated": "Malformed and adversarial seeds plus bounded fault adapters are versioned for later fuzz targets.",
        "remaining": "Fuzz target registration, engines, coverage, minimization, and regression retention are owned by Story 2.2 and later trust-boundary stories.",
    },
    "SR-TST-003": {
        "story_contribution": "partial-story-evidence",
        "evidence": [
            "artifacts/sprints/sprint-2/story-2.1/fixture-security-scan-report.json",
            "artifacts/sprints/sprint-2/story-2.1/summary-comparison-signature.json",
        ],
        "demonstrated": "The fixture/evidence surface is recursively scanned and the independent summary comparison has a verifiable detached signature.",
        "remaining": "Release-grade pinned SAST, SCA, secret, binary, and platform analysis with reviewed suppressions remains incomplete.",
    },
    "SR-TST-004": {
        "story_contribution": "partial-story-evidence",
        "evidence": [
            "fixtures/corpus/v1/manifest.json",
            "fixtures/adversarial-fixture-profile.json",
            "artifacts/sprints/sprint-2/story-2.1/corpus-reproducibility-report.json",
            "artifacts/sprints/sprint-2/story-2.1/input-class-fixture-matrix-report.json",
        ],
        "demonstrated": "Versioned normal, boundary, malformed, hostile, oversized, cancellation, and recovery-oriented synthetic inputs fail within declared side-effect bounds.",
        "remaining": "Each future product input class must bind these fixture classes to its concrete parser and receipt behavior.",
    },
    "SR-TST-005": {
        "story_contribution": "partial-story-evidence",
        "evidence": [
            "fixtures/fault-test-adapter-profile.json",
            "artifacts/sprints/sprint-2/story-2.1/fault-test-adapter-report.json",
            "artifacts/sprints/sprint-2/story-2.1/adapter-mode-verification-report.json",
        ],
        "demonstrated": "Synthetic crashes distinguish before-commit, after-commit, and uncertain completion while every adapter has explicit cleanup.",
        "remaining": "At least 100 real durable-transition crash resumes per applicable component remain required.",
    },
    "SR-TST-006": {
        "story_contribution": "partial-story-evidence",
        "evidence": [
            "fixtures/fault-test-adapter-profile.json",
            "artifacts/sprints/sprint-2/story-2.1/fault-test-adapter-report.json",
        ],
        "demonstrated": "CPU-step, memory, disk, token, file, and output budgets deterministically report exhaustion and cleanup without exhausting the host.",
        "remaining": "Product process, GPU, context, concurrency, responsiveness, and platform resource enforcement remain incomplete.",
    },
    "SR-TST-010": {
        "story_contribution": "demonstrated-story-scope",
        "evidence": [
            "artifacts/sprints/sprint-2/story-2.1/summary-reconciliation-report.json",
            "artifacts/sprints/sprint-2/story-2.1/summary-comparison.json",
            "artifacts/sprints/sprint-2/story-2.1/summary-comparison-signature.json",
            "artifacts/sprints/sprint-2/story-2.1/story-gate-report.json",
        ],
        "demonstrated": "An independent reducer reconciles raw normalized shard results to the producer summary and the content-addressed comparison is signed.",
        "remaining": "Every later suite and release result must retain the same raw, normalized, summary, failure, skip, environment, and tool separation.",
    },
    "SR-DAT-002": {
        "story_contribution": "partial-story-evidence",
        "evidence": [
            "fixtures/expected-output-profile.json",
            "artifacts/sprints/sprint-2/story-2.1/fixture-security-scan-report.json",
        ],
        "demonstrated": "Synthetic fixtures declare data class, prohibited side effects, redaction expectations, and zero private input before evidence persistence.",
        "remaining": "The product-wide pre-persistence policy gate and retention/storage decisions remain unimplemented.",
    },
    "SR-OPS-003": {
        "story_contribution": "partial-story-evidence",
        "evidence": [
            "artifacts/sprints/sprint-2/story-2.1/fixture-security-scan-report.json",
            "artifacts/sprints/sprint-2/story-2.1/adapter-mode-verification-report.json",
        ],
        "demonstrated": "Synthetic canaries are redacted from traces and reports; private paths and credential shapes are rejected across the fixture/evidence surface.",
        "remaining": "Every future log, export, error, crash, and diagnostics path must pass typed redaction and canary tests.",
    },
    "SR-AI-005": {
        "story_contribution": "partial-story-evidence",
        "evidence": [
            "fixtures/adversarial-fixture-profile.json",
            "fixtures/corpus/v1/manifest.json",
            "artifacts/sprints/sprint-2/story-2.1/fault-test-adapter-report.json",
        ],
        "demonstrated": "Prompt-injection, approval-bypass, grant-replay, conflict, and secret-canary fixtures are inert, versioned, and produce no unauthorized side effect in test adapters.",
        "remaining": "The product policy kernel, model boundary, tool mediation, and at least 200 labeled end-to-end injections remain unimplemented.",
    },
}


def read_json(path: Path) -> Any:
    return json.loads(path.read_text(encoding="utf-8"))


def canonical_json(value: Any) -> bytes:
    return (json.dumps(value, indent=2, sort_keys=True) + "\n").encode("utf-8")


def sha256_bytes(value: bytes) -> str:
    return hashlib.sha256(value).hexdigest()


def sha256_file(path: Path) -> str:
    return sha256_bytes(path.read_bytes())


def safe_relative_path(value: Any) -> bool:
    if not isinstance(value, str) or not value:
        return False
    path = PurePosixPath(value)
    return not path.is_absolute() and ".." not in path.parts and str(path) == value


def write_atomic(path: Path, content: bytes) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    descriptor, temporary_name = tempfile.mkstemp(
        prefix=".agentmage-story-2-1-security-", dir=path.parent
    )
    temporary = Path(temporary_name)
    try:
        with os.fdopen(descriptor, "wb") as handle:
            handle.write(content)
            handle.flush()
            os.fsync(handle.fileno())
        temporary.chmod(0o644)
        os.replace(temporary, path)
    except BaseException:
        temporary.unlink(missing_ok=True)
        raise


def build_comparison(root: Path = ROOT) -> dict[str, Any]:
    summary_path = root / SUMMARY_REPORT_PATH.relative_to(ROOT)
    summary = read_json(summary_path)
    if summary.get("status") != "pass" or summary.get("reconciliation", {}).get(
        "exact_match"
    ) is not True:
        raise ValueError("summary reconciliation is not eligible for signing")
    return {
        "schema_version": 1,
        "comparison_id": "agentmage-story-2.1-summary-comparison-v1",
        "generated_at": "2024-01-01T00:05:00Z",
        "source_report": {
            "path": SUMMARY_REPORT_PATH.relative_to(ROOT).as_posix(),
            "sha256": sha256_file(summary_path),
        },
        "comparison": summary["comparison"],
        "reconciliation": summary["reconciliation"],
        "statement": "producer-and-independent-summaries-match-exactly",
        "raw_results_included": False,
        "product_acceptance_claim": "none",
        "macos_execution_status": "blocked-macos",
        "macos_support_claim": "none",
    }


def validate_comparison(comparison: Any, root: Path = ROOT) -> list[str]:
    if not isinstance(comparison, dict):
        return ["summary comparison must be an object"]
    failures: list[str] = []
    if (
        comparison.get("schema_version") != 1
        or comparison.get("comparison_id")
        != "agentmage-story-2.1-summary-comparison-v1"
        or comparison.get("statement")
        != "producer-and-independent-summaries-match-exactly"
    ):
        failures.append("summary comparison identity is invalid")
    if comparison.get("raw_results_included") is not False:
        failures.append("summary comparison retained raw results")
    if comparison.get("product_acceptance_claim") != "none":
        failures.append("summary comparison made a product acceptance claim")
    if comparison.get("macos_execution_status") != "blocked-macos" or comparison.get(
        "macos_support_claim"
    ) != "none":
        failures.append("summary comparison made an invalid macOS claim")
    try:
        expected = build_comparison(root)
    except (OSError, ValueError, KeyError, TypeError) as error:
        failures.append(f"cannot rebuild summary comparison: {error}")
    else:
        if comparison != expected:
            failures.append("summary comparison is stale or non-deterministic")
    return failures


def verify_signature(
    comparison_path: Path,
    public_key_path: Path,
    signature_path: Path,
) -> bool:
    try:
        result = subprocess.run(
            [
                "openssl",
                "pkeyutl",
                "-verify",
                "-pubin",
                "-inkey",
                str(public_key_path),
                "-sigfile",
                str(signature_path),
                "-rawin",
                "-in",
                str(comparison_path),
            ],
            stdin=subprocess.DEVNULL,
            stdout=subprocess.DEVNULL,
            stderr=subprocess.DEVNULL,
            check=False,
            timeout=10,
        )
    except (OSError, subprocess.TimeoutExpired):
        return False
    return result.returncode == 0


def validate_final_evidence_envelope(
    root: Path = ROOT,
    *,
    include_map: bool = True,
    include_signature_record: bool = True,
) -> list[str]:
    failures: list[str] = []
    expected_paths = list(FINAL_EVIDENCE_ENVELOPE)
    if expected_paths != [
        MAP_PATH.relative_to(ROOT).as_posix(),
        COMPARISON_PATH.relative_to(ROOT).as_posix(),
        PUBLIC_KEY_PATH.relative_to(ROOT).as_posix(),
        SIGNATURE_PATH.relative_to(ROOT).as_posix(),
        SIGNATURE_RECORD_PATH.relative_to(ROOT).as_posix(),
    ]:
        failures.append("final evidence envelope registration has drifted")
        return failures

    selected = []
    for path in expected_paths:
        if not include_map and path == MAP_PATH.relative_to(ROOT).as_posix():
            continue
        if (
            not include_signature_record
            and path == SIGNATURE_RECORD_PATH.relative_to(ROOT).as_posix()
        ):
            continue
        selected.append(path)
    metrics = ScanMetrics()
    for relative in selected:
        path = root / relative
        if not path.is_file():
            failures.append(f"final evidence envelope file is missing: {relative}")
            continue
        findings = scan_blob(
            relative,
            path.read_bytes(),
            metrics,
            executable=bool(path.stat().st_mode & 0o111),
        )
        failures.extend(
            f"final evidence envelope finding: {item.category}: {item.path}"
            for item in findings
        )

    public_key = root / PUBLIC_KEY_PATH.relative_to(ROOT)
    if public_key.is_file():
        content = public_key.read_bytes()
        if not content.startswith(b"-----BEGIN PUBLIC KEY-----\n") or not content.endswith(
            b"-----END PUBLIC KEY-----\n"
        ):
            failures.append("summary comparison public key is not a canonical public PEM")
        if b"PRIVATE KEY" in content:
            failures.append("summary comparison public material contains a private key")
    signature = root / SIGNATURE_PATH.relative_to(ROOT)
    if signature.is_file() and signature.stat().st_size != 64:
        failures.append("summary comparison Ed25519 signature is not 64 bytes")

    scan_report_path = root / "artifacts/sprints/sprint-2/story-2.1/fixture-security-scan-report.json"
    try:
        scan_report = read_json(scan_report_path)
    except (OSError, json.JSONDecodeError) as error:
        failures.append(f"cannot read fixture security scan boundary: {error}")
    else:
        scope = scan_report.get("scope", {})
        if scope.get("excluded_final_evidence_envelope") != expected_paths or scope.get(
            "final_evidence_envelope_verifier"
        ) != FINAL_EVIDENCE_VERIFIER:
            failures.append("fixture scanner did not delegate the final envelope exactly")
    return failures


def build_signature_record(root: Path = ROOT) -> dict[str, Any]:
    comparison_path = root / COMPARISON_PATH.relative_to(ROOT)
    public_key_path = root / PUBLIC_KEY_PATH.relative_to(ROOT)
    signature_path = root / SIGNATURE_PATH.relative_to(ROOT)
    envelope_failures = validate_final_evidence_envelope(
        root, include_map=False, include_signature_record=False
    )
    if envelope_failures:
        raise ValueError("; ".join(envelope_failures))
    if not verify_signature(comparison_path, public_key_path, signature_path):
        raise ValueError("summary comparison detached signature is invalid")
    return {
        "schema_version": 1,
        "signature_id": "agentmage-story-2.1-summary-ed25519-v1",
        "status": "verified",
        "algorithm": "Ed25519",
        "verification_tool": "openssl-pkeyutl",
        "signed_artifact": {
            "path": COMPARISON_PATH.relative_to(ROOT).as_posix(),
            "sha256": sha256_file(comparison_path),
        },
        "public_key": {
            "path": PUBLIC_KEY_PATH.relative_to(ROOT).as_posix(),
            "pem_sha256": sha256_file(public_key_path),
            "private_key_retained": False,
            "test_evidence_only": True,
        },
        "detached_signature": {
            "path": SIGNATURE_PATH.relative_to(ROOT).as_posix(),
            "sha256": sha256_file(signature_path),
            "bytes": signature_path.stat().st_size,
        },
        "network_used": False,
        "release_signature_claim": "none",
        "macos_execution_status": "blocked-macos",
        "macos_support_claim": "none",
    }


def validate_signature_record(record: Any, root: Path = ROOT) -> list[str]:
    if not isinstance(record, dict):
        return ["summary signature record must be an object"]
    failures: list[str] = []
    if (
        record.get("schema_version") != 1
        or record.get("signature_id")
        != "agentmage-story-2.1-summary-ed25519-v1"
        or record.get("status") != "verified"
        or record.get("algorithm") != "Ed25519"
    ):
        failures.append("summary signature record identity is invalid")
    public_key = record.get("public_key", {})
    if (
        public_key.get("private_key_retained") is not False
        or public_key.get("test_evidence_only") is not True
    ):
        failures.append("summary signature key-retention contract was weakened")
    if record.get("network_used") is not False or record.get(
        "release_signature_claim"
    ) != "none":
        failures.append("summary signature made an invalid network or release claim")
    if record.get("macos_execution_status") != "blocked-macos" or record.get(
        "macos_support_claim"
    ) != "none":
        failures.append("summary signature made an invalid macOS claim")
    try:
        expected = build_signature_record(root)
    except (OSError, ValueError, KeyError, TypeError) as error:
        failures.append(f"cannot verify summary signature: {error}")
    else:
        if record != expected:
            failures.append("summary signature record is stale or non-deterministic")
    return failures


def build_map(root: Path = ROOT) -> dict[str, Any]:
    signature_record = read_json(root / SIGNATURE_RECORD_PATH.relative_to(ROOT))
    signature_failures = validate_signature_record(signature_record, root)
    if signature_failures:
        raise ValueError("; ".join(signature_failures))
    envelope_failures = validate_final_evidence_envelope(root, include_map=False)
    if envelope_failures:
        raise ValueError("; ".join(envelope_failures))
    requirements = [
        {
            "requirement_id": requirement_id,
            "product_requirement_status": "not-complete",
            **MAPPINGS[requirement_id],
        }
        for requirement_id in EXPECTED_REQUIREMENTS
    ]
    return {
        "schema_version": 1,
        "task_id": "2.1.3.5",
        "status": "complete-evidence-map-blocked-macos",
        "requirements": requirements,
        "artifacts": [
            {"path": path, "sha256": sha256_file(root / path)}
            for path in EVIDENCE_PATHS
        ],
        "signed_summary_comparison": {
            "signature_id": signature_record["signature_id"],
            "status": signature_record["status"],
            "algorithm": signature_record["algorithm"],
            "release_signature_claim": "none",
        },
        "summary": {
            "mapped_requirements": len(EXPECTED_REQUIREMENTS),
            "product_requirements_complete": 0,
            "task_mapping_complete": True,
            "signed_summary_verified": True,
            "canonical_fixture_security_findings": 0,
            "final_evidence_envelope_findings": 0,
            "story_gate_complete": False,
        },
        "macos": {
            "status": "blocked-macos",
            "evidence_substitution": "prohibited",
            "support_claim": "none",
        },
        "release_claim": "none",
    }


def validate_map(evidence_map: Any, root: Path = ROOT) -> list[str]:
    if not isinstance(evidence_map, dict):
        return ["Story 2.1 security evidence map must be an object"]
    failures: list[str] = []
    if evidence_map.get("schema_version") != 1 or evidence_map.get(
        "task_id"
    ) != "2.1.3.5":
        failures.append("Story 2.1 security evidence map identity is invalid")
    if evidence_map.get("status") != "complete-evidence-map-blocked-macos":
        failures.append("Story 2.1 security evidence map status is invalid")
    requirements = evidence_map.get("requirements", [])
    ids = [item.get("requirement_id") for item in requirements]
    if ids != list(EXPECTED_REQUIREMENTS) or len(ids) != len(set(ids)):
        failures.append("Story 2.1 security requirement closure is invalid")
    for item in requirements:
        if item.get("product_requirement_status") != "not-complete":
            failures.append(
                f"Story 2.1 overclaimed product requirement: {item.get('requirement_id')}"
            )
        for path in item.get("evidence", []):
            if not safe_relative_path(path) or path not in EVIDENCE_PATHS:
                failures.append(
                    f"Story 2.1 requirement has invalid evidence: {item.get('requirement_id')}"
                )
    artifacts = evidence_map.get("artifacts", [])
    artifact_paths = [item.get("path") for item in artifacts]
    if artifact_paths != list(EVIDENCE_PATHS) or len(artifact_paths) != len(
        set(artifact_paths)
    ):
        failures.append("Story 2.1 retained evidence closure is invalid")
    for item in artifacts:
        path = item.get("path")
        if not safe_relative_path(path) or not (root / path).is_file():
            failures.append(f"Story 2.1 evidence path is invalid: {path}")
        elif item.get("sha256") != sha256_file(root / path):
            failures.append(f"Story 2.1 evidence hash is invalid: {path}")
    summary = evidence_map.get("summary", {})
    if (
        summary.get("mapped_requirements") != 10
        or summary.get("product_requirements_complete") != 0
        or summary.get("task_mapping_complete") is not True
        or summary.get("signed_summary_verified") is not True
        or summary.get("canonical_fixture_security_findings") != 0
        or summary.get("final_evidence_envelope_findings") != 0
        or summary.get("story_gate_complete") is not False
    ):
        failures.append("Story 2.1 security evidence summary is invalid")
    if evidence_map.get("macos") != {
        "status": "blocked-macos",
        "evidence_substitution": "prohibited",
        "support_claim": "none",
    }:
        failures.append("Story 2.1 security map made an invalid macOS claim")
    if evidence_map.get("release_claim") != "none":
        failures.append("Story 2.1 security map made a release claim")
    try:
        expected = build_map(root)
    except (OSError, ValueError, KeyError, TypeError) as error:
        failures.append(f"cannot rebuild Story 2.1 security map: {error}")
    else:
        if evidence_map != expected:
            failures.append("Story 2.1 security evidence map is stale or non-deterministic")
    return failures


def write_comparison(root: Path = ROOT) -> None:
    write_atomic(
        root / COMPARISON_PATH.relative_to(ROOT), canonical_json(build_comparison(root))
    )


def write_evidence(root: Path = ROOT) -> None:
    write_atomic(
        root / SIGNATURE_RECORD_PATH.relative_to(ROOT),
        canonical_json(build_signature_record(root)),
    )
    write_atomic(root / MAP_PATH.relative_to(ROOT), canonical_json(build_map(root)))


def check_all(root: Path = ROOT) -> list[str]:
    failures: list[str] = []
    try:
        comparison = read_json(root / COMPARISON_PATH.relative_to(ROOT))
    except (OSError, json.JSONDecodeError) as error:
        failures.append(f"cannot read summary comparison: {error}")
    else:
        failures.extend(validate_comparison(comparison, root))
    try:
        signature = read_json(root / SIGNATURE_RECORD_PATH.relative_to(ROOT))
    except (OSError, json.JSONDecodeError) as error:
        failures.append(f"cannot read summary signature record: {error}")
    else:
        failures.extend(validate_signature_record(signature, root))
    try:
        evidence_map = read_json(root / MAP_PATH.relative_to(ROOT))
    except (OSError, json.JSONDecodeError) as error:
        failures.append(f"cannot read Story 2.1 security map: {error}")
    else:
        failures.extend(validate_map(evidence_map, root))
    failures.extend(validate_final_evidence_envelope(root))
    return failures


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--write-comparison", action="store_true")
    parser.add_argument("--write-evidence", action="store_true")
    args = parser.parse_args()
    try:
        if args.write_comparison:
            write_comparison()
            print("Story 2.1 summary comparison written for detached signing")
            return 0
        if args.write_evidence:
            write_evidence()
        failures = check_all()
    except (OSError, ValueError, KeyError, TypeError) as error:
        print(f"Story 2.1 security evidence failed: {error}", file=sys.stderr)
        return 1
    if failures:
        for failure in failures:
            print(f"Story 2.1 security evidence failed: {failure}", file=sys.stderr)
        return 1
    print("Story 2.1 signed summary and security evidence map validated")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
