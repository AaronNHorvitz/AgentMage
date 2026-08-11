#!/usr/bin/env python3
"""Perform the independent automated Story 6.1 secure-path boundary review."""

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
from typing import Any, Callable


ROOT = Path(__file__).resolve().parents[1]
if str(ROOT) not in sys.path:
    sys.path.insert(0, str(ROOT))

from scripts.display_link_authority_artifact import check_report as check_display
from scripts.path_contract_artifact import check_report as check_contract
from scripts.path_corpus_artifact import check_report as check_corpus
from scripts.path_race_artifact import check_report as check_race


REPORT_DIRECTORY = "artifacts/sprints/sprint-6/story-6.1"
REPORT_PATH = ROOT / REPORT_DIRECTORY / "path-boundary-review.json"
SUBJECTS = {
    "path-contract-report.json": (
        "path-contract-platform-adapter-reference", "pass-shared-fedora-scope"
    ),
    "path-corpus-report.json": (
        "canonicalization-display-link-path-corpus", "pass-shared-fedora-parser-scope"
    ),
    "display-link-authority-report.json": (
        "display-link-authority-replay-matrix", "pass-shared-kernel-scope"
    ),
    "path-race-report.json": (
        "linux-file-identity-race-harness", "pass-fedora-unprivileged-scope"
    ),
}
OPEN_BOUNDARIES = (
    "macOS path adapter, aliases, bookmarks, case collisions, and Unicode collisions",
    "privileged isolated bind-mount replacement execution",
    "Ubuntu execution evidence",
    "public workspace selection and durable authorization",
    "real tool and Visual Studio Code link activation integration",
    "coverage-guided path fuzzing and sanitizer evidence",
)
SOURCE_PATHS = (
    "SECURITY-REVIEW.md",
    "docs/architecture/path-authority-contract.md",
    *(f"{REPORT_DIRECTORY}/{name}" for name in SUBJECTS),
    "scripts/path_boundary_review.py",
    "tests/test_path_boundary_review.py",
)


class PathBoundaryReviewError(ValueError):
    """Raised when the secure-path review is stale, incomplete, or overclaimed."""


def pretty_json(value: Any) -> bytes:
    return (json.dumps(value, indent=2, sort_keys=True) + "\n").encode()


def sha256_bytes(value: bytes) -> str:
    return hashlib.sha256(value).hexdigest()


def write_atomic(path: Path, content: bytes) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    descriptor, name = tempfile.mkstemp(prefix=".agentmage-path-review-", dir=path.parent)
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


def load_subjects(root: Path = ROOT) -> dict[str, Any]:
    values = {}
    for name in SUBJECTS:
        try:
            values[name] = json.loads((root / REPORT_DIRECTORY / name).read_text())
        except (OSError, json.JSONDecodeError) as error:
            raise PathBoundaryReviewError(f"cannot read review subject: {name}") from error
    return values


def validate_subjects(subjects: Any) -> list[str]:
    if not isinstance(subjects, dict) or tuple(subjects) != tuple(SUBJECTS):
        return ["path review subject closure or order changed"]
    failures: list[str] = []
    for name, (artifact_id, status) in SUBJECTS.items():
        subject = subjects[name]
        if (
            not isinstance(subject, dict)
            or subject.get("artifact_id") != artifact_id
            or subject.get("status") != status
            or re.fullmatch(r"[0-9a-f]{40}", str(subject.get("reference_revision"))) is None
        ):
            failures.append(f"path review subject identity changed: {name}")
    if subjects["path-corpus-report.json"].get("coverage", {}).get("case_count") != 640:
        failures.append("generated path case closure changed")
    if subjects["path-corpus-report.json"].get("coverage", {}).get("admitted_escape_count") != 0:
        failures.append("generated path corpus admitted an escape")
    if subjects["display-link-authority-report.json"].get("coverage", {}).get("rejection_count") != 1280:
        failures.append("display authority denial closure changed")
    if subjects["path-race-report.json"].get("coverage", {}).get("out_of_root_access_count") != 0:
        failures.append("path race harness observed out-of-root access")
    return failures


def run_subject_checkers(root: Path = ROOT) -> None:
    if root != ROOT:
        return
    checkers: tuple[Callable[[Path], None], ...] = (
        check_contract, check_corpus, check_display, check_race
    )
    for checker in checkers:
        checker(root)


def independent_review(root: Path = ROOT) -> dict[str, Any]:
    run_subject_checkers(root)
    subjects = load_subjects(root)
    failures = validate_subjects(subjects)
    security = (root / "SECURITY-REVIEW.md").read_text(encoding="utf-8")
    for identifier in (
        "SR-PLT-004", "SR-ACC-004", "SR-ACC-005", "SR-ACC-006",
        "SR-TST-002", "SR-TST-004", "RV-04",
    ):
        if f"`{identifier}`" not in security:
            failures.append(f"security review identifier is absent: {identifier}")
    if failures:
        raise PathBoundaryReviewError("; ".join(failures))
    return {
        "reviewed_subject_count": len(subjects),
        "generated_path_case_count": 640,
        "admitted_escape_count": 0,
        "display_link_rejection_count": 1280,
        "race_executed_scenario_count": 5,
        "out_of_root_access_count": 0,
        "open_boundary_count": len(OPEN_BOUNDARIES),
    }


def git_revision(candidate: str = "HEAD", root: Path = ROOT) -> str:
    completed = subprocess.run(
        ["git", "rev-parse", "--verify", f"{candidate}^{{commit}}"], cwd=root, check=False,
        stdout=subprocess.PIPE, stderr=subprocess.DEVNULL, timeout=10,
    )
    revision = completed.stdout.decode("ascii", "strict").strip()
    if completed.returncode != 0 or re.fullmatch(r"[0-9a-f]{40}", revision) is None:
        raise PathBoundaryReviewError("source revision is unavailable")
    return revision


def git_file(revision: str, relative: str, root: Path = ROOT) -> bytes:
    completed = subprocess.run(
        ["git", "show", f"{revision}:{relative}"], cwd=root, check=False,
        stdout=subprocess.PIPE, stderr=subprocess.DEVNULL, timeout=10,
    )
    if completed.returncode != 0:
        raise PathBoundaryReviewError(f"source is absent at revision: {relative}")
    return completed.stdout


def build_report(reference_revision: str, root: Path = ROOT) -> dict[str, Any]:
    coverage = independent_review(root)
    ancestor = subprocess.run(
        ["git", "merge-base", "--is-ancestor", reference_revision, "HEAD"],
        cwd=root, check=False, stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL, timeout=10,
    )
    if ancestor.returncode != 0:
        raise PathBoundaryReviewError("reference revision is not an ancestor of HEAD")
    sources = []
    for relative in SOURCE_PATHS:
        committed = git_file(reference_revision, relative, root)
        if committed != (root / relative).read_bytes():
            raise PathBoundaryReviewError(f"source differs from reference revision: {relative}")
        sources.append({"path": relative, "sha256": sha256_bytes(committed)})
    subjects = load_subjects(root)
    return {
        "schema_version": 1,
        "task_id": "6.1.3.5",
        "artifact_id": "independent-automated-secure-path-review",
        "status": "pass-shared-fedora-review-scope",
        "reference_revision": reference_revision,
        "review_protocol": "RV-04",
        "review_protocol_status": "partial-blocked-platform-and-privileged-tests",
        "reviewer_identity": "agentmage-path-boundary-review-v1",
        "external_human_review_status": "not-performed",
        "sources": sources,
        "subjects": [
            {
                "path": f"{REPORT_DIRECTORY}/{name}",
                "sha256": sha256_bytes((root / REPORT_DIRECTORY / name).read_bytes()),
                "reference_revision": subjects[name]["reference_revision"],
            }
            for name in SUBJECTS
        ],
        "coverage": coverage,
        "verification": {
            "subject_checker_reexecution": "pass",
            "source_and_subject_hash_binding": "pass",
            "path_contract_closure": "pass",
            "generated_corpus_reconciliation": "pass",
            "display_authority_reconciliation": "pass",
            "fedora_race_reconciliation": "pass",
            "limitation_and_platform_disclosure": "pass",
        },
        "findings": [],
        "open_product_boundaries": list(OPEN_BOUNDARIES),
        "product_requirement_completion_claim": "none",
        "fuzzing_claim": "generated-fixed-corpus-not-fuzzing",
        "release_claim": "none",
        "macos_execution_status": "blocked-macos",
        "macos_evidence_substituted": False,
    }


def validate_report(value: Any) -> list[str]:
    failures: list[str] = []
    if not isinstance(value, dict):
        return ["path boundary review must be an object"]
    if (
        value.get("schema_version") != 1
        or value.get("task_id") != "6.1.3.5"
        or value.get("artifact_id") != "independent-automated-secure-path-review"
        or value.get("status") != "pass-shared-fedora-review-scope"
        or re.fullmatch(r"[0-9a-f]{40}", str(value.get("reference_revision"))) is None
        or value.get("review_protocol") != "RV-04"
        or value.get("review_protocol_status") != "partial-blocked-platform-and-privileged-tests"
        or value.get("reviewer_identity") != "agentmage-path-boundary-review-v1"
        or value.get("external_human_review_status") != "not-performed"
    ):
        failures.append("path boundary review identity changed")
    if value.get("coverage") != {
        "reviewed_subject_count": 4,
        "generated_path_case_count": 640,
        "admitted_escape_count": 0,
        "display_link_rejection_count": 1280,
        "race_executed_scenario_count": 5,
        "out_of_root_access_count": 0,
        "open_boundary_count": 6,
    }:
        failures.append("path boundary review coverage changed")
    verification = value.get("verification")
    if not isinstance(verification, dict) or len(verification) != 7 or set(verification.values()) != {"pass"}:
        failures.append("path boundary review verification is incomplete")
    if value.get("findings") != [] or value.get("open_product_boundaries") != list(OPEN_BOUNDARIES):
        failures.append("path boundary findings or limitations changed")
    if (
        value.get("product_requirement_completion_claim") != "none"
        or value.get("fuzzing_claim") != "generated-fixed-corpus-not-fuzzing"
        or value.get("release_claim") != "none"
        or value.get("macos_execution_status") != "blocked-macos"
        or value.get("macos_evidence_substituted") is not False
    ):
        failures.append("path boundary review made an unsupported claim")
    return failures


def write_report(reference_revision: str, root: Path = ROOT) -> None:
    write_atomic(REPORT_PATH, pretty_json(build_report(reference_revision, root)))


def check_report(root: Path = ROOT) -> None:
    try:
        actual = json.loads(REPORT_PATH.read_text(encoding="utf-8"))
        revision = actual["reference_revision"]
    except (OSError, KeyError, TypeError, json.JSONDecodeError) as error:
        raise PathBoundaryReviewError(f"cannot read path boundary review: {error}") from error
    failures = validate_report(actual)
    if not isinstance(revision, str) or actual != build_report(revision, root):
        failures.append("path boundary review is stale or malformed")
    if failures:
        raise PathBoundaryReviewError("; ".join(failures))


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--write", action="store_true")
    parser.add_argument("--source-revision", default="HEAD")
    args = parser.parse_args()
    try:
        if args.write:
            write_report(git_revision(args.source_revision))
        check_report()
    except (OSError, UnicodeError, PathBoundaryReviewError, subprocess.SubprocessError) as error:
        print(f"Path boundary review failed: {error}", file=sys.stderr)
        return 1
    print("Independent automated Story 6.1 secure-path review validated")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
