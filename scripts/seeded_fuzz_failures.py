#!/usr/bin/env python3
"""Run deterministic seeded security failures through the fake fuzz boundary."""

from __future__ import annotations

import argparse
import base64
import hashlib
import json
import os
import sys
import tempfile
from pathlib import Path
from typing import Any, Callable


ROOT = Path(__file__).resolve().parents[1]
if str(ROOT) not in sys.path:
    sys.path.insert(0, str(ROOT))

from scripts.fuzz_story_gate import build_policy as build_gate_policy  # noqa: E402
from scripts.fuzz_story_gate import evaluate_target  # noqa: E402


SEED_CORPUS_PATH = ROOT / "fuzzing/seeds/security-failures-v1.json"
REPORT_PATH = ROOT / "artifacts/sprints/sprint-2/story-2.2/seeded-failure-report.json"
TARGET_REGISTRY_PATH = ROOT / "fuzzing/target-registry.json"
TOOLCHAIN_POLICY_PATH = ROOT / "fuzzing/toolchain-policy.json"
PLATFORM_REPORT_PATH = (
    ROOT
    / "artifacts/sprints/sprint-2/story-2.1/platform-result-recorder-report.json"
)
SEED_SPECS = (
    ("seed-crash-001", "crash", "FT-MANIFEST-001", "configuration-boundary", "high"),
    ("seed-hang-001", "hang", "FT-IPC-001", "ipc-boundary", "high"),
    (
        "seed-resource-001",
        "resource-exhaustion",
        "FT-MODEL-OUTPUT-001",
        "model-output-boundary",
        "high",
    ),
    ("seed-path-001", "path-escape", "FT-PATH-001", "path-boundary", "critical"),
    ("seed-secret-001", "secret-leak", "FT-TEXT-001", "redaction-boundary", "critical"),
    (
        "seed-authorization-001",
        "authorization-bypass",
        "FT-GRANT-001",
        "capability-grant-boundary",
        "critical",
    ),
)
LITERAL_PAYLOADS = {
    "crash": b"noise:CRASH:synthetic-tail",
    "hang": b"noise:HANG:logical-step-budget:synthetic-tail",
    "resource-exhaustion": b"noise:RESOURCE:logical-memory-budget:synthetic-tail",
    "path-escape": b"noise:../../synthetic-outside:tail",
    "authorization-bypass": b"noise:BYPASS:missing-grant:synthetic-tail",
}
DETECTORS: dict[str, Callable[[bytes], bool]] = {
    "crash": lambda value: b"CRASH" in value,
    "hang": lambda value: b"HANG" in value,
    "resource-exhaustion": lambda value: b"RESOURCE" in value,
    "path-escape": lambda value: b"../" in value,
    "secret-leak": lambda value: b"AM_SYNTHETIC_SECRET_" in value,
    "authorization-bypass": lambda value: b"BYPASS" in value,
}


def canonical_json(value: Any) -> bytes:
    return (json.dumps(value, indent=2, sort_keys=True) + "\n").encode("utf-8")


def sha256_bytes(value: bytes) -> str:
    return hashlib.sha256(value).hexdigest()


def sha256_file(path: Path) -> str:
    return sha256_bytes(path.read_bytes())


def read_json(path: Path) -> Any:
    return json.loads(path.read_text(encoding="utf-8"))


def write_atomic(path: Path, content: bytes) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    descriptor, temporary_name = tempfile.mkstemp(
        prefix=".agentmage-seeded-fuzz-", dir=path.parent
    )
    temporary = Path(temporary_name)
    try:
        with os.fdopen(descriptor, "wb") as handle:
            handle.write(content)
            handle.flush()
            os.fsync(handle.fileno())
        temporary.chmod(0o644)
        os.replace(temporary, path)
    except BaseException:
        temporary.unlink(missing_ok=True)
        raise


def derived_secret(seed_id: str) -> bytes:
    digest = sha256_bytes(f"agentmage/{seed_id}/synthetic-canary".encode("ascii"))[:24]
    return f"AM_SYNTHETIC_SECRET_{digest.upper()}".encode("ascii")


def materialize_seed(seed: dict[str, Any]) -> bytes:
    if seed["payload_kind"] == "derived-synthetic-canary":
        return b"noise:" + derived_secret(seed["seed_id"]) + b":synthetic-tail"
    return base64.b64decode(seed["payload_base64"], validate=True)


def seed_record(
    seed_id: str,
    failure_class: str,
    target_id: str,
    owner: str,
    severity: str,
) -> dict[str, Any]:
    if failure_class == "secret-leak":
        payload = {
            "payload_kind": "derived-synthetic-canary",
            "payload_base64": None,
            "derivation_sha256": sha256_bytes(
                f"agentmage/{seed_id}/synthetic-canary".encode("ascii")
            ),
        }
    else:
        content = LITERAL_PAYLOADS[failure_class]
        payload = {
            "payload_kind": "literal-base64",
            "payload_base64": base64.b64encode(content).decode("ascii"),
            "derivation_sha256": None,
        }
    record = {
        "seed_id": seed_id,
        "failure_class": failure_class,
        "target_id": target_id,
        "owner": owner,
        "severity": severity,
        **payload,
        "expected_disposition": "quarantined-blocking",
        "private_user_data": False,
        "real_credential": False,
        "host_resource_exhaustion": False,
    }
    return {**record, "seed_sha256": sha256_bytes(canonical_json(record))}


def build_seed_corpus() -> dict[str, Any]:
    seeds = [seed_record(*spec) for spec in SEED_SPECS]
    value = {
        "schema_version": 1,
        "corpus_id": "agentmage-seeded-security-failures-v1",
        "corpus_version": "1.0.0",
        "status": "bounded-synthetic-security-failures",
        "seeds": seeds,
        "coverage": {
            "required_failure_classes": [item[1] for item in SEED_SPECS],
            "seed_count": len(seeds),
            "complete": True,
        },
        "content_contract": {
            "private_user_data": False,
            "real_credentials": False,
            "network_required": False,
            "external_process_required": False,
            "host_resource_exhaustion_permitted": False,
            "secret_canary_persisted": False,
        },
        "product_boundary_execution_claim": "none",
        "macos_execution_status": "blocked-macos",
        "macos_support_claim": "none",
    }
    return {**value, "corpus_sha256": sha256_bytes(canonical_json(value))}


def validate_seed_corpus(value: Any) -> list[str]:
    if not isinstance(value, dict):
        return ["seeded security failure corpus must be an object"]
    failures: list[str] = []
    if (
        value.get("schema_version") != 1
        or value.get("corpus_id") != "agentmage-seeded-security-failures-v1"
        or value.get("corpus_version") != "1.0.0"
    ):
        failures.append("seeded security failure corpus identity is invalid")
    seeds = value.get("seeds", [])
    if [item.get("failure_class") for item in seeds] != [item[1] for item in SEED_SPECS]:
        failures.append("seeded security failure class closure is invalid")
    for item in seeds:
        core = {key: field for key, field in item.items() if key != "seed_sha256"}
        if item.get("seed_sha256") != sha256_bytes(canonical_json(core)):
            failures.append(f"seeded security failure hash is invalid: {item.get('seed_id')}")
        if item.get("private_user_data") is not False or item.get(
            "real_credential"
        ) is not False or item.get("host_resource_exhaustion") is not False:
            failures.append(f"seeded security failure is unsafe: {item.get('seed_id')}")
        if item.get("failure_class") == "secret-leak" and item.get(
            "payload_base64"
        ) is not None:
            failures.append("secret failure seed persisted a canary value")
        try:
            payload = materialize_seed(item)
        except (ValueError, TypeError, KeyError) as error:
            failures.append(f"seeded security payload is invalid: {error}")
        else:
            if len(payload) > 256 or not DETECTORS[item["failure_class"]](payload):
                failures.append(f"seeded security detector did not match: {item.get('seed_id')}")
    if value.get("content_contract", {}).get("secret_canary_persisted") is not False:
        failures.append("seeded security corpus persisted a canary")
    if value.get("product_boundary_execution_claim") != "none":
        failures.append("seeded security corpus made a product claim")
    if value.get("macos_execution_status") != "blocked-macos" or value.get(
        "macos_support_claim"
    ) != "none":
        failures.append("seeded security corpus made an invalid macOS claim")
    expected = build_seed_corpus()
    if value != expected:
        failures.append("seeded security corpus is stale or non-deterministic")
    return failures


def minimize_payload(payload: bytes, failure_class: str) -> bytes:
    detector = DETECTORS[failure_class]
    if not detector(payload):
        raise ValueError("seed payload does not reproduce expected failure")
    current = payload
    granularity = 2
    while len(current) > 1:
        chunk_size = max(1, (len(current) + granularity - 1) // granularity)
        reduced = False
        for start in range(0, len(current), chunk_size):
            candidate = current[:start] + current[start + chunk_size :]
            if candidate and detector(candidate):
                current = candidate
                granularity = max(2, granularity - 1)
                reduced = True
                break
        if not reduced:
            if granularity >= len(current):
                break
            granularity = min(len(current), granularity * 2)
    return current


def resource_event(failure_class: str) -> dict[str, Any]:
    if failure_class == "hang":
        return {
            "event": "run-timeout",
            "limit": 5,
            "observed": 6,
            "unit": "seconds",
            "host_exhausted": False,
        }
    if failure_class == "resource-exhaustion":
        return {
            "event": "memory",
            "limit": 1073741824,
            "observed": 1073741825,
            "unit": "bytes",
            "host_exhausted": False,
        }
    return {
        "event": "none",
        "limit": None,
        "observed": None,
        "unit": None,
        "host_exhausted": False,
    }


def fuzz_result(
    seed: dict[str, Any],
    payload: bytes,
    minimized: bytes,
    regression_path: str,
    root: Path,
) -> dict[str, Any]:
    failure_class = seed["failure_class"]
    registry_path = root / TARGET_REGISTRY_PATH.relative_to(ROOT)
    toolchain_path = root / TOOLCHAIN_POLICY_PATH.relative_to(ROOT)
    platform_path = root / PLATFORM_REPORT_PATH.relative_to(ROOT)
    toolchain = read_json(toolchain_path)
    target = next(
        item
        for item in read_json(registry_path)["targets"]
        if item["target_id"] == seed["target_id"]
    )
    normalized_frames = [f"fake_{target['boundary_class'].replace('-', '_')}::evaluate"]
    signature_basis = {
        "target_id": seed["target_id"],
        "failure_class": failure_class,
        "sanitizer": "deterministic-security-oracle",
        "normalized_top_frames": normalized_frames,
    }
    crash_signature = sha256_bytes(canonical_json(signature_basis))
    sanitizer_state = {
        "enabled": ["deterministic-security-oracle"],
        "triggered": [f"{failure_class}-oracle"],
        "finding_count": 1,
    }
    coverage_state = {
        "target_id": seed["target_id"],
        "seed_id": seed["seed_id"],
        "features_covered": len(payload),
        "new_features": 1,
        "corpus_entry_count": 1,
    }
    original_hash = sha256_bytes(payload)
    minimized_hash = sha256_bytes(minimized)
    identity = sha256_bytes(canonical_json(signature_basis))[:20]
    value = {
        "schema_version": 1,
        "record_type": "fuzz-result",
        "record_id": f"am-fuzz-result-{identity}",
        "captured_at": "2024-01-01T00:20:00Z",
        "target": {
            "target_id": seed["target_id"],
            "boundary_class": target["boundary_class"],
            "registry_sha256": sha256_file(registry_path),
            "harness_sha256": sha256_file(root / "scripts/seeded_fuzz_failures.py"),
            "corpus_sha256": sha256_file(root / SEED_CORPUS_PATH.relative_to(ROOT)),
            "dictionary_sha256": [item["sha256"] for item in toolchain["dictionaries"]],
            "build_sha256": sha256_file(root / "Cargo.lock"),
            "policy_sha256": sha256_file(toolchain_path),
            "platform_result_sha256": sha256_file(platform_path),
            "engine": {
                "engine_id": "agentmage-bounded-fake-fuzzer",
                "version": "1.0.0",
                "integrity": sha256_bytes(b"agentmage-bounded-fake-fuzzer-v1"),
            },
        },
        "run": {
            "mode": "fuzzing",
            "seed": int(sha256_bytes(seed["seed_id"].encode("ascii"))[:8], 16),
            "duration_seconds": 60,
            "execution_count": len(payload) + len(minimized),
            "maximum_input_bytes": 4096,
            "result_status": failure_class,
        },
        "coverage": {
            "kind": "deterministic-oracle",
            "features_covered": len(payload),
            "new_features": 1,
            "corpus_entry_count": 1,
            "coverage_sha256": sha256_bytes(canonical_json(coverage_state)),
            "unavailable_reason": None,
        },
        "sanitizer": {
            **sanitizer_state,
            "state_sha256": sha256_bytes(canonical_json(sanitizer_state)),
        },
        "failure": {
            "failure_class": failure_class,
            "crash_signature": crash_signature,
            "raw_input_sha256": original_hash,
            "stack_sha256": None,
            "normalized_top_frames": normalized_frames,
            "raw_input_retained": False,
        },
        "minimized_reproducer": {
            "status": "minimized",
            "original_sha256": original_hash,
            "minimized_sha256": minimized_hash,
            "path": regression_path,
            "bytes": len(minimized),
            "failure_signature_preserved": True,
            "raw_private_input_retained": False,
        },
        "timeout_resource_event": resource_event(failure_class),
        "ownership": {
            "owner": seed["owner"],
            "severity": seed["severity"],
            "disposition": "quarantined-blocking",
            "duplicate_of": None,
            "risk_decision": None,
        },
        "evidence": [
            {
                "path": SEED_CORPUS_PATH.relative_to(ROOT).as_posix(),
                "sha256": sha256_file(root / SEED_CORPUS_PATH.relative_to(ROOT)),
            },
            {
                "path": TOOLCHAIN_POLICY_PATH.relative_to(ROOT).as_posix(),
                "sha256": sha256_file(toolchain_path),
            },
        ],
        "redaction": {
            "secret_canary_values_recorded": False,
            "private_paths_recorded": False,
            "raw_payload_recorded": False,
            "redaction_count": 1 if failure_class == "secret-leak" else 0,
        },
        "network_used": False,
        "product_fuzz_claim": "none",
        "macos_execution_status": "blocked-macos",
        "macos_support_claim": "none",
    }
    return {**value, "record_sha256": sha256_bytes(canonical_json(value))}


def campaign(root: Path = ROOT) -> tuple[dict[str, Any], dict[str, bytes]]:
    seed_corpus = read_json(root / SEED_CORPUS_PATH.relative_to(ROOT))
    seed_failures = validate_seed_corpus(seed_corpus)
    if seed_failures:
        raise ValueError("; ".join(seed_failures))
    registry = read_json(root / TARGET_REGISTRY_PATH.relative_to(ROOT))
    targets = {item["target_id"]: item for item in registry["targets"]}
    gate_policy = build_gate_policy(root)
    results = []
    regressions: dict[str, bytes] = {}
    gate_failures = []
    derived_canaries = []
    for seed in seed_corpus["seeds"]:
        payload = materialize_seed(seed)
        if seed["failure_class"] == "secret-leak":
            derived_canaries.append(derived_secret(seed["seed_id"]))
        minimized = minimize_payload(payload, seed["failure_class"])
        if seed["failure_class"] == "secret-leak":
            minimized = b"AM_SYNTHETIC_SECRET_REDACTED"
        if not DETECTORS[seed["failure_class"]](minimized):
            raise ValueError(f"minimized seed lost failure: {seed['seed_id']}")
        minimized_hash = sha256_bytes(minimized)
        regression_path = (
            f"fuzzing/regressions/{seed['target_id']}/{minimized_hash}.bin"
        )
        regressions[regression_path] = minimized
        result = fuzz_result(seed, payload, minimized, regression_path, root)
        results.append(result)
        gate_evidence = {
            "target_id": seed["target_id"],
            "boundary_implemented": True,
            "target_registered": True,
            "target_registry_sha256": gate_policy["inputs"]["target_registry"]["sha256"],
            "corpus_path": SEED_CORPUS_PATH.relative_to(ROOT).as_posix(),
            "corpus_sha256": sha256_file(root / SEED_CORPUS_PATH.relative_to(ROOT)),
            "resource_policy_sha256": gate_policy["inputs"]["toolchain_policy"]["sha256"],
            "result_path": f"artifacts/fuzz/{seed['target_id']}/result.json",
            "result_sha256": result["record_sha256"],
            "result_schema_valid": True,
            "result_status": result["run"]["result_status"],
            "regression_path": regression_path,
            "regression_replay_passed": True,
        }
        failures = evaluate_target(targets[seed["target_id"]], gate_evidence, gate_policy)
        if "result-non-pass" not in failures:
            raise ValueError(f"seeded failure did not block gate: {seed['seed_id']}")
        gate_failures.append(
            {
                "target_id": seed["target_id"],
                "status": "block",
                "failures": failures,
            }
        )
    serialized = canonical_json(results)
    if any(canary in serialized for canary in derived_canaries):
        raise ValueError("seeded failure report exposed a secret canary value")
    report = {
        "schema_version": 1,
        "task_id": "2.2.2.1",
        "status": "pass-seeded-failures-blocked",
        "seed_corpus": {
            "path": SEED_CORPUS_PATH.relative_to(ROOT).as_posix(),
            "sha256": sha256_file(root / SEED_CORPUS_PATH.relative_to(ROOT)),
            "self_sha256": seed_corpus["corpus_sha256"],
            "version": seed_corpus["corpus_version"],
        },
        "results": results,
        "regressions": [
            {"path": path, "sha256": sha256_bytes(content), "bytes": len(content)}
            for path, content in sorted(regressions.items())
        ],
        "gate_results": gate_failures,
        "summary": {
            "seed_count": 6,
            "detected_failure_count": 6,
            "non_pass_result_count": 6,
            "minimized_reproducer_count": 6,
            "owned_disposition_count": 6,
            "blocking_gate_result_count": 6,
            "schema_shaped_result_count": 6,
            "host_resource_exhaustion_count": 0,
            "network_call_count": 0,
        },
        "secret_canary_values_recorded": False,
        "raw_payloads_recorded": False,
        "private_user_data_used": False,
        "product_boundary_execution_claim": "none",
        "macos_execution_status": "blocked-macos",
        "macos_support_claim": "none",
    }
    return report, regressions


def validate_report(value: Any, root: Path = ROOT) -> list[str]:
    if not isinstance(value, dict):
        return ["seeded fuzz failure report must be an object"]
    failures: list[str] = []
    if (
        value.get("schema_version") != 1
        or value.get("task_id") != "2.2.2.1"
        or value.get("status") != "pass-seeded-failures-blocked"
    ):
        failures.append("seeded fuzz failure report identity is invalid")
    results = value.get("results", [])
    if len(results) != 6 or any(
        item.get("run", {}).get("result_status") == "pass" for item in results
    ):
        failures.append("seeded fuzz failure results were omitted or converted to pass")
    if any(
        item.get("minimized_reproducer", {}).get("status") != "minimized"
        or item.get("ownership", {}).get("disposition") != "quarantined-blocking"
        for item in results
    ):
        failures.append("seeded fuzz minimization or ownership is invalid")
    if len(value.get("gate_results", [])) != 6 or any(
        item.get("status") != "block" or "result-non-pass" not in item.get("failures", [])
        for item in value.get("gate_results", [])
    ):
        failures.append("seeded fuzz failures did not block their gates")
    if value.get("summary") != {
        "seed_count": 6,
        "detected_failure_count": 6,
        "non_pass_result_count": 6,
        "minimized_reproducer_count": 6,
        "owned_disposition_count": 6,
        "blocking_gate_result_count": 6,
        "schema_shaped_result_count": 6,
        "host_resource_exhaustion_count": 0,
        "network_call_count": 0,
    }:
        failures.append("seeded fuzz failure report summary is invalid")
    if (
        value.get("secret_canary_values_recorded") is not False
        or value.get("raw_payloads_recorded") is not False
        or value.get("private_user_data_used") is not False
    ):
        failures.append("seeded fuzz failure report exposed prohibited data")
    if value.get("product_boundary_execution_claim") != "none":
        failures.append("seeded fuzz failure report made a product claim")
    if value.get("macos_execution_status") != "blocked-macos" or value.get(
        "macos_support_claim"
    ) != "none":
        failures.append("seeded fuzz failure report made an invalid macOS claim")
    try:
        expected, regressions = campaign(root)
    except (OSError, ValueError, KeyError, TypeError) as error:
        failures.append(f"cannot rebuild seeded fuzz failure report: {error}")
    else:
        if value != expected:
            failures.append("seeded fuzz failure report is stale or non-deterministic")
        for relative, content in regressions.items():
            path = root / relative
            if not path.is_file() or path.read_bytes() != content:
                failures.append(f"seeded fuzz regression is missing or stale: {relative}")
    return failures


def write_seed_corpus(root: Path = ROOT) -> None:
    write_atomic(
        root / SEED_CORPUS_PATH.relative_to(ROOT), canonical_json(build_seed_corpus())
    )


def write_artifacts(root: Path = ROOT) -> None:
    write_seed_corpus(root)
    report, regressions = campaign(root)
    for relative, content in regressions.items():
        write_atomic(root / relative, content)
    write_atomic(root / REPORT_PATH.relative_to(ROOT), canonical_json(report))


def check_artifacts(root: Path = ROOT) -> list[str]:
    failures: list[str] = []
    try:
        seed_corpus = read_json(root / SEED_CORPUS_PATH.relative_to(ROOT))
    except (OSError, json.JSONDecodeError) as error:
        failures.append(f"cannot read seeded security failure corpus: {error}")
    else:
        failures.extend(validate_seed_corpus(seed_corpus))
    try:
        report = read_json(root / REPORT_PATH.relative_to(ROOT))
    except (OSError, json.JSONDecodeError) as error:
        failures.append(f"cannot read seeded fuzz failure report: {error}")
    else:
        failures.extend(validate_report(report, root))
    return failures


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--write-seeds", action="store_true")
    parser.add_argument("--write", action="store_true")
    args = parser.parse_args()
    try:
        if args.write_seeds:
            write_seed_corpus()
            failures = validate_seed_corpus(read_json(SEED_CORPUS_PATH))
        else:
            if args.write:
                write_artifacts()
            failures = check_artifacts()
    except (OSError, ValueError, KeyError, TypeError) as error:
        print(f"seeded fuzz failures failed: {error}", file=sys.stderr)
        return 1
    if failures:
        for failure in failures:
            print(f"seeded fuzz failures failed: {failure}", file=sys.stderr)
        return 1
    print("Story 2.2 seeded fuzz security failures validated")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
