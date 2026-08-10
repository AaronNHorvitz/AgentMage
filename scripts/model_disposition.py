#!/usr/bin/env python3
"""Build and validate the Gemma 4 E4B feasibility disposition."""

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
    from scripts.model_corpus import load_corpus
    from scripts.model_feasibility import read_json, validate_result
except ModuleNotFoundError:
    from model_corpus import load_corpus
    from model_feasibility import read_json, validate_result


ROOT: Final = Path(__file__).resolve().parents[1]
PROFILE_ROOT: Final = ROOT / "model-profiles" / "candidates" / "gemma-4-e4b"
DEFAULT_RECORD: Final = PROFILE_ROOT / "feasibility-disposition.json"
POLICY: Final = ROOT / "MODEL-PROVENANCE-POLICY.md"
SOURCE_ADMISSION: Final = PROFILE_ROOT / "source-admission.json"
ARTIFACT_ADMISSION: Final = PROFILE_ROOT / "artifact-admission.json"
CORPUS: Final = ROOT / "model-profiles" / "evaluation" / "corpus-v1.json"
NATIVE_EVIDENCE: Final = ROOT / "artifacts" / "sprints" / "sprint-0" / "story-0.3"
DMR_EVIDENCE: Final = ROOT / "artifacts" / "sprints" / "sprint-0" / "story-0.3-dmr"
NATIVE_RESULT: Final = NATIVE_EVIDENCE / "native-linux-result.json"
DMR_RESULT: Final = DMR_EVIDENCE / "dmr-linux-result.json"
NATIVE_MANIFEST: Final = NATIVE_EVIDENCE / "evidence-manifest.json"
DMR_MANIFEST: Final = DMR_EVIDENCE / "evidence-manifest.json"
EVALUATED_AT: Final = "2026-08-10T10:05:00Z"
COMMIT: Final = re.compile(r"^[0-9a-f]{40}$")
SHA256: Final = re.compile(r"^[0-9a-f]{64}$")
RESOURCE_THRESHOLDS: Final = (
    "minimum_generation_tokens_per_second",
    "maximum_time_to_first_token_seconds",
    "maximum_gpu_or_unified_memory_fraction",
    "maximum_system_memory_fraction",
    "maximum_swap_growth_bytes",
)


class ModelDispositionError(ValueError):
    """Raised when the disposition cannot be built or validated."""


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


def resolve_revision(revision: str) -> str:
    result = subprocess.run(
        ["git", "rev-parse", "--verify", f"{revision}^{{commit}}"],
        cwd=ROOT,
        check=False,
        capture_output=True,
        text=True,
    )
    if result.returncode != 0:
        raise ModelDispositionError(f"cannot resolve decision source revision {revision}")
    return result.stdout.strip()


def file_binding(path: Path) -> dict[str, object]:
    return {
        "path": relative(path),
        "sha256": sha256_file(path),
    }


def failed_cases(result: dict[str, Any]) -> list[str]:
    return [str(case["case_id"]) for case in result["cases"] if not case["passed"]]


def failed_thresholds(result: dict[str, Any]) -> list[str]:
    return [
        str(name)
        for name, value in result["threshold_results"].items()
        if not value["passed"]
    ]


def adapter_record(
    result: dict[str, Any],
    result_path: Path,
    manifest_path: Path,
    deployment: dict[str, object],
) -> dict[str, object]:
    resource_results = {
        name: result["threshold_results"][name] for name in RESOURCE_THRESHOLDS
    }
    return {
        "adapter_id": result["adapter_id"],
        "execution_status": "COMPLETE",
        "quality_status": result["status"],
        "result": file_binding(result_path),
        "evidence_manifest": file_binding(manifest_path),
        "result_source_revision": result["source_revision"],
        "runner_transform_version": result["runner_transform_version"],
        "trials_completed": sum(case["trials_completed"] for case in result["cases"]),
        "failed_cases": failed_cases(result),
        "failed_thresholds": failed_thresholds(result),
        "metrics": result["metrics"],
        "threshold_results": result["threshold_results"],
        "resource_fit": {
            "status": (
                "PASS"
                if all(item["passed"] for item in resource_results.values())
                else "FAIL"
            ),
            "threshold_results": resource_results,
        },
        "runtime_settings": result["runtime_settings"],
        "identities": result["identities"],
        "deployment": deployment,
    }


def build_record(decision_source_revision: str) -> dict[str, object]:
    decision_source_revision = resolve_revision(decision_source_revision)
    corpus = load_corpus(CORPUS)
    source_admission = read_json(SOURCE_ADMISSION)
    artifact_admission = read_json(ARTIFACT_ADMISSION)
    native = read_json(NATIVE_RESULT)
    dmr = read_json(DMR_RESULT)

    result_failures = validate_result(native, corpus, artifact_admission)
    result_failures.extend(validate_result(dmr, corpus, artifact_admission))
    if result_failures:
        raise ModelDispositionError(
            "cannot build from invalid feasibility evidence: " + "; ".join(result_failures)
        )

    return {
        "schema_version": 1,
        "record_type": "model_feasibility_disposition",
        "profile_id": "gemma-4-e4b-it",
        "evaluated_at": EVALUATED_AT,
        "decision_source_revision": decision_source_revision,
        "data_classification": "public_synthetic_only",
        "scope": (
            "v0.1 Gemma 4 E4B feasibility on every available Fedora runtime path; "
            "the unavailable MacBook Pro M5 path is not substituted"
        ),
        "policy": file_binding(POLICY),
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
            },
            "artifact": {
                **file_binding(ARTIFACT_ADMISSION),
                "status": artifact_admission["decision"]["status"],
            },
        },
        "adapters": {
            "native_linux": adapter_record(
                native,
                NATIVE_RESULT,
                NATIVE_MANIFEST,
                {
                    "platform": "Fedora Kinoite 44 x86_64",
                    "backend": "Vulkan",
                    "runtime": "llama.cpp b10333",
                    "transport": "loopback_http",
                },
            ),
            "dmr_linux": adapter_record(
                dmr,
                DMR_RESULT,
                DMR_MANIFEST,
                {
                    "platform": "Fedora Kinoite 44 x86_64",
                    "backend": "CUDA 13.3",
                    "runtime": "Docker Model Runner v1.2.6 CUDA with llama.cpp b9879",
                    "transport": "unix_domain_socket",
                    "exact_dmr_image_used": True,
                    "compatibility_container_engine": "Podman 5.8.4",
                    "docker_engine_directly_tested": False,
                },
            ),
            "macos_native": {
                "adapter_id": "macos-native-metal",
                "execution_status": "NOT_RUN",
                "quality_status": "UNKNOWN",
                "reason_code": "REQUIRED_HARDWARE_UNAVAILABLE",
                "reason": "The required MacBook Pro M5 evaluation host is not available.",
                "linux_evidence_substituted": False,
            },
        },
        "cross_adapter": {
            "available_linux_paths_complete": True,
            "shared_failed_cases": sorted(
                set(failed_cases(native)).intersection(failed_cases(dmr))
            ),
            "shared_failed_thresholds": sorted(
                set(failed_thresholds(native)).intersection(failed_thresholds(dmr))
            ),
            "runtime_differences": [
                "Native used llama.cpp b10333 with Vulkan over loopback HTTP; DMR used llama.cpp b9879 with CUDA in the exact pinned DMR image over a Unix socket.",
                "The exact DMR image ran under rootless Podman compatibility deployment; Docker Engine was not directly tested.",
                "Native citation precision was 0.75; DMR citation precision was 1.0. Both produced 0.5 citation recall and 0.0 exact tool-call validity.",
            ],
        },
        "decision": {
            "status": "REJECTED",
            "candidate_enabled": False,
            "quality_failure_independently_dispositive": True,
            "reasons": [
                {
                    "code": "MANDATORY-CITATION-RECALL-FAILED",
                    "detail": "Both completed Linux paths measured 0.5 against the fixed 1.0 threshold.",
                },
                {
                    "code": "MANDATORY-TOOL-CALL-VALIDITY-FAILED",
                    "detail": "Both completed Linux paths measured 0.0 against the fixed 1.0 threshold.",
                },
                {
                    "code": "ADMISSION-BLOCKERS-REMAIN",
                    "detail": "Source lineage, reproducible GGUF conversion, and direct Docker Engine verification remain blocked independently of quality.",
                },
            ],
            "macos_effect": (
                "The missing Mac result blocks a macOS support claim but cannot reverse "
                "mandatory failures on the required Fedora reference platform."
            ),
            "remediation": [
                "Do not weaken or retune corpus-v1 thresholds after viewing these results.",
                "Evaluate Gemma 4 12B Unified as a separate disabled candidate through the complete admission and corpus gate.",
                "Re-open E4B only through a versioned decision after an artifact, adapter, prompt transform, or corpus revision and a complete rerun.",
            ],
        },
        "fallback": {
            "profile_id": "gemma-4-12b-unified",
            "state": "EVALUATION_REQUIRED_DISABLED",
            "trigger": "E4B failed mandatory quality and tool-calling thresholds.",
            "evaluation_complete": False,
            "automatic_switch": False,
            "activation_authorized": False,
        },
        "review": {
            "independent_review_performed": False,
            "release_approval": False,
        },
    }


def validate_file_binding(
    value: object, expected_path: Path, label: str, failures: list[str]
) -> None:
    if not isinstance(value, dict):
        failures.append(f"{label} binding must be an object")
        return
    if value.get("path") != relative(expected_path):
        failures.append(f"{label} path binding changed")
    if value.get("sha256") != sha256_file(expected_path):
        failures.append(f"{label} hash binding changed")


def validate_adapter(
    value: object,
    result: dict[str, Any],
    result_path: Path,
    manifest_path: Path,
    label: str,
    failures: list[str],
) -> None:
    if not isinstance(value, dict):
        failures.append(f"{label} adapter disposition must be an object")
        return
    if value.get("adapter_id") != result["adapter_id"]:
        failures.append(f"{label} adapter identity changed")
    if value.get("execution_status") != "COMPLETE" or value.get("quality_status") != "FAIL":
        failures.append(f"{label} must record completed mandatory failure")
    validate_file_binding(value.get("result"), result_path, f"{label} result", failures)
    validate_file_binding(
        value.get("evidence_manifest"), manifest_path, f"{label} evidence manifest", failures
    )
    expected = adapter_record(result, result_path, manifest_path, value.get("deployment", {}))
    for field in (
        "result_source_revision",
        "runner_transform_version",
        "trials_completed",
        "failed_cases",
        "failed_thresholds",
        "metrics",
        "threshold_results",
        "resource_fit",
        "runtime_settings",
        "identities",
    ):
        if value.get(field) != expected[field]:
            failures.append(f"{label} {field} does not match raw evidence")


def validate_record(record: dict[str, object], check_revision: bool = True) -> list[str]:
    failures: list[str] = []
    expected_fields = {
        "schema_version",
        "record_type",
        "profile_id",
        "evaluated_at",
        "decision_source_revision",
        "data_classification",
        "scope",
        "policy",
        "corpus",
        "admission",
        "adapters",
        "cross_adapter",
        "decision",
        "fallback",
        "review",
    }
    if set(record) != expected_fields:
        failures.append("disposition top-level fields do not match the schema")
        return failures
    if record["schema_version"] != 1 or record["record_type"] != "model_feasibility_disposition":
        failures.append("unsupported feasibility-disposition schema identity")
    if record["profile_id"] != "gemma-4-e4b-it":
        failures.append("unexpected feasibility profile identity")
    if record["data_classification"] != "public_synthetic_only":
        failures.append("feasibility disposition overstates its data boundary")

    revision = str(record["decision_source_revision"])
    if not COMMIT.fullmatch(revision):
        failures.append("decision source revision is not immutable")
    elif check_revision:
        try:
            resolve_revision(revision)
        except ModelDispositionError as error:
            failures.append(str(error))

    validate_file_binding(record["policy"], POLICY, "policy", failures)
    corpus_record = record["corpus"]
    if not isinstance(corpus_record, dict):
        failures.append("corpus disposition must be an object")
    else:
        validate_file_binding(corpus_record, CORPUS, "corpus", failures)
        corpus = load_corpus(CORPUS)
        if corpus_record.get("corpus_id") != corpus["corpus_id"]:
            failures.append("corpus identity changed")
        if corpus_record.get("version") != corpus["version"]:
            failures.append("corpus version changed")
        if corpus_record.get("thresholds") != corpus["global_thresholds"]:
            failures.append("recorded thresholds differ from the fixed corpus")
        if corpus_record.get("thresholds_changed_after_results") is not False:
            failures.append("disposition permits post-result threshold changes")

    source_admission = read_json(SOURCE_ADMISSION)
    artifact_admission = read_json(ARTIFACT_ADMISSION)
    admission = record["admission"]
    if not isinstance(admission, dict) or set(admission) != {"source", "artifact"}:
        failures.append("admission bindings are incomplete")
    else:
        for name, path, source in (
            ("source", SOURCE_ADMISSION, source_admission),
            ("artifact", ARTIFACT_ADMISSION, artifact_admission),
        ):
            value = admission.get(name)
            validate_file_binding(value, path, f"{name} admission", failures)
            if not isinstance(value, dict) or value.get("status") != source["decision"]["status"]:
                failures.append(f"{name} admission status changed")

    native = read_json(NATIVE_RESULT)
    dmr = read_json(DMR_RESULT)
    corpus = load_corpus(CORPUS)
    for label, result in (("native Linux", native), ("DMR Linux", dmr)):
        for failure in validate_result(result, corpus, artifact_admission):
            failures.append(f"{label} raw result: {failure}")
    adapters = record["adapters"]
    if not isinstance(adapters, dict) or set(adapters) != {
        "native_linux",
        "dmr_linux",
        "macos_native",
    }:
        failures.append("adapter dispositions are incomplete")
    else:
        validate_adapter(
            adapters["native_linux"],
            native,
            NATIVE_RESULT,
            NATIVE_MANIFEST,
            "native Linux",
            failures,
        )
        validate_adapter(
            adapters["dmr_linux"],
            dmr,
            DMR_RESULT,
            DMR_MANIFEST,
            "DMR Linux",
            failures,
        )
        dmr_deployment = adapters["dmr_linux"].get("deployment", {})
        if not isinstance(dmr_deployment, dict):
            failures.append("DMR deployment record is missing")
        else:
            if dmr_deployment.get("compatibility_container_engine") != "Podman 5.8.4":
                failures.append("DMR compatibility engine changed")
            if dmr_deployment.get("docker_engine_directly_tested") is not False:
                failures.append("disposition overstates direct Docker Engine testing")
            if dmr_deployment.get("exact_dmr_image_used") is not True:
                failures.append("disposition does not preserve exact DMR image use")
        mac = adapters["macos_native"]
        if not isinstance(mac, dict):
            failures.append("macOS disposition is missing")
        elif (
            mac.get("execution_status") != "NOT_RUN"
            or mac.get("quality_status") != "UNKNOWN"
            or mac.get("reason_code") != "REQUIRED_HARDWARE_UNAVAILABLE"
            or mac.get("linux_evidence_substituted") is not False
        ):
            failures.append("macOS unavailability or non-substitution changed")

    expected_shared_cases = sorted(set(failed_cases(native)).intersection(failed_cases(dmr)))
    expected_shared_thresholds = sorted(
        set(failed_thresholds(native)).intersection(failed_thresholds(dmr))
    )
    cross_adapter = record["cross_adapter"]
    if not isinstance(cross_adapter, dict):
        failures.append("cross-adapter conclusion is missing")
    else:
        if cross_adapter.get("available_linux_paths_complete") is not True:
            failures.append("completed Linux path state changed")
        if cross_adapter.get("shared_failed_cases") != expected_shared_cases:
            failures.append("shared failed-case conclusion changed")
        if cross_adapter.get("shared_failed_thresholds") != expected_shared_thresholds:
            failures.append("shared failed-threshold conclusion changed")
        if len(cross_adapter.get("runtime_differences", [])) < 3:
            failures.append("runtime differences are incomplete")

    mandatory_failure = bool(failed_thresholds(native) or failed_thresholds(dmr))
    decision = record["decision"]
    if not isinstance(decision, dict):
        failures.append("model decision is missing")
    else:
        if mandatory_failure and decision.get("status") != "REJECTED":
            failures.append("mandatory threshold failure must produce REJECTED")
        if decision.get("candidate_enabled") is not False:
            failures.append("rejected E4B candidate cannot be enabled")
        if decision.get("quality_failure_independently_dispositive") is not True:
            failures.append("quality failure is not recorded as independently dispositive")
        reason_codes = {
            reason.get("code")
            for reason in decision.get("reasons", [])
            if isinstance(reason, dict)
        }
        expected_codes = {
            "MANDATORY-CITATION-RECALL-FAILED",
            "MANDATORY-TOOL-CALL-VALIDITY-FAILED",
            "ADMISSION-BLOCKERS-REMAIN",
        }
        if reason_codes != expected_codes:
            failures.append("rejection reason codes are incomplete")
        if len(decision.get("remediation", [])) < 3:
            failures.append("rejection remediation is incomplete")

    fallback = record["fallback"]
    if not isinstance(fallback, dict):
        failures.append("fallback disposition is missing")
    elif (
        fallback.get("profile_id") != "gemma-4-12b-unified"
        or fallback.get("state") != "EVALUATION_REQUIRED_DISABLED"
        or fallback.get("evaluation_complete") is not False
        or fallback.get("automatic_switch") is not False
        or fallback.get("activation_authorized") is not False
    ):
        failures.append("fallback is not retained as disabled pending evaluation")

    review = record["review"]
    if not isinstance(review, dict) or review != {
        "independent_review_performed": False,
        "release_approval": False,
    }:
        failures.append("disposition overstates review or release approval")
    return failures


def load_record(path: Path = DEFAULT_RECORD) -> dict[str, object]:
    value = json.loads(path.read_text(encoding="utf-8"))
    if not isinstance(value, dict):
        raise ModelDispositionError("feasibility disposition must be an object")
    return value


def write_record(path: Path, decision_source_revision: str) -> None:
    if path.exists():
        raise ModelDispositionError(f"refusing to overwrite existing disposition: {path}")
    record = build_record(decision_source_revision)
    failures = validate_record(record)
    if failures:
        raise ModelDispositionError("generated disposition is invalid: " + "; ".join(failures))
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
    except (OSError, json.JSONDecodeError, ModelDispositionError, ValueError) as error:
        print(f"Model feasibility disposition validation failed: {error}", file=sys.stderr)
        return 1
    if failures:
        print("Model feasibility disposition validation failed:", file=sys.stderr)
        for failure in failures:
            print(f"- {failure}", file=sys.stderr)
        return 1
    print(f"Validated Gemma 4 E4B feasibility disposition at {relative(args.record)}.")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
