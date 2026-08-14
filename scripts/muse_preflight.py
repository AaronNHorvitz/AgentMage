#!/usr/bin/env python3
"""Run the fail-closed Muse 8k text-profile preflight without loading a model."""

from __future__ import annotations

import argparse
import hashlib
import json
import os
import platform
import shutil
import subprocess
from pathlib import Path
from typing import Any, Final

try:
    from scripts import muse_candidate_admission as admission
except ModuleNotFoundError:  # Direct script execution adds scripts/, not the repo root.
    import muse_candidate_admission as admission


ROOT: Final = Path(__file__).resolve().parents[1]
OUTPUT: Final = ROOT / "artifacts/sprints/sprint-13/story-13.3/muse-preflight.json"
MODEL_BYTES: Final = 16_756_683_904
MODEL_SHA256: Final = "4cc57c0f51040a226e5a72cc47b7613f7772950e460a665f7083de89f183f60e"
RUNTIME_BYTES: Final = 32_989_764
RUNTIME_SHA256: Final = "3b1194ef38f4b02b6329d698e29532435a5a7c3567c84b8bb822459ca0893286"
MIN_SYSTEM_BYTES: Final = 32 * 1024 * 1024 * 1024
MIN_ACCELERATOR_BYTES: Final = 20 * 1024 * 1024 * 1024
MIN_DISK_BYTES: Final = MODEL_BYTES * 2 + 8 * 1024 * 1024 * 1024


def sha256_file(path: Path) -> str:
    digest = hashlib.sha256()
    with path.open("rb") as stream:
        for block in iter(lambda: stream.read(4 * 1024 * 1024), b""):
            digest.update(block)
    return digest.hexdigest()


def observe_file(path: Path | None, expected_bytes: int, expected_sha256: str) -> dict[str, Any]:
    if path is None:
        return {"present": False, "regular": False, "bytes": None, "sha256": None, "exact": False}
    try:
        metadata = path.stat(follow_symlinks=False)
    except OSError:
        return {"present": False, "regular": False, "bytes": None, "sha256": None, "exact": False}
    regular = path.is_file() and not path.is_symlink()
    digest = sha256_file(path) if regular and metadata.st_size == expected_bytes else None
    return {
        "present": True,
        "regular": regular,
        "bytes": metadata.st_size,
        "sha256": digest,
        "exact": regular and metadata.st_size == expected_bytes and digest == expected_sha256,
    }


def _nvidia_observation() -> dict[str, Any]:
    try:
        result = subprocess.run(
            [
                "nvidia-smi",
                "--query-gpu=name,memory.total,memory.free,driver_version",
                "--format=csv,noheader,nounits",
            ],
            check=True,
            capture_output=True,
            text=True,
            timeout=10,
        )
        line = result.stdout.strip().splitlines()[0]
        name, total_mib, free_mib, driver = [value.strip() for value in line.split(",")]
        return {
            "available": True,
            "name": name,
            "total_bytes": int(total_mib) * 1024 * 1024,
            "free_bytes": int(free_mib) * 1024 * 1024,
            "driver": driver,
        }
    except (FileNotFoundError, IndexError, OSError, subprocess.SubprocessError, ValueError):
        return {"available": False, "name": None, "total_bytes": 0, "free_bytes": 0, "driver": None}


def observe_host() -> dict[str, Any]:
    pages = os.sysconf("SC_PHYS_PAGES")
    page_size = os.sysconf("SC_PAGE_SIZE")
    disk = shutil.disk_usage(ROOT)
    return {
        "system": platform.system(),
        "architecture": platform.machine(),
        "system_memory_bytes": pages * page_size,
        "disk_available_bytes": disk.free,
        "accelerator": _nvidia_observation(),
    }


def _check(identifier: str, passed: bool, passed_code: str, blocked_code: str) -> dict[str, str]:
    return {
        "id": identifier,
        "result": "PASS" if passed else "BLOCKED",
        "code": passed_code if passed else blocked_code,
    }


def evaluate(
    *,
    host: dict[str, Any],
    model: dict[str, Any],
    runtime_archive: dict[str, Any],
    source_revision: str,
) -> dict[str, Any]:
    record_valid = admission.validate() == []
    accelerator = host.get("accelerator", {})
    checks = [
        _check("source-admission", record_valid, "exact-records-valid", "admission-record-invalid"),
        _check(
            "platform",
            host.get("system") == "Linux" and host.get("architecture") == "x86_64",
            "linux-x86_64",
            "unsupported-platform",
        ),
        _check(
            "system-memory",
            int(host.get("system_memory_bytes", 0)) >= MIN_SYSTEM_BYTES,
            "system-memory-sufficient",
            "system-memory-insufficient",
        ),
        _check(
            "disk",
            int(host.get("disk_available_bytes", 0)) >= MIN_DISK_BYTES,
            "disk-sufficient",
            "disk-insufficient",
        ),
        _check(
            "accelerator-memory",
            bool(accelerator.get("available"))
            and int(accelerator.get("total_bytes", 0)) >= MIN_ACCELERATOR_BYTES,
            "accelerator-memory-sufficient",
            "accelerator-memory-insufficient",
        ),
        _check(
            "driver",
            accelerator.get("driver") == "610.43.03",
            "driver-exact",
            "driver-unmeasured-or-changed",
        ),
        _check("model-artifact", bool(model.get("exact")), "model-artifact-exact", "model-artifact-missing-or-drifted"),
        _check(
            "runtime-archive",
            bool(runtime_archive.get("exact")),
            "runtime-archive-exact",
            "runtime-archive-missing-or-drifted",
        ),
        _check("runtime-package", False, "runtime-package-admitted", "runtime-package-not-admitted"),
        _check("runtime-adapter", False, "runtime-adapter-conformant", "runtime-adapter-not-conformant"),
        _check("zero-egress", False, "zero-egress-observed", "zero-egress-not-executed"),
        _check("codec-conformance", False, "codec-conformance-pass", "codec-conformance-incomplete"),
        _check("quality", False, "quality-evidence-pass", "quality-evidence-not-measured"),
        _check("repeatability", False, "repeatability-evidence-pass", "repeatability-not-measured"),
    ]
    blockers = [check["code"] for check in checks if check["result"] != "PASS"]
    return {
        "schema_version": 1,
        "record_type": "muse_isolated_preflight",
        "profile_id": "muse-glimmer-30b-q4-k-m-text-8k-fedora",
        "source_revision": source_revision,
        "tuple": {
            "model_sha256": MODEL_SHA256,
            "runtime_sha256": RUNTIME_SHA256,
            "context_tokens": 8192,
            "parallel_slots": 1,
            "network_egress": False,
            "vision": False,
            "speculative_draft": False,
            "synthetic_data_only": True,
        },
        "observations": {
            "host": host,
            "model_artifact": model,
            "runtime_archive": runtime_archive,
        },
        "checks": checks,
        "disposition": {
            "status": "BLOCKED" if blockers else "PASS-EVALUATION",
            "blockers": blockers,
            "activation": False,
            "automatic_fallback": False,
            "model_loaded": False,
        },
        "limitations": [
            "This preflight does not load or execute a model.",
            "A host observation is evidence only for this exact machine state and tuple.",
            "A BLOCKED result cannot be reused as product support or release evidence.",
        ],
    }


def validate_report(report: dict[str, Any]) -> list[str]:
    failures: list[str] = []
    if set(report) != {
        "schema_version", "record_type", "profile_id", "source_revision", "tuple",
        "observations", "checks", "disposition", "limitations",
    }:
        failures.append("preflight fields are not closed")
        return failures
    if report.get("schema_version") != 1 or report.get("record_type") != "muse_isolated_preflight":
        failures.append("preflight schema identity changed")
    checks = report.get("checks")
    if not isinstance(checks, list) or len(checks) != 14:
        failures.append("preflight check closure changed")
    elif len({check.get("id") for check in checks if isinstance(check, dict)}) != len(checks):
        failures.append("preflight check identities are duplicated")
    disposition = report.get("disposition")
    if not isinstance(disposition, dict):
        failures.append("preflight disposition is absent")
    else:
        blockers = [check.get("code") for check in checks if check.get("result") != "PASS"]
        if disposition.get("blockers") != blockers:
            failures.append("preflight blocker list does not match checks")
        expected = "BLOCKED" if blockers else "PASS-EVALUATION"
        if disposition.get("status") != expected:
            failures.append("preflight disposition overstates its checks")
        if any(disposition.get(field) is not False for field in ("activation", "automatic_fallback", "model_loaded")):
            failures.append("preflight gained activation, fallback, or load state")
    return failures


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser()
    parser.add_argument("--model-artifact", type=Path)
    parser.add_argument("--runtime-archive", type=Path)
    parser.add_argument("--source-revision", default="working-tree")
    parser.add_argument("--write", action="store_true")
    return parser.parse_args()


def main() -> int:
    args = parse_args()
    report = evaluate(
        host=observe_host(),
        model=observe_file(args.model_artifact, MODEL_BYTES, MODEL_SHA256),
        runtime_archive=observe_file(args.runtime_archive, RUNTIME_BYTES, RUNTIME_SHA256),
        source_revision=args.source_revision,
    )
    failures = validate_report(report)
    if failures:
        for failure in failures:
            print(f"- {failure}")
        return 1
    if args.write:
        OUTPUT.parent.mkdir(parents=True, exist_ok=True)
        OUTPUT.write_text(json.dumps(report, indent=2, sort_keys=True) + "\n", encoding="utf-8")
    print(f"Muse preflight: {report['disposition']['status']}")
    for blocker in report["disposition"]["blockers"]:
        print(f"- {blocker}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
