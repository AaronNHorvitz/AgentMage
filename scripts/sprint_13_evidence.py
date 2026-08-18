#!/usr/bin/env python3
"""Build the local Sprint 13 contract, parity, and Muse evidence closure."""

from __future__ import annotations

import argparse
import hashlib
import json
import re
import shutil
import subprocess
from pathlib import Path
from typing import Any, Final


ROOT: Final = Path(__file__).resolve().parents[1]
OUTPUT: Final = ROOT / "artifacts/sprints/sprint-13/local-evidence-report.json"
SOURCE_PATHS: Final = (
    "kernel/contracts/src/model.rs",
    "kernel/engine/src/model_codec.rs",
    "kernel/engine/src/model_response.rs",
    "kernel/engine/src/model_runtime.rs",
    "platforms/linux-inference/src/llama_server_driver.rs",
    "platforms/linux-inference/src/muse_atem_codec.rs",
    "platforms/linux-inference/src/native_model_adapter.rs",
    "platforms/linux-inference/tests/muse_live_evaluation.rs",
    "model-profiles/exact-profile-catalog.json",
    "scripts/sprint_13_evidence.py",
    "scripts/sprint_13_cross_adapter_parity.py",
    "tests/test_sprint_13_evidence.py",
    "tests/test_sprint_13_cross_adapter_parity.py",
)
EVIDENCE_PATHS: Final = (
    "artifacts/sprints/sprint-13/story-13.1/muse-native-adapter-contract-v2.json",
    "artifacts/sprints/sprint-13/story-13.3/d027-s13-muse-codec.json",
    "artifacts/sprints/sprint-13/story-13.3/muse-live-install.json",
    "artifacts/sprints/sprint-13/story-13.3/muse-sandbox-inference.json",
    "artifacts/sprints/sprint-13/story-13.3/muse-profile-evaluation.json",
    "model-profiles/candidates/muse-glimmer-30b-text-8k/early-evaluation-disposition.json",
    "model-profiles/candidates/gemma-4-e4b/feasibility-disposition.json",
    "model-profiles/candidates/gemma-4-12b-unified/feasibility-disposition.json",
    "artifacts/sprints/sprint-13/story-13.2/linux-cross-adapter-parity.json",
)
COMMANDS: Final = (
    (
        "model-contracts",
        ("cargo", "test", "-p", "agentmage-kernel-contracts", "--locked"),
        "13.1",
    ),
    (
        "model-engine",
        ("cargo", "test", "-p", "agentmage-kernel-engine", "--locked"),
        "13.1",
    ),
    (
        "linux-native-adapter",
        ("cargo", "test", "-p", "agentmage-platform-linux-inference", "--locked"),
        "13.1",
    ),
    (
        "model-json-contracts",
        (
            "node",
            "--test",
            "tests/test_model_contract_schemas.mjs",
            "tests/test_model_profile_catalog.mjs",
        ),
        "13.1",
    ),
    (
        "model-policy-evidence",
        (
            "python",
            "-m",
            "unittest",
            "tests.test_model_activation",
            "tests.test_model_admission",
            "tests.test_model_substitution",
            "tests.test_muse_candidate_admission",
            "tests.test_muse_codec_evidence",
            "tests.test_muse_live_inference_evidence",
            "tests.test_muse_profile_evaluation_evidence",
            "tests.test_muse_early_disposition",
        ),
        "13.2",
    ),
)
EXPECTED_SECURITY: Final = {
    "13.1": (
        "SR-PLT-007",
        "SR-SUP-006",
        "SR-SUP-007",
        "SR-SUP-008",
        "SR-SUP-009",
        *(f"SR-AI-{index:03d}" for index in range(1, 17)),
    ),
    "13.2": (
        "SR-SUP-006",
        "SR-SUP-007",
        "SR-SUP-008",
        "SR-AI-006",
        *(f"SR-AI-{index:03d}" for index in range(10, 17)),
        "SR-TST-006",
    ),
    "13.3": ("SR-AI-015", "SR-AI-016", "SR-MGM-004"),
}
REVISION: Final = re.compile(r"^[0-9a-f]{40}$")
SHA256: Final = re.compile(r"^[0-9a-f]{64}$")


def sha256_bytes(value: bytes) -> str:
    return hashlib.sha256(value).hexdigest()


def sha256_file(path: Path) -> str:
    return hashlib.sha256(path.read_bytes()).hexdigest()


def git_file(revision: str, relative: str) -> bytes:
    result = subprocess.run(
        ["git", "show", f"{revision}:{relative}"],
        cwd=ROOT,
        check=False,
        capture_output=True,
        timeout=10,
    )
    if result.returncode != 0:
        raise ValueError(f"committed Sprint 13 source is absent: {relative}")
    return result.stdout


def run_commands() -> list[dict[str, Any]]:
    results = []
    for identifier, argv, story in COMMANDS:
        executable = shutil.which(argv[0])
        if executable is None:
            exit_code = 127
            output = b"executable-unavailable"
        else:
            process = subprocess.run(
                (executable, *argv[1:]),
                cwd=ROOT,
                check=False,
                capture_output=True,
                timeout=900,
            )
            exit_code = process.returncode
            output = process.stdout + process.stderr
        results.append(
            {
                "id": identifier,
                "story_id": story,
                "argv": list(argv),
                "exit_code": exit_code,
                "output_sha256": sha256_bytes(output),
            }
        )
    return results


def production_family_references() -> list[dict[str, Any]]:
    references = []
    for path in sorted((ROOT / "kernel").glob("*/src/*.rs")):
        production = path.read_text(encoding="utf-8").split("#[cfg(test)]", 1)[0]
        for line_number, line in enumerate(production.splitlines(), start=1):
            for family in ("muse", "gemma"):
                if family in line.lower():
                    references.append(
                        {
                            "path": str(path.relative_to(ROOT)),
                            "line": line_number,
                            "family": family,
                        }
                    )
    return references


def evidence_state() -> dict[str, Any]:
    values = {
        path: json.loads((ROOT / path).read_text(encoding="utf-8"))
        for path in EVIDENCE_PATHS
    }
    return {
        "adapter_contract": values[EVIDENCE_PATHS[0]]["disposition"]["status"],
        "codec_contract": values[EVIDENCE_PATHS[1]]["disposition"]["status"],
        "live_lifecycle": values[EVIDENCE_PATHS[2]]["disposition"]["status"],
        "live_inference": values[EVIDENCE_PATHS[3]]["disposition"]["status"],
        "muse_evaluation": values[EVIDENCE_PATHS[4]]["disposition"]["status"],
        "muse_disposition": values[EVIDENCE_PATHS[5]]["decision"]["status"],
        "gemma_e4b_disposition": values[EVIDENCE_PATHS[6]]["decision"]["status"],
        "gemma_12b_disposition": values[EVIDENCE_PATHS[7]]["decision"]["status"],
        "packet_capture_executed": values[EVIDENCE_PATHS[3]]["disposition"][
            "packet_capture_executed"
        ],
        "quality_trial_count": values[EVIDENCE_PATHS[4]]["quality"]["trial_count"],
        "repeatability_trial_count": values[EVIDENCE_PATHS[4]][
            "diagnostic_repeatability"
        ]["trial_count"],
        "linux_parity_status": values[EVIDENCE_PATHS[8]]["disposition"]["status"],
        "linux_parity_trials_complete": values[EVIDENCE_PATHS[8]]["disposition"][
            "matched_linux_trials_complete"
        ],
        "linux_parity_thresholds_passed": values[EVIDENCE_PATHS[8]]["disposition"][
            "all_adapters_meet_thresholds"
        ],
    }


def build_report(source_revision: str, commands: list[dict[str, Any]]) -> dict[str, Any]:
    references = production_family_references()
    state = evidence_state()
    command_pass = all(item["exit_code"] == 0 for item in commands)
    expected_state = {
        "adapter_contract": "CONTRACT-PASS-LIVE-BLOCKED",
        "codec_contract": "PASS-CONTRACT",
        "live_lifecycle": "ISOLATED-LIFECYCLE-PASS",
        "live_inference": "SANDBOXED-LIVE-INFERENCE-PASS",
        "muse_evaluation": "REJECTED",
        "muse_disposition": "REJECTED",
        "gemma_e4b_disposition": "REJECTED",
        "gemma_12b_disposition": "REJECTED",
        "packet_capture_executed": False,
        "quality_trial_count": 12,
        "repeatability_trial_count": 5,
        "linux_parity_status": "COMPLETE-NEGATIVE-BLOCKED-QUALITY",
        "linux_parity_trials_complete": True,
        "linux_parity_thresholds_passed": False,
    }
    local_contract_pass = command_pass and not references and state == expected_state
    return {
        "schema_version": 1,
        "record_type": "sprint_13_local_evidence",
        "source_revision": source_revision,
        "source_sha256": {
            path: sha256_bytes(git_file(source_revision, path)) for path in SOURCE_PATHS
        },
        "evidence_sha256": {path: sha256_file(ROOT / path) for path in EVIDENCE_PATHS},
        "commands": commands,
        "production_kernel_family_references": references,
        "security_requirement_ids": EXPECTED_SECURITY,
        "evidence_state": state,
        "stories": [
            {
                "story_id": "13.1",
                "status": (
                    "PASS-LOCAL-CONTRACT-BLOCKED-MACOS"
                    if local_contract_pass
                    else "BLOCKED"
                ),
                "local_contract_passed": local_contract_pass,
                "macos_adapter_implemented": False,
                "macos_execution_evidence": False,
            },
            {
                "story_id": "13.2",
                "status": (
                    "COMPLETE-NEGATIVE-LINUX-PARITY-BLOCKED-MACOS"
                    if local_contract_pass
                    else "BLOCKED"
                ),
                "no_fallback_passed": local_contract_pass,
                "linux_native_live_evidence": local_contract_pass,
                "docker_live_parity_evidence": True,
                "all_linux_adapters_meet_thresholds": False,
                "macos_live_parity_evidence": False,
            },
            {
                "story_id": "13.3",
                "status": (
                    "REJECTED-CANDIDATE-BLOCKED-PACKET-CAPTURE"
                    if local_contract_pass
                    else "BLOCKED"
                ),
                "exact_profile_disposition": state["muse_disposition"],
                "quality_and_repeatability_measured": local_contract_pass,
                "packet_capture_executed": False,
                "profile_enabled": False,
            },
        ],
        "blockers": [
            {
                "id": "macos-native-adapter",
                "status": "BLOCKED-EXTERNAL-PLATFORM",
                "substitution": False,
            },
            {
                "id": "cross-adapter-live-parity",
                "status": "BLOCKED-MACOS-AND-QUALITY",
                "substitution": False,
            },
            {
                "id": "packet-capture",
                "status": "BLOCKED-CAPTURE-TOOL-UNAVAILABLE",
                "substitution": False,
            },
        ],
        "summary": {
            "local_contract_passed": local_contract_pass,
            "candidate_disposition": "REJECTED",
            "profile_enabled": False,
            "automatic_fallback": False,
            "product_acceptance": False,
            "release_approval": False,
            "sprint_status": "BLOCKED",
        },
        "limitations": [
            "Complete negative Linux parity does not substitute for macOS or packet-capture evidence and does not satisfy quality thresholds.",
            "The rejected Muse profile remains disabled; another profile requires independent admission.",
            "No model output or prompt is retained in this report.",
        ],
    }


def validate_report(report: Any, *, verify_current: bool = True) -> list[str]:
    if not isinstance(report, dict):
        return ["Sprint 13 evidence must be an object"]
    failures: list[str] = []
    if set(report) != {
        "schema_version",
        "record_type",
        "source_revision",
        "source_sha256",
        "evidence_sha256",
        "commands",
        "production_kernel_family_references",
        "security_requirement_ids",
        "evidence_state",
        "stories",
        "blockers",
        "summary",
        "limitations",
    }:
        return ["Sprint 13 evidence fields are not closed"]
    if (
        report.get("schema_version") != 1
        or report.get("record_type") != "sprint_13_local_evidence"
        or not REVISION.fullmatch(str(report.get("source_revision", "")))
    ):
        failures.append("Sprint 13 evidence identity changed")
    sources = report.get("source_sha256", {})
    evidence = report.get("evidence_sha256", {})
    if set(sources) != set(SOURCE_PATHS) or any(
        not SHA256.fullmatch(str(value)) for value in sources.values()
    ):
        failures.append("Sprint 13 source closure changed")
    if set(evidence) != set(EVIDENCE_PATHS) or any(
        not SHA256.fullmatch(str(value)) for value in evidence.values()
    ):
        failures.append("Sprint 13 evidence closure changed")
    commands = report.get("commands", [])
    expected_commands = {identifier: list(argv) for identifier, argv, _ in COMMANDS}
    if (
        len(commands) != len(COMMANDS)
        or {item.get("id") for item in commands} != set(expected_commands)
        or any(item.get("argv") != expected_commands.get(item.get("id")) for item in commands)
        or any(item.get("exit_code") != 0 for item in commands)
        or any(not SHA256.fullmatch(str(item.get("output_sha256", ""))) for item in commands)
    ):
        failures.append("Sprint 13 command evidence is invalid")
    if report.get("production_kernel_family_references") != []:
        failures.append("production kernel contains a model-family reference")
    stories = report.get("stories", [])
    if (
        len(stories) != 3
        or [item.get("story_id") for item in stories] != ["13.1", "13.2", "13.3"]
        or stories[0].get("macos_adapter_implemented") is not False
        or stories[1].get("docker_live_parity_evidence") is not True
        or stories[1].get("all_linux_adapters_meet_thresholds") is not False
        or stories[2].get("profile_enabled") is not False
    ):
        failures.append("Sprint 13 story truth changed")
    blockers = report.get("blockers", [])
    if (
        [item.get("id") for item in blockers]
        != ["macos-native-adapter", "cross-adapter-live-parity", "packet-capture"]
        or any(item.get("substitution") is not False for item in blockers)
    ):
        failures.append("Sprint 13 blocker closure changed")
    if report.get("summary") != {
        "local_contract_passed": True,
        "candidate_disposition": "REJECTED",
        "profile_enabled": False,
        "automatic_fallback": False,
        "product_acceptance": False,
        "release_approval": False,
        "sprint_status": "BLOCKED",
    }:
        failures.append("Sprint 13 summary overstates or loses local evidence")
    if verify_current:
        for path, digest in evidence.items():
            if sha256_file(ROOT / path) != digest:
                failures.append(f"Sprint 13 evidence is stale: {path}")
    return failures


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--source-revision", required=True)
    parser.add_argument("--write", action="store_true")
    args = parser.parse_args()
    commands = run_commands()
    report = build_report(args.source_revision, commands)
    failures = validate_report(report)
    if failures:
        for failure in failures:
            print(f"- {failure}")
        return 1
    if args.write:
        OUTPUT.parent.mkdir(parents=True, exist_ok=True)
        OUTPUT.write_text(json.dumps(report, indent=2, sort_keys=True) + "\n", encoding="utf-8")
    print("Sprint 13 local contracts pass; sprint remains blocked on three explicit evidence classes")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
