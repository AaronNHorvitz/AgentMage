#!/usr/bin/env python3
"""Independently review the committed Story 22.2 runtime-artifact boundary."""

from __future__ import annotations

import argparse
import hashlib
import json
import re
import subprocess
from pathlib import Path
from typing import Any, Final

try:
    from scripts.evidence_core import atomic_write, canonical_json_bytes
except ModuleNotFoundError:
    from evidence_core import atomic_write, canonical_json_bytes


ROOT: Final = Path(__file__).resolve().parents[1]
REPORT_PATH: Final = (
    ROOT / "artifacts/sprints/sprint-22/story-22.2/artifact-boundary-review.json"
)
REVISION: Final = re.compile(r"^[0-9a-f]{40}$")
SHA256: Final = re.compile(r"^[0-9a-f]{64}$")
ZERO_SHA256: Final = "0" * 64
SCHEMAS: Final = (
    "schemas/runtime/runtime-artifact-reference.schema.json",
    "schemas/runtime/runtime-artifact-manifest.schema.json",
    "schemas/runtime/runtime-artifact-operator-view.schema.json",
    "schemas/runtime/runtime-resume-binding.schema.json",
)
SOURCE_PATHS: Final = (
    *SCHEMAS,
    "kernel/contracts/src/runtime_artifact.rs",
    "kernel/engine/src/runtime_artifact.rs",
    "kernel/engine/migrations/operational-store/0007-runtime-artifacts.sql",
    "platforms/linux/src/runtime_artifact_store.rs",
    "docs/architecture/runtime-artifact-lifecycle.md",
    "TASKS.md",
    "scripts/runtime_artifact_boundary_review.py",
    "tests/test_runtime_artifact_boundary_review.py",
)
EVIDENCE_PATHS: Final = (
    "artifacts/sprints/sprint-22/story-22.2/native-artifact-crash-matrix.json",
    "artifacts/sprints/sprint-22/story-22.2/native-artifact-crash-matrix.log",
    "artifacts/sprints/sprint-22/story-22.2/native-artifact-integrity.json",
    "artifacts/sprints/sprint-22/story-22.2/native-artifact-integrity.log",
    "artifacts/sprints/sprint-22/story-22.2/native-artifact-pressure.json",
    "artifacts/sprints/sprint-22/story-22.2/native-artifact-pressure.log",
    "artifacts/sprints/sprint-22/story-22.2/native-artifact-resume.json",
    "artifacts/sprints/sprint-22/story-22.2/native-artifact-resume.log",
    "artifacts/sprints/sprint-22/story-22.2/evidence-index.json",
)
PROHIBITED_NETWORK_TOKENS: Final = (
    "reqwest::",
    "TcpStream",
    "UdpSocket",
    "hyper::Client",
    "tonic::transport",
)
LIMITATIONS: Final = (
    "This is an independent automated source-boundary review, not an independent human or cryptographic review.",
    "Stops inside filesystem or SQLite syscalls, physical storage faults, and host power loss are not exercised.",
    "Installed clients, Windows storage, additional supported-platform packages, and real-model sessions are not reviewed.",
    "Concurrent collection and quarantined-payload operator recovery remain open.",
    "Manual fuzzing remains deferred and was not executed.",
    "No release approval, model enablement, platform support, or deployment authority follows from this review.",
)


class ArtifactReviewError(ValueError):
    """Raised when committed artifact-review inputs are unavailable or invalid."""


def sha256_bytes(value: bytes) -> str:
    return hashlib.sha256(value).hexdigest()


def git_revision(candidate: str) -> str:
    result = subprocess.run(
        ["git", "rev-parse", "--verify", f"{candidate}^{{commit}}"],
        cwd=ROOT,
        capture_output=True,
        text=True,
        timeout=30,
        check=False,
    )
    revision = result.stdout.strip()
    if result.returncode or REVISION.fullmatch(revision) is None:
        raise ArtifactReviewError("runtime.artifact_review.source_revision")
    return revision


def git_blob(revision: str, relative: str) -> bytes:
    result = subprocess.run(
        ["git", "show", f"{revision}:{relative}"],
        cwd=ROOT,
        stdout=subprocess.PIPE,
        stderr=subprocess.DEVNULL,
        timeout=60,
        check=False,
    )
    if result.returncode or not result.stdout:
        raise ArtifactReviewError(
            f"runtime.artifact_review.source_unavailable.{relative}"
        )
    return result.stdout


def json_blob(revision: str, relative: str) -> dict[str, Any]:
    try:
        value = json.loads(git_blob(revision, relative))
    except (UnicodeError, json.JSONDecodeError) as error:
        raise ArtifactReviewError(
            f"runtime.artifact_review.json_invalid.{relative}"
        ) from error
    if not isinstance(value, dict):
        raise ArtifactReviewError(f"runtime.artifact_review.json_type.{relative}")
    return value


def source_records(revision: str) -> list[dict[str, Any]]:
    return [
        {"path": path, "bytes": len(content), "sha256": sha256_bytes(content)}
        for path in (*SOURCE_PATHS, *EVIDENCE_PATHS)
        for content in [git_blob(revision, path)]
    ]


def check_record(identifier: str, passed: bool, detail: str) -> dict[str, Any]:
    return {"check_id": identifier, "passed": passed, "detail": detail}


def production_source(source: str) -> str:
    return source.split("\n#[cfg(test)]", maxsplit=1)[0]


def schema_contains_key(value: Any, prohibited: set[str]) -> bool:
    if isinstance(value, dict):
        return any(
            key in prohibited or schema_contains_key(child, prohibited)
            for key, child in value.items()
        )
    if isinstance(value, list):
        return any(schema_contains_key(child, prohibited) for child in value)
    return False


def evidence_report_valid(
    report: dict[str, Any], raw_log: bytes, expected_status: str
) -> bool:
    trace = report.get("raw_trace", {})
    return bool(
        report.get("status") == expected_status
        and report.get("external_network_used") is False
        and report.get("private_user_data_used") is False
        and trace.get("sha256") == sha256_bytes(raw_log)
        and trace.get("redactions") == ["repository-root"]
        and REVISION.fullmatch(str(report.get("source_revision", ""))) is not None
    )


def review_checks(revision: str) -> list[dict[str, Any]]:
    schemas = [json_blob(revision, path) for path in SCHEMAS]
    contract = git_blob(revision, "kernel/contracts/src/runtime_artifact.rs").decode()
    engine = git_blob(revision, "kernel/engine/src/runtime_artifact.rs").decode()
    migration = git_blob(
        revision,
        "kernel/engine/migrations/operational-store/0007-runtime-artifacts.sql",
    ).decode()
    linux = git_blob(revision, "platforms/linux/src/runtime_artifact_store.rs").decode()
    architecture = git_blob(
        revision, "docs/architecture/runtime-artifact-lifecycle.md"
    ).decode()
    tasks = git_blob(revision, "TASKS.md").decode()

    crash = json_blob(revision, EVIDENCE_PATHS[0])
    integrity = json_blob(revision, EVIDENCE_PATHS[2])
    pressure = json_blob(revision, EVIDENCE_PATHS[4])
    resume = json_blob(revision, EVIDENCE_PATHS[6])
    index = json_blob(revision, EVIDENCE_PATHS[8])

    closed_path_free_schemas = all(
        schema.get("additionalProperties") is False
        and not schema_contains_key(schema.get("properties", {}), {"path", "native_path"})
        for schema in schemas
    )
    path_free_contract = all(
        marker in contract
        for marker in (
            "pub struct RuntimeArtifactRef",
            "pub struct RuntimeArtifactManifest",
            "pub struct RuntimeArtifactOperatorView",
            "pub struct RuntimeResumeBinding",
        )
    ) and all(
        marker not in production_source(contract)
        for marker in ("native_path", "PathBuf", "std::path::Path")
    )
    owner_bound_reads = all(
        marker in engine
        for marker in (
            "authorize_runtime_artifact_read",
            "&manifest.session_id != session_id",
            "&manifest.task_id != task_id",
            "manifest.policy_sha256 != policy_sha256",
            "RuntimeArtifactLifecycleState::Active",
            "RuntimeArtifactIntegrityState::Verified",
            "request.maximum_bytes",
            "MAX_RUNTIME_ARTIFACT_PREVIEW_BYTES",
        )
    )
    transactional_lifecycle = all(
        marker in engine + migration
        for marker in (
            "runtime_artifacts",
            "runtime_artifact_events",
            "runtime_resume_artifacts",
            "TransactionBehavior::Immediate",
            "current_checkpoint_references",
            "active_reference_count",
        )
    )
    descriptor_safe_linux = all(
        marker in linux
        for marker in (
            "OFlags::NOFOLLOW",
            "OFlags::EXCL",
            "RenameFlags::NOREPLACE",
            "same_object",
            "stat.st_dev != directory_stat.st_dev",
            "stat.st_mode & 0o777 != 0o600",
            "Mode::from_raw_mode(0o700)",
        )
    )
    encrypted_integrity = all(
        marker in architecture + linux
        for marker in (
            "XChaCha20-Poly1305",
            "HKDF-SHA-256",
            "staging_and_objects_are_encrypted_randomized_and_key_bound",
            "corruption_quarantine_and_verified_delete_leave_no_active_object",
        )
    )
    production_closure = "\n".join(
        production_source(source) for source in (contract, engine, linux)
    )
    memory_and_network_safe = (
        "unsafe {" not in production_closure
        and all(token not in production_closure for token in PROHIBITED_NETWORK_TOKENS)
    )
    namespace_separation = all(
        marker in linux + architecture
        for marker in (
            "repository_evidence_namespace_is_never_private_artifact_authority",
            ".agentmage-runtime-payloads-v1",
            "never searched, opened, collected",
        )
    )
    crash_valid = evidence_report_valid(
        crash, git_blob(revision, EVIDENCE_PATHS[1]),
        "pass-current-linux-native-source-boundary",
    ) and crash.get("metrics", {}).get("case_count") == 14
    integrity_valid = evidence_report_valid(
        integrity, git_blob(revision, EVIDENCE_PATHS[3]),
        "pass-current-linux-source-boundary",
    ) and len(integrity.get("coverage", {})) == 9
    pressure_metrics = pressure.get("metrics", {})
    pressure_valid = evidence_report_valid(
        pressure, git_blob(revision, EVIDENCE_PATHS[5]),
        "pass-current-linux-native-reference-host",
    ) and all(
        pressure_metrics.get(key) == expected
        for key, expected in (
            ("maximum_payload_bytes", 64 * 1024 * 1024),
            ("checkpoint_reference_count", 1024),
            ("overflow_reference_count_rejected", 1025),
            ("mixed_object_count", 64),
            ("final_active_object_count", 0),
        )
    )
    resume_metrics = resume.get("metrics", {})
    resume_valid = evidence_report_valid(
        resume, git_blob(revision, EVIDENCE_PATHS[7]),
        "partial-pass-current-linux-native-source-boundary",
    ) and bool(
        resume_metrics.get("exact-checkpoint-resume", {}).get("artifact_set") == "exact"
        and resume_metrics.get("large-command-test-resume", {}).get(
            "exact_artifact_set_restored"
        ) is True
        and resume_metrics.get("large-model-terminal-restart", {}).get(
            "payload_recovered_exactly"
        ) is True
    )
    index_summary = index.get("summary", {})
    index_truthful = bool(
        index.get("disposition") == "partial-local-evidence"
        and index_summary.get("story_complete") is False
        and index_summary.get("sprint_complete") is False
        and index_summary.get("release_approved") is False
        and SHA256.fullmatch(str(index.get("index_sha256", ""))) is not None
    )
    limitations = (architecture + "\n" + tasks).lower()
    limitation_truth = all(
        phrase in limitations
        for phrase in (
            "individual filesystem or sqlite syscall",
            "independent",
            "manual fuzzing",
            "concurrent collection",
            "windows native artifact-store",
        )
    )
    return [
        check_record("closed-path-free-schemas", closed_path_free_schemas, "All four public schemas are closed and contain no path authority."),
        check_record("path-free-contract", path_free_contract, "Artifact references, manifests, operator views, and resume bindings expose no native path type."),
        check_record("owner-bound-bounded-reads", owner_bound_reads, "Reads bind exact owner, policy, state, integrity, and byte ceilings."),
        check_record("transactional-reference-lifecycle", transactional_lifecycle, "Metadata, lifecycle, checkpoint references, and active counts remain transactional."),
        check_record("descriptor-safe-linux-store", descriptor_safe_linux, "Linux storage uses held descriptors, no-follow access, no-replace placement, exact modes, and identity checks."),
        check_record("encrypted-integrity-boundary", encrypted_integrity, "Versioned encryption, domain separation, authentication, quarantine, and deletion tests remain named."),
        check_record("memory-safe-network-free-closure", memory_and_network_safe, "Production artifact closure contains no unsafe block or network client token."),
        check_record("repository-evidence-separation", namespace_separation, "Repository evidence is not private runtime-artifact authority."),
        check_record("crash-report-integrity", crash_valid, "Fourteen declared native stop positions and their raw trace reconcile."),
        check_record("integrity-report-closure", integrity_valid, "Nine integrity and hostile-boundary groups and their raw trace reconcile."),
        check_record("pressure-report-integrity", pressure_valid, "Payload, reference, mixed-object, rejection, and cleanup ceilings reconcile."),
        check_record("resume-report-integrity", resume_valid, "Exact checkpoint, command/test, and terminal model artifacts recover without replay overclaim."),
        check_record("evidence-index-truth", index_truthful, "The source-bound index denies story, sprint, and release completion."),
        check_record("limitation-truth", limitation_truth, "Physical, concurrency, platform, independent-review, and fuzzing gaps remain visible."),
    ]


def build_report(revision: str) -> dict[str, Any]:
    checks = review_checks(revision)
    return {
        "schema_version": 1,
        "artifact_id": "story-22.2-runtime-artifact-boundary-review",
        "source_revision": revision,
        "status": (
            "pass-independent-automated-source-boundary-review"
            if all(check["passed"] for check in checks)
            else "fail-independent-automated-source-boundary-review"
        ),
        "review_type": "independent-automated-boundary-review",
        "checks": checks,
        "sources": source_records(revision),
        "external_network_used": False,
        "private_user_data_used": False,
        "independent_human_review_performed": False,
        "independent_cryptographic_review_performed": False,
        "manual_fuzzing_executed": False,
        "release_approved": False,
        "limitations": list(LIMITATIONS),
        "report_sha256": ZERO_SHA256,
    }


def seal_report(report: dict[str, Any]) -> dict[str, Any]:
    sealed = json.loads(json.dumps(report))
    sealed["report_sha256"] = ZERO_SHA256
    sealed["report_sha256"] = sha256_bytes(canonical_json_bytes(sealed))
    return sealed


def validate_report(report: Any) -> list[str]:
    if not isinstance(report, dict):
        return ["runtime.artifact_review.report_type"]
    revision = report.get("source_revision")
    if not isinstance(revision, str) or REVISION.fullmatch(revision) is None:
        return ["runtime.artifact_review.report_revision"]
    try:
        expected = seal_report(build_report(revision))
    except ArtifactReviewError:
        return ["runtime.artifact_review.source_unavailable"]
    failures = [] if report == expected else ["runtime.artifact_review.report_drift"]
    if report.get("status") != "pass-independent-automated-source-boundary-review":
        failures.append("runtime.artifact_review.failed")
    if any(check.get("passed") is not True for check in report.get("checks", [])):
        failures.append("runtime.artifact_review.check_failed")
    for field in (
        "external_network_used",
        "private_user_data_used",
        "independent_human_review_performed",
        "independent_cryptographic_review_performed",
        "manual_fuzzing_executed",
        "release_approved",
    ):
        if report.get(field) is not False:
            failures.append(f"runtime.artifact_review.overclaim.{field}")
    return failures


def read_report() -> Any:
    try:
        return json.loads(REPORT_PATH.read_text(encoding="utf-8"))
    except (OSError, UnicodeError, json.JSONDecodeError) as error:
        raise ArtifactReviewError("runtime.artifact_review.report_unavailable") from error


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--write", action="store_true")
    parser.add_argument("--source-revision", default="HEAD")
    arguments = parser.parse_args()
    if arguments.write:
        revision = git_revision(arguments.source_revision)
        report = seal_report(build_report(revision))
        if report["status"] != "pass-independent-automated-source-boundary-review":
            raise ArtifactReviewError("runtime.artifact_review.check_failed")
        atomic_write(REPORT_PATH, canonical_json_bytes(report))
    failures = validate_report(read_report())
    if failures:
        raise ArtifactReviewError("; ".join(failures))
    print("Story 22.2 independent automated artifact review: pass")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
