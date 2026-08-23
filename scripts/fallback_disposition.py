#!/usr/bin/env python3
"""Build and validate the Gemma 4 12B Unified fallback disposition."""

from __future__ import annotations

import argparse
import hashlib
import json
import re
import subprocess
import sys
from pathlib import Path
from typing import Any, Final

try:
    from scripts.fallback_admission import validate_all as validate_admission
    from scripts.model_corpus import load_corpus
    from scripts.model_feasibility import read_json, validate_result
    from scripts.story_0_3_fallback_evidence import check_bundle
except ModuleNotFoundError:
    from fallback_admission import validate_all as validate_admission
    from model_corpus import load_corpus
    from model_feasibility import read_json, validate_result
    from story_0_3_fallback_evidence import check_bundle


ROOT: Final = Path(__file__).resolve().parents[1]
PROFILE_ROOT: Final = ROOT / "model-profiles" / "candidates" / "gemma-4-12b-unified"
DEFAULT_RECORD: Final = PROFILE_ROOT / "feasibility-disposition.json"
POLICY: Final = ROOT / "MODEL-PROVENANCE-POLICY.md"
SOURCE_ADMISSION: Final = PROFILE_ROOT / "source-admission.json"
ARTIFACT_ADMISSION: Final = PROFILE_ROOT / "artifact-admission.json"
EVALUATION_PLAN: Final = PROFILE_ROOT / "evaluation-plan.json"
TRIGGER_DISPOSITION: Final = (
    ROOT
    / "model-profiles"
    / "candidates"
    / "gemma-4-e4b"
    / "feasibility-disposition.json"
)
CORPUS: Final = ROOT / "model-profiles" / "evaluation" / "corpus-v1.json"
EVIDENCE: Final = ROOT / "artifacts" / "sprints" / "sprint-0" / "story-0.3-fallback"
EVIDENCE_MANIFEST: Final = EVIDENCE / "evidence-manifest.json"
STAGING_RECEIPT: Final = EVIDENCE / "artifact-staging.json"
NATIVE_RESULT: Final = EVIDENCE / "native-linux-result.json"
DMR_RESULT: Final = EVIDENCE / "dmr-linux-result.json"
EVALUATED_AT: Final = "2026-08-10T10:45:00Z"
COMMIT: Final = re.compile(r"^[0-9a-f]{40}$")


class FallbackDispositionError(ValueError):
    """Raised when the fallback disposition cannot be built or reconciled."""


def canonical_json(value: object) -> bytes:
    return (json.dumps(value, indent=2, sort_keys=True) + "\n").encode("utf-8")


def relative(path: Path) -> str:
    return str(path.relative_to(ROOT))


def sha256_file(path: Path) -> str:
    digest = hashlib.sha256()
    with path.open("rb") as handle:
        for block in iter(lambda: handle.read(1024 * 1024), b""):
            digest.update(block)
    return digest.hexdigest()


def file_binding(path: Path) -> dict[str, object]:
    return {"path": relative(path), "sha256": sha256_file(path)}


def revision_file_binding(path: Path, revision: str) -> dict[str, object]:
    result = subprocess.run(
        ["git", "show", f"{revision}:{relative(path)}"],
        cwd=ROOT,
        check=False,
        capture_output=True,
    )
    if result.returncode != 0:
        raise FallbackDispositionError(
            f"cannot read {relative(path)} at fallback decision revision {revision}"
        )
    return {"path": relative(path), "sha256": hashlib.sha256(result.stdout).hexdigest()}


def resolve_revision(revision: str) -> str:
    result = subprocess.run(
        ["git", "rev-parse", "--verify", f"{revision}^{{commit}}"],
        cwd=ROOT,
        check=False,
        capture_output=True,
        text=True,
    )
    if result.returncode != 0:
        raise FallbackDispositionError(
            f"cannot resolve fallback decision source revision {revision}"
        )
    return result.stdout.strip()


def result_record(result: dict[str, Any], path: Path) -> dict[str, object]:
    return {
        "adapter_id": result["adapter_id"],
        "execution_status": "COMPLETE",
        "quality_status": result["status"],
        "result": file_binding(path),
        "result_source_revision": result["source_revision"],
        "runner_transform_version": result["runner_transform_version"],
        "trials_completed": sum(case["trials_completed"] for case in result["cases"]),
        "failed_cases": [
            case["case_id"] for case in result["cases"] if not case["passed"]
        ],
        "failed_thresholds": [
            name
            for name, value in result["threshold_results"].items()
            if not value["passed"]
        ],
        "metrics": result["metrics"],
        "threshold_results": result["threshold_results"],
        "runtime_settings": result["runtime_settings"],
        "identities": result["identities"],
    }


def build_record(decision_source_revision: str) -> dict[str, object]:
    revision = resolve_revision(decision_source_revision)
    admission_failures = validate_admission()
    evidence_failures = check_bundle(EVIDENCE)
    if admission_failures or evidence_failures:
        raise FallbackDispositionError(
            "cannot build from invalid fallback inputs: "
            + "; ".join(admission_failures + evidence_failures)
        )

    corpus = load_corpus(CORPUS)
    source_admission = read_json(SOURCE_ADMISSION)
    artifact_admission = read_json(ARTIFACT_ADMISSION)
    evaluation_plan = read_json(EVALUATION_PLAN)
    trigger = read_json(TRIGGER_DISPOSITION)
    evidence_manifest = read_json(EVIDENCE_MANIFEST)
    staging = read_json(STAGING_RECEIPT)
    evidence_disposition = read_json(EVIDENCE / "adapter-disposition.json")
    native = read_json(NATIVE_RESULT)
    dmr = read_json(DMR_RESULT)
    result_failures = validate_result(native, corpus, artifact_admission)
    result_failures.extend(validate_result(dmr, corpus, artifact_admission))
    if result_failures:
        raise FallbackDispositionError(
            "cannot build from invalid fallback results: " + "; ".join(result_failures)
        )

    native_record = result_record(native, NATIVE_RESULT)
    dmr_record = result_record(dmr, DMR_RESULT)
    return {
        "schema_version": 1,
        "record_type": "fallback_model_feasibility_disposition",
        "profile_id": "gemma-4-12b-unified-it",
        "evaluated_at": EVALUATED_AT,
        "decision_source_revision": revision,
        "data_classification": "public_synthetic_only",
        "scope": (
            "complete fallback admission and fixed-corpus gate on every available Fedora "
            "runtime path; unavailable MacBook Pro M5 evidence is not substituted"
        ),
        "policy": revision_file_binding(POLICY, revision),
        "trigger_disposition": {
            **file_binding(TRIGGER_DISPOSITION),
            "profile_id": trigger["profile_id"],
            "status": trigger["decision"]["status"],
        },
        "evaluation_plan": {
            **file_binding(EVALUATION_PLAN),
            "candidate_state_at_start": evaluation_plan["candidate_state"],
            "threshold_change_authorized": evaluation_plan["corpus"][
                "threshold_change_authorized"
            ],
        },
        "corpus": {
            **file_binding(CORPUS),
            "corpus_id": corpus["corpus_id"],
            "version": corpus["version"],
            "thresholds": corpus["global_thresholds"],
            "thresholds_changed_after_results": False,
        },
        "admission": {
            "source": {
                **file_binding(SOURCE_ADMISSION),
                "status": source_admission["decision"]["status"],
                "blocker_codes": [
                    item["code"] for item in source_admission["decision"]["blockers"]
                ],
            },
            "artifact": {
                **file_binding(ARTIFACT_ADMISSION),
                "status": artifact_admission["decision"]["status"],
                "blocker_codes": [
                    item["code"] for item in artifact_admission["decision"]["blockers"]
                ],
                "evaluation_staging_completed_after_initial_record": staging["state"][
                    "evaluation_staging_complete"
                ],
            },
        },
        "evidence": {
            "manifest": file_binding(EVIDENCE_MANIFEST),
            "artifact_staging": file_binding(STAGING_RECEIPT),
            "bundle_id": evidence_manifest["bundle_id"],
            "verification_revision": evidence_manifest["verification_revision"],
        },
        "adapters": {
            "native_linux": native_record,
            "dmr_linux": {
                **dmr_record,
                "deployment": evidence_disposition["dmr_linux"]["deployment"],
            },
            "macos_native": evidence_disposition["macos_native"],
        },
        "cross_adapter": {
            "available_linux_paths_complete": True,
            "shared_failed_cases": evidence_disposition["cross_adapter"][
                "shared_failed_cases"
            ],
            "shared_failed_thresholds": evidence_disposition["cross_adapter"][
                "shared_failed_thresholds"
            ],
            "hardware_fit_on_available_linux_paths": True,
            "zero_egress_on_available_linux_paths": True,
            "runtime_differences": [
                "Native used llama.cpp b10333 with Vulkan over isolated loopback HTTP; DMR used llama.cpp b9879 with CUDA over a bind-mounted Unix socket.",
                "The exact DMR image ran under rootless Podman 5.8.4 compatibility deployment; Docker Engine was not directly tested.",
                "The DMR container required manual NVIDIA device exposure, an explicit bundled CUDA compatibility-library path, and disabled SELinux label separation.",
                "Native preserved repository facts and unsupported-action refusal while DMR measured 0.0 repository-fact accuracy and a 0.667 unsupported-action rate.",
            ],
        },
        "decision": {
            "status": "REJECTED",
            "candidate_enabled": False,
            "quality_failure_independently_dispositive": True,
            "admission_failure_independently_blocking": True,
            "reasons": [
                {
                    "code": "MANDATORY-TOOL-CALL-VALIDITY-FAILED",
                    "detail": "Both available Linux adapters measured 0.0 against the fixed 1.0 threshold.",
                },
                {
                    "code": "NATIVE-MANDATORY-CITATION-RECALL-FAILED",
                    "detail": "Native Linux measured 0.5 against the fixed 1.0 threshold.",
                },
                {
                    "code": "DMR-MANDATORY-GROUNDING-AND-ACTION-CONTROL-FAILED",
                    "detail": (
                        "The DMR path measured 0.0 repository-fact accuracy, 0.5 citation "
                        "precision, 0.2 schema validity, and a 0.667 unsupported-action rate."
                    ),
                },
                {
                    "code": "ADMISSION-BLOCKERS-REMAIN",
                    "detail": (
                        "Source lineage, reproducible selected-GGUF conversion, and direct "
                        "Docker Engine verification remain blocked; evaluation staging does "
                        "not resolve those admission findings."
                    ),
                },
            ],
            "macos_effect": (
                "The missing Mac result blocks a macOS support claim but cannot reverse "
                "mandatory failures on the required Fedora reference platform."
            ),
            "remediation": [
                "Do not weaken or retune corpus-v1 thresholds after viewing these results.",
                "Keep Gemma 4 12B Unified unavailable to profile selection and user data.",
                "Re-open only through a versioned plan after an artifact, adapter, prompt transform, or corpus revision and complete reruns.",
                "Do not select another model automatically or silently; any future candidate requires separate admission and an explicit decision.",
            ],
        },
        "state": {
            "task_0_3_2_4": "COMPLETE_REJECTED",
            "available_platform_evaluation_complete": True,
            "cross_platform_evaluation_complete": False,
            "candidate_state": "REJECTED_DISABLED",
            "automatic_switch": False,
            "activation_authorized": False,
        },
        "review": {
            "independent_review_performed": False,
            "release_approval": False,
        },
    }


def validate_record(record: dict[str, object], check_revision: bool = True) -> list[str]:
    failures: list[str] = []
    revision = str(record.get("decision_source_revision", ""))
    if not COMMIT.fullmatch(revision):
        return ["fallback decision source revision is not immutable"]
    if check_revision:
        try:
            resolve_revision(revision)
        except FallbackDispositionError as error:
            failures.append(str(error))
    try:
        expected = build_record(revision)
    except (OSError, KeyError, TypeError, ValueError, FallbackDispositionError) as error:
        failures.append(f"cannot reconstruct fallback disposition: {error}")
        return failures
    if record != expected:
        failures.append("fallback disposition does not match its hash-bound source evidence")
    decision = record.get("decision", {})
    state = record.get("state", {})
    review = record.get("review", {})
    if not isinstance(decision, dict) or (
        decision.get("status") != "REJECTED"
        or decision.get("candidate_enabled") is not False
        or decision.get("quality_failure_independently_dispositive") is not True
        or decision.get("admission_failure_independently_blocking") is not True
    ):
        failures.append("fallback rejection state is not fail-closed")
    if not isinstance(state, dict) or (
        state.get("task_0_3_2_4") != "COMPLETE_REJECTED"
        or state.get("candidate_state") != "REJECTED_DISABLED"
        or state.get("automatic_switch") is not False
        or state.get("activation_authorized") is not False
        or state.get("cross_platform_evaluation_complete") is not False
    ):
        failures.append("fallback state overstates completion or permits activation")
    if review != {"independent_review_performed": False, "release_approval": False}:
        failures.append("fallback disposition overstates review or release approval")
    return failures


def load_record(path: Path = DEFAULT_RECORD) -> dict[str, object]:
    value = json.loads(path.read_text(encoding="utf-8"))
    if not isinstance(value, dict):
        raise FallbackDispositionError("fallback disposition must be an object")
    return value


def write_record(path: Path, decision_source_revision: str) -> None:
    if path.exists():
        raise FallbackDispositionError(f"refusing to overwrite existing disposition: {path}")
    record = build_record(decision_source_revision)
    failures = validate_record(record)
    if failures:
        raise FallbackDispositionError(
            "generated fallback disposition is invalid: " + "; ".join(failures)
        )
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_bytes(canonical_json(record))


def main(argv: list[str] | None = None) -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--record", type=Path, default=DEFAULT_RECORD)
    parser.add_argument("--decision-source-revision", default="HEAD")
    parser.add_argument("--write", action="store_true")
    args = parser.parse_args(argv)
    try:
        if args.write:
            write_record(args.record, args.decision_source_revision)
        record = load_record(args.record)
        failures = validate_record(record)
    except (OSError, json.JSONDecodeError, FallbackDispositionError, ValueError) as error:
        print(f"Fallback disposition validation failed: {error}", file=sys.stderr)
        return 1
    if failures:
        print("Fallback disposition validation failed:", file=sys.stderr)
        for failure in failures:
            print(f"- {failure}", file=sys.stderr)
        return 1
    print(f"Validated Gemma 4 12B fallback disposition at {relative(args.record)}.")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
