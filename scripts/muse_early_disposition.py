#!/usr/bin/env python3
"""Build the append-only early Muse exact-profile disposition."""

from __future__ import annotations

import argparse
import hashlib
import json
import re
import subprocess
from pathlib import Path
from typing import Any, Final


ROOT: Final = Path(__file__).resolve().parents[1]
OUTPUT: Final = (
    ROOT
    / "model-profiles/candidates/muse-glimmer-30b-text-8k/early-evaluation-disposition.json"
)
SOURCE_ADMISSION: Final = (
    ROOT / "model-profiles/candidates/muse-glimmer-30b-text-8k/source-admission.json"
)
RUNTIME_SUPPORT: Final = (
    ROOT / "model-profiles/candidates/muse-glimmer-30b-text-8k/runtime-support.json"
)
CODEC_EVIDENCE: Final = (
    ROOT / "artifacts/sprints/sprint-13/story-13.3/d027-s13-muse-codec.json"
)
INSTALL_EVIDENCE: Final = (
    ROOT / "artifacts/sprints/sprint-13/story-13.3/muse-live-install.json"
)
INFERENCE_EVIDENCE: Final = (
    ROOT / "artifacts/sprints/sprint-13/story-13.3/muse-sandbox-inference.json"
)
EVALUATION_EVIDENCE: Final = (
    ROOT / "artifacts/sprints/sprint-13/story-13.3/muse-profile-evaluation.json"
)
INPUTS: Final = (
    SOURCE_ADMISSION,
    RUNTIME_SUPPORT,
    CODEC_EVIDENCE,
    INSTALL_EVIDENCE,
    INFERENCE_EVIDENCE,
    EVALUATION_EVIDENCE,
)
QUALITY_PROFILE: Final = "muse-glimmer-30b-q4-k-m-text-8k-fedora-first-party-quality"
REPEAT_PROFILE: Final = (
    "muse-glimmer-30b-q4-k-m-text-8k-fedora-diagnostic-repeatability"
)
REVISION: Final = re.compile(r"^[0-9a-f]{40}$")
SHA256: Final = re.compile(r"^[0-9a-f]{64}$")


def relative(path: Path) -> str:
    return str(path.relative_to(ROOT))


def sha256_file(path: Path) -> str:
    return hashlib.sha256(path.read_bytes()).hexdigest()


def load(path: Path) -> dict[str, Any]:
    value = json.loads(path.read_text(encoding="utf-8"))
    if not isinstance(value, dict):
        raise ValueError(f"{relative(path)} must contain one object")
    return value


def resolve_revision(revision: str) -> str:
    result = subprocess.run(
        ["git", "rev-parse", "--verify", f"{revision}^{{commit}}"],
        cwd=ROOT,
        check=False,
        capture_output=True,
        text=True,
        timeout=10,
    )
    if result.returncode != 0:
        raise ValueError("disposition source revision is unavailable")
    return result.stdout.strip()


def binding(path: Path) -> dict[str, str]:
    return {"path": relative(path), "sha256": sha256_file(path)}


def build_record(source_revision: str) -> dict[str, Any]:
    source_revision = resolve_revision(source_revision)
    codec = load(CODEC_EVIDENCE)
    install = load(INSTALL_EVIDENCE)
    inference = load(INFERENCE_EVIDENCE)
    evaluation = load(EVALUATION_EVIDENCE)
    if codec.get("disposition", {}).get("status") != "PASS-CONTRACT":
        raise ValueError("Muse codec contract has not passed")
    if install.get("disposition", {}).get("status") != "ISOLATED-LIFECYCLE-PASS":
        raise ValueError("Muse isolated lifecycle has not passed")
    if inference.get("disposition", {}).get("status") != "SANDBOXED-LIVE-INFERENCE-PASS":
        raise ValueError("Muse sandboxed inference has not passed")
    if evaluation.get("disposition", {}).get("status") != "REJECTED":
        raise ValueError("Muse early quality failure is not retained")
    return {
        "schema_version": 1,
        "record_type": "muse_early_exact_profile_disposition",
        "source_revision": source_revision,
        "profile_ids": [QUALITY_PROFILE, REPEAT_PROFILE],
        "evidence": [binding(path) for path in INPUTS],
        "observed": {
            "source_and_license_recorded": True,
            "exact_artifact_verified": True,
            "exact_runtime_verified": True,
            "codec_contract_passed": True,
            "isolated_lifecycle_passed": True,
            "sandboxed_streaming_passed": True,
            "mid_generation_cancellation_passed": True,
            "quality_trial_count": evaluation["quality"]["trial_count"],
            "quality_closed_proposal_count": evaluation["quality"][
                "closed_proposal_count"
            ],
            "quality_pass_count": evaluation["quality"]["passed_count"],
            "false_completion_count": evaluation["quality"][
                "false_completion_count"
            ],
            "repeatability_trial_count": evaluation["diagnostic_repeatability"][
                "trial_count"
            ],
            "repeatability_unique_response_count": evaluation[
                "diagnostic_repeatability"
            ]["unique_response_count"],
            "packet_capture_executed": inference["disposition"][
                "packet_capture_executed"
            ],
        },
        "decision": {
            "status": "REJECTED",
            "reason_code": "CLOSED-PROPOSAL-QUALITY-THRESHOLD-FAILED",
            "quality_profile_enabled": False,
            "repeatability_profile_enabled": False,
            "automatic_fallback": False,
            "product_activation": False,
            "release_approval": False,
            "family_wide_conclusion": False,
        },
        "re_review_triggers": [
            "model-artifact-change",
            "runtime-build-change",
            "tokenizer-change",
            "template-change",
            "codec-change",
            "context-change",
            "decoding-change",
            "hardware-or-driver-change",
            "evaluation-corpus-or-grader-change",
            "quality-threshold-change",
        ],
        "limitations": [
            "Historical source, runtime, and preflight records remain immutable snapshots of earlier states.",
            "The exact Fedora runtime boundary passed, but the evaluated quality profile produced zero valid closed proposals.",
            "Five equal diagnostic response hashes apply only to the exact recorded tuple and do not establish universal determinism.",
            "Separate packet-capture, macOS, Windows, and Docker evidence is not supplied by this record.",
            "This rejection enables no profile and does not select a fallback model.",
        ],
    }


def validate_record(record: Any, *, verify_inputs: bool = True) -> list[str]:
    if not isinstance(record, dict):
        return ["Muse disposition must be an object"]
    failures: list[str] = []
    if set(record) != {
        "schema_version",
        "record_type",
        "source_revision",
        "profile_ids",
        "evidence",
        "observed",
        "decision",
        "re_review_triggers",
        "limitations",
    }:
        return ["Muse disposition fields are not closed"]
    if (
        record.get("schema_version") != 1
        or record.get("record_type") != "muse_early_exact_profile_disposition"
        or not REVISION.fullmatch(str(record.get("source_revision", "")))
        or record.get("profile_ids") != [QUALITY_PROFILE, REPEAT_PROFILE]
    ):
        failures.append("Muse disposition identity changed")
    evidence = record.get("evidence", [])
    if (
        len(evidence) != len(INPUTS)
        or [item.get("path") for item in evidence] != [relative(path) for path in INPUTS]
        or any(not SHA256.fullmatch(str(item.get("sha256", ""))) for item in evidence)
    ):
        failures.append("Muse evidence closure changed")
    elif verify_inputs:
        for item, path in zip(evidence, INPUTS, strict=True):
            if item["sha256"] != sha256_file(path):
                failures.append(f"Muse evidence is stale: {relative(path)}")
    observed = record.get("observed", {})
    if (
        observed.get("source_and_license_recorded") is not True
        or observed.get("exact_artifact_verified") is not True
        or observed.get("exact_runtime_verified") is not True
        or observed.get("codec_contract_passed") is not True
        or observed.get("isolated_lifecycle_passed") is not True
        or observed.get("sandboxed_streaming_passed") is not True
        or observed.get("mid_generation_cancellation_passed") is not True
        or observed.get("quality_trial_count") != 12
        or observed.get("quality_closed_proposal_count") != 0
        or observed.get("quality_pass_count") != 0
        or observed.get("false_completion_count") != 0
        or observed.get("repeatability_trial_count") != 5
        or observed.get("repeatability_unique_response_count") != 1
        or observed.get("packet_capture_executed") is not False
    ):
        failures.append("Muse observations do not match the early evaluation")
    decision = record.get("decision", {})
    if (
        decision.get("status") != "REJECTED"
        or decision.get("reason_code")
        != "CLOSED-PROPOSAL-QUALITY-THRESHOLD-FAILED"
        or decision.get("quality_profile_enabled") is not False
        or decision.get("repeatability_profile_enabled") is not False
        or decision.get("automatic_fallback") is not False
        or decision.get("product_activation") is not False
        or decision.get("release_approval") is not False
        or decision.get("family_wide_conclusion") is not False
    ):
        failures.append("Muse rejection or non-activation contract changed")
    triggers = record.get("re_review_triggers", [])
    if len(triggers) != 10 or len(set(triggers)) != 10:
        failures.append("Muse re-review trigger closure changed")
    return failures


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--source-revision", default="HEAD")
    parser.add_argument("--write", action="store_true")
    args = parser.parse_args()
    record = build_record(args.source_revision)
    failures = validate_record(record)
    if failures:
        for failure in failures:
            print(f"- {failure}")
        return 1
    if args.write:
        OUTPUT.write_text(json.dumps(record, indent=2, sort_keys=True) + "\n", encoding="utf-8")
    print("Muse exact-profile early disposition: REJECTED; profiles remain disabled")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
