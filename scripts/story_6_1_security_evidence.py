#!/usr/bin/env python3
"""Build and validate the Story 6.1 product-security evidence map."""

from __future__ import annotations

import argparse
import hashlib
import json
import os
import re
import subprocess
import sys
import tempfile
from pathlib import Path, PurePosixPath
from typing import Any


ROOT = Path(__file__).resolve().parents[1]
if str(ROOT) not in sys.path:
    sys.path.insert(0, str(ROOT))

from scripts.path_boundary_review import check_report as check_boundary_review


REPORT_PATH = ROOT / "artifacts/sprints/sprint-6/story-6.1/security-evidence-map.json"
EXPECTED_REQUIREMENTS = (
    "SR-PLT-004", "SR-ACC-004", "SR-ACC-005", "SR-ACC-006", "SR-TST-002", "SR-TST-004"
)
EVIDENCE_PATHS = (
    "SECURITY-REVIEW.md",
    "docs/architecture/path-authority-contract.md",
    "fixtures/paths/README.md",
    "fixtures/paths/v1/corpus.json",
    "fixtures/paths/v1/manifest.json",
    "fixtures/paths/v1/display-link-corpus.json",
    "kernel/contracts/src/path.rs",
    "kernel/contracts/src/display_link.rs",
    "kernel/contracts/src/platform_path.rs",
    "kernel/contracts/tests/path_corpus.rs",
    "kernel/engine/tests/display_link_authority.rs",
    "platforms/linux/src/lib.rs",
    "artifacts/sprints/sprint-6/story-6.1/path-contract-report.json",
    "artifacts/sprints/sprint-6/story-6.1/path-corpus-report.json",
    "artifacts/sprints/sprint-6/story-6.1/display-link-authority-report.json",
    "artifacts/sprints/sprint-6/story-6.1/path-race-report.json",
    "artifacts/sprints/sprint-6/story-6.1/path-boundary-review.json",
    "scripts/path_boundary_review.py",
    "scripts/story_6_1_security_evidence.py",
    "tests/test_path_boundary_review.py",
    "tests/test_story_6_1_security_evidence.py",
)
MAPPINGS = {
    "SR-PLT-004": {
        "story_contribution": "partial-story-evidence",
        "evidence": [
            "kernel/contracts/src/platform_path.rs", "platforms/linux/src/lib.rs",
            "artifacts/sprints/sprint-6/story-6.1/path-contract-report.json",
            "artifacts/sprints/sprint-6/story-6.1/path-race-report.json",
        ],
        "demonstrated": "The shared adapter accepts only canonical WorkspacePath values and the Fedora adapter resolves beneath one held root with strict openat2 flags, held descriptors, mount/object identity, and exact preimages.",
        "remaining": "Public user workspace authorization, macOS security-scoped bookmarks and aliases, real case and Unicode collision behavior, Ubuntu execution, and privileged mount-swap execution remain open.",
    },
    "SR-ACC-004": {
        "story_contribution": "demonstrated-shared-parser-scope",
        "evidence": [
            "kernel/contracts/src/path.rs", "fixtures/paths/v1/corpus.json",
            "artifacts/sprints/sprint-6/story-6.1/path-corpus-report.json",
            "artifacts/sprints/sprint-6/story-6.1/display-link-authority-report.json",
        ],
        "demonstrated": "WorkspacePath contains only one validated workspace identity and bounded canonical relative components. The typed corpus rejects 512 escape-shaped cases and all 1,280 display-link authority replays before observation.",
        "remaining": "Platform collection collision checks and the complete cross-platform integration suite must prove the same result through every real tool boundary.",
    },
    "SR-ACC-005": {
        "story_contribution": "partial-story-evidence",
        "evidence": [
            "kernel/contracts/src/platform_path.rs", "platforms/linux/src/lib.rs",
            "artifacts/sprints/sprint-6/story-6.1/path-race-report.json",
            "artifacts/sprints/sprint-6/story-6.1/path-boundary-review.json",
        ],
        "demonstrated": "Fedora holds root and object descriptors across check and use, enforces no-follow and no-cross-mount resolution, rejects hard links, checks identity/preimages, and records zero out-of-root access across unprivileged race scenarios.",
        "remaining": "All macOS alias, stale-bookmark, mount-change, case, and Unicode race cases remain unexecuted; the equivalent Ubuntu execution is also pending.",
    },
    "SR-ACC-006": {
        "story_contribution": "partial-story-evidence",
        "evidence": [
            "docs/architecture/path-authority-contract.md",
            "artifacts/sprints/sprint-6/story-6.1/path-contract-report.json",
            "artifacts/sprints/sprint-6/story-6.1/path-race-report.json",
        ],
        "demonstrated": "The current Linux adapter has no public ambient-path authorization constructor and strict resolution cannot traverse outside its held root through tested symlink and replacement attacks.",
        "remaining": "The running product must add explicit prohibited-root canaries for home, credentials, browsers, SSH, cloud sync, clipboard, and adjacent repositories across every process and model context path.",
    },
    "SR-TST-002": {
        "story_contribution": "generated-corpus-only",
        "evidence": [
            "fixtures/paths/v1/corpus.json", "kernel/contracts/tests/path_corpus.rs",
            "artifacts/sprints/sprint-6/story-6.1/path-corpus-report.json",
        ],
        "demonstrated": "A deterministic 640-case fixed corpus exercises constructor and deserializer boundary classes, including malformed UTF-8, with zero admitted escape.",
        "remaining": "This is not coverage-guided fuzzing. Pinned fuzz duration, coverage, sanitizer, crash, minimization, and regression-corpus evidence remains required.",
    },
    "SR-TST-004": {
        "story_contribution": "partial-story-evidence",
        "evidence": [
            "fixtures/paths/v1/corpus.json", "fixtures/paths/v1/display-link-corpus.json",
            "artifacts/sprints/sprint-6/story-6.1/path-race-report.json",
            "artifacts/sprints/sprint-6/story-6.1/path-boundary-review.json",
        ],
        "demonstrated": "Malformed, hostile, ambiguous, encoded, stale-identity, and concurrent replacement inputs produce bounded rejection or a continuously held safe object in the shared and Fedora scopes.",
        "remaining": "Oversized and conflicting classes must be completed across all product parsers, adapters, processes, and supported platforms with receipts and resource bounds.",
    },
}


def pretty_json(value: Any) -> bytes:
    return (json.dumps(value, indent=2, sort_keys=True) + "\n").encode()


def sha256_file(path: Path) -> str:
    return hashlib.sha256(path.read_bytes()).hexdigest()


def git_revision(candidate: str = "HEAD", root: Path = ROOT) -> str:
    completed = subprocess.run(
        ["git", "rev-parse", "--verify", f"{candidate}^{{commit}}"], cwd=root, check=False,
        stdout=subprocess.PIPE, stderr=subprocess.DEVNULL, timeout=10,
    )
    revision = completed.stdout.decode("ascii", "strict").strip()
    if completed.returncode != 0 or re.fullmatch(r"[0-9a-f]{40}", revision) is None:
        raise ValueError("source revision is unavailable")
    return revision


def safe_relative_path(value: Any) -> bool:
    if not isinstance(value, str) or not value:
        return False
    path = PurePosixPath(value)
    return not path.is_absolute() and ".." not in path.parts and str(path) == value


def write_atomic(path: Path, content: bytes) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    descriptor, name = tempfile.mkstemp(prefix=".agentmage-story-6-1-security-", dir=path.parent)
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


def validate_inputs(root: Path = ROOT) -> list[str]:
    failures: list[str] = []
    try:
        check_boundary_review(root)
    except (OSError, RuntimeError, ValueError, subprocess.SubprocessError) as error:
        failures.append(f"independent path review: {error}")
    security = (root / "SECURITY-REVIEW.md").read_text(encoding="utf-8")
    for identifier in (*EXPECTED_REQUIREMENTS, "RV-04"):
        if f"`{identifier}`" not in security:
            failures.append(f"security identifier is missing: {identifier}")
    return failures


def evidence_records(root: Path = ROOT) -> list[dict[str, Any]]:
    return [
        {"path": path, "sha256": sha256_file(root / path), "bytes": (root / path).stat().st_size}
        for path in EVIDENCE_PATHS
    ]


def build_map(source_revision: str, root: Path = ROOT) -> dict[str, Any]:
    failures = validate_inputs(root)
    if failures:
        raise ValueError("; ".join(failures))
    return {
        "schema_version": 1,
        "story_id": "6.1",
        "task_id": "6.1.3.5",
        "source_revision": source_revision,
        "status": "pass-shared-fedora-security-mapping",
        "requirements": [
            {"requirement_id": identifier, "product_requirement_status": "not-complete", **MAPPINGS[identifier]}
            for identifier in EXPECTED_REQUIREMENTS
        ],
        "reviewer_protocol": {
            "protocol_id": "RV-04",
            "status": "partial-blocked-platform-and-privileged-tests",
            "evidence": [
                "artifacts/sprints/sprint-6/story-6.1/path-boundary-review.json",
                "artifacts/sprints/sprint-6/story-6.1/path-race-report.json",
            ],
        },
        "artifacts": evidence_records(root),
        "summary": {
            "mapped_requirement_count": 6,
            "demonstrated_shared_scope_count": 1,
            "partial_or_generated_scope_count": 5,
            "product_requirements_complete": 0,
            "retained_artifact_count": len(EVIDENCE_PATHS),
            "story_gate_complete": False,
        },
        "private_user_data_used": False,
        "network_used": False,
        "independent_review_type": "independent-automated-boundary-review",
        "external_human_review_status": "not-performed",
        "product_requirement_completion_claim": "none",
        "fuzzing_claim": "generated-fixed-corpus-not-fuzzing",
        "release_claim": "none",
        "macos_execution_status": "blocked-macos",
        "macos_evidence_substituted": False,
    }


def validate_map(value: Any, root: Path = ROOT) -> list[str]:
    failures: list[str] = []
    if not isinstance(value, dict):
        return ["Story 6.1 security evidence map must be an object"]
    if (
        value.get("schema_version") != 1 or value.get("story_id") != "6.1"
        or value.get("task_id") != "6.1.3.5"
        or value.get("status") != "pass-shared-fedora-security-mapping"
        or re.fullmatch(r"[0-9a-f]{40}", str(value.get("source_revision"))) is None
    ):
        failures.append("Story 6.1 security evidence identity is invalid")
    requirements = value.get("requirements")
    ids = [item.get("requirement_id") for item in requirements] if isinstance(requirements, list) else []
    if ids != list(EXPECTED_REQUIREMENTS) or len(ids) != len(set(ids)):
        failures.append("Story 6.1 security requirement closure is invalid")
    else:
        for item in requirements:
            identifier = item["requirement_id"]
            if item != {"requirement_id": identifier, "product_requirement_status": "not-complete", **MAPPINGS[identifier]}:
                failures.append(f"Story 6.1 security mapping changed: {identifier}")
            if any(not safe_relative_path(path) or path not in EVIDENCE_PATHS for path in item.get("evidence", [])):
                failures.append(f"Story 6.1 security evidence path is invalid: {identifier}")
    protocol = value.get("reviewer_protocol")
    if not isinstance(protocol, dict) or protocol.get("protocol_id") != "RV-04" or protocol.get("status") != "partial-blocked-platform-and-privileged-tests":
        failures.append("Story 6.1 reviewer protocol status is invalid")
    expected_summary = {
        "mapped_requirement_count": 6,
        "demonstrated_shared_scope_count": 1,
        "partial_or_generated_scope_count": 5,
        "product_requirements_complete": 0,
        "retained_artifact_count": len(EVIDENCE_PATHS),
        "story_gate_complete": False,
    }
    if value.get("summary") != expected_summary:
        failures.append("Story 6.1 security summary is invalid")
    try:
        expected_artifacts = evidence_records(root)
    except OSError:
        failures.append("Story 6.1 retained evidence is unavailable")
    else:
        if value.get("artifacts") != expected_artifacts:
            failures.append("Story 6.1 retained evidence closure is stale")
    if (
        value.get("private_user_data_used") is not False or value.get("network_used") is not False
        or value.get("independent_review_type") != "independent-automated-boundary-review"
        or value.get("external_human_review_status") != "not-performed"
        or value.get("product_requirement_completion_claim") != "none"
        or value.get("fuzzing_claim") != "generated-fixed-corpus-not-fuzzing"
        or value.get("release_claim") != "none"
        or value.get("macos_execution_status") != "blocked-macos"
        or value.get("macos_evidence_substituted") is not False
    ):
        failures.append("Story 6.1 security evidence made an unsupported claim")
    return failures


def check_map(root: Path = ROOT) -> list[str]:
    try:
        value = json.loads((root / REPORT_PATH.relative_to(ROOT)).read_text())
    except (OSError, json.JSONDecodeError) as error:
        return [f"cannot read Story 6.1 security evidence map: {error}"]
    return [*validate_inputs(root), *validate_map(value, root)]


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--write", action="store_true")
    parser.add_argument("--source-revision", default="HEAD")
    args = parser.parse_args()
    try:
        if args.write:
            write_atomic(REPORT_PATH, pretty_json(build_map(git_revision(args.source_revision))))
        failures = check_map()
    except (OSError, ValueError, KeyError, TypeError, subprocess.SubprocessError) as error:
        print(f"Story 6.1 security evidence failed: {error}", file=sys.stderr)
        return 1
    if failures:
        for failure in failures:
            print(f"Story 6.1 security evidence failed: {failure}", file=sys.stderr)
        return 1
    print("Story 6.1 security requirements mapped without product or macOS promotion")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
