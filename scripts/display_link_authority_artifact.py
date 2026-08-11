#!/usr/bin/env python3
"""Generate and verify display-link replay denials across path authority APIs."""

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
CORPUS_PATH = ROOT / "fixtures/paths/v1/display-link-corpus.json"
REPORT_PATH = ROOT / "artifacts/sprints/sprint-6/story-6.1/display-link-authority-report.json"
LINK_COUNT = 128
AUTHORITY_SURFACES = (
    "workspace_path_constructor",
    "workspace_path_deserializer",
    "grant_issuer",
    "policy_builder",
    "platform_adapter_typed_input",
)
SOURCE_PATHS = (
    "fixtures/paths/README.md",
    "fixtures/paths/v1/display-link-corpus.json",
    "kernel/contracts/src/display_link.rs",
    "kernel/contracts/src/path.rs",
    "kernel/contracts/src/platform_path.rs",
    "kernel/engine/src/grants.rs",
    "kernel/engine/src/policy.rs",
    "kernel/engine/tests/display_link_authority.rs",
    "scripts/display_link_authority_artifact.py",
    "tests/test_display_link_authority_artifact.py",
)


class DisplayLinkAuthorityArtifactError(ValueError):
    """Raised when display-link authority evidence is stale or overclaimed."""


def canonical_json(value: Any) -> bytes:
    return (json.dumps(value, sort_keys=True, separators=(",", ":"), ensure_ascii=True) + "\n").encode()


def pretty_json(value: Any) -> bytes:
    return (json.dumps(value, indent=2, sort_keys=True, ensure_ascii=True) + "\n").encode()


def sha256_bytes(value: bytes) -> str:
    return hashlib.sha256(value).hexdigest()


def write_atomic(path: Path, content: bytes) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    descriptor, name = tempfile.mkstemp(prefix=".agentmage-display-authority-", dir=path.parent)
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


def generated_corpus() -> bytes:
    links = []
    for index in range(LINK_COUNT):
        line = index + 1
        file_uri = f"file:///synthetic/workspace/Note%20{index:03d}%23%25.rs"
        links.append({
            "schema_version": 1,
            "case_id": f"display-link-{index:03d}",
            "file_uri": file_uri,
            "line": line,
            "rendered_target": f"{file_uri}#L{line}",
        })
    return canonical_json({
        "schema_version": 1,
        "corpus_id": "agentmage-display-link-replay-v1",
        "link_count": LINK_COUNT,
        "authority_surface_count": len(AUTHORITY_SURFACES),
        "authority_surfaces": list(AUTHORITY_SURFACES),
        "links": links,
    })


def validate_corpus(content: bytes) -> dict[str, Any]:
    try:
        corpus = json.loads(content)
    except (UnicodeDecodeError, json.JSONDecodeError) as error:
        raise DisplayLinkAuthorityArtifactError("display-link corpus is malformed") from error
    if not isinstance(corpus, dict) or set(corpus) != {
        "schema_version", "corpus_id", "link_count", "authority_surface_count",
        "authority_surfaces", "links",
    }:
        raise DisplayLinkAuthorityArtifactError("display-link corpus envelope is invalid")
    links = corpus["links"]
    if (
        corpus["schema_version"] != 1
        or corpus["corpus_id"] != "agentmage-display-link-replay-v1"
        or corpus["link_count"] != LINK_COUNT
        or corpus["authority_surface_count"] != len(AUTHORITY_SURFACES)
        or tuple(corpus["authority_surfaces"]) != AUTHORITY_SURFACES
        or not isinstance(links, list)
        or len(links) != LINK_COUNT
    ):
        raise DisplayLinkAuthorityArtifactError("display-link corpus closure is incomplete")
    identities: set[str] = set()
    for index, link in enumerate(links):
        if not isinstance(link, dict) or set(link) != {
            "schema_version", "case_id", "file_uri", "line", "rendered_target"
        }:
            raise DisplayLinkAuthorityArtifactError("display-link case shape is invalid")
        expected_uri = f"file:///synthetic/workspace/Note%20{index:03d}%23%25.rs"
        if (
            link["schema_version"] != 1
            or link["case_id"] != f"display-link-{index:03d}"
            or link["case_id"] in identities
            or link["file_uri"] != expected_uri
            or link["line"] != index + 1
            or link["rendered_target"] != f"{expected_uri}#L{index + 1}"
        ):
            raise DisplayLinkAuthorityArtifactError("display-link case expectation changed")
        identities.add(link["case_id"])
    return {
        "authority_surface_count": len(AUTHORITY_SURFACES),
        "candidate_form_count": 2,
        "display_link_count": len(links),
        "filesystem_observation_count": 0,
        "issued_grant_count": 0,
        "rejection_count": len(links) * 2 * len(AUTHORITY_SURFACES),
    }


def write_corpus(root: Path = ROOT) -> None:
    write_atomic(root / CORPUS_PATH.relative_to(ROOT), generated_corpus())


def check_corpus(root: Path = ROOT) -> dict[str, Any]:
    actual = (root / CORPUS_PATH.relative_to(ROOT)).read_bytes()
    if actual != generated_corpus():
        raise DisplayLinkAuthorityArtifactError("committed display-link corpus is stale")
    return validate_corpus(actual)


def run_typed_matrix(root: Path = ROOT) -> None:
    completed = subprocess.run(
        ["cargo", "test", "--locked", "--offline", "-p", "agentmage-kernel-engine",
         "--test", "display_link_authority",
         "generated_display_links_are_rejected_by_every_runtime_authority_boundary",
         "--", "--exact", "--test-threads=1"],
        cwd=root, check=False, stdin=subprocess.DEVNULL,
        stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL, timeout=120,
    )
    if completed.returncode != 0:
        raise DisplayLinkAuthorityArtifactError("typed display-link authority matrix failed")


def validate_source_boundary(root: Path = ROOT) -> None:
    adapter = (root / "kernel/contracts/src/platform_path.rs").read_text(encoding="utf-8")
    display = (root / "kernel/contracts/src/display_link.rs").read_text(encoding="utf-8")
    if "path: &WorkspacePath" not in adapter:
        raise DisplayLinkAuthorityArtifactError("adapter authority input is not WorkspacePath")
    if "impl<'de> Deserialize<'de> for DisplayFileLink" in display:
        raise DisplayLinkAuthorityArtifactError("display link became deserializable")
    if re.search(r"impl\s+(?:From|TryFrom)<DisplayFileLink>", display):
        raise DisplayLinkAuthorityArtifactError("display link converts into authority")


def git_revision(candidate: str = "HEAD", root: Path = ROOT) -> str:
    completed = subprocess.run(
        ["git", "rev-parse", "--verify", f"{candidate}^{{commit}}"], cwd=root, check=False,
        stdout=subprocess.PIPE, stderr=subprocess.DEVNULL, timeout=10,
    )
    revision = completed.stdout.decode("ascii", "strict").strip()
    if completed.returncode != 0 or re.fullmatch(r"[0-9a-f]{40}", revision) is None:
        raise DisplayLinkAuthorityArtifactError("source revision is unavailable")
    return revision


def git_file(revision: str, relative: str, root: Path = ROOT) -> bytes:
    completed = subprocess.run(
        ["git", "show", f"{revision}:{relative}"], cwd=root, check=False,
        stdout=subprocess.PIPE, stderr=subprocess.DEVNULL, timeout=10,
    )
    if completed.returncode != 0:
        raise DisplayLinkAuthorityArtifactError(f"source is absent at revision: {relative}")
    return completed.stdout


def build_report(reference_revision: str, root: Path = ROOT) -> dict[str, Any]:
    coverage = check_corpus(root)
    validate_source_boundary(root)
    run_typed_matrix(root)
    ancestor = subprocess.run(
        ["git", "merge-base", "--is-ancestor", reference_revision, "HEAD"],
        cwd=root, check=False, stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL, timeout=10,
    )
    if ancestor.returncode != 0:
        raise DisplayLinkAuthorityArtifactError("reference revision is not an ancestor of HEAD")
    sources = []
    for relative in SOURCE_PATHS:
        committed = git_file(reference_revision, relative, root)
        if committed != (root / relative).read_bytes():
            raise DisplayLinkAuthorityArtifactError(f"source differs from reference revision: {relative}")
        sources.append({"path": relative, "sha256": sha256_bytes(committed)})
    return {
        "schema_version": 1,
        "task_ids": ["6.1.2.2", "6.1.3.3"],
        "artifact_id": "display-link-authority-replay-matrix",
        "status": "pass-shared-kernel-scope",
        "reference_revision": reference_revision,
        "sources": sources,
        "coverage": coverage,
        "verification": {surface: "pass" for surface in AUTHORITY_SURFACES},
        "direct_grant_target_status": "descriptive-candidate-only",
        "macos_execution_status": "blocked-macos",
        "macos_evidence_substituted": False,
        "release_claim": "none",
        "limitations": [
            "Visual Studio Code link activation is not implemented",
            "macOS display-link generation is not executed",
        ],
    }


def validate_report(value: Any) -> list[str]:
    failures: list[str] = []
    if not isinstance(value, dict):
        return ["display-link authority report must be an object"]
    if (
        value.get("schema_version") != 1
        or value.get("task_ids") != ["6.1.2.2", "6.1.3.3"]
        or value.get("artifact_id") != "display-link-authority-replay-matrix"
        or value.get("status") != "pass-shared-kernel-scope"
        or re.fullmatch(r"[0-9a-f]{40}", str(value.get("reference_revision"))) is None
    ):
        failures.append("display-link authority report identity changed")
    if value.get("coverage") != {
        "authority_surface_count": 5,
        "candidate_form_count": 2,
        "display_link_count": 128,
        "filesystem_observation_count": 0,
        "issued_grant_count": 0,
        "rejection_count": 1280,
    }:
        failures.append("display-link authority coverage changed")
    if value.get("verification") != {surface: "pass" for surface in AUTHORITY_SURFACES}:
        failures.append("display-link authority verification is incomplete")
    if (
        value.get("direct_grant_target_status") != "descriptive-candidate-only"
        or value.get("macos_execution_status") != "blocked-macos"
        or value.get("macos_evidence_substituted") is not False
        or value.get("release_claim") != "none"
    ):
        failures.append("display-link authority report made an unsupported claim")
    if not isinstance(value.get("limitations"), list) or len(value["limitations"]) != 2:
        failures.append("display-link authority limitations are incomplete")
    return failures


def write_report(reference_revision: str, root: Path = ROOT) -> None:
    write_atomic(REPORT_PATH, pretty_json(build_report(reference_revision, root)))


def check_report(root: Path = ROOT) -> None:
    try:
        actual = json.loads(REPORT_PATH.read_text(encoding="utf-8"))
        revision = actual["reference_revision"]
    except (OSError, KeyError, TypeError, json.JSONDecodeError) as error:
        raise DisplayLinkAuthorityArtifactError(f"cannot read display-link authority report: {error}") from error
    failures = validate_report(actual)
    if not isinstance(revision, str) or actual != build_report(revision, root):
        failures.append("display-link authority report is stale or malformed")
    if failures:
        raise DisplayLinkAuthorityArtifactError("; ".join(failures))


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--write-corpus", action="store_true")
    parser.add_argument("--write-report", action="store_true")
    parser.add_argument("--corpus-only", action="store_true")
    parser.add_argument("--source-revision", default="HEAD")
    args = parser.parse_args()
    try:
        if args.write_corpus:
            write_corpus()
        check_corpus()
        if not args.corpus_only:
            if args.write_report:
                write_report(git_revision(args.source_revision))
            check_report()
    except (OSError, UnicodeError, DisplayLinkAuthorityArtifactError, subprocess.SubprocessError) as error:
        print(f"Display-link authority artifact failed: {error}", file=sys.stderr)
        return 1
    print("Story 6.1 display-link authority matrix validated")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
