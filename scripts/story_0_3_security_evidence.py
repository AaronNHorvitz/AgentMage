#!/usr/bin/env python3
"""Build and verify consolidated Story 0.3 product-security feasibility evidence."""

from __future__ import annotations

import argparse
import hashlib
import json
import subprocess
import sys
from pathlib import Path
from typing import Final

try:
    from scripts.fallback_admission import validate_all as validate_fallback_admission
    from scripts.fallback_disposition import load_record as load_fallback_disposition
    from scripts.fallback_disposition import validate_record as validate_fallback_disposition
    from scripts.model_admission import load_record, validate_artifact_record, validate_record
    from scripts.model_corpus import load_corpus, validate_corpus
    from scripts.model_disposition import load_record as load_e4b_disposition
    from scripts.model_disposition import validate_record as validate_e4b_disposition
    from scripts.story_0_3_dmr_evidence import check_bundle as check_dmr_evidence
    from scripts.story_0_3_evidence import check_bundle as check_native_evidence
    from scripts.story_0_3_fallback_evidence import check_bundle as check_fallback_evidence
    from scripts.story_0_3_reachability_evidence import check_bundle as check_reachability
    from scripts.model_substitution import check_bundle as check_substitution
except ModuleNotFoundError:
    from fallback_admission import validate_all as validate_fallback_admission
    from fallback_disposition import load_record as load_fallback_disposition
    from fallback_disposition import validate_record as validate_fallback_disposition
    from model_admission import load_record, validate_artifact_record, validate_record
    from model_corpus import load_corpus, validate_corpus
    from model_disposition import load_record as load_e4b_disposition
    from model_disposition import validate_record as validate_e4b_disposition
    from story_0_3_dmr_evidence import check_bundle as check_dmr_evidence
    from story_0_3_evidence import check_bundle as check_native_evidence
    from story_0_3_fallback_evidence import check_bundle as check_fallback_evidence
    from story_0_3_reachability_evidence import check_bundle as check_reachability
    from model_substitution import check_bundle as check_substitution


ROOT: Final = Path(__file__).resolve().parents[1]
DEFAULT_OUTPUT: Final = (
    ROOT / "artifacts" / "sprints" / "sprint-0" / "story-0.3-security"
)
EVIDENCE_DATE: Final = "2026-08-10"
CORPUS: Final = ROOT / "model-profiles" / "evaluation" / "corpus-v1.json"
POLICY: Final = ROOT / "MODEL-PROVENANCE-POLICY.md"
E4B_ROOT: Final = ROOT / "model-profiles" / "candidates" / "gemma-4-e4b"
FALLBACK_ROOT: Final = (
    ROOT / "model-profiles" / "candidates" / "gemma-4-12b-unified"
)
E4B_SOURCE: Final = E4B_ROOT / "source-admission.json"
E4B_ARTIFACT: Final = E4B_ROOT / "artifact-admission.json"
E4B_DECISION: Final = E4B_ROOT / "feasibility-disposition.json"
FALLBACK_SOURCE: Final = FALLBACK_ROOT / "source-admission.json"
FALLBACK_ARTIFACT: Final = FALLBACK_ROOT / "artifact-admission.json"
FALLBACK_PLAN: Final = FALLBACK_ROOT / "evaluation-plan.json"
FALLBACK_DECISION: Final = FALLBACK_ROOT / "feasibility-disposition.json"
SPRINT_ROOT: Final = ROOT / "artifacts" / "sprints" / "sprint-0"
NATIVE_EVIDENCE: Final = SPRINT_ROOT / "story-0.3"
DMR_EVIDENCE: Final = SPRINT_ROOT / "story-0.3-dmr"
FALLBACK_EVIDENCE: Final = SPRINT_ROOT / "story-0.3-fallback"
SUBSTITUTION_EVIDENCE: Final = SPRINT_ROOT / "story-0.3-substitution"
REACHABILITY_EVIDENCE: Final = SPRINT_ROOT / "story-0.3-reachability"
ARTIFACT_NAMES: Final = (
    "control-map.json",
    "evidence-index.json",
    "raw-checker-output.json",
    "reviewer-disposition.json",
    "summary.md",
)


class Story03SecurityEvidenceError(ValueError):
    """Raised when Story 0.3 security evidence cannot be reconciled."""


def canonical_json(value: object) -> bytes:
    return (json.dumps(value, indent=2, sort_keys=True) + "\n").encode("utf-8")


def read_json(path: Path) -> dict[str, object]:
    value = json.loads(path.read_text(encoding="utf-8"))
    if not isinstance(value, dict):
        raise Story03SecurityEvidenceError(f"expected JSON object: {path}")
    return value


def sha256(content: bytes) -> str:
    return hashlib.sha256(content).hexdigest()


def sha256_file(path: Path) -> str:
    digest = hashlib.sha256()
    with path.open("rb") as handle:
        for block in iter(lambda: handle.read(1024 * 1024), b""):
            digest.update(block)
    return digest.hexdigest()


def relative(path: Path) -> str:
    return str(path.relative_to(ROOT))


def binding(path: Path) -> dict[str, object]:
    return {"path": relative(path), "sha256": sha256_file(path), "size": path.stat().st_size}


def binding_at_revision(path: Path, revision: str) -> dict[str, object]:
    result = subprocess.run(
        ["git", "show", f"{revision}:{relative(path)}"],
        cwd=ROOT,
        check=False,
        capture_output=True,
    )
    if result.returncode != 0:
        raise Story03SecurityEvidenceError(
            f"cannot read {relative(path)} at verification revision {revision}"
        )
    return {
        "path": relative(path),
        "sha256": sha256(result.stdout),
        "size": len(result.stdout),
    }


def resolve_revision(revision: str) -> str:
    result = subprocess.run(
        ["git", "rev-parse", "--verify", f"{revision}^{{commit}}"],
        cwd=ROOT,
        check=False,
        capture_output=True,
        text=True,
    )
    if result.returncode != 0:
        raise Story03SecurityEvidenceError(
            f"cannot resolve security-evidence verification revision {revision}"
        )
    return result.stdout.strip()


def validate_inputs() -> list[str]:
    failures: list[str] = []
    e4b_source = load_record(E4B_SOURCE)
    e4b_artifact = load_record(E4B_ARTIFACT)
    failures.extend(f"E4B source: {item}" for item in validate_record(e4b_source))
    failures.extend(
        f"E4B artifact: {item}" for item in validate_artifact_record(e4b_artifact)
    )
    failures.extend(
        f"fallback admission: {item}" for item in validate_fallback_admission()
    )
    failures.extend(
        f"E4B disposition: {item}"
        for item in validate_e4b_disposition(load_e4b_disposition(E4B_DECISION))
    )
    failures.extend(
        f"fallback disposition: {item}"
        for item in validate_fallback_disposition(
            load_fallback_disposition(FALLBACK_DECISION)
        )
    )
    corpus = load_corpus(CORPUS)
    failures.extend(f"corpus: {item}" for item in validate_corpus(corpus))
    for label, checker in (
        ("native evidence", check_native_evidence),
        ("DMR evidence", check_dmr_evidence),
        ("fallback evidence", check_fallback_evidence),
        ("substitution evidence", check_substitution),
        ("reachability evidence", check_reachability),
    ):
        failures.extend(f"{label}: {item}" for item in checker())
    return failures


def evidence_index(verification_revision: str) -> dict[str, object]:
    records = (
        ("model_policy", POLICY, ["RV-13"]),
        ("fixed_corpus", CORPUS, ["RV-13", "RV-14", "RV-16"]),
        ("e4b_source_admission", E4B_SOURCE, ["RV-13"]),
        ("e4b_artifact_admission", E4B_ARTIFACT, ["RV-13"]),
        ("e4b_decision", E4B_DECISION, ["RV-13", "RV-14", "RV-16"]),
        ("fallback_source_admission", FALLBACK_SOURCE, ["RV-13"]),
        ("fallback_artifact_admission", FALLBACK_ARTIFACT, ["RV-13"]),
        ("fallback_evaluation_plan", FALLBACK_PLAN, ["RV-13", "RV-14"]),
        ("fallback_decision", FALLBACK_DECISION, ["RV-13", "RV-14", "RV-16"]),
        (
            "e4b_native_manifest",
            NATIVE_EVIDENCE / "evidence-manifest.json",
            ["RV-06", "RV-13", "RV-14", "RV-16"],
        ),
        (
            "e4b_dmr_manifest",
            DMR_EVIDENCE / "evidence-manifest.json",
            ["RV-06", "RV-13", "RV-14", "RV-16"],
        ),
        (
            "fallback_manifest",
            FALLBACK_EVIDENCE / "evidence-manifest.json",
            ["RV-06", "RV-13", "RV-14", "RV-16"],
        ),
        (
            "substitution_manifest",
            SUBSTITUTION_EVIDENCE / "evidence-manifest.json",
            ["RV-13"],
        ),
        (
            "reachability_manifest",
            REACHABILITY_EVIDENCE / "evidence-manifest.json",
            ["RV-06"],
        ),
    )
    return {
        "schema_version": 1,
        "record_type": "story_0_3_security_evidence_index",
        "verification_revision": verification_revision,
        "index_signature_status": "NOT_SIGNED_NO_RELEASE_SIGNING_IDENTITY",
        "release_signature_claim": False,
        "records": [
            {
                "evidence_id": evidence_id,
                **(
                    binding_at_revision(path, verification_revision)
                    if evidence_id == "model_policy"
                    else binding(path)
                ),
                "supports": supports,
            }
            for evidence_id, path, supports in records
        ],
    }


def corpus_results() -> list[tuple[str, dict[str, object]]]:
    return [
        ("e4b_native", read_json(NATIVE_EVIDENCE / "native-linux-result.json")),
        ("e4b_dmr", read_json(DMR_EVIDENCE / "dmr-linux-result.json")),
        (
            "fallback_native",
            read_json(FALLBACK_EVIDENCE / "native-linux-result.json"),
        ),
        ("fallback_dmr", read_json(FALLBACK_EVIDENCE / "dmr-linux-result.json")),
    ]


def protocol_map() -> dict[str, object]:
    results = corpus_results()
    reachability = read_json(REACHABILITY_EVIDENCE / "raw-reachability-result.json")
    substitutions = read_json(SUBSTITUTION_EVIDENCE / "raw-checker-output.json")
    adapter_observations = [
        {
            "result_id": result_id,
            "adapter_id": result["adapter_id"],
            "status": result["status"],
            "post_install_egress_bytes": result["metrics"]["post_install_egress_bytes"],
            "cancellation_max_seconds": result["metrics"]["cancellation_max_seconds"],
            "maximum_swap_growth_bytes": result["metrics"]["maximum_swap_growth_bytes"],
            "minimum_generation_tokens_per_second": result["metrics"][
                "minimum_generation_tokens_per_second"
            ],
            "maximum_time_to_first_token_seconds": result["metrics"][
                "maximum_time_to_first_token_seconds"
            ],
        }
        for result_id, result in results
    ]
    return {
        "schema_version": 1,
        "record_type": "story_0_3_reviewer_protocol_control_map",
        "protocols": {
            "RV-06": {
                "feasibility_execution_status": "COMPLETE",
                "feasibility_result": "PASS",
                "full_protocol_status": "NOT_COMPLETE",
                "completed": [
                    "All four retained Linux adapter results record zero post-install egress bytes under their declared isolated topology.",
                    "The guarded DMR probe admits only its authenticated path and denies the same-user, tool-container, separate-namespace, and non-loopback peer probes.",
                    "The private guarded DMR namespace records only loopback, zero routes, zero TCP listeners, and zero interface bytes.",
                ],
                "remaining": [
                    "Run the integrated product for at least 60 minutes under packet, DNS, socket, process, and firewall observation.",
                    "Exercise repository mapping, tools, crash recovery, diagnostics, extension-host, and tool-worker paths after they exist.",
                    "Repeat from a separately administered physical LAN peer and attribute unrelated editor and operating-system traffic.",
                ],
                "observations": adapter_observations,
                "reachability_status": reachability["status"],
            },
            "RV-13": {
                "feasibility_execution_status": "COMPLETE",
                "feasibility_result": "COMPLETE_WITH_REJECTED_CANDIDATES",
                "full_protocol_status": "NOT_COMPLETE",
                "completed": [
                    "Source, license, publisher/control, lineage, tokenizer, context, GGUF, conversion, quantization, native runtime, and OCI identities are hash-bound for E4B and the 12B fallback.",
                    "Sixteen one-field substitutions across both profiles were quarantined before admission-decision evaluation with zero inference invocations.",
                    "The same fixed corpus ran through native Linux and DMR compatibility paths for both candidates, and runtime differences remain visible.",
                ],
                "remaining": [
                    "No model identity is approved or enabled; both evaluated candidates are rejected and retain admission blockers.",
                    "Run the complete protocol on the MacBook Pro M5 and future production adapters.",
                    "Perform independent provenance review and signed release-manifest verification for any replacement candidate.",
                ],
                "substitution_status": substitutions["status"],
                "substitution_scenarios": substitutions["scenario_count"],
                "inference_invocations_after_substitution": substitutions[
                    "inference_invocation_count"
                ],
            },
            "RV-14": {
                "feasibility_execution_status": "COMPLETE",
                "feasibility_result": "NEGATIVE_RESULTS_RETAINED",
                "full_protocol_status": "NOT_COMPLETE",
                "completed": [
                    "All four available Linux runs retain every fixed trial, metric, failed case, and unchanged threshold.",
                    "Mandatory citation, tool-call, grounding, schema, and unsupported-action failures remain visible and produced rejected decisions.",
                    "Malformed-output refusal, cancellation, context, performance, memory, and egress outcomes remain independently inspectable.",
                ],
                "remaining": [
                    "Run future approved candidates across the complete repeated uncertainty, conflicting-evidence, stale-citation, and false-completion release corpus.",
                    "Complete the MacBook Pro M5 run and independent evidence-integrity review.",
                ],
                "adapter_results": [
                    {
                        "result_id": result_id,
                        "status": result["status"],
                        "failed_cases": [
                            case["case_id"] for case in result["cases"] if not case["passed"]
                        ],
                        "failed_thresholds": [
                            name
                            for name, value in result["threshold_results"].items()
                            if not value["passed"]
                        ],
                    }
                    for result_id, result in results
                ],
            },
            "RV-16": {
                "feasibility_execution_status": "COMPLETE",
                "feasibility_result": "COMPLETE_WITH_RETAINED_FAILURES",
                "full_protocol_status": "NOT_COMPLETE",
                "completed": [
                    "Every available adapter result records bounded cancellation, context behavior, throughput, time to first token, GPU/system-memory fractions, swap growth, and unload behavior.",
                    "All four runs pass cancellation, memory-fraction, zero-swap-growth, malformed-output, and zero-egress thresholds; the fallback DMR context and performance-completion case failures remain visible.",
                ],
                "remaining": [
                    "Exercise oversized repositories/files/results, deep trees, full disk, low memory, GPU loss, rapid requests, process termination, and sleep/wake in the integrated product.",
                    "Verify scratch cleanup, state recovery, and endpoint responsiveness across every product phase.",
                ],
                "observations": adapter_observations,
            },
        },
    }


def reviewer_disposition(verification_revision: str) -> dict[str, object]:
    return {
        "schema_version": 1,
        "record_type": "story_0_3_security_reviewer_disposition",
        "evidence_date": EVIDENCE_DATE,
        "verification_revision": verification_revision,
        "task_0_3_3_3_status": "COMPLETE",
        "feasibility_protocols": {
            "RV-06": "COMPLETE",
            "RV-13": "COMPLETE_WITH_REJECTED_CANDIDATES",
            "RV-14": "COMPLETE_NEGATIVE",
            "RV-16": "COMPLETE_WITH_RETAINED_FAILURES",
        },
        "full_protocols_complete": False,
        "story_0_3_status": "BLOCKED_REQUIRED_MAC_HARDWARE",
        "e4b_status": "REJECTED_DISABLED",
        "fallback_status": "REJECTED_DISABLED",
        "docker_adapter_production_status": "BLOCKED",
        "macos_evidence_substituted": False,
        "independent_review_performed": False,
        "release_approval": False,
    }


def raw_checker_output(
    index: dict[str, object], controls: dict[str, object], decision: dict[str, object]
) -> dict[str, object]:
    protocols = controls["protocols"]
    return {
        "schema_version": 1,
        "record_type": "story_0_3_security_raw_checker_output",
        "status": "PASS",
        "input_validation_failures": [],
        "evidence_record_count": len(index["records"]),
        "protocol_count": len(protocols),
        "feasibility_protocols_complete": all(
            item["feasibility_execution_status"] == "COMPLETE"
            for item in protocols.values()
        ),
        "full_protocols_complete": decision["full_protocols_complete"],
        "release_approval": decision["release_approval"],
    }


def summary_markdown(
    controls: dict[str, object], decision: dict[str, object]
) -> bytes:
    protocols = controls["protocols"]
    rows = "\n".join(
        f"| `{protocol_id}` | `{record['feasibility_result']}` | `{record['full_protocol_status']}` |"
        for protocol_id, record in protocols.items()
    )
    text = f"""# Story 0.3 Product-Security Feasibility Summary

| Protocol | Feasibility result | Full protocol |
|---|---|---|
{rows}

The feasibility portions of `RV-06`, `RV-13`, `RV-14`, and `RV-16` are complete. The evidence index binds the model policy, fixed corpus, both candidates' source and artifact records, both rejected decisions, four raw Linux adapter results, the 16-scenario substitution report, and the guarded DMR reachability matrix without copying or rewriting their immutable bundles.

The result is intentionally mixed. Available Linux runtime isolation, zero egress, cancellation, context, performance, memory, and substitution refusal were demonstrated within the declared spike scope. Model quality did not pass: E4B and Gemma 4 12B Unified are both rejected and disabled. Production DMR support remains blocked. The required MacBook Pro M5 run is unavailable and was not substituted.

Full reviewer protocols remain incomplete until their owning sprints exercise the integrated product, 60-minute monitoring, full workflow and failure matrices, future approved model/adapters, a physical peer, Mac hardware, independent review, and signed release evidence. This bundle is not signed because no release signing identity exists; its files are SHA-256-bound by the evidence manifest. Independent review and release approval are both false.

Story status: `{decision['story_0_3_status']}`.
"""
    return text.encode("utf-8")


def build_bundle(verification_revision: str) -> dict[str, bytes]:
    failures = validate_inputs()
    if failures:
        raise Story03SecurityEvidenceError(
            "cannot build from invalid Story 0.3 inputs: " + "; ".join(failures)
        )
    index = evidence_index(verification_revision)
    controls = protocol_map()
    decision = reviewer_disposition(verification_revision)
    raw = raw_checker_output(index, controls, decision)
    return {
        "control-map.json": canonical_json(controls),
        "evidence-index.json": canonical_json(index),
        "raw-checker-output.json": canonical_json(raw),
        "reviewer-disposition.json": canonical_json(decision),
        "summary.md": summary_markdown(controls, decision),
    }


def build_manifest(bundle: dict[str, bytes], revision: str) -> dict[str, object]:
    return {
        "schema_version": 1,
        "bundle_id": "sprint-0-story-0.3-product-security-feasibility-evidence",
        "evidence_date": EVIDENCE_DATE,
        "verification_revision": revision,
        "scope": "Sub-task 0.3.3.3 feasibility portions of RV-06, RV-13, RV-14, and RV-16",
        "signature_status": "NOT_SIGNED_NO_RELEASE_SIGNING_IDENTITY",
        "files": [
            {"path": name, "sha256": sha256(bundle[name]), "size": len(bundle[name])}
            for name in ARTIFACT_NAMES
        ],
    }


def write_bundle(output: Path, verification_revision: str) -> None:
    if output.exists():
        raise Story03SecurityEvidenceError(f"refusing to overwrite existing evidence: {output}")
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
        index = read_json(output / "evidence-index.json")
        controls = read_json(output / "control-map.json")
        raw = read_json(output / "raw-checker-output.json")
        decision = read_json(output / "reviewer-disposition.json")
    except (OSError, json.JSONDecodeError, Story03SecurityEvidenceError) as error:
        return [f"cannot load Story 0.3 security evidence: {error}"]
    entries = manifest.get("files")
    names = [item.get("path") for item in entries if isinstance(item, dict)] if isinstance(entries, list) else []
    if names != list(ARTIFACT_NAMES):
        failures.append("Story 0.3 security manifest membership or order is invalid")
    else:
        for item in entries:
            path = output / str(item["path"])
            try:
                content = path.read_bytes()
            except OSError as error:
                failures.append(f"cannot read Story 0.3 security evidence {path.name}: {error}")
                continue
            if item.get("sha256") != sha256(content) or item.get("size") != len(content):
                failures.append(f"Story 0.3 security evidence binding changed: {path.name}")
    input_failures = validate_inputs()
    failures.extend(input_failures)
    revision = str(manifest.get("verification_revision", ""))
    try:
        expected = build_bundle(revision)
    except (OSError, KeyError, TypeError, ValueError, Story03SecurityEvidenceError) as error:
        failures.append(f"cannot reconstruct Story 0.3 security evidence: {error}")
        return failures
    expected_values = {
        "evidence-index.json": index,
        "control-map.json": controls,
        "raw-checker-output.json": raw,
        "reviewer-disposition.json": decision,
    }
    for name, value in expected_values.items():
        if value != json.loads(expected[name]):
            failures.append(f"Story 0.3 security artifact does not reconcile: {name}")
    if raw.get("status") != "PASS" or raw.get("feasibility_protocols_complete") is not True:
        failures.append("Story 0.3 feasibility protocols are not complete")
    if raw.get("full_protocols_complete") is not False or raw.get("release_approval") is not False:
        failures.append("Story 0.3 evidence overstates full review or release approval")
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
    except (OSError, json.JSONDecodeError, Story03SecurityEvidenceError, ValueError) as error:
        print(f"Story 0.3 security evidence error: {error}", file=sys.stderr)
        return 1
    if failures:
        print("Story 0.3 security evidence validation failed:", file=sys.stderr)
        for failure in failures:
            print(f"- {failure}", file=sys.stderr)
        return 1
    print(f"Validated Story 0.3 security evidence at {relative(args.output)}.")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
