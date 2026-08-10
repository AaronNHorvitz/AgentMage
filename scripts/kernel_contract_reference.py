#!/usr/bin/env python3
"""Build and verify the immutable Story 4.1 kernel-contract reference report."""

from __future__ import annotations

import argparse
import hashlib
import json
import os
import re
import subprocess
import sys
import tarfile
import tempfile
from pathlib import Path
from typing import Any

try:
    from scripts.kernel_contract_package import (
        ARCHIVE_PATH,
        PREFIX,
        git_file,
        load_archive,
    )
except ModuleNotFoundError:
    from kernel_contract_package import ARCHIVE_PATH, PREFIX, git_file, load_archive


ROOT = Path(__file__).resolve().parents[1]
DOC_PATH = ROOT / "docs/architecture/kernel-contract-reference.md"
REPORT_PATH = (
    ROOT
    / "artifacts/sprints/sprint-4/story-4.1/kernel-contract-reference-report.json"
)
REFERENCE_SOURCE_PATHS = (
    "docs/architecture/kernel-contract-reference.md",
    "scripts/kernel_contract_reference.py",
    "tests/test_kernel_contract_reference.py",
)
REQUIRED_HEADINGS = (
    "Authority Boundary",
    "Wire Rules",
    "Parse Failures",
    "Contract Families",
    "Versioned Top-Level Types",
    "Compatibility",
    "Client Checklist",
    "Public Symbol Index",
    "Scope Limits",
)


class ReferenceValidationError(ValueError):
    """Raised when the contract reference is incomplete or overclaims evidence."""


def sha256_bytes(value: bytes) -> str:
    return hashlib.sha256(value).hexdigest()


def canonical_json(value: Any) -> bytes:
    return (json.dumps(value, indent=2, sort_keys=True) + "\n").encode("utf-8")


def write_atomic(path: Path, content: bytes) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    descriptor, temporary_name = tempfile.mkstemp(prefix=".agentmage-reference-", dir=path.parent)
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


def exported_symbols(lib_source: str) -> tuple[str, ...]:
    symbols: set[str] = set()
    for block in re.findall(r"pub use [^;]+?\{(.*?)\};", lib_source, re.DOTALL):
        symbols.update(item.strip() for item in block.replace("\n", " ").split(",") if item.strip())
    symbols.update(re.findall(r"pub const ([A-Z][A-Z0-9_]+)", lib_source))
    return tuple(sorted(symbols))


def versioned_contracts(serialization_source: str) -> tuple[str, ...]:
    match = re.search(
        r"impl_versioned_contract!\((.*?)\);", serialization_source, re.DOTALL
    )
    if match is None:
        raise ReferenceValidationError("versioned-contract declaration is absent")
    return tuple(
        sorted(
            item.strip().removeprefix("crate::")
            for item in match.group(1).split(",")
            if item.strip()
        )
    )


def boundary_error_codes(serialization_source: str) -> tuple[str, ...]:
    return tuple(sorted(set(re.findall(r'"(contract\.[a-z_.]+)"', serialization_source))))


def validate_reference_text(
    text: str,
    exports: tuple[str, ...],
    versioned: tuple[str, ...],
    error_codes: tuple[str, ...],
) -> list[str]:
    failures = []
    for heading in REQUIRED_HEADINGS:
        if f"## {heading}" not in text:
            failures.append(f"missing reference heading: {heading}")
    for symbol in exports:
        if f"`{symbol}`" not in text:
            failures.append(f"missing public symbol: {symbol}")
    for contract in versioned:
        if f"`{contract}`" not in text:
            failures.append(f"missing versioned contract: {contract}")
    for code in error_codes:
        if f"`{code}`" not in text:
            failures.append(f"missing boundary error code: {code}")
    for prohibited in (
        "macOS verification has passed",
        "published crates.io package",
        "CapabilityGrant is implemented",
        "production tool execution",
    ):
        if prohibited in text:
            failures.append(f"unsupported reference claim: {prohibited}")
    return failures


def package_reference_inputs() -> tuple[
    dict[str, tuple[bytes, int]], tuple[str, ...], tuple[str, ...], tuple[str, ...]
]:
    files = load_archive(ARCHIVE_PATH)
    lib_source = files["src/lib.rs"][0].decode("utf-8")
    serialization_source = files["src/serialization.rs"][0].decode("utf-8")
    exports = exported_symbols(lib_source)
    versioned = versioned_contracts(serialization_source)
    error_codes = boundary_error_codes(serialization_source)
    if len(exports) != 65 or len(versioned) != 13 or len(error_codes) != 11:
        raise ReferenceValidationError("frozen package API counts are unexpected")
    return files, exports, versioned, error_codes


def verify_package_rustdoc(files: dict[str, tuple[bytes, int]]) -> None:
    with tempfile.TemporaryDirectory(prefix="agentmage-contract-reference-") as temporary_name:
        package_root = Path(temporary_name) / PREFIX
        for relative, (content, mode) in files.items():
            destination = package_root / relative
            destination.parent.mkdir(parents=True, exist_ok=True)
            destination.write_bytes(content)
            destination.chmod(mode & 0o666)
        completed = subprocess.run(
            ["cargo", "doc", "--locked", "--offline", "--no-deps"],
            cwd=package_root,
            check=False,
            stdin=subprocess.DEVNULL,
            stdout=subprocess.PIPE,
            stderr=subprocess.STDOUT,
            timeout=120,
            env={**os.environ, "RUSTDOCFLAGS": "-Dwarnings"},
        )
        if completed.returncode != 0:
            raise ReferenceValidationError("frozen package Rustdoc verification failed")


def git_revision(root: Path = ROOT) -> str:
    completed = subprocess.run(
        ["git", "rev-parse", "HEAD"],
        cwd=root,
        check=False,
        stdout=subprocess.PIPE,
        stderr=subprocess.DEVNULL,
        timeout=10,
    )
    revision = completed.stdout.decode("ascii", "strict").strip()
    if completed.returncode != 0 or re.fullmatch(r"[0-9a-f]{40}", revision) is None:
        raise ReferenceValidationError("reference source revision is unavailable")
    return revision


def build_report(reference_revision: str, root: Path = ROOT) -> dict[str, Any]:
    files, exports, versioned, error_codes = package_reference_inputs()
    ancestor = subprocess.run(
        ["git", "merge-base", "--is-ancestor", reference_revision, "HEAD"],
        cwd=root,
        check=False,
        stdout=subprocess.DEVNULL,
        stderr=subprocess.DEVNULL,
        timeout=10,
    )
    if ancestor.returncode != 0:
        raise ReferenceValidationError("reference revision is not an ancestor of HEAD")
    source_records = []
    for relative in REFERENCE_SOURCE_PATHS:
        committed = git_file(reference_revision, relative, root)
        current = (root / relative).read_bytes()
        if committed != current:
            raise ReferenceValidationError(f"reference source differs from its revision: {relative}")
        source_records.append({"path": relative, "sha256": sha256_bytes(committed)})
    text = (root / REFERENCE_SOURCE_PATHS[0]).read_text(encoding="utf-8")
    failures = validate_reference_text(text, exports, versioned, error_codes)
    if failures:
        raise ReferenceValidationError("; ".join(failures))
    verify_package_rustdoc(files)
    package_report = json.loads(
        (
            root
            / "artifacts/sprints/sprint-4/story-4.1/kernel-contract-package-report.json"
        ).read_text(encoding="utf-8")
    )
    return {
        "schema_version": 1,
        "task_id": "4.1.2.2",
        "artifact_id": "kernel-contract-reference-documentation",
        "status": "pass-linux-reference",
        "reference_revision": reference_revision,
        "reference_sources": source_records,
        "frozen_package": {
            "path": package_report["package"]["path"],
            "sha256": package_report["package"]["sha256"],
            "source_revision": package_report["package"]["source_revision"],
            "cargo_version": package_report["package"]["cargo_version"],
            "contract_schema_version": package_report["package"][
                "contract_schema_version"
            ],
        },
        "coverage": {
            "public_export_count": len(exports),
            "public_exports": list(exports),
            "versioned_contract_count": len(versioned),
            "versioned_contracts": list(versioned),
            "boundary_error_code_count": len(error_codes),
            "boundary_error_codes": list(error_codes),
            "required_heading_count": len(REQUIRED_HEADINGS),
        },
        "verification": {
            "markdown_policy_validation": "pass",
            "symbol_coverage": "pass",
            "versioned_contract_coverage": "pass",
            "error_code_coverage": "pass",
            "frozen_package_rustdoc": "pass",
            "rustdoc_command": "cargo doc --locked --offline --no-deps",
        },
        "platform_status": {
            "linux_reference": "verified-local",
            "macos_reference": "blocked-macos",
            "macos_implementation_claim": "none",
        },
        "limitations": [
            "The reference describes frozen wire schema version 1, not a product release.",
            "Golden serialization and compatibility fixtures remain Task 4.1.2.3.",
            "No macOS build or execution evidence is claimed.",
            "No CapabilityGrant or positive execution path exists in this package.",
        ],
    }


def write_report(root: Path = ROOT) -> None:
    revision = git_revision(root)
    write_atomic(REPORT_PATH, canonical_json(build_report(revision, root)))


def check_report(root: Path = ROOT) -> None:
    try:
        actual = json.loads(REPORT_PATH.read_text(encoding="utf-8"))
        revision = actual["reference_revision"]
    except (OSError, KeyError, TypeError, json.JSONDecodeError) as error:
        raise ReferenceValidationError(f"cannot read reference report: {error}") from error
    if not isinstance(revision, str):
        raise ReferenceValidationError("reference report revision is invalid")
    if actual != build_report(revision, root):
        raise ReferenceValidationError("kernel-contract reference report is stale or malformed")


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--write", action="store_true")
    args = parser.parse_args()
    try:
        if args.write:
            write_report()
        check_report()
    except (OSError, ReferenceValidationError, subprocess.SubprocessError) as error:
        print(f"Kernel-contract reference validation failed: {error}", file=sys.stderr)
        return 1
    print("Kernel-contract reference documentation validated")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
