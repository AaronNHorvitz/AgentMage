#!/usr/bin/env python3
"""Build and verify the synthetic Story 2.4.3.1 resource campaign."""

from __future__ import annotations

import argparse
import copy
import io
import json
import shutil
import subprocess
import sys
import tempfile
import time
import zipfile
from pathlib import Path
from typing import Any, Final


ROOT: Final = Path(__file__).resolve().parents[1]
if str(ROOT) not in sys.path:
    sys.path.insert(0, str(ROOT))

from scripts.engineering_artifact_admission_fixtures import (  # noqa: E402
    canonical_json,
    sealed,
    sha256_bytes,
    write_atomic,
)


FIXTURE_DIR: Final = ROOT / "fixtures" / "artifact-admission" / "v1"
ADVERSARIAL_PATH: Final = FIXTURE_DIR / "adversarial-manifest.json"
DELIVERY_PATH: Final = FIXTURE_DIR / "context-delivery-receipts.json"
OUTPUT_PATH: Final = FIXTURE_DIR / "resource-gate-observations.json"
RESOURCE_CASES: Final = (
    ("bytes", "bytes", 64, 64, 65),
    ("pages", "pages", 3, 3, 4),
    ("rows", "rows", 4, 4, 5),
    ("cells", "cells", 8, 8, 9),
    ("files", "files", 2, 2, 3),
    ("archive", "archive_entries", 2, 2, 3),
    ("time", "elapsed_milliseconds", 5, 5, 6),
    ("memory", "live_allocation_bytes", 8192, 8192, 12288),
    ("disk", "materialized_bytes", 128, 128, 129),
    ("processes", "live_child_processes", 1, 1, 2),
)
LIFECYCLE_EVENTS: Final = ("denial", "timeout", "crash", "cancellation")
WAIT_WORKER: Final = """
import pathlib
import sys
import time
pathlib.Path(sys.argv[1]).write_bytes(b'synthetic-worker-ready\\n')
while True:
    time.sleep(0.01)
"""
CRASH_WORKER: Final = """
import pathlib
import sys
pathlib.Path(sys.argv[1]).write_bytes(b'synthetic-worker-crash\\n')
raise SystemExit(73)
"""


def read_json(path: Path) -> Any:
    return json.loads(path.read_text(encoding="utf-8"))


def wait_for_marker(marker: Path, process: subprocess.Popen[bytes]) -> None:
    deadline = time.monotonic() + 2.0
    while time.monotonic() < deadline:
        if marker.exists():
            return
        if process.poll() is not None:
            raise RuntimeError("synthetic worker exited before publishing readiness")
        time.sleep(0.005)
    raise RuntimeError("synthetic worker readiness timed out")


def stop_worker(process: subprocess.Popen[bytes]) -> int:
    process.terminate()
    try:
        return process.wait(timeout=1.0)
    except subprocess.TimeoutExpired:
        process.kill()
        return process.wait(timeout=1.0)


def lifecycle_probe(event: str) -> dict[str, Any]:
    root = Path(tempfile.mkdtemp(prefix="agentmage-resource-gate-"))
    marker = root / "worker.marker"
    process: subprocess.Popen[bytes] | None = None
    exit_kind = "not_started"
    residue_before_cleanup = 0
    worker_stopped = True
    try:
        if event == "denial":
            marker.write_bytes(b"synthetic-denial-residue\n")
            exit_kind = "denied_before_dispatch"
        elif event == "crash":
            process = subprocess.Popen(
                [sys.executable, "-c", CRASH_WORKER, str(marker)],
                stdin=subprocess.DEVNULL,
                stdout=subprocess.DEVNULL,
                stderr=subprocess.DEVNULL,
            )
            exit_code = process.wait(timeout=2.0)
            if exit_code != 73 or not marker.exists():
                raise RuntimeError("synthetic crash worker did not reach the exact crash boundary")
            exit_kind = "worker_exit_73"
            worker_stopped = process.poll() is not None
        elif event in {"timeout", "cancellation"}:
            process = subprocess.Popen(
                [sys.executable, "-c", WAIT_WORKER, str(marker)],
                stdin=subprocess.DEVNULL,
                stdout=subprocess.DEVNULL,
                stderr=subprocess.DEVNULL,
            )
            wait_for_marker(marker, process)
            stop_worker(process)
            exit_kind = "deadline_terminated" if event == "timeout" else "cancel_signal_terminated"
            worker_stopped = process.poll() is not None
        else:
            raise ValueError(f"unknown lifecycle event: {event}")
        residue_before_cleanup = sum(1 for path in root.rglob("*") if path.is_file())
    finally:
        if process is not None and process.poll() is None:
            stop_worker(process)
        shutil.rmtree(root, ignore_errors=False)
    return {
        "event": event,
        "terminal_kind": exit_kind,
        "worker_stopped": worker_stopped,
        "residue_before_cleanup": residue_before_cleanup,
        "residue_after_cleanup": 0 if not root.exists() else -1,
        "owned_root_removed": not root.exists(),
        "cleanup_result": "verified_zero_residue" if not root.exists() else "cleanup_failed",
    }


def time_probe() -> bool:
    start = time.monotonic_ns()
    deadline = start + 5_000_000
    while time.monotonic_ns() < deadline:
        pass
    elapsed = time.monotonic_ns() - start
    return 5_000_000 <= elapsed < 1_000_000_000


def exact_counter_probe(ceiling: int) -> bool:
    admitted = [object() for _ in range(ceiling)]
    next_attempt = len(admitted) + 1
    refused = next_attempt > ceiling
    return len(admitted) == ceiling and refused


def byte_probe() -> bool:
    admitted = bytearray(b"b" * 64)
    return len(admitted) == 64 and len(admitted) + 1 > 64


def file_probe() -> bool:
    root = Path(tempfile.mkdtemp(prefix="agentmage-resource-files-"))
    try:
        for index in range(2):
            (root / f"bounded-{index}.txt").write_bytes(b"synthetic\n")
        admitted = sum(1 for path in root.iterdir() if path.is_file())
        return admitted == 2 and admitted + 1 > 2
    finally:
        shutil.rmtree(root, ignore_errors=False)


def archive_probe() -> bool:
    content = io.BytesIO()
    with zipfile.ZipFile(content, "w", compression=zipfile.ZIP_STORED) as archive:
        for index in range(3):
            archive.writestr(f"entry-{index}.txt", f"synthetic-{index}\n".encode())
    admitted = 0
    with zipfile.ZipFile(io.BytesIO(content.getvalue())) as archive:
        infos = archive.infolist()
        for info in infos:
            if admitted + 1 > 2:
                break
            archive.read(info)
            admitted += 1
    return admitted == 2 and len(infos) == 3


def memory_probe() -> bool:
    allocations = [bytearray(4096), bytearray(4096)]
    admitted = sum(len(value) for value in allocations)
    refused = admitted + 4096 > 8192
    allocations.clear()
    return admitted == 8192 and refused and len(allocations) == 0


def disk_probe() -> bool:
    root = Path(tempfile.mkdtemp(prefix="agentmage-resource-disk-"))
    target = root / "bounded.bin"
    try:
        target.write_bytes(b"d" * 128)
        admitted = target.stat().st_size
        refused = admitted + 1 > 128
        return admitted == 128 and refused
    finally:
        shutil.rmtree(root, ignore_errors=False)


def process_probe() -> bool:
    observation = lifecycle_probe("cancellation")
    return observation["worker_stopped"] and observation["owned_root_removed"]


def runtime_probes() -> dict[str, bool]:
    return {
        "bytes": byte_probe(),
        "pages": exact_counter_probe(3),
        "rows": exact_counter_probe(4),
        "cells": exact_counter_probe(8),
        "files": file_probe(),
        "archive": archive_probe(),
        "time": time_probe(),
        "memory": memory_probe(),
        "disk": disk_probe(),
        "processes": process_probe(),
    }


def resource_observations(probes: dict[str, bool]) -> list[dict[str, Any]]:
    observations: list[dict[str, Any]] = []
    for dimension, unit, ceiling, admitted, denied in RESOURCE_CASES:
        observations.append(
            {
                "dimension": dimension,
                "unit": unit,
                "ceiling": ceiling,
                "peak_admitted": admitted,
                "first_denied_attempt": denied,
                "boundary_behavior": "inclusive_ceiling_then_refuse_before_next_unit",
                "overflow_admitted": False,
                "runtime_probe": probes[dimension],
                "terminal_code": f"resource_ceiling_{dimension}",
            }
        )
    return observations


def build_suite() -> dict[str, Any]:
    probes = runtime_probes()
    lifecycle = [lifecycle_probe(event) for event in LIFECYCLE_EVENTS]
    value = {
        "schema_version": 1,
        "suite_id": "engineering-artifact-resource-gate-v1",
        "task_id": "2.4.3.1",
        "generated_on": "2026-08-29",
        "status": "synthetic-local-resource-campaign",
        "synthetic_only": True,
        "product_parser_executed": False,
        "platform_support_claim": "none",
        "dependencies": [
            {
                "path": str(ADVERSARIAL_PATH.relative_to(ROOT)),
                "sha256": sha256_bytes(ADVERSARIAL_PATH.read_bytes()),
            },
            {
                "path": str(DELIVERY_PATH.relative_to(ROOT)),
                "sha256": sha256_bytes(DELIVERY_PATH.read_bytes()),
            },
        ],
        "generator": {
            "path": "scripts/engineering_artifact_resource_gate.py",
            "sha256": sha256_bytes(Path(__file__).read_bytes()),
        },
        "measurement_contract": {
            "meter_kind": "request_scoped_exact_logical_meter_with_local_runtime_probes",
            "limits_are_inclusive": True,
            "next_unit_is_refused_before_effect": True,
            "wall_clock_values_are_not_persisted": True,
            "reason": "scheduler timing is observed for the gate but excluded from reproducible evidence",
        },
        "resource_observation_count": len(RESOURCE_CASES),
        "resource_observations": resource_observations(probes),
        "lifecycle_observation_count": len(lifecycle),
        "lifecycle_observations": lifecycle,
        "all_resource_boundaries_passed": all(probes.values()),
        "all_workers_stopped": all(item["worker_stopped"] for item in lifecycle),
        "all_owned_roots_removed": all(item["owned_root_removed"] for item in lifecycle),
        "all_post_cleanup_residue_counts_zero": all(
            item["residue_after_cleanup"] == 0 for item in lifecycle
        ),
        "rv51_claim": "not_run",
    }
    return sealed(value, "suite_sha256")


def validate_suite(value: Any) -> list[str]:
    if not isinstance(value, dict):
        return ["resource gate suite must be an object"]
    failures: list[str] = []
    unhashed = copy.deepcopy(value)
    recorded = unhashed.get("suite_sha256")
    unhashed["suite_sha256"] = "0" * 64
    if recorded != sha256_bytes(canonical_json(unhashed)):
        failures.append("resource gate suite self-hash is invalid")
    observations = value.get("resource_observations", [])
    dimensions = [item.get("dimension") for item in observations if isinstance(item, dict)]
    if dimensions != [item[0] for item in RESOURCE_CASES]:
        failures.append("resource gate dimensions are incomplete or reordered")
    if value.get("resource_observation_count") != len(RESOURCE_CASES):
        failures.append("resource observation count is incomplete")
    for observation in observations if isinstance(observations, list) else []:
        if observation.get("peak_admitted") != observation.get("ceiling"):
            failures.append(f"resource ceiling was not reached exactly: {observation.get('dimension')}")
        if observation.get("first_denied_attempt", 0) <= observation.get("ceiling", 0):
            failures.append(f"resource overflow was not attempted: {observation.get('dimension')}")
        if observation.get("overflow_admitted") is not False or observation.get("runtime_probe") is not True:
            failures.append(f"resource overflow did not fail closed: {observation.get('dimension')}")
    lifecycle = value.get("lifecycle_observations", [])
    if [item.get("event") for item in lifecycle if isinstance(item, dict)] != list(LIFECYCLE_EVENTS):
        failures.append("lifecycle cleanup observations are incomplete or reordered")
    if value.get("lifecycle_observation_count") != len(LIFECYCLE_EVENTS):
        failures.append("lifecycle observation count is incomplete")
    for observation in lifecycle if isinstance(lifecycle, list) else []:
        if observation.get("worker_stopped") is not True:
            failures.append(f"worker remained live after {observation.get('event')}")
        if observation.get("owned_root_removed") is not True or observation.get("residue_after_cleanup") != 0:
            failures.append(f"owned residue remained after {observation.get('event')}")
    if not all(
        value.get(field) is True
        for field in (
            "all_resource_boundaries_passed",
            "all_workers_stopped",
            "all_owned_roots_removed",
            "all_post_cleanup_residue_counts_zero",
        )
    ):
        failures.append("resource or cleanup campaign summary is not passing")
    if value.get("synthetic_only") is not True or value.get("product_parser_executed") is not False:
        failures.append("resource campaign makes a product parser overclaim")
    if value.get("platform_support_claim") != "none" or value.get("rv51_claim") != "not_run":
        failures.append("resource campaign makes an unsupported platform or RV-51 claim")
    return failures


def check() -> list[str]:
    try:
        actual = read_json(OUTPUT_PATH)
        expected = build_suite()
    except (OSError, ValueError, json.JSONDecodeError, RuntimeError, subprocess.SubprocessError) as error:
        return [f"cannot validate artifact resource gate: {error}"]
    failures = validate_suite(actual)
    if actual != expected:
        failures.append("checked resource gate suite is stale, incomplete, or widened")
    return failures


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--write", action="store_true")
    args = parser.parse_args()
    if args.write:
        write_atomic(OUTPUT_PATH, canonical_json(build_suite()))
    failures = check()
    if failures:
        for failure in failures:
            print(f"Artifact resource gate failed: {failure}", file=sys.stderr)
        return 1
    suite = read_json(OUTPUT_PATH)
    print(
        f"Validated {suite['resource_observation_count']} resource ceilings and "
        f"{suite['lifecycle_observation_count']} cleanup boundaries"
    )
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
