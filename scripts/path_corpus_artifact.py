#!/usr/bin/env python3
"""Generate, execute, and verify the Story 6.1 canonical-path corpus."""

from __future__ import annotations

import argparse
import hashlib
import json
import os
import re
import subprocess
import sys
import tempfile
from collections import Counter
from pathlib import Path
from typing import Any


ROOT = Path(__file__).resolve().parents[1]
CORPUS_PATH = ROOT / "fixtures/paths/v1/corpus.json"
MANIFEST_PATH = ROOT / "fixtures/paths/v1/manifest.json"
REPORT_PATH = ROOT / "artifacts/sprints/sprint-6/story-6.1/path-corpus-report.json"
CASE_CLASSES = (
    "absolute",
    "empty",
    "dot",
    "traversal",
    "alternate_separator",
    "nul",
    "invalid_encoding",
    "case_collision_candidate",
    "unicode_ambiguity",
    "valid_control",
)
CASES_PER_CLASS = 64
CASE_COUNT = len(CASE_CLASSES) * CASES_PER_CLASS
SOURCE_PATHS = (
    "fixtures/paths/README.md",
    "fixtures/paths/v1/corpus.json",
    "fixtures/paths/v1/manifest.json",
    "kernel/contracts/src/path.rs",
    "kernel/contracts/tests/path_corpus.rs",
    "scripts/path_corpus_artifact.py",
    "tests/test_path_corpus_artifact.py",
)


class PathCorpusArtifactError(ValueError):
    """Raised when generated path evidence is stale, malformed, or overclaimed."""


def canonical_json(value: Any) -> bytes:
    return (json.dumps(value, sort_keys=True, separators=(",", ":"), ensure_ascii=True) + "\n").encode()


def pretty_json(value: Any) -> bytes:
    return (json.dumps(value, indent=2, sort_keys=True, ensure_ascii=True) + "\n").encode()


def sha256_bytes(value: bytes) -> str:
    return hashlib.sha256(value).hexdigest()


def write_atomic(path: Path, content: bytes) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    descriptor, name = tempfile.mkstemp(prefix=".agentmage-path-corpus-", dir=path.parent)
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


def component_case(case_class: str, index: int, components: list[str], outcome: str,
                   code: str | None, component_index: int | None, escape: bool) -> dict[str, Any]:
    return {
        "schema_version": 1,
        "case_id": f"path-{case_class}-{index:03d}",
        "case_class": case_class,
        "input_mode": "components",
        "components": components,
        "serialized_json_hex": None,
        "expected_outcome": outcome,
        "expected_code": code,
        "expected_component_index": component_index,
        "escape_attempt": escape,
        "admitted_escape_count": 0,
    }


def generated_cases() -> list[dict[str, Any]]:
    cases: list[dict[str, Any]] = []
    alternate = ("/", "\\", "\u2044", "\u2215", "\u29f8", "\uff0f", "\uff3c")
    for index in range(CASES_PER_CLASS):
        absolute_variants = (
            ([f"/outside-{index}"], "path.component.rooted"),
            ([f"\\outside-{index}"], "path.component.rooted"),
            ([f"C:outside-{index}"], "path.component.colon"),
        )
        components, code = absolute_variants[index % len(absolute_variants)]
        cases.append(component_case("absolute", index, components, "rejected", code, 0, True))

        empty_components = [] if index % 2 == 0 else [""]
        empty_code = "path.components.empty" if not empty_components else "path.component.empty"
        empty_index = None if not empty_components else 0
        cases.append(component_case("empty", index, empty_components, "rejected", empty_code, empty_index, True))
        cases.append(component_case("dot", index, ["."], "rejected", "path.component.current_directory", 0, True))

        traversal = ".." if index % 2 == 0 else f"outside-%2e%2e-{index}"
        traversal_code = "path.component.parent_traversal" if traversal == ".." else "path.component.encoded_syntax"
        cases.append(component_case("traversal", index, [traversal], "rejected", traversal_code, 0, True))

        separator = alternate[index % len(alternate)]
        cases.append(component_case(
            "alternate_separator", index, [f"inside{separator}outside-{index}"],
            "rejected", "path.component.separator", 0, True,
        ))
        cases.append(component_case(
            "nul", index, [f"inside\x00outside-{index}"],
            "rejected", "path.component.nul", 0, True,
        ))

        prefix = b'{"workspace_id":"workspace-corpus-0001","components":["invalid-'
        suffix = f'-{index}"]}}'.encode("ascii")
        invalid_bytes = prefix + bytes((0x80 + index % 0x40,)) + suffix
        cases.append({
            "schema_version": 1,
            "case_id": f"path-invalid_encoding-{index:03d}",
            "case_class": "invalid_encoding",
            "input_mode": "serialized_json_hex",
            "components": None,
            "serialized_json_hex": invalid_bytes.hex(),
            "expected_outcome": "rejected",
            "expected_code": "serde.invalid_utf8",
            "expected_component_index": None,
            "escape_attempt": True,
            "admitted_escape_count": 0,
        })

        case_value = f"CasePair-{index // 2:02d}" if index % 2 == 0 else f"casepair-{index // 2:02d}"
        cases.append(component_case(
            "case_collision_candidate", index, [case_value], "accepted", None, None, False,
        ))

        unicode_value = f"e\u0301-{index}" if index % 2 == 0 else f"fullwidth-\uff21-{index}"
        cases.append(component_case(
            "unicode_ambiguity", index, [unicode_value], "rejected",
            "path.component.noncanonical_unicode", 0, True,
        ))
        cases.append(component_case(
            "valid_control", index, [f"folder-{index:03d}", f"file-{index:03d}.rs"],
            "accepted", None, None, False,
        ))
    return sorted(cases, key=lambda case: (CASE_CLASSES.index(case["case_class"]), case["case_id"]))


def generated_corpus() -> bytes:
    return canonical_json({
        "schema_version": 1,
        "corpus_id": "agentmage-canonical-paths-v1",
        "seed_algorithm": "closed-class-counter-v1",
        "case_count": CASE_COUNT,
        "cases_per_class": CASES_PER_CLASS,
        "case_classes": list(CASE_CLASSES),
        "cases": generated_cases(),
    })


def validate_corpus(content: bytes) -> dict[str, Any]:
    try:
        corpus = json.loads(content)
    except (UnicodeDecodeError, json.JSONDecodeError) as error:
        raise PathCorpusArtifactError("path corpus is malformed") from error
    if not isinstance(corpus, dict) or set(corpus) != {
        "schema_version", "corpus_id", "seed_algorithm", "case_count",
        "cases_per_class", "case_classes", "cases",
    }:
        raise PathCorpusArtifactError("path corpus envelope is invalid")
    cases = corpus["cases"]
    if (
        corpus["schema_version"] != 1
        or corpus["corpus_id"] != "agentmage-canonical-paths-v1"
        or corpus["seed_algorithm"] != "closed-class-counter-v1"
        or corpus["case_count"] != CASE_COUNT
        or corpus["cases_per_class"] != CASES_PER_CLASS
        or tuple(corpus["case_classes"]) != CASE_CLASSES
        or not isinstance(cases, list)
        or len(cases) != CASE_COUNT
    ):
        raise PathCorpusArtifactError("path corpus closure is incomplete")
    expected_fields = {
        "schema_version", "case_id", "case_class", "input_mode", "components",
        "serialized_json_hex", "expected_outcome", "expected_code",
        "expected_component_index", "escape_attempt", "admitted_escape_count",
    }
    counts: Counter[str] = Counter()
    identities: set[str] = set()
    for case in cases:
        if not isinstance(case, dict) or set(case) != expected_fields:
            raise PathCorpusArtifactError("path case shape is invalid")
        case_class = case["case_class"]
        if (
            case["schema_version"] != 1
            or case_class not in CASE_CLASSES
            or re.fullmatch(rf"path-{re.escape(case_class)}-[0-9]{{3}}", str(case["case_id"])) is None
            or case["case_id"] in identities
            or case["input_mode"] not in {"components", "serialized_json_hex"}
            or case["expected_outcome"] not in {"accepted", "rejected"}
            or case["admitted_escape_count"] != 0
        ):
            raise PathCorpusArtifactError("path case expectation changed")
        if case["input_mode"] == "components":
            if not isinstance(case["components"], list) or case["serialized_json_hex"] is not None:
                raise PathCorpusArtifactError("component path case input is invalid")
        else:
            if case["components"] is not None or re.fullmatch(r"[0-9a-f]+", str(case["serialized_json_hex"])) is None:
                raise PathCorpusArtifactError("serialized path case input is invalid")
        identities.add(case["case_id"])
        counts[case_class] += 1
    if counts != Counter({case_class: CASES_PER_CLASS for case_class in CASE_CLASSES}):
        raise PathCorpusArtifactError("path corpus class distribution changed")
    return {
        "accepted_control_count": sum(case["expected_outcome"] == "accepted" for case in cases),
        "admitted_escape_count": sum(case["admitted_escape_count"] for case in cases),
        "case_count": len(cases),
        "case_class_count": len(counts),
        "cases_per_class": CASES_PER_CLASS,
        "invalid_encoding_count": counts["invalid_encoding"],
        "rejected_escape_attempt_count": sum(
            case["expected_outcome"] == "rejected" and case["escape_attempt"] for case in cases
        ),
    }


def manifest_for(content: bytes, coverage: dict[str, Any]) -> dict[str, Any]:
    return {
        "schema_version": 1,
        "corpus_id": "agentmage-canonical-paths-v1",
        "media_type": "application/json",
        "sha256": sha256_bytes(content),
        "byte_length": len(content),
        "coverage": coverage,
        "data_classification": "public-synthetic",
        "macos_collision_execution_status": "blocked-macos",
    }


def write_corpus(root: Path = ROOT) -> None:
    content = generated_corpus()
    coverage = validate_corpus(content)
    write_atomic(root / CORPUS_PATH.relative_to(ROOT), content)
    write_atomic(root / MANIFEST_PATH.relative_to(ROOT), pretty_json(manifest_for(content, coverage)))


def check_corpus(root: Path = ROOT) -> dict[str, Any]:
    content = (root / CORPUS_PATH.relative_to(ROOT)).read_bytes()
    if content != generated_corpus():
        raise PathCorpusArtifactError("committed path corpus is stale")
    coverage = validate_corpus(content)
    actual_manifest = json.loads((root / MANIFEST_PATH.relative_to(ROOT)).read_text())
    if actual_manifest != manifest_for(content, coverage):
        raise PathCorpusArtifactError("path corpus manifest is stale")
    return coverage


def run_typed_corpus(root: Path = ROOT) -> None:
    completed = subprocess.run(
        ["cargo", "test", "--locked", "--offline", "-p", "agentmage-kernel-contracts",
         "--test", "path_corpus", "generated_path_corpus_fails_closed_without_authority_or_observation",
         "--", "--exact", "--test-threads=1"],
        cwd=root, check=False, stdin=subprocess.DEVNULL,
        stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL, timeout=120,
    )
    if completed.returncode != 0:
        raise PathCorpusArtifactError("typed path corpus execution failed")


def git_revision(candidate: str = "HEAD", root: Path = ROOT) -> str:
    completed = subprocess.run(
        ["git", "rev-parse", "--verify", f"{candidate}^{{commit}}"], cwd=root, check=False,
        stdout=subprocess.PIPE, stderr=subprocess.DEVNULL, timeout=10,
    )
    revision = completed.stdout.decode("ascii", "strict").strip()
    if completed.returncode != 0 or re.fullmatch(r"[0-9a-f]{40}", revision) is None:
        raise PathCorpusArtifactError("source revision is unavailable")
    return revision


def git_file(revision: str, relative: str, root: Path = ROOT) -> bytes:
    completed = subprocess.run(
        ["git", "show", f"{revision}:{relative}"], cwd=root, check=False,
        stdout=subprocess.PIPE, stderr=subprocess.DEVNULL, timeout=10,
    )
    if completed.returncode != 0:
        raise PathCorpusArtifactError(f"source is absent at revision: {relative}")
    return completed.stdout


def build_report(reference_revision: str, root: Path = ROOT) -> dict[str, Any]:
    coverage = check_corpus(root)
    run_typed_corpus(root)
    ancestor = subprocess.run(
        ["git", "merge-base", "--is-ancestor", reference_revision, "HEAD"],
        cwd=root, check=False, stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL, timeout=10,
    )
    if ancestor.returncode != 0:
        raise PathCorpusArtifactError("reference revision is not an ancestor of HEAD")
    sources = []
    for relative in SOURCE_PATHS:
        committed = git_file(reference_revision, relative, root)
        if committed != (root / relative).read_bytes():
            raise PathCorpusArtifactError(f"source differs from reference revision: {relative}")
        sources.append({"path": relative, "sha256": sha256_bytes(committed)})
    return {
        "schema_version": 1,
        "task_ids": ["6.1.2.2", "6.1.3.1"],
        "artifact_id": "canonicalization-display-link-path-corpus",
        "status": "pass-shared-fedora-parser-scope",
        "reference_revision": reference_revision,
        "sources": sources,
        "coverage": coverage,
        "verification": {
            "deterministic_regeneration": "pass",
            "real_constructor_execution": "pass",
            "real_deserializer_execution": "pass",
            "invalid_utf8_rejection": "pass",
            "case_spelling_preservation": "pass",
            "zero_accepted_escape": "pass",
        },
        "platform_status": {
            "shared_contract": "verified",
            "fedora": "verified-local",
            "ubuntu": "not-executed",
            "macos_collision_handling": "blocked-macos",
        },
        "macos_evidence_substituted": False,
        "filesystem_observation_count": 0,
        "release_claim": "none",
        "limitations": [
            "case-collision candidates prove spelling preservation only",
            "macOS case and Unicode collision behavior is not executed",
            "display-link replay cases are retained in a separate authority-boundary artifact",
        ],
    }


def validate_report(value: Any) -> list[str]:
    failures: list[str] = []
    if not isinstance(value, dict):
        return ["path corpus report must be an object"]
    if (
        value.get("schema_version") != 1
        or value.get("task_ids") != ["6.1.2.2", "6.1.3.1"]
        or value.get("artifact_id") != "canonicalization-display-link-path-corpus"
        or value.get("status") != "pass-shared-fedora-parser-scope"
        or re.fullmatch(r"[0-9a-f]{40}", str(value.get("reference_revision"))) is None
    ):
        failures.append("path corpus report identity changed")
    if value.get("coverage") != {
        "accepted_control_count": 128,
        "admitted_escape_count": 0,
        "case_count": 640,
        "case_class_count": 10,
        "cases_per_class": 64,
        "invalid_encoding_count": 64,
        "rejected_escape_attempt_count": 512,
    }:
        failures.append("path corpus coverage changed")
    verification = value.get("verification")
    if not isinstance(verification, dict) or len(verification) != 6 or set(verification.values()) != {"pass"}:
        failures.append("path corpus verification is incomplete")
    if value.get("platform_status") != {
        "shared_contract": "verified", "fedora": "verified-local",
        "ubuntu": "not-executed", "macos_collision_handling": "blocked-macos",
    }:
        failures.append("path corpus platform status changed")
    if (
        value.get("macos_evidence_substituted") is not False
        or value.get("filesystem_observation_count") != 0
        or value.get("release_claim") != "none"
    ):
        failures.append("path corpus report made an unsupported claim")
    if not isinstance(value.get("limitations"), list) or len(value["limitations"]) != 3:
        failures.append("path corpus limitations are incomplete")
    return failures


def write_report(reference_revision: str, root: Path = ROOT) -> None:
    write_atomic(REPORT_PATH, pretty_json(build_report(reference_revision, root)))


def check_report(root: Path = ROOT) -> None:
    try:
        actual = json.loads(REPORT_PATH.read_text(encoding="utf-8"))
        revision = actual["reference_revision"]
    except (OSError, KeyError, TypeError, json.JSONDecodeError) as error:
        raise PathCorpusArtifactError(f"cannot read path corpus report: {error}") from error
    failures = validate_report(actual)
    if not isinstance(revision, str) or actual != build_report(revision, root):
        failures.append("path corpus report is stale or malformed")
    if failures:
        raise PathCorpusArtifactError("; ".join(failures))


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
    except (OSError, UnicodeError, PathCorpusArtifactError, subprocess.SubprocessError) as error:
        print(f"Path corpus artifact failed: {error}", file=sys.stderr)
        return 1
    print("Story 6.1 canonical path corpus validated")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
