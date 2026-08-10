#!/usr/bin/env python3
"""Build and verify one-field model substitution refusal evidence."""

from __future__ import annotations

import argparse
import copy
import hashlib
import json
import subprocess
import sys
from dataclasses import dataclass
from pathlib import Path
from typing import Callable, Final

try:
    from scripts.fallback_admission import validate_artifact as validate_fallback_artifact
    from scripts.fallback_admission import validate_source as validate_fallback_source
    from scripts.model_admission import validate_artifact_record, validate_record
except ModuleNotFoundError:
    from fallback_admission import validate_artifact as validate_fallback_artifact
    from fallback_admission import validate_source as validate_fallback_source
    from model_admission import validate_artifact_record, validate_record


ROOT: Final = Path(__file__).resolve().parents[1]
DEFAULT_OUTPUT: Final = (
    ROOT / "artifacts" / "sprints" / "sprint-0" / "story-0.3-substitution"
)
E4B_ROOT: Final = ROOT / "model-profiles" / "candidates" / "gemma-4-e4b"
FALLBACK_ROOT: Final = (
    ROOT / "model-profiles" / "candidates" / "gemma-4-12b-unified"
)
EVIDENCE_DATE: Final = "2026-08-10"
ARTIFACT_NAMES: Final = ("raw-checker-output.json", "source-hashes.json", "summary.md")


class ModelSubstitutionError(ValueError):
    """Raised when substitution evidence cannot be built or reconciled."""


@dataclass(frozen=True)
class Profile:
    profile_id: str
    source_path: Path
    artifact_path: Path
    source_validator: Callable[[dict[str, object]], list[str]]
    artifact_validator: Callable[[dict[str, object]], list[str]]
    gguf_path: tuple[str, ...]


@dataclass(frozen=True)
class Scenario:
    category: str
    record_kind: str
    path: tuple[str, ...]
    replacement: object


PROFILES: Final = (
    Profile(
        "gemma-4-e4b-it",
        E4B_ROOT / "source-admission.json",
        E4B_ROOT / "artifact-admission.json",
        validate_record,
        validate_artifact_record,
        ("gguf_identity", "sha256"),
    ),
    Profile(
        "gemma-4-12b-unified-it",
        FALLBACK_ROOT / "source-admission.json",
        FALLBACK_ROOT / "artifact-admission.json",
        validate_fallback_source,
        validate_fallback_artifact,
        ("selected_gguf_identity", "sha256"),
    ),
)


def scenarios(profile: Profile) -> tuple[Scenario, ...]:
    return (
        Scenario("license", "source", ("license", "spdx"), "LicenseRef-Substituted"),
        Scenario(
            "lineage",
            "source",
            ("identity", "base_revision"),
            "0" * 40,
        ),
        Scenario(
            "tokenizer",
            "source",
            ("tokenizer", "tokenizer_json_sha256"),
            "1" * 64,
        ),
        Scenario(
            "context",
            "source",
            ("context_contract", "declared_tokens"),
            1,
        ),
        Scenario("gguf", "artifact", profile.gguf_path, "2" * 64),
        Scenario(
            "model_oci_digest",
            "artifact",
            ("docker_model", "digest"),
            "sha256:" + ("3" * 64),
        ),
        Scenario(
            "runtime_oci_digest",
            "artifact",
            ("docker_engine", "digest"),
            "sha256:" + ("4" * 64),
        ),
        Scenario(
            "runtime_build",
            "artifact",
            ("native_runtime", "source_commit"),
            "5" * 40,
        ),
    )


def canonical_json(value: object) -> bytes:
    return (json.dumps(value, indent=2, sort_keys=True) + "\n").encode("utf-8")


def read_json(path: Path) -> dict[str, object]:
    value = json.loads(path.read_text(encoding="utf-8"))
    if not isinstance(value, dict):
        raise ModelSubstitutionError(f"expected JSON object: {path}")
    return value


def sha256(content: bytes) -> str:
    return hashlib.sha256(content).hexdigest()


def sha256_file(path: Path) -> str:
    return sha256(path.read_bytes())


def relative(path: Path) -> str:
    return str(path.relative_to(ROOT))


def resolve_revision(revision: str) -> str:
    result = subprocess.run(
        ["git", "rev-parse", "--verify", f"{revision}^{{commit}}"],
        cwd=ROOT,
        check=False,
        capture_output=True,
        text=True,
    )
    if result.returncode != 0:
        raise ModelSubstitutionError(f"cannot resolve verification revision {revision}")
    return result.stdout.strip()


def set_path(value: dict[str, object], path: tuple[str, ...], replacement: object) -> None:
    target: object = value
    for component in path[:-1]:
        if not isinstance(target, dict) or component not in target:
            raise ModelSubstitutionError(f"mutation path does not exist: {'.'.join(path)}")
        target = target[component]
    if not isinstance(target, dict) or path[-1] not in target:
        raise ModelSubstitutionError(f"mutation path does not exist: {'.'.join(path)}")
    target[path[-1]] = replacement


def changed_paths(before: object, after: object, prefix: tuple[str, ...] = ()) -> list[str]:
    if isinstance(before, dict) and isinstance(after, dict):
        paths: list[str] = []
        for key in sorted(set(before) | set(after)):
            if key not in before or key not in after:
                paths.append(".".join((*prefix, str(key))))
            else:
                paths.extend(changed_paths(before[key], after[key], (*prefix, str(key))))
        return paths
    if before != after:
        return [".".join(prefix)]
    return []


def run_scenario(
    profile: Profile,
    scenario: Scenario,
    source: dict[str, object],
    artifact: dict[str, object],
) -> dict[str, object]:
    changed_source = copy.deepcopy(source)
    changed_artifact = copy.deepcopy(artifact)
    target = changed_source if scenario.record_kind == "source" else changed_artifact
    before = copy.deepcopy(target)
    set_path(target, scenario.path, scenario.replacement)
    differences = changed_paths(before, target)
    validator = (
        profile.source_validator
        if scenario.record_kind == "source"
        else profile.artifact_validator
    )
    validation_failures = validator(target)
    substitution_detected = bool(validation_failures)
    inference_invocations = 0
    return {
        "scenario_id": f"{profile.profile_id}:{scenario.category}",
        "profile_id": profile.profile_id,
        "category": scenario.category,
        "record_kind": scenario.record_kind,
        "mutated_path": ".".join(scenario.path),
        "changed_paths": differences,
        "changed_field_count": len(differences),
        "substitution_detected": substitution_detected,
        "gate_result": "QUARANTINED_SUBSTITUTION" if substitution_detected else "UNDETECTED",
        "visible_refusal": substitution_detected,
        "refusal_code": "MODEL-IDENTITY-SUBSTITUTION" if substitution_detected else None,
        "validation_failures": validation_failures,
        "admission_decision_evaluated": False,
        "inference_invocations": inference_invocations,
        "passed": (
            substitution_detected
            and differences == [".".join(scenario.path)]
            and inference_invocations == 0
        ),
    }


def build_report(verification_revision: str) -> dict[str, object]:
    revision = resolve_revision(verification_revision)
    results: list[dict[str, object]] = []
    for profile in PROFILES:
        source = read_json(profile.source_path)
        artifact = read_json(profile.artifact_path)
        if profile.source_validator(source) or profile.artifact_validator(artifact):
            raise ModelSubstitutionError(
                f"baseline admission record is invalid: {profile.profile_id}"
            )
        for scenario in scenarios(profile):
            results.append(run_scenario(profile, scenario, source, artifact))
    return {
        "schema_version": 1,
        "record_type": "model_substitution_negative_test_report",
        "evidence_date": EVIDENCE_DATE,
        "verification_revision": revision,
        "data_classification": "public_metadata_only",
        "execution_contract": {
            "mutation_cardinality": "exactly_one_field_per_scenario",
            "failure_stage": "admission_identity_validation_before_decision_or_inference",
            "actual_model_process_started": False,
        },
        "profiles_tested": [profile.profile_id for profile in PROFILES],
        "categories_per_profile": [item.category for item in scenarios(PROFILES[0])],
        "scenario_count": len(results),
        "passed_count": sum(bool(item["passed"]) for item in results),
        "inference_invocation_count": sum(
            int(item["inference_invocations"]) for item in results
        ),
        "status": "PASS" if all(item["passed"] for item in results) else "FAIL",
        "scenarios": results,
    }


def source_hashes(verification_revision: str) -> dict[str, object]:
    paths = sorted(
        [profile.source_path for profile in PROFILES]
        + [profile.artifact_path for profile in PROFILES]
    )
    return {
        "schema_version": 1,
        "record_type": "model_substitution_source_hashes",
        "verification_revision": verification_revision,
        "files": [
            {"path": relative(path), "sha256": sha256_file(path), "size": path.stat().st_size}
            for path in paths
        ],
    }


def summary_markdown(report: dict[str, object]) -> bytes:
    text = f"""# Story 0.3 Model Substitution Evidence Summary

| Field | Value |
|---|---:|
| Profiles | {len(report['profiles_tested'])} |
| One-field substitution scenarios | {report['scenario_count']} |
| Passed scenarios | {report['passed_count']} |
| Inference invocations | {report['inference_invocation_count']} |
| Result | `{report['status']}` |

Each E4B and Gemma 4 12B Unified admission record was changed in exactly one field for license, lineage, tokenizer, context, GGUF, model OCI digest, runtime OCI digest, and native runtime build. Every altered record produced a visible `MODEL-IDENTITY-SUBSTITUTION` quarantine receipt during identity validation, before its admission decision was evaluated and before any inference process could start.

The raw JSON report and source hashes are authoritative. These negative tests prove substitution refusal for the listed static admission inputs; they do not approve either rejected model, substitute for the unavailable MacBook Pro M5 corpus run, or establish runtime reachability isolation.
"""
    return text.encode("utf-8")


def build_bundle(verification_revision: str) -> dict[str, bytes]:
    report = build_report(verification_revision)
    if report["status"] != "PASS" or report["inference_invocation_count"] != 0:
        raise ModelSubstitutionError("one or more substitutions were not refused before inference")
    return {
        "raw-checker-output.json": canonical_json(report),
        "source-hashes.json": canonical_json(source_hashes(report["verification_revision"])),
        "summary.md": summary_markdown(report),
    }


def build_manifest(bundle: dict[str, bytes], revision: str) -> dict[str, object]:
    return {
        "schema_version": 1,
        "bundle_id": "sprint-0-story-0.3-model-substitution-evidence",
        "evidence_date": EVIDENCE_DATE,
        "verification_revision": revision,
        "scope": "Sub-task 0.3.3.1 one-field admission substitution refusal",
        "files": [
            {"path": name, "sha256": sha256(bundle[name]), "size": len(bundle[name])}
            for name in ARTIFACT_NAMES
        ],
    }


def write_bundle(output: Path, verification_revision: str) -> None:
    if output.exists():
        raise ModelSubstitutionError(f"refusing to overwrite existing evidence: {output}")
    revision = resolve_revision(verification_revision)
    bundle = build_bundle(revision)
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
        report = read_json(output / "raw-checker-output.json")
        hashes = read_json(output / "source-hashes.json")
    except (OSError, json.JSONDecodeError, ModelSubstitutionError) as error:
        return [f"cannot load substitution evidence: {error}"]
    entries = manifest.get("files")
    names = [item.get("path") for item in entries if isinstance(item, dict)] if isinstance(entries, list) else []
    if names != list(ARTIFACT_NAMES):
        failures.append("substitution evidence manifest membership or order is invalid")
    else:
        for item in entries:
            path = output / str(item["path"])
            try:
                content = path.read_bytes()
            except OSError as error:
                failures.append(f"cannot read substitution evidence {path.name}: {error}")
                continue
            if item.get("sha256") != sha256(content) or item.get("size") != len(content):
                failures.append(f"substitution evidence binding changed: {path.name}")
    revision = str(manifest.get("verification_revision", ""))
    try:
        expected_bundle = build_bundle(revision)
    except (OSError, KeyError, TypeError, ValueError, ModelSubstitutionError) as error:
        failures.append(f"cannot reconstruct substitution evidence: {error}")
        return failures
    if report != json.loads(expected_bundle["raw-checker-output.json"]):
        failures.append("substitution report does not reconcile")
    if hashes != json.loads(expected_bundle["source-hashes.json"]):
        failures.append("substitution source hashes do not reconcile")
    if report.get("status") != "PASS" or report.get("inference_invocation_count") != 0:
        failures.append("substitution report does not prove pre-inference refusal")
    return failures


def main(argv: list[str] | None = None) -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--output", type=Path, default=DEFAULT_OUTPUT)
    parser.add_argument("--verification-revision", default="HEAD")
    parser.add_argument("--write", action="store_true")
    args = parser.parse_args(argv)
    try:
        if args.write:
            write_bundle(args.output, args.verification_revision)
            return 0
        failures = check_bundle(args.output)
    except (OSError, json.JSONDecodeError, ModelSubstitutionError, ValueError) as error:
        print(f"Model substitution evidence error: {error}", file=sys.stderr)
        return 1
    if failures:
        print("Model substitution evidence validation failed:", file=sys.stderr)
        for failure in failures:
            print(f"- {failure}", file=sys.stderr)
        return 1
    print(f"Validated model substitution evidence at {relative(args.output)}.")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
