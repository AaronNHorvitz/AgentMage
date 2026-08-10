#!/usr/bin/env python3
"""Build and verify golden wire and compatibility fixtures for contract schema v1."""

from __future__ import annotations

import argparse
import hashlib
import json
import os
import re
import subprocess
import sys
import tempfile
from pathlib import Path
from typing import Any

try:
    from scripts.kernel_contract_package import ARCHIVE_PATH, PREFIX, git_file, load_archive
except ModuleNotFoundError:
    from kernel_contract_package import ARCHIVE_PATH, PREFIX, git_file, load_archive


ROOT = Path(__file__).resolve().parents[1]
FIXTURE_ROOT = ROOT / "fixtures/contracts"
VALID_ROOT = FIXTURE_ROOT / "v1/valid"
COMPATIBILITY_ROOT = FIXTURE_ROOT / "compatibility"
MANIFEST_PATH = FIXTURE_ROOT / "v1/manifest.json"
COMPATIBILITY_PATH = FIXTURE_ROOT / "compatibility.json"
REPORT_PATH = ROOT / "artifacts/sprints/sprint-4/story-4.1/kernel-contract-fixture-report.json"
EMIT_SOURCE = "kernel/contracts/tests/contract_family.rs"
VERIFY_SOURCE = "fixtures/contracts/fixture_verifier.rs"
SOURCE_PATHS = (
    EMIT_SOURCE,
    VERIFY_SOURCE,
    "scripts/kernel_contract_fixtures.py",
    "tests/test_kernel_contract_fixtures.py",
)
EXPECTED_VALID_NAMES = (
    "action",
    "boundary_failure",
    "cancellation_signal",
    "contract_error",
    "evidence_reference",
    "plan",
    "prompt",
    "receipt",
    "task",
    "tool_call",
    "tool_definition",
    "tool_result",
    "work_packet",
)
INVALID_EXPECTATIONS = {
    "task.v0.unsupported.json": "contract.version.unsupported",
    "task.v2.unsupported.json": "contract.version.unsupported",
    "task.v1.missing-field.json": "contract.field.missing",
    "task.v1.unknown-field.json": "contract.field.unknown",
    "task.v1.duplicate-field.json": "contract.field.duplicate",
    "task.v1.malformed.json": "contract.parse.eof",
    "task.v1.trailing-value.json": "contract.parse.syntax",
}
MARKER = "AGENTMAGE_CONTRACT_FIXTURES="


class FixtureValidationError(ValueError):
    """Raised when a fixture or its provenance cannot satisfy the closed corpus."""


def sha256_bytes(value: bytes) -> str:
    return hashlib.sha256(value).hexdigest()


def canonical_json(value: Any) -> bytes:
    return (json.dumps(value, indent=2, sort_keys=True) + "\n").encode("utf-8")


def write_atomic(path: Path, content: bytes) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    descriptor, temporary_name = tempfile.mkstemp(prefix=".agentmage-golden-", dir=path.parent)
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


def run(
    command: list[str], cwd: Path, environment: dict[str, str]
) -> subprocess.CompletedProcess[bytes]:
    return subprocess.run(
        command,
        cwd=cwd,
        check=False,
        stdin=subprocess.DEVNULL,
        stdout=subprocess.PIPE,
        stderr=subprocess.STDOUT,
        timeout=120,
        env=environment,
    )


def unpack_package(destination: Path) -> Path:
    files = load_archive(ARCHIVE_PATH)
    package_root = destination / PREFIX
    for relative, (content, mode) in files.items():
        target = package_root / relative
        target.parent.mkdir(parents=True, exist_ok=True)
        target.write_bytes(content)
        target.chmod(mode & 0o666)
    return package_root


def enable_fixture_verifier(package_root: Path) -> None:
    manifest_path = package_root / "Cargo.toml"
    manifest_path.write_bytes(
        manifest_path.read_bytes()
        + b'\n[[test]]\nname = "fixture_verifier"\npath = "tests/fixture_verifier.rs"\n'
    )


def parse_bundle(output: bytes) -> dict[str, bytes]:
    if len(output) > 2 * 1024 * 1024:
        raise FixtureValidationError("fixture generator output is oversized")
    decoded = output.decode("utf-8", "strict")
    payloads = [line.split(MARKER, 1)[1] for line in decoded.splitlines() if MARKER in line]
    if len(payloads) != 1:
        raise FixtureValidationError("fixture generator emitted an invalid marker count")
    try:
        records = json.loads(payloads[0])
    except json.JSONDecodeError as error:
        raise FixtureValidationError("fixture bundle is malformed") from error
    if not isinstance(records, list):
        raise FixtureValidationError("fixture bundle must be an array")
    fixtures: dict[str, bytes] = {}
    for record in records:
        if not isinstance(record, dict) or set(record) != {"canonical_json", "name"}:
            raise FixtureValidationError("fixture bundle record shape is invalid")
        name = record["name"]
        canonical = record["canonical_json"]
        if (
            not isinstance(name, str)
            or not isinstance(canonical, str)
            or re.fullmatch(r"[a-z_]+", name) is None
            or name in fixtures
        ):
            raise FixtureValidationError("fixture identity or content is invalid")
        fixtures[name] = canonical.encode("utf-8")
    if tuple(sorted(fixtures)) != EXPECTED_VALID_NAMES:
        raise FixtureValidationError("fixture bundle type closure is incomplete or broadened")
    return fixtures


def generated_valid_fixtures(revision: str) -> dict[str, bytes]:
    with tempfile.TemporaryDirectory(prefix="agentmage-contract-goldens-") as temporary_name:
        package_root = unpack_package(Path(temporary_name))
        (package_root / "tests/contract_family.rs").write_bytes(
            git_file(revision, EMIT_SOURCE)
        )
        environment = {**os.environ, "AGENTMAGE_EMIT_CONTRACT_FIXTURES": "1"}
        completed = run(
            [
                "cargo",
                "test",
                "--locked",
                "--offline",
                "--test",
                "contract_family",
                "complete_contract_family_preserves_linked_identities",
                "--",
                "--exact",
                "--nocapture",
                "--test-threads=1",
            ],
            package_root,
            environment,
        )
        if completed.returncode != 0:
            raise FixtureValidationError("typed fixture generator test failed")
        return parse_bundle(completed.stdout)


def invalid_fixtures(task: bytes) -> dict[str, bytes]:
    try:
        parsed = json.loads(task)
    except json.JSONDecodeError as error:
        raise FixtureValidationError("canonical task fixture is malformed") from error
    if not isinstance(parsed, dict) or parsed.get("schema_version") != 1:
        raise FixtureValidationError("canonical task fixture identity is invalid")

    def compact(value: Any) -> bytes:
        return json.dumps(value, separators=(",", ":"), ensure_ascii=True).encode("utf-8")

    version_zero = {**parsed, "schema_version": 0}
    version_two = {**parsed, "schema_version": 2}
    missing = dict(parsed)
    missing.pop("objective")
    unknown = {**parsed, "capability_grant": {"claimed": True}}
    return {
        "task.v0.unsupported.json": compact(version_zero),
        "task.v2.unsupported.json": compact(version_two),
        "task.v1.missing-field.json": compact(missing),
        "task.v1.unknown-field.json": compact(unknown),
        "task.v1.duplicate-field.json": task[:-1] + b',"status":"ready"}',
        "task.v1.malformed.json": task[:-1],
        "task.v1.trailing-value.json": task + b"[]",
    }


def compatibility_record() -> dict[str, Any]:
    return {
        "schema_version": 1,
        "record_type": "kernel-contract-compatibility",
        "current_wire_version": 1,
        "supported_wire_versions": [1],
        "cargo_package_version": "0.0.0",
        "older_version_disposition": {"version": 0, "status": "rejected"},
        "newer_version_disposition": {"version": 2, "status": "rejected"},
        "unknown_fields": "rejected",
        "duplicate_fields": "rejected",
        "missing_fields": "rejected",
        "trailing_values": "rejected",
        "canonical_bytes": "stable-for-exact-schema-v1-values",
        "automatic_migration": "none",
        "positive_authority_path": "absent",
        "macos_status": "blocked-macos",
    }


def manifest(
    revision: str,
    valid: dict[str, bytes],
    invalid: dict[str, bytes],
    compatibility_bytes: bytes,
) -> dict[str, Any]:
    package_report = json.loads(
        (
            ROOT
            / "artifacts/sprints/sprint-4/story-4.1/kernel-contract-package-report.json"
        ).read_text(encoding="utf-8")
    )
    return {
        "schema_version": 1,
        "fixture_set": "kernel-contracts-v1",
        "generator_revision": revision,
        "frozen_package_sha256": package_report["package"]["sha256"],
        "contract_schema_version": 1,
        "valid_fixtures": [
            {
                "contract": name,
                "path": f"fixtures/contracts/v1/valid/{name}.json",
                "sha256": sha256_bytes(content),
                "size_bytes": len(content),
            }
            for name, content in sorted(valid.items())
        ],
        "invalid_fixtures": [
            {
                "path": f"fixtures/contracts/compatibility/{name}",
                "expected_error_code": INVALID_EXPECTATIONS[name],
                "sha256": sha256_bytes(content),
                "size_bytes": len(content),
            }
            for name, content in sorted(invalid.items())
        ],
        "generated_oversized_case": {
            "size_bytes": 1_048_577,
            "expected_error_code": "contract.size.exceeded",
            "persisted": False,
        },
        "compatibility_record": {
            "path": "fixtures/contracts/compatibility.json",
            "sha256": sha256_bytes(compatibility_bytes),
        },
    }


def verify_with_rust(revision: str, fixture_root: Path) -> None:
    with tempfile.TemporaryDirectory(prefix="agentmage-contract-verifier-") as temporary_name:
        package_root = unpack_package(Path(temporary_name))
        (package_root / "tests/fixture_verifier.rs").write_bytes(
            git_file(revision, VERIFY_SOURCE)
        )
        enable_fixture_verifier(package_root)
        environment = {
            **os.environ,
            "AGENTMAGE_CONTRACT_FIXTURE_ROOT": str(fixture_root),
        }
        completed = run(
            [
                "cargo",
                "test",
                "--locked",
                "--offline",
                "--test",
                "fixture_verifier",
                "--",
                "--test-threads=1",
            ],
            package_root,
            environment,
        )
        if completed.returncode != 0:
            raise FixtureValidationError("Rust golden-corpus verification failed")


def generator_revision() -> str:
    completed = run(
        ["git", "rev-parse", "HEAD"], ROOT, {**os.environ, "LC_ALL": "C"}
    )
    revision = completed.stdout.decode("ascii", "strict").strip()
    if completed.returncode != 0 or re.fullmatch(r"[0-9a-f]{40}", revision) is None:
        raise FixtureValidationError("generator revision is unavailable")
    for relative in SOURCE_PATHS:
        if git_file(revision, relative) != (ROOT / relative).read_bytes():
            raise FixtureValidationError(f"generator source is not committed: {relative}")
    return revision


def expected_outputs(revision: str) -> tuple[dict[str, bytes], dict[str, Any]]:
    valid = generated_valid_fixtures(revision)
    invalid = invalid_fixtures(valid["task"])
    compatibility_bytes = canonical_json(compatibility_record())
    fixture_manifest = manifest(revision, valid, invalid, compatibility_bytes)
    outputs = {
        **{f"v1/valid/{name}.json": content for name, content in valid.items()},
        **{f"compatibility/{name}": content for name, content in invalid.items()},
        "compatibility.json": compatibility_bytes,
        "v1/manifest.json": canonical_json(fixture_manifest),
    }
    source_records = [
        {"path": relative, "sha256": sha256_bytes(git_file(revision, relative))}
        for relative in SOURCE_PATHS
    ]
    report = {
        "schema_version": 1,
        "task_id": "4.1.2.3",
        "artifact_id": "golden-contract-serialization-and-compatibility-fixtures",
        "status": "pass-linux-golden-corpus",
        "generator_revision": revision,
        "generator_sources": source_records,
        "manifest_sha256": sha256_bytes(outputs["v1/manifest.json"]),
        "valid_fixture_count": len(valid),
        "invalid_fixture_count": len(invalid),
        "generated_oversized_case_count": 1,
        "canonical_byte_stability": "pass",
        "public_parser_round_trip": "pass",
        "compatibility_rejection": "pass",
        "network_authority": "none",
        "positive_execution_authority": "none",
        "platform_status": {
            "linux_fixture_verification": "verified-local",
            "macos_fixture_verification": "blocked-macos",
            "macos_implementation_claim": "none",
        },
        "limitations": [
            "Only wire schema version 1 is supported.",
            "Version 0 and version 2 fixtures prove rejection, not migration support.",
            "The oversized case is generated in memory and is not retained as a 1 MiB file.",
            "No macOS execution evidence or positive authority path is claimed.",
        ],
    }
    return outputs, report


def write_outputs() -> None:
    revision = generator_revision()
    outputs, report = expected_outputs(revision)
    for relative, content in outputs.items():
        write_atomic(FIXTURE_ROOT / relative, content)
    verify_with_rust(revision, FIXTURE_ROOT)
    write_atomic(REPORT_PATH, canonical_json(report))


def check_outputs() -> None:
    try:
        actual_report = json.loads(REPORT_PATH.read_text(encoding="utf-8"))
        revision = actual_report["generator_revision"]
    except (OSError, KeyError, TypeError, json.JSONDecodeError) as error:
        raise FixtureValidationError(f"cannot read fixture report: {error}") from error
    if not isinstance(revision, str):
        raise FixtureValidationError("fixture report revision is invalid")
    outputs, expected_report = expected_outputs(revision)
    for relative, expected in outputs.items():
        try:
            actual = (FIXTURE_ROOT / relative).read_bytes()
        except OSError as error:
            raise FixtureValidationError(f"cannot read fixture {relative}: {error}") from error
        if actual != expected:
            raise FixtureValidationError(f"golden fixture is stale or mutated: {relative}")
    expected_paths = set(outputs)
    actual_paths = {
        path.relative_to(FIXTURE_ROOT).as_posix()
        for path in FIXTURE_ROOT.rglob("*")
        if path.is_file() and path.name != "fixture_verifier.rs"
    }
    if actual_paths != expected_paths:
        raise FixtureValidationError("fixture file closure is incomplete or broadened")
    verify_with_rust(revision, FIXTURE_ROOT)
    if actual_report != expected_report:
        raise FixtureValidationError("golden fixture report is stale or malformed")


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--write", action="store_true")
    args = parser.parse_args()
    try:
        if args.write:
            write_outputs()
        check_outputs()
    except (OSError, FixtureValidationError, subprocess.SubprocessError) as error:
        print(f"Kernel-contract fixture validation failed: {error}", file=sys.stderr)
        return 1
    print("Kernel-contract golden fixtures validated")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
