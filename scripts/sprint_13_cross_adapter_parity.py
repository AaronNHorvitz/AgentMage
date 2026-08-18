#!/usr/bin/env python3
"""Build and verify the bounded Sprint 13 Linux cross-adapter parity record."""

from __future__ import annotations

import argparse
import copy
import hashlib
import json
import subprocess
import sys
from pathlib import Path
from typing import Any, Final

try:
    from scripts.model_corpus import load_corpus
    from scripts.model_feasibility import validate_result
except ModuleNotFoundError:
    from model_corpus import load_corpus
    from model_feasibility import validate_result


ROOT: Final = Path(__file__).resolve().parents[1]
OUTPUT: Final = (
    ROOT
    / "artifacts"
    / "sprints"
    / "sprint-13"
    / "story-13.2"
    / "linux-cross-adapter-parity.json"
)
CORPUS: Final = ROOT / "model-profiles/evaluation/corpus-v1.json"
ADMISSION: Final = (
    ROOT / "model-profiles/candidates/gemma-4-12b-unified/artifact-admission.json"
)
NATIVE_RESULT: Final = (
    ROOT / "artifacts/sprints/sprint-0/story-0.3-fallback/native-linux-result.json"
)
DOCKER_RESULT: Final = (
    ROOT / "artifacts/sprints/sprint-0/story-0.3-fallback/dmr-linux-result.json"
)
SOURCE_PATHS: Final = (
    "scripts/sprint_13_cross_adapter_parity.py",
    "tests/test_sprint_13_cross_adapter_parity.py",
)
INPUT_PATHS: Final = (
    "model-profiles/evaluation/corpus-v1.json",
    "model-profiles/candidates/gemma-4-12b-unified/artifact-admission.json",
    "artifacts/sprints/sprint-0/story-0.3-fallback/native-linux-result.json",
    "artifacts/sprints/sprint-0/story-0.3-fallback/dmr-linux-result.json",
)
MUTATIONS: Final = (
    ("model-hash", ("shared", "model_sha256"), "1" * 64),
    ("image-digest", ("docker", "runtime_sha256"), "sha256:" + "2" * 64),
    ("template", ("shared", "template_sha256"), "3" * 64),
    ("context-setting", ("shared", "context_tokens"), 4096),
    ("decoding-setting", ("shared", "decoding", "seed"), 9999),
    ("runtime-build", ("native", "runtime_sha256"), "4" * 64),
)


class ParityEvidenceError(ValueError):
    """Raised when retained parity inputs cannot be reconciled."""


def canonical_json(value: object) -> bytes:
    return (json.dumps(value, indent=2, sort_keys=True) + "\n").encode()


def sha256_bytes(value: bytes) -> str:
    return hashlib.sha256(value).hexdigest()


def sha256_file(path: Path) -> str:
    return sha256_bytes(path.read_bytes())


def read_json(path: Path) -> dict[str, Any]:
    value = json.loads(path.read_text(encoding="utf-8"))
    if not isinstance(value, dict):
        raise ParityEvidenceError(f"expected an object: {path.relative_to(ROOT)}")
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
        raise ParityEvidenceError("source revision is unavailable")
    return result.stdout.strip()


def git_sha256(revision: str, relative: str) -> str:
    result = subprocess.run(
        ["git", "show", f"{revision}:{relative}"],
        cwd=ROOT,
        check=False,
        capture_output=True,
    )
    if result.returncode != 0:
        raise ParityEvidenceError(f"source is absent at revision: {relative}")
    return sha256_bytes(result.stdout)


def failed_thresholds(result: dict[str, Any]) -> list[str]:
    return sorted(
        name
        for name, value in result["threshold_results"].items()
        if not value["passed"]
    )


def result_summary(result: dict[str, Any]) -> dict[str, object]:
    return {
        "adapter_id": result["adapter_id"],
        "result_sha256": sha256_bytes(canonical_json(result)),
        "source_revision": result["source_revision"],
        "trial_count": sum(case["trials_completed"] for case in result["cases"]),
        "status": result["status"],
        "failed_thresholds": failed_thresholds(result),
        "eligible": result["status"] == "PASS" and not failed_thresholds(result),
    }


def comparison_tuple(
    native: dict[str, Any], docker: dict[str, Any], admission: dict[str, Any]
) -> dict[str, object]:
    native_decoder = native["runtime_settings"]["decoder"]
    docker_decoder = docker["runtime_settings"]["decoder"]
    if native_decoder != docker_decoder:
        raise ParityEvidenceError("adapter decoding profiles differ")
    if native["runtime_settings"]["context_tokens"] != docker["runtime_settings"]["context_tokens"]:
        raise ParityEvidenceError("adapter context settings differ")
    if native["identities"]["model"] != docker["identities"]["model"]:
        raise ParityEvidenceError("adapter model identities differ")
    if native["identities"]["projector"] != docker["identities"]["projector"]:
        raise ParityEvidenceError("adapter projector identities differ")
    prompt = native_decoder["system_prompt"]
    return {
        "shared": {
            "model_sha256": native["identities"]["model"],
            "projector_sha256": native["identities"]["projector"],
            "template_sha256": sha256_bytes(prompt.encode()),
            "context_tokens": native["runtime_settings"]["context_tokens"],
            "decoding": {
                key: native_decoder[key]
                for key in (
                    "max_output_tokens",
                    "operational_context_tokens",
                    "seed",
                    "temperature",
                    "top_k",
                    "top_p",
                )
            },
        },
        "native": {
            "adapter_id": native["adapter_id"],
            "runtime_sha256": native["identities"]["runtime"],
            "runtime_source_revision": admission["native_runtime"]["source_commit"],
        },
        "docker": {
            "adapter_id": docker["adapter_id"],
            "runtime_sha256": docker["identities"]["runtime_image"],
            "runtime_source_revision": admission["docker_engine"][
                "runtime_source_revision"
            ],
            "model_manifest_sha256": docker["identities"]["model_manifest"],
        },
    }


def set_path(value: dict[str, object], path: tuple[str, ...], replacement: object) -> None:
    target: object = value
    for part in path[:-1]:
        if not isinstance(target, dict):
            raise ParityEvidenceError("invalid mutation path")
        target = target[part]
    if not isinstance(target, dict):
        raise ParityEvidenceError("invalid mutation target")
    target[path[-1]] = replacement


def changed_paths(before: object, after: object, prefix: tuple[str, ...] = ()) -> list[str]:
    if isinstance(before, dict) and isinstance(after, dict):
        output: list[str] = []
        for key in sorted(set(before) | set(after)):
            if key not in before or key not in after:
                output.append(".".join((*prefix, str(key))))
            else:
                output.extend(changed_paths(before[key], after[key], (*prefix, str(key))))
        return output
    return [".".join(prefix)] if before != after else []


def mutation_results(baseline: dict[str, object]) -> list[dict[str, object]]:
    results = []
    for identifier, path, replacement in MUTATIONS:
        changed = copy.deepcopy(baseline)
        set_path(changed, path, replacement)
        differences = changed_paths(baseline, changed)
        expected = ".".join(path)
        quarantined = differences == [expected]
        results.append(
            {
                "id": identifier,
                "mutated_path": expected,
                "changed_field_count": len(differences),
                "changed_paths": differences,
                "disposition": "QUARANTINED-INCOMPARABLE" if quarantined else "INVALID-TEST",
                "merged": False,
                "enabled": False,
                "passed": quarantined,
            }
        )
    return results


def build_report(source_revision: str) -> dict[str, object]:
    revision = resolve_revision(source_revision)
    corpus = load_corpus(CORPUS)
    admission = read_json(ADMISSION)
    native = read_json(NATIVE_RESULT)
    docker = read_json(DOCKER_RESULT)
    for label, result in (("native", native), ("Docker", docker)):
        failures = validate_result(result, corpus, admission)
        if failures:
            raise ParityEvidenceError(f"{label} result is invalid: {'; '.join(failures)}")
    if native["corpus_sha256"] != docker["corpus_sha256"]:
        raise ParityEvidenceError("adapter corpus identities differ")
    baseline = comparison_tuple(native, docker, admission)
    mutations = mutation_results(baseline)
    adapter_results = [result_summary(native), result_summary(docker)]
    return {
        "schema_version": 1,
        "record_type": "sprint_13_linux_cross_adapter_parity",
        "source_revision": revision,
        "source_sha256": {
            path: git_sha256(revision, path) for path in SOURCE_PATHS
        },
        "input_sha256": {path: sha256_file(ROOT / path) for path in INPUT_PATHS},
        "profile_id": "gemma-4-12b-unified-it",
        "data_classification": "public-synthetic-only",
        "corpus": {
            "id": native["corpus_id"],
            "version": native["corpus_version"],
            "sha256": native["corpus_sha256"],
        },
        "comparison_tuple": baseline,
        "adapter_results": adapter_results,
        "mutation_results": mutations,
        "disposition": {
            "matched_linux_trials_complete": all(
                item["trial_count"] == 82 for item in adapter_results
            ),
            "one_field_quarantine_complete": all(item["passed"] for item in mutations),
            "all_adapters_meet_thresholds": all(item["eligible"] for item in adapter_results),
            "eligible_for_merge": False,
            "profile_enabled": False,
            "automatic_fallback": False,
            "status": "COMPLETE-NEGATIVE-BLOCKED-QUALITY",
        },
        "limitations": [
            "The retained Linux trials are real prior runs; this verifier does not rerun inference.",
            "Docker Model Runner ran through rootless Podman compatibility, not Docker Engine.",
            "Both Linux adapters failed one or more published quality thresholds.",
            "No macOS adapter result, independent review, activation, or release approval is claimed.",
        ],
    }


def validate_report(report: Any, *, verify_current: bool = True) -> list[str]:
    if not isinstance(report, dict):
        return ["parity evidence must be an object"]
    failures: list[str] = []
    expected_fields = {
        "schema_version", "record_type", "source_revision", "source_sha256",
        "input_sha256", "profile_id", "data_classification", "corpus",
        "comparison_tuple", "adapter_results", "mutation_results", "disposition",
        "limitations",
    }
    if set(report) != expected_fields:
        return ["parity evidence fields are not closed"]
    if report.get("schema_version") != 1 or report.get("record_type") != "sprint_13_linux_cross_adapter_parity":
        failures.append("parity evidence identity changed")
    if set(report.get("source_sha256", {})) != set(SOURCE_PATHS):
        failures.append("parity source closure changed")
    if set(report.get("input_sha256", {})) != set(INPUT_PATHS):
        failures.append("parity input closure changed")
    adapters = report.get("adapter_results", [])
    if (
        len(adapters) != 2
        or {item.get("adapter_id") for item in adapters}
        != {"linux-native-vulkan", "linux-docker-model-runner-cuda"}
        or any(item.get("trial_count") != 82 for item in adapters)
        or any(item.get("eligible") is not False for item in adapters)
    ):
        failures.append("adapter result truth changed")
    mutations = report.get("mutation_results", [])
    if (
        [item.get("id") for item in mutations] != [item[0] for item in MUTATIONS]
        or any(item.get("changed_field_count") != 1 for item in mutations)
        or any(item.get("disposition") != "QUARANTINED-INCOMPARABLE" for item in mutations)
        or any(item.get("merged") is not False or item.get("enabled") is not False for item in mutations)
        or any(item.get("passed") is not True for item in mutations)
    ):
        failures.append("one-field quarantine evidence changed")
    expected_disposition = {
        "matched_linux_trials_complete": True,
        "one_field_quarantine_complete": True,
        "all_adapters_meet_thresholds": False,
        "eligible_for_merge": False,
        "profile_enabled": False,
        "automatic_fallback": False,
        "status": "COMPLETE-NEGATIVE-BLOCKED-QUALITY",
    }
    if report.get("disposition") != expected_disposition:
        failures.append("parity disposition overstates the result")
    if verify_current:
        for path, digest in report.get("input_sha256", {}).items():
            if sha256_file(ROOT / path) != digest:
                failures.append(f"parity input is stale: {path}")
    return failures


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--source-revision", default="HEAD")
    parser.add_argument("--write", action="store_true")
    args = parser.parse_args()
    try:
        if args.write:
            if OUTPUT.exists():
                raise ParityEvidenceError("refusing to overwrite parity evidence")
            report = build_report(args.source_revision)
            failures = validate_report(report)
            if failures:
                raise ParityEvidenceError("; ".join(failures))
            OUTPUT.parent.mkdir(parents=True)
            OUTPUT.write_bytes(canonical_json(report))
        else:
            report = read_json(OUTPUT)
            failures = validate_report(report)
            if failures:
                raise ParityEvidenceError("; ".join(failures))
    except (OSError, KeyError, TypeError, json.JSONDecodeError, ParityEvidenceError) as error:
        print(f"Sprint 13 parity evidence failed: {error}", file=sys.stderr)
        return 1
    print("Validated negative Linux cross-adapter parity evidence.")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
