#!/usr/bin/env python3
"""Build and verify immutable Gemma 4 12B fallback evidence for Story 0.3."""

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
    from scripts.model_feasibility import read_json, validate_result, validate_result_directory
except ModuleNotFoundError:
    from model_corpus import load_corpus
    from model_feasibility import read_json, validate_result, validate_result_directory


ROOT: Final = Path(__file__).resolve().parents[1]
PROFILE_ROOT: Final = ROOT / "model-profiles" / "candidates" / "gemma-4-12b-unified"
ADMISSION: Final = PROFILE_ROOT / "artifact-admission.json"
CORPUS: Final = ROOT / "model-profiles" / "evaluation" / "corpus-v1.json"
DEFAULT_OUTPUT: Final = (
    ROOT / "artifacts" / "sprints" / "sprint-0" / "story-0.3-fallback"
)
EVIDENCE_DATE: Final = "2026-08-10"
ARTIFACT_NAMES: Final = (
    "adapter-disposition.json",
    "artifact-staging.json",
    "dmr-linux-result.json",
    "dmr-linux-server.log",
    "native-linux-result.json",
    "native-linux-server.log",
    "summary.md",
)
EXPECTED_OUTCOMES: Final = {
    "linux-native-vulkan": {
        "failed_cases": ("CITE-001", "TOOL-001", "TOOL-002"),
        "failed_thresholds": ("citation_recall", "tool_call_valid_rate"),
    },
    "linux-docker-model-runner-cuda": {
        "failed_cases": (
            "REPO-001",
            "TOOL-001",
            "TOOL-002",
            "CONTEXT-001",
            "PERF-001",
        ),
        "failed_thresholds": (
            "citation_precision",
            "repository_fact_accuracy",
            "schema_valid_rate",
            "tool_call_valid_rate",
            "unsupported_action_rate",
        ),
    },
}
HOME_PATH = re.compile(r"/(?:var/)?home/[^/\s'\"]+/.local/share/agentmage/evaluation")
SENSITIVE_TEXT = re.compile(
    r"(?:/(?:var/)?home/[^/\s]+|[A-Za-z0-9._%+-]+@[A-Za-z0-9.-]+\.[A-Za-z]{2,}|"
    r"-----BEGIN [A-Z ]*PRIVATE KEY-----|(?:password|api[_ -]?key|secret)\s*[:=])",
    re.IGNORECASE,
)


class FallbackEvidenceError(ValueError):
    """Raised when fallback evidence cannot be built or reconciled."""


def canonical_json(value: object) -> bytes:
    return (json.dumps(value, indent=2, sort_keys=True) + "\n").encode("utf-8")


def sha256(content: bytes) -> str:
    return hashlib.sha256(content).hexdigest()


def sha256_file(path: Path) -> str:
    digest = hashlib.sha256()
    with path.open("rb") as handle:
        for block in iter(lambda: handle.read(1024 * 1024), b""):
            digest.update(block)
    return digest.hexdigest()


def relative(path: Path) -> str:
    try:
        return str(path.relative_to(ROOT))
    except ValueError:
        return str(path)


def resolve_revision(revision: str) -> str:
    result = subprocess.run(
        ["git", "rev-parse", "--verify", f"{revision}^{{commit}}"],
        cwd=ROOT,
        check=False,
        capture_output=True,
        text=True,
    )
    if result.returncode != 0:
        raise FallbackEvidenceError(f"cannot resolve verification revision {revision}")
    return result.stdout.strip()


def sanitize_server_log(content: str) -> str:
    sanitized = HOME_PATH.sub("<EVALUATION_DATA_ROOT>", content)
    if SENSITIVE_TEXT.search(sanitized):
        raise FallbackEvidenceError(
            "fallback server log still contains a sensitive path or identifier pattern"
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


def resource_fit(result: dict[str, Any]) -> dict[str, object]:
    names = (
        "minimum_generation_tokens_per_second",
        "maximum_time_to_first_token_seconds",
        "maximum_gpu_or_unified_memory_fraction",
        "maximum_system_memory_fraction",
        "maximum_swap_growth_bytes",
        "post_install_egress_bytes",
    )
    thresholds = {name: result["threshold_results"][name] for name in names}
    return {
        "status": "PASS" if all(item["passed"] for item in thresholds.values()) else "FAIL",
        "threshold_results": thresholds,
    }


def adapter_record(result: dict[str, Any], deployment: dict[str, object]) -> dict[str, object]:
    return {
        "adapter_id": result["adapter_id"],
        "execution_status": "COMPLETE",
        "quality_status": result["status"],
        "trials_completed": sum(case["trials_completed"] for case in result["cases"]),
        "failed_cases": failed_cases(result),
        "failed_thresholds": failed_thresholds(result),
        "metrics": result["metrics"],
        "resource_and_isolation_fit": resource_fit(result),
        "runtime_settings": result["runtime_settings"],
        "identities": result["identities"],
        "result_source_revision": result["source_revision"],
        "runner_transform_version": result["runner_transform_version"],
        "deployment": deployment,
    }


def staging_receipt(native: dict[str, Any], dmr: dict[str, Any]) -> dict[str, object]:
    admission = read_json(ADMISSION)
    selected = admission["selected_gguf_identity"]
    docker_model = admission["docker_model"]
    return {
        "schema_version": 1,
        "record_type": "fallback_artifact_staging_receipt",
        "profile_id": "gemma-4-12b-unified-it",
        "evidence_date": EVIDENCE_DATE,
        "data_classification": "public_synthetic_only",
        "scope": "quarantined evaluation only",
        "source_weights_downloaded": False,
        "selected_gguf": {
            "sha256": selected["sha256"],
            "size": selected["size"],
            "hash_verified_before_native_inference": (
                native["identities"]["model"] == selected["sha256"]
            ),
        },
        "selected_projector": {
            "sha256": selected["multimodal_projector"]["sha256"],
            "size": selected["multimodal_projector"]["size"],
            "hash_verified_before_native_inference": (
                native["identities"]["projector"]
                == selected["multimodal_projector"]["sha256"]
            ),
        },
        "docker_model": {
            "manifest_digest": docker_model["digest"],
            "config_digest": docker_model["config_digest"],
            "model_layer_digest": docker_model["model_layer_digest"],
            "projector_layer_digest": docker_model["projector_layer_digest"],
            "all_required_identities_verified_before_dmr_inference": (
                dmr["identities"]["model_manifest"]
                == docker_model["digest"].removeprefix("sha256:")
                and dmr["identities"]["model_config"]
                == docker_model["config_digest"].removeprefix("sha256:")
                and "sha256:" + dmr["identities"]["model"]
                == docker_model["model_layer_digest"]
                and "sha256:" + dmr["identities"]["projector"]
                == docker_model["projector_layer_digest"]
            ),
        },
        "state": {
            "evaluation_staging_complete": True,
            "admission_blockers_resolved": False,
            "candidate_enabled": False,
            "automatic_switch": False,
            "release_approval": False,
        },
    }


def adapter_disposition(
    native: dict[str, Any], dmr: dict[str, Any], verification_revision: str
) -> dict[str, object]:
    return {
        "schema_version": 1,
        "record_type": "story_0_3_fallback_adapter_disposition",
        "profile_id": "gemma-4-12b-unified-it",
        "evidence_date": EVIDENCE_DATE,
        "verification_revision": verification_revision,
        "data_classification": "public_synthetic_only",
        "native_linux": adapter_record(
            native,
            {
                "platform": "Fedora Kinoite 44 x86_64",
                "backend": "Vulkan",
                "runtime": "llama.cpp b10333",
                "transport": "isolated_loopback_http",
            },
        ),
        "dmr_linux": adapter_record(
            dmr,
            {
                "platform": "Fedora Kinoite 44 x86_64",
                "backend": "CUDA 13.3",
                "runtime": "Docker Model Runner v1.2.6 CUDA with llama.cpp b9879",
                "transport": "bind_mounted_unix_domain_socket",
                "compatibility_container_engine": "Podman 5.8.4",
                "docker_engine_directly_tested": False,
                "exact_dmr_image_used": True,
                "gpu_compatibility_configuration": (
                    "manual NVIDIA device exposure, explicit bundled CUDA compatibility "
                    "library path, and SELinux label separation disabled for the container"
                ),
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
        "cross_adapter": {
            "available_linux_paths_complete": True,
            "shared_failed_cases": sorted(
                set(failed_cases(native)).intersection(failed_cases(dmr))
            ),
            "shared_failed_thresholds": sorted(
                set(failed_thresholds(native)).intersection(failed_thresholds(dmr))
            ),
        },
        "task_0_3_2_4_status": "COMPLETE_REJECTED",
        "candidate_state": "REJECTED_DISABLED",
        "automatic_switch": False,
        "activation_authorized": False,
        "independent_review_performed": False,
        "release_approval": False,
        "limitations": [
            "Source lineage and selected-GGUF conversion reproducibility remain blocked.",
            "Docker Engine was not directly tested; the exact DMR image ran under rootless Podman compatibility deployment.",
            "The DMR compatibility run required SELinux label separation to be disabled for manual NVIDIA device access.",
            "The MacBook Pro M5 result remains unavailable and Linux evidence was not substituted.",
            "No fallback activation, support claim, automatic switch, or release approval is authorized.",
        ],
    }


def summary_markdown(native: dict[str, Any], dmr: dict[str, Any]) -> bytes:
    native_metrics = native["metrics"]
    dmr_metrics = dmr["metrics"]
    text = f"""# Story 0.3 Gemma 4 12B Fallback Evidence Summary

| Field | Native Linux | Docker Model Runner compatibility |
|---|---:|---:|
| Result | `{native['status']}` | `{dmr['status']}` |
| Fixed corpus trials | {sum(case['trials_completed'] for case in native['cases'])} | {sum(case['trials_completed'] for case in dmr['cases'])} |
| Minimum generation rate | {native_metrics['minimum_generation_tokens_per_second']:.2f} tok/s | {dmr_metrics['minimum_generation_tokens_per_second']:.2f} tok/s |
| Maximum time to first token | {native_metrics['maximum_time_to_first_token_seconds']:.3f} s | {dmr_metrics['maximum_time_to_first_token_seconds']:.3f} s |
| Citation recall | {native_metrics['citation_recall']:.3f} | {dmr_metrics['citation_recall']:.3f} |
| Repository fact accuracy | {native_metrics['repository_fact_accuracy']:.3f} | {dmr_metrics['repository_fact_accuracy']:.3f} |
| Tool-call validity | {native_metrics['tool_call_valid_rate']:.3f} | {dmr_metrics['tool_call_valid_rate']:.3f} |
| Unsupported action rate | {native_metrics['unsupported_action_rate']:.3f} | {dmr_metrics['unsupported_action_rate']:.3f} |
| Post-install egress | {native_metrics['post_install_egress_bytes']} bytes | {dmr_metrics['post_install_egress_bytes']} bytes |

Both available Linux adapters completed all 82 fixed public-synthetic trials with unchanged corpus-v1 thresholds. Native Linux passed the resource, isolation, schema, repository-fact, and unsupported-action thresholds but failed citation recall and exact tool-call validity. The DMR compatibility path passed resource, cancellation, malformed-output, zero-egress, and citation-recall thresholds but failed repository accuracy, citation precision, schema validity, exact tool-call validity, and unsupported-action thresholds.

The selected GGUF and projector were hash-verified before native inference. The exact OCI model manifest, config, model layer, projector layer, and DMR runtime image were hash-verified before isolated DMR inference. This resolves evaluation staging only; source-lineage and reproducible-conversion admission blockers remain.

The exact DMR image ran under rootless Podman 5.8.4, not Docker Engine. GPU access required manual NVIDIA device exposure, an explicit bundled CUDA compatibility-library path, and disabled SELinux label separation for that container. These compatibility findings do not establish Docker Engine support or an approved production topology.

The MacBook Pro M5 run remains unavailable and Linux evidence was not substituted. The Linux quality failures are independently dispositive: Gemma 4 12B Unified remains disabled and is rejected for this fixed v0.1 profile. No automatic fallback, profile activation, support claim, independent review, or release approval follows from this evidence. Raw JSON results are authoritative over this summary.
"""
    return text.encode("utf-8")


def validate_expected_result(result: dict[str, Any]) -> list[str]:
    failures: list[str] = []
    expected = EXPECTED_OUTCOMES.get(str(result.get("adapter_id")))
    if expected is None:
        return ["unexpected fallback adapter identity"]
    if result.get("status") != "FAIL":
        failures.append("fallback result no longer records mandatory failure")
    if sum(case["trials_completed"] for case in result["cases"]) != 82:
        failures.append("fallback result does not contain all 82 fixed trials")
    if tuple(failed_cases(result)) != expected["failed_cases"]:
        failures.append("fallback failed-case set changed")
    if tuple(failed_thresholds(result)) != expected["failed_thresholds"]:
        failures.append("fallback failed-threshold set changed")
    return failures


def build_bundle(
    native_result_dir: Path,
    dmr_result_dir: Path,
    verification_revision: str,
) -> dict[str, bytes]:
    admission = read_json(ADMISSION)
    corpus = load_corpus(CORPUS)
    results: dict[str, dict[str, Any]] = {}
    for label, directory in (
        ("native", native_result_dir),
        ("DMR", dmr_result_dir),
    ):
        failures = validate_result_directory(directory, CORPUS, ADMISSION)
        if failures:
            raise FallbackEvidenceError(
                f"{label} result bundle is invalid: " + "; ".join(failures)
            )
        result = read_json(directory / "results.json")
        failures = validate_result(result, corpus, admission) + validate_expected_result(result)
        if failures:
            raise FallbackEvidenceError(
                f"{label} result outcome is invalid: " + "; ".join(failures)
            )
        results[label] = result

    native = results["native"]
    dmr = results["DMR"]
    disposition = adapter_disposition(native, dmr, verification_revision)
    staging = staging_receipt(native, dmr)
    if not staging["selected_gguf"]["hash_verified_before_native_inference"]:
        raise FallbackEvidenceError("native model identity did not match the admitted GGUF")
    if not staging["selected_projector"]["hash_verified_before_native_inference"]:
        raise FallbackEvidenceError("native projector identity did not match admission")
    if not staging["docker_model"]["all_required_identities_verified_before_dmr_inference"]:
        raise FallbackEvidenceError("DMR model identities did not match admission")
    return {
        "adapter-disposition.json": canonical_json(disposition),
        "artifact-staging.json": canonical_json(staging),
        "dmr-linux-result.json": (dmr_result_dir / "results.json").read_bytes(),
        "dmr-linux-server.log": sanitize_server_log(
            (dmr_result_dir / "server.log").read_text(encoding="utf-8")
        ).encode("utf-8"),
        "native-linux-result.json": (native_result_dir / "results.json").read_bytes(),
        "native-linux-server.log": sanitize_server_log(
            (native_result_dir / "server.log").read_text(encoding="utf-8")
        ).encode("utf-8"),
        "summary.md": summary_markdown(native, dmr),
    }


def build_manifest(bundle: dict[str, bytes], verification_revision: str) -> dict[str, object]:
    return {
        "schema_version": 1,
        "bundle_id": "sprint-0-story-0.3-gemma-4-12b-fallback-evidence",
        "evidence_date": EVIDENCE_DATE,
        "verification_revision": verification_revision,
        "scope": "complete available-platform fallback admission and corpus disposition",
        "files": [
            {"path": name, "sha256": sha256(bundle[name]), "size": len(bundle[name])}
            for name in ARTIFACT_NAMES
        ],
    }


def write_bundle(
    output: Path,
    native_result_dir: Path,
    dmr_result_dir: Path,
    verification_revision: str,
) -> None:
    if output.exists():
        raise FallbackEvidenceError(f"refusing to overwrite existing evidence: {output}")
    revision = resolve_revision(verification_revision)
    bundle = build_bundle(native_result_dir, dmr_result_dir, revision)
    output.mkdir(parents=True)
    for name, content in bundle.items():
        (output / name).write_bytes(content)
    (output / "evidence-manifest.json").write_bytes(
        canonical_json(build_manifest(bundle, revision))
    )


def check_bundle(output: Path = DEFAULT_OUTPUT) -> list[str]:
    failures: list[str] = []
    try:
        manifest = read_json(output / "evidence-manifest.json")
        disposition = read_json(output / "adapter-disposition.json")
        staging = read_json(output / "artifact-staging.json")
        native = read_json(output / "native-linux-result.json")
        dmr = read_json(output / "dmr-linux-result.json")
        admission = read_json(ADMISSION)
        corpus = load_corpus(CORPUS)
    except (OSError, json.JSONDecodeError, ValueError) as error:
        return [f"cannot load fallback evidence: {error}"]

    entries = manifest.get("files")
    observed_names = [entry.get("path") for entry in entries if isinstance(entry, dict)] if isinstance(entries, list) else []
    if observed_names != list(ARTIFACT_NAMES):
        failures.append("fallback evidence manifest membership or order is invalid")
    else:
        for entry in entries:
            path = output / str(entry["path"])
            try:
                content = path.read_bytes()
            except OSError as error:
                failures.append(f"cannot read fallback evidence artifact {path.name}: {error}")
                continue
            if entry.get("sha256") != sha256(content):
                failures.append(f"fallback evidence hash mismatch: {path.name}")
            if entry.get("size") != len(content):
                failures.append(f"fallback evidence size mismatch: {path.name}")

    for label, result in (("native", native), ("DMR", dmr)):
        for failure in validate_result(result, corpus, admission) + validate_expected_result(result):
            failures.append(f"{label} fallback result: {failure}")
    expected_disposition = adapter_disposition(
        native, dmr, str(disposition.get("verification_revision", ""))
    )
    if disposition != expected_disposition:
        failures.append("fallback adapter disposition does not reconcile with raw results")
    if staging != staging_receipt(native, dmr):
        failures.append("fallback artifact-staging receipt does not reconcile")
    if manifest.get("verification_revision") != disposition.get("verification_revision"):
        failures.append("fallback verification revision does not reconcile")
    try:
        resolve_revision(str(manifest.get("verification_revision", "")))
    except FallbackEvidenceError as error:
        failures.append(str(error))
    for name in ("native-linux-server.log", "dmr-linux-server.log"):
        try:
            log = (output / name).read_text(encoding="utf-8")
        except OSError as error:
            failures.append(f"cannot read sanitized fallback log {name}: {error}")
            continue
        if SENSITIVE_TEXT.search(log):
            failures.append(f"sanitized fallback log contains sensitive text: {name}")
    return failures


def main(argv: list[str] | None = None) -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--output", type=Path, default=DEFAULT_OUTPUT)
    parser.add_argument("--native-result-dir", type=Path)
    parser.add_argument("--dmr-result-dir", type=Path)
    parser.add_argument("--verification-revision", default="HEAD")
    parser.add_argument("--write", action="store_true")
    args = parser.parse_args(argv)
    try:
        if args.write:
            if args.native_result_dir is None or args.dmr_result_dir is None:
                raise FallbackEvidenceError(
                    "--write requires --native-result-dir and --dmr-result-dir"
                )
            write_bundle(
                args.output,
                args.native_result_dir,
                args.dmr_result_dir,
                args.verification_revision,
            )
            return 0
        failures = check_bundle(args.output)
    except (OSError, json.JSONDecodeError, FallbackEvidenceError, ValueError) as error:
        print(f"Fallback evidence error: {error}", file=sys.stderr)
        return 1
    if failures:
        print("Fallback evidence validation failed:", file=sys.stderr)
        for failure in failures:
            print(f"- {failure}", file=sys.stderr)
        return 1
    print(f"Validated Gemma 4 12B fallback evidence at {relative(args.output)}.")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
