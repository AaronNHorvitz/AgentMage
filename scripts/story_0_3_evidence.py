#!/usr/bin/env python3
"""Build and verify public synthetic partial evidence for Story 0.3."""

from __future__ import annotations

import argparse
import hashlib
import json
import re
import subprocess
import sys
from pathlib import Path
from typing import Final

try:
    from scripts.model_corpus import load_corpus
    from scripts.model_feasibility import (
        DEFAULT_ADMISSION,
        DEFAULT_CORPUS,
        read_json,
        validate_result,
        validate_result_directory,
    )
except ModuleNotFoundError:
    from model_corpus import load_corpus
    from model_feasibility import (
        DEFAULT_ADMISSION,
        DEFAULT_CORPUS,
        read_json,
        validate_result,
        validate_result_directory,
    )


ROOT: Final = Path(__file__).resolve().parents[1]
DEFAULT_OUTPUT: Final = ROOT / "artifacts" / "sprints" / "sprint-0" / "story-0.3"
EVIDENCE_DATE: Final = "2026-08-10"
ARTIFACT_NAMES: Final = (
    "adapter-disposition.json",
    "native-linux-result.json",
    "native-linux-server.log",
    "summary.md",
)
FAILED_CASES: Final = ("REPO-001", "CITE-001", "TOOL-001")
FAILED_THRESHOLDS: Final = (
    "citation_precision",
    "citation_recall",
    "tool_call_valid_rate",
)
HOME_PATH = re.compile(r"/(?:var/)?home/[^/\s'\"]+/.local/share/agentmage/evaluation")
SENSITIVE_TEXT = re.compile(
    r"(?:/(?:var/)?home/[^/\s]+|[A-Za-z0-9._%+-]+@[A-Za-z0-9.-]+\.[A-Za-z]{2,}|"
    r"-----BEGIN [A-Z ]*PRIVATE KEY-----|(?:password|api[_ -]?key|secret)\s*[:=])",
    re.IGNORECASE,
)


class Story03EvidenceError(ValueError):
    """Raised when Story 0.3 evidence cannot be built or reconciled."""


def json_bytes(value: object) -> bytes:
    return (json.dumps(value, indent=2, sort_keys=True) + "\n").encode("utf-8")


def sha256(content: bytes) -> str:
    return hashlib.sha256(content).hexdigest()


def resolve_revision(revision: str) -> str:
    result = subprocess.run(
        ["git", "rev-parse", "--verify", f"{revision}^{{commit}}"],
        cwd=ROOT,
        check=False,
        capture_output=True,
        text=True,
    )
    if result.returncode != 0:
        raise Story03EvidenceError(f"cannot resolve verification revision {revision}")
    return result.stdout.strip()


def sanitize_server_log(content: str) -> str:
    sanitized = HOME_PATH.sub("<EVALUATION_DATA_ROOT>", content)
    if SENSITIVE_TEXT.search(sanitized):
        raise Story03EvidenceError("server log still contains a sensitive path or identifier pattern")
    return sanitized


def adapter_disposition(result: dict[str, object], verification_revision: str) -> dict[str, object]:
    failed_cases = [case["case_id"] for case in result["cases"] if not case["passed"]]
    failed_thresholds = [
        name for name, value in result["threshold_results"].items() if not value["passed"]
    ]
    return {
        "schema_version": 1,
        "record_type": "story_0_3_partial_adapter_disposition",
        "evidence_date": EVIDENCE_DATE,
        "result_source_revision": result["source_revision"],
        "verification_revision": verification_revision,
        "data_classification": "public_synthetic_only",
        "native_linux": {
            "adapter_id": "linux-native-vulkan",
            "execution_status": "COMPLETE",
            "quality_status": "FAIL",
            "failed_cases": failed_cases,
            "failed_thresholds": failed_thresholds,
        },
        "docker_model_runner": {
            "adapter_id": "linux-docker-model-runner-cuda",
            "execution_status": "BLOCKED",
            "blocker": "Docker and Docker Model Runner are not installed on the evaluation host.",
            "substitute_runtime_used": False,
        },
        "macos_native": {
            "adapter_id": "macos-native-metal",
            "execution_status": "BLOCKED",
            "blocker": "The required MacBook Pro M5 evaluation host is not available.",
            "linux_evidence_substituted": False,
        },
        "task_0_3_2_1_status": "BLOCKED",
        "profile_decision": "PENDING",
        "fallback_evaluated": False,
        "independent_review_performed": False,
        "release_approval": False,
        "limitations": [
            "This bundle records only the completed native Fedora adapter run.",
            "The server log replaces the local evaluation-data root with a declared placeholder.",
            "Docker and macOS evidence remain required before the cross-adapter decision.",
            "The blocked source-lineage and GGUF reproducibility findings remain unresolved.",
        ],
    }


def summary_markdown(result: dict[str, object], disposition: dict[str, object]) -> bytes:
    passed_cases = sum(case["passed"] for case in result["cases"])
    passed_thresholds = sum(item["passed"] for item in result["threshold_results"].values())
    metrics = result["metrics"]
    text = f"""# Story 0.3 Partial Evidence Summary

| Field | Value |
|---|---|
| Result source revision | `{result['source_revision']}` |
| Verification revision | `{disposition['verification_revision']}` |
| Evidence date | {EVIDENCE_DATE} |
| Native adapter | `linux-native-vulkan` |
| Native result | `{result['status']}` |
| Corpus cases | {passed_cases} passed, {len(result['cases']) - passed_cases} failed |
| Global thresholds | {passed_thresholds} passed, {len(result['threshold_results']) - passed_thresholds} failed |
| Docker adapter | `BLOCKED` - required runtime unavailable |
| macOS adapter | `BLOCKED` - required hardware unavailable |
| Profile decision | `PENDING` |
| Independent review | Not performed |
| Release approval | No |

The native run completed all 72 fixed trials. It failed repository citation precision, evidence citation recall, and exact tool-argument validity. Structured chat, unavailable-write refusal, malformed-output blocking, cancellation, context handling, performance, memory, and isolated zero-egress cases passed.

Key native measurements were {metrics['minimum_generation_tokens_per_second']:.2f} minimum generated tokens per second, {metrics['maximum_time_to_first_token_seconds']:.3f} seconds maximum time to first token, {metrics['cancellation_max_seconds']:.3f} seconds maximum post-cancel quiescence, {metrics['maximum_gpu_or_unified_memory_fraction']:.3f} maximum GPU-memory fraction, and {metrics['post_install_egress_bytes']} post-install egress bytes.

The raw JSON result is authoritative over this summary. The native failure is not a final E4B profile decision because the required Docker and MacBook Pro M5 adapter runs remain unavailable. No fallback, profile activation, support claim, or release approval is authorized by this bundle.
"""
    return text.encode("utf-8")


def build_bundle(result_dir: Path, verification_revision: str) -> dict[str, bytes]:
    failures = validate_result_directory(result_dir)
    if failures:
        raise Story03EvidenceError("source result bundle is invalid: " + "; ".join(failures))
    result = read_json(result_dir / "results.json")
    server_log = (result_dir / "server.log").read_text(encoding="utf-8")
    disposition = adapter_disposition(result, verification_revision)
    return {
        "adapter-disposition.json": json_bytes(disposition),
        "native-linux-result.json": (result_dir / "results.json").read_bytes(),
        "native-linux-server.log": sanitize_server_log(server_log).encode("utf-8"),
        "summary.md": summary_markdown(result, disposition),
    }


def build_manifest(bundle: dict[str, bytes], disposition: dict[str, object]) -> dict[str, object]:
    return {
        "schema_version": 1,
        "bundle_id": "sprint-0-story-0.3-native-linux-partial-evidence",
        "evidence_date": EVIDENCE_DATE,
        "result_source_revision": disposition["result_source_revision"],
        "verification_revision": disposition["verification_revision"],
        "scope": "Story 0.3 native Linux feasibility result; Docker and macOS remain blocked",
        "files": [
            {"path": name, "sha256": sha256(bundle[name]), "size": len(bundle[name])}
            for name in ARTIFACT_NAMES
        ],
    }


def write_bundle(output: Path, result_dir: Path, verification_revision: str) -> None:
    if output.exists():
        raise Story03EvidenceError(f"refusing to overwrite existing evidence: {output}")
    verification_revision = resolve_revision(verification_revision)
    bundle = build_bundle(result_dir, verification_revision)
    disposition = json.loads(bundle["adapter-disposition.json"])
    output.mkdir(parents=True)
    for name, content in bundle.items():
        (output / name).write_bytes(content)
    (output / "evidence-manifest.json").write_bytes(
        json_bytes(build_manifest(bundle, disposition))
    )


def check_bundle(output: Path = DEFAULT_OUTPUT) -> list[str]:
    failures: list[str] = []
    try:
        manifest = read_json(output / "evidence-manifest.json")
        result = read_json(output / "native-linux-result.json")
        disposition = read_json(output / "adapter-disposition.json")
        corpus = load_corpus(DEFAULT_CORPUS)
        admission = read_json(DEFAULT_ADMISSION)
    except (OSError, json.JSONDecodeError, ValueError) as error:
        return [f"cannot load Story 0.3 evidence: {error}"]
    entries = manifest.get("files")
    if not isinstance(entries, list) or [entry.get("path") for entry in entries if isinstance(entry, dict)] != list(ARTIFACT_NAMES):
        failures.append("evidence manifest membership or order is invalid")
    else:
        for entry in entries:
            path = output / str(entry.get("path"))
            try:
                content = path.read_bytes()
            except OSError as error:
                failures.append(f"cannot read evidence artifact {path.name}: {error}")
                continue
            if entry.get("sha256") != sha256(content):
                failures.append(f"evidence hash mismatch: {path.name}")
            if entry.get("size") != len(content):
                failures.append(f"evidence size mismatch: {path.name}")

    try:
        failures.extend(validate_result(result, corpus, admission))
        failed_cases = tuple(case["case_id"] for case in result["cases"] if not case["passed"])
        failed_thresholds = tuple(
            name for name, value in result["threshold_results"].items() if not value["passed"]
        )
        if failed_cases != FAILED_CASES:
            failures.append("native failed-case set changed")
        if failed_thresholds != FAILED_THRESHOLDS:
            failures.append("native failed-threshold set changed")
        if disposition.get("native_linux", {}).get("quality_status") != "FAIL":
            failures.append("native disposition does not record threshold failure")
        if disposition.get("docker_model_runner", {}).get("execution_status") != "BLOCKED":
            failures.append("Docker runtime blocker is missing")
        if disposition.get("docker_model_runner", {}).get("substitute_runtime_used") is not False:
            failures.append("Docker disposition permits a substitute runtime")
        if disposition.get("macos_native", {}).get("execution_status") != "BLOCKED":
            failures.append("macOS hardware blocker is missing")
        if disposition.get("profile_decision") != "PENDING" or disposition.get("fallback_evaluated") is not False:
            failures.append("partial evidence overstates the profile decision")
        if disposition.get("independent_review_performed") is not False or disposition.get("release_approval") is not False:
            failures.append("partial evidence overstates review or release approval")
        if manifest.get("result_source_revision") != result.get("source_revision"):
            failures.append("manifest result revision does not reconcile")
        if manifest.get("verification_revision") != disposition.get("verification_revision"):
            failures.append("manifest verification revision does not reconcile")
        resolve_revision(str(disposition["verification_revision"]))
        log = (output / "native-linux-server.log").read_text(encoding="utf-8")
        if SENSITIVE_TEXT.search(log) or "<EVALUATION_DATA_ROOT>" not in log:
            failures.append("sanitized server log contains sensitive text or lacks its placeholder")
    except (OSError, KeyError, TypeError, Story03EvidenceError) as error:
        failures.append(f"cannot reconcile Story 0.3 evidence: {error}")
    return failures


def main(argv: list[str] | None = None) -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--output", type=Path, default=DEFAULT_OUTPUT)
    parser.add_argument("--result-dir", type=Path)
    parser.add_argument("--verification-revision", default="HEAD")
    parser.add_argument("--write", action="store_true")
    args = parser.parse_args(argv)
    try:
        if args.write:
            if args.result_dir is None:
                raise Story03EvidenceError("--write requires --result-dir")
            write_bundle(args.output, args.result_dir, args.verification_revision)
            return 0
        failures = check_bundle(args.output)
    except (OSError, Story03EvidenceError) as error:
        print(f"Story 0.3 evidence error: {error}", file=sys.stderr)
        return 1
    if failures:
        print("Story 0.3 evidence validation failed:", file=sys.stderr)
        for failure in failures:
            print(f"- {failure}", file=sys.stderr)
        return 1
    print(f"Validated partial Story 0.3 evidence bundle at {args.output.relative_to(ROOT)}.")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
