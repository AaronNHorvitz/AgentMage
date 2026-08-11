#!/usr/bin/env python3
"""Build and verify the seeded adversarial capability-grant corpus."""

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
CORPUS_PATH = ROOT / "fixtures/grants/adversarial/v1/corpus.json"
MANIFEST_PATH = ROOT / "fixtures/grants/adversarial/v1/manifest.json"
REPORT_PATH = ROOT / "artifacts/sprints/sprint-5/story-5.1/adversarial-grant-corpus-report.json"
MARKER = "AGENTMAGE_ADVERSARIAL_GRANT_CORPUS="
EXPECTED_CLASSES = (
    "actor",
    "session",
    "task",
    "action",
    "tool",
    "path",
    "argument",
    "preimage",
    "side_effect",
    "expiry",
    "nonce",
    "use_count",
    "parent",
    "preview_digest",
)
EXPECTED_SCOPES = {
    "actor": "actor",
    "session": "session",
    "task": "task",
    "action": "action",
    "tool": "tool",
    "path": "path",
    "argument": "argument",
    "preimage": "preimage",
    "side_effect": "side_effect",
    "expiry": "grant",
    "nonce": "grant",
    "use_count": "grant",
    "parent": "grant",
    "preview_digest": "preview",
}
FORGED_CLASSES = {"nonce", "use_count", "parent"}
SOURCE_PATHS = (
    "fixtures/grants/adversarial/README.md",
    "fixtures/grants/adversarial/v1/corpus.json",
    "fixtures/grants/adversarial/v1/manifest.json",
    "kernel/contracts/src/grant.rs",
    "kernel/engine/src/grants.rs",
    "kernel/engine/src/policy.rs",
    "kernel/engine/tests/adversarial_grant_corpus.rs",
    "scripts/adversarial_grant_corpus.py",
    "tests/test_adversarial_grant_corpus.py",
)


class AdversarialGrantCorpusError(ValueError):
    """Raised when the corpus or its evidence fails closed validation."""


def sha256_bytes(value: bytes) -> str:
    return hashlib.sha256(value).hexdigest()


def canonical_json(value: Any) -> bytes:
    return (json.dumps(value, indent=2, sort_keys=True) + "\n").encode("utf-8")


def write_atomic(path: Path, content: bytes) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    descriptor, temporary_name = tempfile.mkstemp(
        prefix=".agentmage-adversarial-grant-", dir=path.parent
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


def generated_corpus(root: Path = ROOT) -> bytes:
    environment = {**os.environ, "AGENTMAGE_EMIT_ADVERSARIAL_GRANT_CORPUS": "1"}
    completed = subprocess.run(
        [
            "cargo",
            "test",
            "--locked",
            "--offline",
            "-p",
            "agentmage-kernel-engine",
            "--test",
            "adversarial_grant_corpus",
            "adversarial_grant_corpus_denies_every_seeded_case",
            "--",
            "--exact",
            "--nocapture",
            "--test-threads=1",
        ],
        cwd=root,
        check=False,
        stdin=subprocess.DEVNULL,
        stdout=subprocess.PIPE,
        stderr=subprocess.STDOUT,
        timeout=120,
        env=environment,
    )
    if completed.returncode != 0:
        raise AdversarialGrantCorpusError("typed adversarial corpus generator failed")
    if len(completed.stdout) > 8 * 1024 * 1024:
        raise AdversarialGrantCorpusError("typed adversarial corpus output is oversized")
    decoded = completed.stdout.decode("utf-8", "strict")
    payloads = [line.split(MARKER, 1)[1] for line in decoded.splitlines() if MARKER in line]
    if len(payloads) != 1:
        raise AdversarialGrantCorpusError("typed adversarial corpus marker count is invalid")
    content = payloads[0].encode("utf-8")
    try:
        json.loads(content)
    except json.JSONDecodeError as error:
        raise AdversarialGrantCorpusError("typed adversarial corpus is malformed") from error
    return content


def validate_corpus(content: bytes) -> dict[str, Any]:
    try:
        corpus = json.loads(content)
    except json.JSONDecodeError as error:
        raise AdversarialGrantCorpusError("adversarial corpus is malformed") from error
    if not isinstance(corpus, dict) or set(corpus) != {
        "schema_version",
        "corpus_id",
        "base_profile",
        "seed_algorithm",
        "base_seed",
        "cases_per_class",
        "case_count",
        "mutation_classes",
        "cases",
    }:
        raise AdversarialGrantCorpusError("adversarial corpus envelope is invalid")
    cases = corpus["cases"]
    if (
        corpus["schema_version"] != 1
        or corpus["corpus_id"] != "agentmage-adversarial-grants-v1"
        or tuple(corpus["mutation_classes"]) != EXPECTED_CLASSES
        or corpus["cases_per_class"] != 40
        or corpus["case_count"] != 560
        or not isinstance(cases, list)
        or len(cases) != 560
    ):
        raise AdversarialGrantCorpusError("adversarial corpus closure is incomplete")
    expected_fields = {
        "schema_version",
        "case_id",
        "seed",
        "mutation_class",
        "mutation_variant",
        "evaluation_path",
        "expected_denial_scope",
        "expected_code",
        "resulting_status",
        "resulting_revision",
        "resulting_use_count",
        "admitted_attempts",
    }
    identities: set[str] = set()
    seeds: set[int] = set()
    counts: Counter[str] = Counter()
    for case in cases:
        if not isinstance(case, dict) or set(case) != expected_fields:
            raise AdversarialGrantCorpusError("adversarial case shape is invalid")
        mutation_class = case["mutation_class"]
        expected_scope = EXPECTED_SCOPES.get(mutation_class)
        forged = mutation_class in FORGED_CLASSES
        expected_status = "issued" if forged else "expired" if mutation_class == "expiry" else "invalidated"
        expected_revision = 1 if forged else 2
        expected_path = "forged_candidate_evaluation" if forged else "atomic_consumption"
        if (
            case["schema_version"] != 1
            or not isinstance(case["case_id"], str)
            or re.fullmatch(r"grant-[a-z_]+-[0-9a-f]{16}", case["case_id"]) is None
            or not isinstance(case["seed"], int)
            or not isinstance(case["mutation_variant"], str)
            or case["evaluation_path"] != expected_path
            or case["expected_denial_scope"] != expected_scope
            or case["expected_code"] != f"policy.deny.{expected_scope}"
            or case["resulting_status"] != expected_status
            or case["resulting_revision"] != expected_revision
            or case["resulting_use_count"] != 0
            or case["admitted_attempts"] != 0
        ):
            raise AdversarialGrantCorpusError("adversarial case expectation changed")
        if case["case_id"] in identities or case["seed"] in seeds:
            raise AdversarialGrantCorpusError("adversarial case identity or seed is duplicated")
        identities.add(case["case_id"])
        seeds.add(case["seed"])
        counts[mutation_class] += 1
    if counts != Counter({mutation_class: 40 for mutation_class in EXPECTED_CLASSES}):
        raise AdversarialGrantCorpusError("adversarial mutation-class distribution changed")
    return {
        "case_count": len(cases),
        "mutation_class_count": len(counts),
        "cases_per_class": 40,
        "atomic_consumption_case_count": sum(
            1 for case in cases if case["evaluation_path"] == "atomic_consumption"
        ),
        "forged_candidate_case_count": sum(
            1 for case in cases if case["evaluation_path"] == "forged_candidate_evaluation"
        ),
        "admitted_attempt_count": sum(case["admitted_attempts"] for case in cases),
    }


def manifest_for(content: bytes, coverage: dict[str, Any]) -> dict[str, Any]:
    return {
        "schema_version": 1,
        "corpus_id": "agentmage-adversarial-grants-v1",
        "path": "fixtures/grants/adversarial/v1/corpus.json",
        "sha256": sha256_bytes(content),
        "size_bytes": len(content),
        "coverage": coverage,
        "generator": "kernel/engine/tests/adversarial_grant_corpus.rs",
        "platform_status": {
            "shared_contracts": "verified-local",
            "linux_reference": "verified-local",
            "macos": "blocked-macos",
            "macos_implementation_claim": "none",
        },
        "limitations": [
            "Cases exercise in-memory issuer and policy behavior without a production worker.",
            "Forged nonce, use-count, and parent candidates are evaluated but cannot enter consumption.",
            "Canonical platform path resolution remains assigned to Sprint 6.",
            "No macOS implementation or execution evidence is claimed.",
        ],
    }


def write_corpus(root: Path = ROOT) -> None:
    content = generated_corpus(root)
    coverage = validate_corpus(content)
    write_atomic(root / CORPUS_PATH.relative_to(ROOT), content)
    write_atomic(
        root / MANIFEST_PATH.relative_to(ROOT),
        canonical_json(manifest_for(content, coverage)),
    )


def check_corpus(root: Path = ROOT) -> dict[str, Any]:
    generated = generated_corpus(root)
    retained = (root / CORPUS_PATH.relative_to(ROOT)).read_bytes()
    if retained != generated:
        raise AdversarialGrantCorpusError("retained adversarial grant corpus is stale")
    coverage = validate_corpus(retained)
    manifest = json.loads((root / MANIFEST_PATH.relative_to(ROOT)).read_text(encoding="utf-8"))
    if manifest != manifest_for(retained, coverage):
        raise AdversarialGrantCorpusError("adversarial grant corpus manifest is stale")
    return coverage


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
        raise AdversarialGrantCorpusError("source revision is unavailable")
    return revision


def git_file(revision: str, relative: str, root: Path = ROOT) -> bytes:
    completed = subprocess.run(
        ["git", "show", f"{revision}:{relative}"],
        cwd=root,
        check=False,
        stdout=subprocess.PIPE,
        stderr=subprocess.DEVNULL,
        timeout=10,
    )
    if completed.returncode != 0:
        raise AdversarialGrantCorpusError(f"source is absent at revision: {relative}")
    return completed.stdout


def build_report(reference_revision: str, root: Path = ROOT) -> dict[str, Any]:
    coverage = check_corpus(root)
    ancestor = subprocess.run(
        ["git", "merge-base", "--is-ancestor", reference_revision, "HEAD"],
        cwd=root,
        check=False,
        stdout=subprocess.DEVNULL,
        stderr=subprocess.DEVNULL,
        timeout=10,
    )
    if ancestor.returncode != 0:
        raise AdversarialGrantCorpusError("reference revision is not an ancestor of HEAD")
    sources = []
    for relative in SOURCE_PATHS:
        committed = git_file(reference_revision, relative, root)
        current = (root / relative).read_bytes()
        if committed != current:
            raise AdversarialGrantCorpusError(f"source differs from reference revision: {relative}")
        sources.append({"path": relative, "sha256": sha256_bytes(committed)})
    manifest = json.loads((root / MANIFEST_PATH.relative_to(ROOT)).read_text(encoding="utf-8"))
    return {
        "schema_version": 1,
        "task_id": "5.1.2.4",
        "artifact_id": "adversarial-grant-corpus",
        "status": "pass-shared-linux-reference",
        "reference_revision": reference_revision,
        "sources": sources,
        "coverage": coverage,
        "verification": {
            "typed_seeded_generation": "pass",
            "mutation_class_closure": "pass",
            "first_denial_scope": "pass",
            "terminal_state": "pass",
            "zero_admitted_attempts": "pass",
            "exact_byte_reproducibility": "pass",
        },
        "platform_status": manifest["platform_status"],
        "limitations": manifest["limitations"],
    }


def write_report(root: Path = ROOT) -> None:
    write_atomic(REPORT_PATH, canonical_json(build_report(git_revision(root), root)))


def check_report(root: Path = ROOT) -> None:
    try:
        actual = json.loads(REPORT_PATH.read_text(encoding="utf-8"))
        revision = actual["reference_revision"]
    except (OSError, KeyError, TypeError, json.JSONDecodeError) as error:
        raise AdversarialGrantCorpusError(f"cannot read adversarial corpus report: {error}") from error
    if not isinstance(revision, str) or actual != build_report(revision, root):
        raise AdversarialGrantCorpusError("adversarial grant corpus report is stale or malformed")


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--write-corpus", action="store_true")
    parser.add_argument("--corpus-only", action="store_true")
    parser.add_argument("--write-report", action="store_true")
    args = parser.parse_args()
    try:
        if sum((args.write_corpus, args.corpus_only, args.write_report)) > 1:
            raise AdversarialGrantCorpusError("select only one operation")
        if args.write_corpus:
            write_corpus()
        elif args.corpus_only:
            check_corpus()
        elif args.write_report:
            write_report()
        else:
            check_report()
    except (OSError, AdversarialGrantCorpusError, subprocess.SubprocessError) as error:
        print(f"Adversarial grant corpus validation failed: {error}", file=sys.stderr)
        return 1
    print("Adversarial grant corpus validated")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
