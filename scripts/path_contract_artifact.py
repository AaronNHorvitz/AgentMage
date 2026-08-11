#!/usr/bin/env python3
"""Build and verify the Story 6.1 path-contract reference artifact."""

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


ROOT = Path(__file__).resolve().parents[1]
REPORT_PATH = ROOT / "artifacts/sprints/sprint-6/story-6.1/path-contract-report.json"
SOURCE_PATHS = (
    "docs/architecture/path-authority-contract.md",
    "kernel/contracts/src/path.rs",
    "kernel/contracts/src/platform_path.rs",
    "kernel/contracts/src/display_link.rs",
    "kernel/contracts/src/grant.rs",
    "platforms/linux/src/lib.rs",
    "scripts/path_contract_artifact.py",
    "tests/test_path_contract_artifact.py",
)
PATH_INTENTS = ("Metadata", "ReadFile", "ReadDirectory", "ContentHash")
PLATFORMS = ("DeterministicFake", "Linux", "MacOs")
ADAPTER_ERRORS = (
    "WorkspaceMismatch",
    "ForeignHandle",
    "StaleAuthorization",
    "UnsupportedPrimitive",
    "UnsafeComponent",
    "SymbolicLink",
    "Alias",
    "HardLink",
    "IdentityChanged",
    "MountChanged",
    "NotFound",
    "ObjectKindMismatch",
    "ResourceLimitExceeded",
    "PermissionDenied",
    "PlatformFailure",
)
STRICT_FLAGS = ("BENEATH", "NO_SYMLINKS", "NO_MAGICLINKS", "NO_XDEV")
REQUIRED_HEADINGS = (
    "Status and Scope",
    "Authority Types",
    "Resolution Interface",
    "Linux Enforcement",
    "Display-Only Links",
    "Platform Evidence",
    "Review Checklist",
    "Limitations",
)


class PathContractArtifactError(ValueError):
    """Raised when path-contract evidence is stale, malformed, or overclaimed."""


def canonical_json(value: Any) -> bytes:
    return (json.dumps(value, indent=2, sort_keys=True) + "\n").encode("utf-8")


def sha256_bytes(value: bytes) -> str:
    return hashlib.sha256(value).hexdigest()


def write_atomic(path: Path, content: bytes) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    descriptor, name = tempfile.mkstemp(prefix=".agentmage-path-contract-", dir=path.parent)
    temporary = Path(name)
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


def enum_variants(source: str, name: str) -> tuple[str, ...]:
    match = re.search(rf"pub enum {re.escape(name)}\s*\{{(.*?)\n\}}", source, re.DOTALL)
    if match is None:
        raise PathContractArtifactError(f"missing Rust enum: {name}")
    return tuple(
        re.findall(
            r"^\s*([A-Z][A-Za-z0-9_]*)\s*(?:\([^\n]*\)|\{[^\n]*\})?,?\s*$",
            match.group(1),
            re.MULTILINE,
        )
    )


def trait_methods(source: str, name: str) -> tuple[str, ...]:
    match = re.search(rf"pub trait {re.escape(name)}[^\{{]*\{{(.*?)\n\}}", source, re.DOTALL)
    if match is None:
        raise PathContractArtifactError(f"missing Rust trait: {name}")
    return tuple(re.findall(r"^\s*fn\s+([a-z][a-z0-9_]*)\s*\(", match.group(1), re.MULTILINE))


def validate_sources(root: Path = ROOT) -> dict[str, Any]:
    path_source = (root / "kernel/contracts/src/path.rs").read_text(encoding="utf-8")
    adapter_source = (root / "kernel/contracts/src/platform_path.rs").read_text(encoding="utf-8")
    display_source = (root / "kernel/contracts/src/display_link.rs").read_text(encoding="utf-8")
    linux_source = (root / "platforms/linux/src/lib.rs").read_text(encoding="utf-8")
    document = (root / "docs/architecture/path-authority-contract.md").read_text(encoding="utf-8")

    failures: list[str] = []
    for heading in REQUIRED_HEADINGS:
        if f"## {heading}" not in document:
            failures.append(f"missing reference heading: {heading}")
    for token in (
        "`WorkspacePath`",
        "`AuthorizedWorkspaceHandle`",
        "`HeldWorkspaceObject`",
        "`PlatformPathAdapter`",
        "`DisplayFileLink`",
        "`blocked-macos`",
        "`not-executed`",
    ):
        if token not in document:
            failures.append(f"missing reference token: {token}")
    for prohibited in (
        "macOS verification passed",
        "Ubuntu verification passed",
        "product release is approved",
    ):
        if prohibited in document:
            failures.append(f"unsupported documentation claim: {prohibited}")

    actual_platforms = enum_variants(adapter_source, "PathPlatform")
    actual_intents = enum_variants(adapter_source, "PathResolutionIntent")
    actual_errors = enum_variants(adapter_source, "PathAdapterErrorKind")
    handle_methods = trait_methods(adapter_source, "AuthorizedWorkspaceHandle")
    held_methods = trait_methods(adapter_source, "HeldWorkspaceObject")
    adapter_methods = trait_methods(adapter_source, "PlatformPathAdapter")
    for label, actual, expected in (
        ("path platforms", actual_platforms, PLATFORMS),
        ("resolution intents", actual_intents, PATH_INTENTS),
        ("adapter error classes", actual_errors, ADAPTER_ERRORS),
        (
            "authorized handle methods",
            handle_methods,
            ("workspace_id", "authorization_id", "adapter_instance_id", "platform"),
        ),
        (
            "held object methods",
            held_methods,
            (
                "workspace_path",
                "authorization_id",
                "adapter_instance_id",
                "intent",
                "object_kind",
                "object_identity",
                "preimage",
            ),
        ),
        ("adapter methods", adapter_methods, ("adapter_instance_id", "platform", "resolve")),
    ):
        if actual != expected:
            failures.append(f"{label} changed: expected {expected!r}, got {actual!r}")

    if "path: &WorkspacePath" not in adapter_source:
        failures.append("adapter resolve input is not WorkspacePath")
    if "impl<'de> Deserialize<'de> for WorkspacePath" not in path_source:
        failures.append("WorkspacePath custom deserializer is absent")
    if "impl<'de> Deserialize<'de> for DisplayFileLink" in display_source:
        failures.append("DisplayFileLink unexpectedly implements Deserialize")
    if re.search(r"impl\s+(?:From|TryFrom)<DisplayFileLink>", display_source):
        failures.append("DisplayFileLink unexpectedly converts to authority")
    if "pub fn render_target" not in display_source or "pub fn file_uri" not in display_source:
        failures.append("explicit display rendering interface is absent")
    for flag in STRICT_FLAGS:
        if f"ResolveFlags::{flag}" not in linux_source:
            failures.append(f"strict openat2 flag is absent: {flag}")
    if "resolver_preference: ResolverPreference::Auto" not in linux_source:
        failures.append("public Linux constructor does not select automatic resolution")
    if "pub enum ResolverPreference" in linux_source or "pub fn fallback_adapter" in linux_source:
        failures.append("Linux fallback selection became public")
    if "impl LinuxAuthorizedWorkspace" in linux_source:
        failures.append("ambient Linux workspace authorization constructor became public")
    if "PathAdapterErrorKind::UnsupportedPrimitive" not in linux_source:
        failures.append("Linux strict-resolution failure is absent")
    if failures:
        raise PathContractArtifactError("; ".join(failures))

    return {
        "adapter_error_class_count": len(actual_errors),
        "adapter_method_count": len(adapter_methods),
        "authorized_handle_method_count": len(handle_methods),
        "held_object_method_count": len(held_methods),
        "path_platform_count": len(actual_platforms),
        "resolution_intent_count": len(actual_intents),
        "strict_openat2_flag_count": len(STRICT_FLAGS),
    }


def git_revision(candidate: str = "HEAD", root: Path = ROOT) -> str:
    completed = subprocess.run(
        ["git", "rev-parse", "--verify", f"{candidate}^{{commit}}"], cwd=root, check=False,
        stdout=subprocess.PIPE, stderr=subprocess.DEVNULL, timeout=10,
    )
    revision = completed.stdout.decode("ascii", "strict").strip()
    if completed.returncode != 0 or re.fullmatch(r"[0-9a-f]{40}", revision) is None:
        raise PathContractArtifactError("source revision is unavailable")
    return revision


def git_file(revision: str, relative: str, root: Path = ROOT) -> bytes:
    completed = subprocess.run(
        ["git", "show", f"{revision}:{relative}"], cwd=root, check=False,
        stdout=subprocess.PIPE, stderr=subprocess.DEVNULL, timeout=10,
    )
    if completed.returncode != 0:
        raise PathContractArtifactError(f"source is absent at revision: {relative}")
    return completed.stdout


def build_report(reference_revision: str, root: Path = ROOT) -> dict[str, Any]:
    coverage = validate_sources(root)
    ancestor = subprocess.run(
        ["git", "merge-base", "--is-ancestor", reference_revision, "HEAD"],
        cwd=root, check=False, stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL, timeout=10,
    )
    if ancestor.returncode != 0:
        raise PathContractArtifactError("reference revision is not an ancestor of HEAD")
    sources = []
    for relative in SOURCE_PATHS:
        committed = git_file(reference_revision, relative, root)
        if committed != (root / relative).read_bytes():
            raise PathContractArtifactError(f"source differs from reference revision: {relative}")
        sources.append({"path": relative, "sha256": sha256_bytes(committed)})
    return {
        "schema_version": 1,
        "task_id": "6.1.2.1",
        "artifact_id": "path-contract-platform-adapter-reference",
        "status": "pass-shared-fedora-scope",
        "reference_revision": reference_revision,
        "sources": sources,
        "coverage": coverage,
        "verification": {
            "authority_type_closure": "pass",
            "content_free_error_closure": "pass",
            "display_link_one_way_boundary": "pass",
            "held_descriptor_interface": "pass",
            "linux_strict_resolution_flags": "pass",
            "public_fallback_absence": "pass",
            "platform_claim_disclosure": "pass",
        },
        "platform_status": {
            "fedora": "verified-local",
            "ubuntu": "not-executed",
            "macos": "blocked-macos",
        },
        "macos_evidence_substituted": False,
        "release_claim": "none",
        "limitations": [
            "public workspace authorization is not implemented",
            "Ubuntu execution is not performed",
            "macOS path implementation and execution are blocked",
            "privileged bind-mount attack execution is not performed",
        ],
    }


def validate_report(value: Any) -> list[str]:
    failures: list[str] = []
    if not isinstance(value, dict):
        return ["path-contract report must be an object"]
    if (
        value.get("schema_version") != 1
        or value.get("task_id") != "6.1.2.1"
        or value.get("artifact_id") != "path-contract-platform-adapter-reference"
        or value.get("status") != "pass-shared-fedora-scope"
    ):
        failures.append("path-contract report identity changed")
    if re.fullmatch(r"[0-9a-f]{40}", str(value.get("reference_revision"))) is None:
        failures.append("path-contract reference revision is not immutable")
    expected_coverage = {
        "adapter_error_class_count": 15,
        "adapter_method_count": 3,
        "authorized_handle_method_count": 4,
        "held_object_method_count": 7,
        "path_platform_count": 3,
        "resolution_intent_count": 4,
        "strict_openat2_flag_count": 4,
    }
    if value.get("coverage") != expected_coverage:
        failures.append("path-contract coverage changed")
    verification = value.get("verification")
    if not isinstance(verification, dict) or len(verification) != 7 or set(verification.values()) != {"pass"}:
        failures.append("path-contract verification is incomplete")
    if value.get("platform_status") != {
        "fedora": "verified-local", "ubuntu": "not-executed", "macos": "blocked-macos"
    }:
        failures.append("platform evidence status changed")
    if value.get("macos_evidence_substituted") is not False or value.get("release_claim") != "none":
        failures.append("path-contract report made an unsupported claim")
    limitations = value.get("limitations")
    if not isinstance(limitations, list) or len(limitations) != 4:
        failures.append("path-contract limitations are incomplete")
    return failures


def write_report(reference_revision: str, root: Path = ROOT) -> None:
    write_atomic(REPORT_PATH, canonical_json(build_report(reference_revision, root)))


def check_report(root: Path = ROOT) -> None:
    try:
        actual = json.loads(REPORT_PATH.read_text(encoding="utf-8"))
        reference_revision = actual["reference_revision"]
    except (OSError, KeyError, TypeError, json.JSONDecodeError) as error:
        raise PathContractArtifactError(f"cannot read path-contract report: {error}") from error
    failures = validate_report(actual)
    if not isinstance(reference_revision, str) or actual != build_report(reference_revision, root):
        failures.append("path-contract report is stale or malformed")
    if failures:
        raise PathContractArtifactError("; ".join(failures))


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--write", action="store_true")
    parser.add_argument("--source-revision", default=None)
    args = parser.parse_args()
    try:
        if args.write:
            write_report(git_revision(args.source_revision or "HEAD"))
        check_report()
    except (OSError, UnicodeError, PathContractArtifactError, subprocess.SubprocessError) as error:
        print(f"Path-contract artifact failed: {error}", file=sys.stderr)
        return 1
    print("Story 6.1 path-contract artifact validated")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
