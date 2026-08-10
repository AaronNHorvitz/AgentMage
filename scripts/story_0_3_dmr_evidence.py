#!/usr/bin/env python3
"""Build and verify immutable Docker Model Runner evidence for Story 0.3."""

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
DEFAULT_OUTPUT: Final = ROOT / "artifacts" / "sprints" / "sprint-0" / "story-0.3-dmr"
NATIVE_EVIDENCE: Final = ROOT / "artifacts" / "sprints" / "sprint-0" / "story-0.3"
EVIDENCE_DATE: Final = "2026-08-10"
ARTIFACT_NAMES: Final = (
    "adapter-disposition.json",
    "dmr-linux-diagnostic-result.json",
    "dmr-linux-result.json",
    "dmr-linux-server.log",
    "summary.md",
)
FINAL_FAILED_CASES: Final = ("CITE-001", "TOOL-001")
FINAL_FAILED_THRESHOLDS: Final = ("citation_recall", "tool_call_valid_rate")
DIAGNOSTIC_FAILED_CASES: Final = ("CITE-001", "TOOL-001", "CONTEXT-001", "PERF-001")
HOME_PATH = re.compile(r"/(?:var/)?home/[^/\s'\"]+/.local/share/agentmage/evaluation")
SENSITIVE_TEXT = re.compile(
    r"(?:/(?:var/)?home/[^/\s]+|[A-Za-z0-9._%+-]+@[A-Za-z0-9.-]+\.[A-Za-z]{2,}|"
    r"-----BEGIN [A-Z ]*PRIVATE KEY-----|(?:password|api[_ -]?key|secret)\s*[:=])",
    re.IGNORECASE,
)


class Story03DMREvidenceError(ValueError):
    """Raised when DMR evidence cannot be built or reconciled."""


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
        raise Story03DMREvidenceError(f"cannot resolve verification revision {revision}")
    return result.stdout.strip()


def sanitize_server_log(content: str) -> str:
    sanitized = HOME_PATH.sub("<EVALUATION_DATA_ROOT>", content)
    if SENSITIVE_TEXT.search(sanitized):
        raise Story03DMREvidenceError(
            "Docker Model Runner log still contains a sensitive path or identifier pattern"
        )
    return sanitized


def failed_cases(result: dict[str, object]) -> list[str]:
    return [case["case_id"] for case in result["cases"] if not case["passed"]]


def failed_thresholds(result: dict[str, object]) -> list[str]:
    return [
        name for name, value in result["threshold_results"].items() if not value["passed"]
    ]


def adapter_disposition(
    result: dict[str, object],
    diagnostic: dict[str, object],
    verification_revision: str,
) -> dict[str, object]:
    native_manifest = (NATIVE_EVIDENCE / "evidence-manifest.json").read_bytes()
    return {
        "schema_version": 1,
        "record_type": "story_0_3_dmr_adapter_disposition",
        "evidence_date": EVIDENCE_DATE,
        "result_source_revision": result["source_revision"],
        "diagnostic_source_revision": diagnostic["source_revision"],
        "verification_revision": verification_revision,
        "data_classification": "public_synthetic_only",
        "native_linux": {
            "adapter_id": "linux-native-vulkan",
            "execution_status": "COMPLETE",
            "quality_status": "FAIL",
            "evidence_path": "artifacts/sprints/sprint-0/story-0.3",
            "evidence_manifest_sha256": sha256(native_manifest),
        },
        "docker_model_runner": {
            "adapter_id": "linux-docker-model-runner-cuda",
            "execution_status": "COMPLETE",
            "quality_status": "FAIL",
            "compatibility_container_engine": "Podman 5.8.4",
            "docker_engine_directly_tested": False,
            "exact_dmr_image_used": True,
            "runtime_image_digest": result["identities"]["runtime_image"],
            "model_manifest_digest": "sha256:" + result["identities"]["model_manifest"],
            "failed_cases": failed_cases(result),
            "failed_thresholds": failed_thresholds(result),
        },
        "token_counter_diagnostic": {
            "execution_status": "COMPLETE",
            "result_status": diagnostic["status"],
            "failed_cases": failed_cases(diagnostic),
            "finding": (
                "Cached OpenAI usage values could not serve as total prompt-token counts; "
                "transform 1.1.1 disabled prompt caching and consumed exact structured "
                "over-context counts."
            ),
            "superseded_for_decision_by_transform": result["runner_transform_version"],
        },
        "macos_native": {
            "adapter_id": "macos-native-metal",
            "execution_status": "BLOCKED",
            "blocker": "The required MacBook Pro M5 evaluation host is not available.",
            "linux_evidence_substituted": False,
        },
        "task_0_3_2_1_status": "COMPLETE",
        "task_0_3_2_status": "PENDING_MACOS_AND_DECISION",
        "profile_decision": "PENDING",
        "fallback_evaluated": False,
        "independent_review_performed": False,
        "release_approval": False,
        "limitations": [
            "The exact Docker Model Runner image ran under rootless Podman compatibility deployment; Docker Engine itself was not tested.",
            "The earlier immutable native summary states 72 trials; the fixed corpus and raw result contain 82 trials and remain authoritative.",
            "The Docker Model Runner log includes startup and diagnostic activity from the dedicated public-synthetic evaluation container.",
            "MacBook Pro M5 evidence remains required before a cross-adapter profile decision.",
            "The blocked source-lineage and GGUF reproducibility findings remain unresolved.",
        ],
    }


def summary_markdown(
    result: dict[str, object], disposition: dict[str, object]
) -> bytes:
    passed_cases = sum(case["passed"] for case in result["cases"])
    passed_threshold_count = sum(
        item["passed"] for item in result["threshold_results"].values()
    )
    trial_count = sum(case["trials_completed"] for case in result["cases"])
    metrics = result["metrics"]
    text = f"""# Story 0.3 Docker Model Runner Evidence Summary

| Field | Value |
|---|---|
| Result source revision | `{result['source_revision']}` |
| Verification revision | `{disposition['verification_revision']}` |
| Evidence date | {EVIDENCE_DATE} |
| Adapter | `linux-docker-model-runner-cuda` |
| Runtime deployment | Exact DMR image under rootless Podman 5.8.4 |
| Docker Engine directly tested | No |
| DMR result | `{result['status']}` |
| Fixed corpus trials | {trial_count} completed |
| Corpus cases | {passed_cases} passed, {len(result['cases']) - passed_cases} failed |
| Global thresholds | {passed_threshold_count} passed, {len(result['threshold_results']) - passed_threshold_count} failed |
| Fedora native plus DMR task | `COMPLETE` |
| macOS adapter | `BLOCKED` - required hardware unavailable |
| Profile decision | `PENDING` |
| Independent review | Not performed |
| Release approval | No |

The corrected DMR run completed all {trial_count} fixed trials. It passed ordinary chat, repository citations, unavailable-write refusal, malformed-output blocking, cancellation, 7,923-token context handling, bounded overflow, performance, memory, and isolated zero-egress cases. It failed required evidence-citation recall and exact tool-argument validity.

Key DMR measurements were {metrics['minimum_generation_tokens_per_second']:.2f} minimum generated tokens per second, {metrics['maximum_time_to_first_token_seconds']:.3f} seconds maximum time to first token, {metrics['cancellation_max_seconds']:.3f} seconds maximum post-cancel quiescence, {metrics['maximum_gpu_or_unified_memory_fraction']:.3f} maximum GPU-memory fraction, and {metrics['post_install_egress_bytes']} post-install egress bytes.

The retained diagnostic run exposed that cached OpenAI usage values in this DMR build are not total prompt-token counts. Transform `1.1.1` disabled prompt caching for count probes and used the structured `n_prompt_tokens` field on over-context responses. The diagnostic result is evidence of the corrected measurement path and is not used for the model-quality decision.

The earlier immutable native summary says 72 trials; the corpus and raw native result contain 82. This bundle records the correction without rewriting historical evidence. Raw JSON results remain authoritative.

The Fedora native and isolated DMR portions of Sub-task 0.3.2.1 are complete. This is not a Docker Engine support claim, final E4B profile decision, fallback authorization, or release approval. The MacBook Pro M5 run, final disposition, and independent review remain pending.
"""
    return text.encode("utf-8")


def build_bundle(
    result_dir: Path,
    diagnostic_result_dir: Path,
    verification_revision: str,
) -> dict[str, bytes]:
    for label, directory in (
        ("final", result_dir),
        ("diagnostic", diagnostic_result_dir),
    ):
        failures = validate_result_directory(directory)
        if failures:
            raise Story03DMREvidenceError(
                f"{label} result bundle is invalid: " + "; ".join(failures)
            )
    result = read_json(result_dir / "results.json")
    diagnostic = read_json(diagnostic_result_dir / "results.json")
    disposition = adapter_disposition(result, diagnostic, verification_revision)
    server_log = (result_dir / "server.log").read_text(encoding="utf-8")
    return {
        "adapter-disposition.json": json_bytes(disposition),
        "dmr-linux-diagnostic-result.json": (diagnostic_result_dir / "results.json").read_bytes(),
        "dmr-linux-result.json": (result_dir / "results.json").read_bytes(),
        "dmr-linux-server.log": sanitize_server_log(server_log).encode("utf-8"),
        "summary.md": summary_markdown(result, disposition),
    }


def build_manifest(
    bundle: dict[str, bytes], disposition: dict[str, object]
) -> dict[str, object]:
    return {
        "schema_version": 1,
        "bundle_id": "sprint-0-story-0.3-dmr-linux-evidence",
        "evidence_date": EVIDENCE_DATE,
        "result_source_revision": disposition["result_source_revision"],
        "diagnostic_source_revision": disposition["diagnostic_source_revision"],
        "verification_revision": disposition["verification_revision"],
        "scope": "Story 0.3 Fedora native plus isolated DMR compatibility execution; macOS and final decision remain pending",
        "files": [
            {"path": name, "sha256": sha256(bundle[name]), "size": len(bundle[name])}
            for name in ARTIFACT_NAMES
        ],
    }


def write_bundle(
    output: Path,
    result_dir: Path,
    diagnostic_result_dir: Path,
    verification_revision: str,
) -> None:
    if output.exists():
        raise Story03DMREvidenceError(f"refusing to overwrite existing evidence: {output}")
    verification_revision = resolve_revision(verification_revision)
    bundle = build_bundle(result_dir, diagnostic_result_dir, verification_revision)
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
        result = read_json(output / "dmr-linux-result.json")
        diagnostic = read_json(output / "dmr-linux-diagnostic-result.json")
        disposition = read_json(output / "adapter-disposition.json")
        native_result = read_json(NATIVE_EVIDENCE / "native-linux-result.json")
        corpus = load_corpus(DEFAULT_CORPUS)
        admission = read_json(DEFAULT_ADMISSION)
    except (OSError, json.JSONDecodeError, ValueError) as error:
        return [f"cannot load Story 0.3 DMR evidence: {error}"]

    entries = manifest.get("files")
    if not isinstance(entries, list) or [
        entry.get("path") for entry in entries if isinstance(entry, dict)
    ] != list(ARTIFACT_NAMES):
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

    for label, evidence_result in (
        ("DMR", result),
        ("diagnostic DMR", diagnostic),
        ("native", native_result),
    ):
        for failure in validate_result(evidence_result, corpus, admission):
            failures.append(f"{label} result: {failure}")
    if tuple(failed_cases(result)) != FINAL_FAILED_CASES:
        failures.append("final DMR failed-case set changed")
    if tuple(failed_thresholds(result)) != FINAL_FAILED_THRESHOLDS:
        failures.append("final DMR failed-threshold set changed")
    if tuple(failed_cases(diagnostic)) != DIAGNOSTIC_FAILED_CASES:
        failures.append("diagnostic DMR failed-case set changed")
    if result.get("runner_transform_version") != "1.1.1":
        failures.append("final DMR result is not the corrected transform")
    if sum(case["trials_completed"] for case in result["cases"]) != 82:
        failures.append("final DMR result does not contain all 82 fixed trials")

    dmr = disposition.get("docker_model_runner", {})
    if dmr.get("execution_status") != "COMPLETE" or dmr.get("quality_status") != "FAIL":
        failures.append("DMR disposition does not record complete threshold failure")
    if dmr.get("compatibility_container_engine") != "Podman 5.8.4":
        failures.append("DMR compatibility container engine is not disclosed")
    if dmr.get("docker_engine_directly_tested") is not False:
        failures.append("DMR disposition overstates Docker Engine coverage")
    if dmr.get("exact_dmr_image_used") is not True:
        failures.append("DMR disposition does not bind the exact runtime image")
    if disposition.get("task_0_3_2_1_status") != "COMPLETE":
        failures.append("Fedora native plus DMR task is not complete")
    if disposition.get("macos_native", {}).get("execution_status") != "BLOCKED":
        failures.append("macOS hardware blocker is missing")
    if disposition.get("profile_decision") != "PENDING":
        failures.append("partial evidence overstates the profile decision")
    if disposition.get("fallback_evaluated") is not False:
        failures.append("partial evidence overstates fallback evaluation")
    if disposition.get("independent_review_performed") is not False:
        failures.append("partial evidence overstates independent review")
    if disposition.get("release_approval") is not False:
        failures.append("partial evidence overstates release approval")
    if manifest.get("result_source_revision") != result.get("source_revision"):
        failures.append("manifest result revision does not reconcile")
    if manifest.get("diagnostic_source_revision") != diagnostic.get("source_revision"):
        failures.append("manifest diagnostic revision does not reconcile")
    if manifest.get("verification_revision") != disposition.get("verification_revision"):
        failures.append("manifest verification revision does not reconcile")
    try:
        resolve_revision(str(disposition["verification_revision"]))
        log = (output / "dmr-linux-server.log").read_text(encoding="utf-8")
        if SENSITIVE_TEXT.search(log):
            failures.append("sanitized DMR server log contains sensitive text")
    except (OSError, KeyError, Story03DMREvidenceError) as error:
        failures.append(f"cannot reconcile Story 0.3 DMR evidence: {error}")
    return failures


def main(argv: list[str] | None = None) -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--output", type=Path, default=DEFAULT_OUTPUT)
    parser.add_argument("--result-dir", type=Path)
    parser.add_argument("--diagnostic-result-dir", type=Path)
    parser.add_argument("--verification-revision", default="HEAD")
    parser.add_argument("--write", action="store_true")
    args = parser.parse_args(argv)
    try:
        if args.write:
            if args.result_dir is None or args.diagnostic_result_dir is None:
                raise Story03DMREvidenceError(
                    "--write requires --result-dir and --diagnostic-result-dir"
                )
            write_bundle(
                args.output,
                args.result_dir,
                args.diagnostic_result_dir,
                args.verification_revision,
            )
            return 0
        failures = check_bundle(args.output)
    except (OSError, Story03DMREvidenceError) as error:
        print(f"Story 0.3 DMR evidence error: {error}", file=sys.stderr)
        return 1
    if failures:
        print("Story 0.3 DMR evidence validation failed:", file=sys.stderr)
        for failure in failures:
            print(f"- {failure}", file=sys.stderr)
        return 1
    print(f"Validated Story 0.3 DMR evidence bundle at {output_relative(args.output)}.")
    return 0


def output_relative(output: Path) -> str:
    try:
        return str(output.relative_to(ROOT))
    except ValueError:
        return str(output)


if __name__ == "__main__":
    raise SystemExit(main())
