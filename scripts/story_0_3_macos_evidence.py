#!/usr/bin/env python3
"""Build and verify immutable Mac-native extension evidence for Story 0.3."""

from __future__ import annotations

import argparse
import hashlib
import json
import re
import subprocess
import sys
from datetime import datetime, timezone
from pathlib import Path
from typing import Any, Final

try:
    from scripts.macos_model_feasibility import (
        ADAPTER_ID,
        DEFAULT_MODEL_ADMISSION,
        DEFAULT_RUNTIME_ADMISSION,
        validate_directory,
        validate_result,
    )
    from scripts.macos_runtime_admission import load_record as load_runtime_record
    from scripts.model_corpus import DEFAULT_CORPUS, load_corpus
except ModuleNotFoundError:
    from macos_model_feasibility import (
        ADAPTER_ID,
        DEFAULT_MODEL_ADMISSION,
        DEFAULT_RUNTIME_ADMISSION,
        validate_directory,
        validate_result,
    )
    from macos_runtime_admission import load_record as load_runtime_record
    from model_corpus import DEFAULT_CORPUS, load_corpus


ROOT: Final = Path(__file__).resolve().parents[1]
DEFAULT_OUTPUT: Final = ROOT / "artifacts" / "sprints" / "sprint-0" / "story-0.3-macos"
E4B_DECISION: Final = (
    ROOT / "model-profiles" / "candidates" / "gemma-4-e4b" / "feasibility-disposition.json"
)
ARTIFACT_NAMES: Final = (
    "adapter-disposition.json",
    "macos-native-result.json",
    "macos-native-server.log",
    "raw-source-receipt.json",
    "summary.md",
)
EVALUATION_ROOT = re.compile(r"/Users/[^/\s'\"]+/.local/share/agentmage/evaluation")
SENSITIVE_TEXT = re.compile(
    r"(?:/Users/[^/\s]+|[A-Za-z0-9._%+-]+@[A-Za-z0-9.-]+\.[A-Za-z]{2,}|"
    r"-----BEGIN [A-Z ]*PRIVATE KEY-----|(?:password|api[_ -]?key|secret)\s*[:=])",
    re.IGNORECASE,
)


class Story03MacEvidenceError(ValueError):
    """Raised when Mac-native evidence cannot be admitted or reconciled."""


def json_bytes(value: object) -> bytes:
    return (json.dumps(value, indent=2, sort_keys=True) + "\n").encode("utf-8")


def sha256(content: bytes) -> str:
    return hashlib.sha256(content).hexdigest()


def read_json(path: Path) -> dict[str, Any]:
    value = json.loads(path.read_text(encoding="utf-8"))
    if not isinstance(value, dict):
        raise Story03MacEvidenceError(f"expected a JSON object: {path}")
    return value


def resolve_revision(revision: str) -> str:
    result = subprocess.run(
        ["git", "rev-parse", "--verify", f"{revision}^{{commit}}"],
        cwd=ROOT,
        check=False,
        capture_output=True,
        text=True,
    )
    if result.returncode != 0:
        raise Story03MacEvidenceError(f"cannot resolve verification revision {revision}")
    return result.stdout.strip()


def sanitize_server_log(content: str) -> str:
    sanitized = EVALUATION_ROOT.sub("<EVALUATION_DATA_ROOT>", content)
    if SENSITIVE_TEXT.search(sanitized):
        raise Story03MacEvidenceError(
            "macOS server log still contains a sensitive path or identifier pattern"
        )
    return sanitized


def failed_cases(result: dict[str, Any]) -> list[str]:
    return [str(case["case_id"]) for case in result["cases"] if not case["passed"]]


def failed_thresholds(result: dict[str, Any]) -> list[str]:
    return [
        str(name)
        for name, value in result["threshold_results"].items()
        if not value["passed"]
    ]


def evidence_date(result: dict[str, Any]) -> str:
    timestamp = result.get("completed_at_epoch")
    if not isinstance(timestamp, (int, float)):
        raise Story03MacEvidenceError("macOS result completion timestamp is invalid")
    return datetime.fromtimestamp(timestamp, tz=timezone.utc).date().isoformat()


def adapter_disposition(
    result: dict[str, Any], verification_revision: str
) -> dict[str, object]:
    hardware = result["runtime_settings"]["hardware"]
    decision = read_json(E4B_DECISION)
    if decision.get("decision", {}).get("status") != "REJECTED":
        raise Story03MacEvidenceError("the controlling E4B decision is not REJECTED")
    return {
        "schema_version": 1,
        "record_type": "story_0_3_macos_adapter_disposition",
        "evidence_date": evidence_date(result),
        "verification_revision": verification_revision,
        "result_source_revision": result["source_revision"],
        "data_classification": "public_synthetic_only",
        "macos_native": {
            "adapter_id": ADAPTER_ID,
            "execution_status": "COMPLETE",
            "quality_status": result["status"],
            "hardware": hardware,
            "trials_completed": sum(case["trials_completed"] for case in result["cases"]),
            "failed_cases": failed_cases(result),
            "failed_thresholds": failed_thresholds(result),
            "metrics": result["metrics"],
            "identities": result["identities"],
            "runtime_settings": {
                key: value
                for key, value in result["runtime_settings"].items()
                if key != "hardware"
            },
        },
        "task_0_3_2_2_status": "COMPLETE",
        "linux_evidence_substituted": False,
        "controlling_profile_decision": "REJECTED",
        "candidate_enabled": False,
        "automatic_fallback": False,
        "production_support_claim": False,
        "independent_review_performed": False,
        "release_approval": False,
        "limitations": [
            "This extension records the required physical MacBook Pro M5 feasibility run only.",
            "The raw source directory remains outside the repository; its exact file hashes are retained in the source receipt.",
            "Only the machine-local evaluation root is replaced in the admitted public server log.",
            "The Mac result cannot reverse the mandatory Linux failures or the controlling rejected-and-disabled E4B decision.",
            "Full platform, product-security, independent-review, and release protocols remain assigned to their owning sprints.",
        ],
    }


def source_receipt(result_dir: Path, admitted_log: bytes) -> dict[str, object]:
    source_manifest = (result_dir / "manifest.json").read_bytes()
    source_result = (result_dir / "results.json").read_bytes()
    source_log = (result_dir / "server.log").read_bytes()
    return {
        "schema_version": 1,
        "record_type": "story_0_3_macos_raw_source_receipt",
        "source_directory_committed": False,
        "source_directory_retained_outside_repository": True,
        "source_files": {
            "manifest.json": {"sha256": sha256(source_manifest), "size": len(source_manifest)},
            "results.json": {"sha256": sha256(source_result), "size": len(source_result)},
            "server.log": {"sha256": sha256(source_log), "size": len(source_log)},
        },
        "admitted_files": {
            "macos-native-result.json": {
                "sha256": sha256(source_result),
                "byte_identical_to_source": True,
            },
            "macos-native-server.log": {
                "sha256": sha256(admitted_log),
                "byte_identical_to_source": admitted_log == source_log,
                "permitted_transformation": "evaluation_data_root_placeholder_only",
            },
        },
        "contains_user_data": False,
        "release_approval": False,
    }


def metric(value: object, digits: int = 3) -> str:
    return f"{value:.{digits}f}" if isinstance(value, (int, float)) else "not measured"


def summary_markdown(
    result: dict[str, Any], disposition: dict[str, object]
) -> bytes:
    cases_passed = sum(bool(case["passed"]) for case in result["cases"])
    thresholds_passed = sum(
        bool(item["passed"]) for item in result["threshold_results"].values()
    )
    metrics = result["metrics"]
    hardware = result["runtime_settings"]["hardware"]
    text = f"""# Story 0.3 Mac-Native Evidence Summary

| Field | Value |
|---|---|
| Result source revision | `{result['source_revision']}` |
| Verification revision | `{disposition['verification_revision']}` |
| Evidence date | {disposition['evidence_date']} |
| Adapter | `{ADAPTER_ID}` |
| Hardware | {hardware['machine_name']} / {hardware['chip']} |
| Runtime | `llama.cpp b10333` / Metal |
| Result | `{result['status']}` |
| Fixed corpus trials | {sum(case['trials_completed'] for case in result['cases'])} completed |
| Corpus cases | {cases_passed} passed, {len(result['cases']) - cases_passed} failed |
| Global thresholds | {thresholds_passed} passed, {len(result['threshold_results']) - thresholds_passed} failed |
| E4B decision | `REJECTED` and disabled |
| Independent review | Not performed |
| Release approval | No |

The required physical MacBook Pro M5 feasibility path completed without substituting Linux evidence. The authoritative raw result records every trial, failed case, threshold, runtime identity, hardware field, memory sample, loopback-listener check, and disconnected-network observation.

Key measurements were {metric(metrics['minimum_generation_tokens_per_second'], 2)} minimum generated tokens per second, {metric(metrics['maximum_time_to_first_token_seconds'])} seconds maximum time to first token, {metric(metrics['cancellation_max_seconds'])} seconds maximum cancellation latency, {metric(metrics['maximum_gpu_or_unified_memory_fraction'])} maximum unified-memory fraction, and {metrics['post_install_egress_bytes']} post-install egress bytes.

This result does not enable E4B or alter its controlling rejection. The source receipt binds the untouched external result bundle; the admitted log changes only the machine-local evaluation root to `<EVALUATION_DATA_ROOT>`. No production support, independent review, or release approval is claimed.
"""
    return text.encode("utf-8")


def build_bundle(result_dir: Path, verification_revision: str) -> dict[str, bytes]:
    failures = validate_directory(result_dir)
    if failures:
        raise Story03MacEvidenceError("source Mac result is invalid: " + "; ".join(failures))
    result = read_json(result_dir / "results.json")
    sanitized_log = sanitize_server_log(
        (result_dir / "server.log").read_text(encoding="utf-8")
    ).encode("utf-8")
    disposition = adapter_disposition(result, verification_revision)
    return {
        "adapter-disposition.json": json_bytes(disposition),
        "macos-native-result.json": (result_dir / "results.json").read_bytes(),
        "macos-native-server.log": sanitized_log,
        "raw-source-receipt.json": json_bytes(source_receipt(result_dir, sanitized_log)),
        "summary.md": summary_markdown(result, disposition),
    }


def build_manifest(
    bundle: dict[str, bytes], disposition: dict[str, Any]
) -> dict[str, object]:
    return {
        "schema_version": 1,
        "bundle_id": "sprint-0-story-0.3-macos-native-extension-evidence",
        "evidence_date": disposition["evidence_date"],
        "result_source_revision": disposition["result_source_revision"],
        "verification_revision": disposition["verification_revision"],
        "scope": "Story 0.3 physical MacBook Pro M5 native llama.cpp/Metal feasibility",
        "files": [
            {"path": name, "sha256": sha256(bundle[name]), "size": len(bundle[name])}
            for name in ARTIFACT_NAMES
        ],
    }


def write_bundle(output: Path, result_dir: Path, verification_revision: str) -> None:
    if output.exists():
        raise Story03MacEvidenceError(f"refusing to overwrite existing evidence: {output}")
    revision = resolve_revision(verification_revision)
    bundle = build_bundle(result_dir, revision)
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
        result = read_json(output / "macos-native-result.json")
        disposition = read_json(output / "adapter-disposition.json")
        receipt = read_json(output / "raw-source-receipt.json")
        corpus = load_corpus(DEFAULT_CORPUS)
        model_admission = read_json(DEFAULT_MODEL_ADMISSION)
        runtime_admission = load_runtime_record(DEFAULT_RUNTIME_ADMISSION)
    except (OSError, json.JSONDecodeError, ValueError, Story03MacEvidenceError) as error:
        return [f"cannot load Story 0.3 Mac evidence: {error}"]
    entries = manifest.get("files")
    if not isinstance(entries, list) or [
        entry.get("path") for entry in entries if isinstance(entry, dict)
    ] != list(ARTIFACT_NAMES):
        failures.append("Mac evidence manifest membership or order is invalid")
    else:
        for entry in entries:
            path = output / str(entry.get("path"))
            try:
                content = path.read_bytes()
            except OSError as error:
                failures.append(f"cannot read Mac evidence artifact {path.name}: {error}")
                continue
            if entry.get("sha256") != sha256(content):
                failures.append(f"Mac evidence hash mismatch: {path.name}")
            if entry.get("size") != len(content):
                failures.append(f"Mac evidence size mismatch: {path.name}")
    try:
        failures.extend(validate_result(result, corpus, model_admission, runtime_admission))
    except (OSError, ValueError) as error:
        failures.append(f"Mac result validation failed: {error}")
    if disposition.get("macos_native", {}).get("execution_status") != "COMPLETE":
        failures.append("Mac execution is not complete")
    if disposition.get("task_0_3_2_2_status") != "COMPLETE":
        failures.append("Sub-task 0.3.2.2 is not complete in the disposition")
    if disposition.get("linux_evidence_substituted") is not False:
        failures.append("Mac disposition substitutes Linux evidence")
    if disposition.get("controlling_profile_decision") != "REJECTED":
        failures.append("Mac evidence changes the controlling E4B rejection")
    for prohibited in (
        "candidate_enabled",
        "automatic_fallback",
        "production_support_claim",
        "independent_review_performed",
        "release_approval",
    ):
        if disposition.get(prohibited) is not False:
            failures.append(f"Mac evidence overstates {prohibited}")
    result_bytes = (output / "macos-native-result.json").read_bytes()
    log_bytes = (output / "macos-native-server.log").read_bytes()
    admitted = receipt.get("admitted_files", {})
    if admitted.get("macos-native-result.json", {}).get("sha256") != sha256(result_bytes):
        failures.append("Mac source receipt does not bind the admitted result")
    if admitted.get("macos-native-server.log", {}).get("sha256") != sha256(log_bytes):
        failures.append("Mac source receipt does not bind the admitted server log")
    if receipt.get("source_directory_retained_outside_repository") is not True:
        failures.append("Mac source receipt does not retain the external raw bundle")
    try:
        sanitize_server_log(log_bytes.decode("utf-8"))
    except (UnicodeDecodeError, Story03MacEvidenceError) as error:
        failures.append(f"admitted Mac server log is invalid: {error}")
    if manifest.get("result_source_revision") != result.get("source_revision"):
        failures.append("Mac manifest result revision does not reconcile")
    if manifest.get("verification_revision") != disposition.get("verification_revision"):
        failures.append("Mac manifest verification revision does not reconcile")
    return failures


def parser() -> argparse.ArgumentParser:
    value = argparse.ArgumentParser(description=__doc__)
    value.add_argument("--output", type=Path, default=DEFAULT_OUTPUT)
    value.add_argument("--result-dir", type=Path)
    value.add_argument("--verification-revision", default="HEAD")
    value.add_argument("--write", action="store_true")
    return value


def main(argv: list[str] | None = None) -> int:
    args = parser().parse_args(argv)
    try:
        if args.write:
            if args.result_dir is None:
                raise Story03MacEvidenceError("--result-dir is required with --write")
            write_bundle(args.output, args.result_dir, args.verification_revision)
        failures = check_bundle(args.output)
    except (OSError, json.JSONDecodeError, Story03MacEvidenceError) as error:
        print(f"Story 0.3 Mac evidence failed: {error}", file=sys.stderr)
        return 1
    if failures:
        for failure in failures:
            print(f"- {failure}", file=sys.stderr)
        return 1
    print(f"Validated Story 0.3 Mac evidence at {args.output}.")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
