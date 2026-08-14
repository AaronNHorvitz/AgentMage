#!/usr/bin/env python3
"""Produce immutable evidence for the inactive Muse llama.cpp package."""

from __future__ import annotations

import argparse
import json
import os
import subprocess
import tempfile
from pathlib import Path
from typing import Any, Final

try:
    from scripts import muse_llama_runtime as runtime
except ModuleNotFoundError:
    import muse_llama_runtime as runtime


ROOT: Final = Path(__file__).resolve().parents[1]
OUTPUT: Final = ROOT / "artifacts/sprints/sprint-13/story-13.1/muse-native-runtime-package.json"


def build_report(archive: Path, package: Path, source_revision: str) -> dict[str, Any]:
    profile, profile_sha256 = runtime.load_profile()
    payload = runtime.read_payload(archive, profile)
    first = runtime.build_bytes(profile, profile_sha256, payload)
    second = runtime.build_bytes(profile, profile_sha256, payload)
    package.parent.mkdir(parents=True, exist_ok=True)
    package.write_bytes(first)
    verification = runtime.verify_package(package, profile, profile_sha256)

    with tempfile.TemporaryDirectory(prefix="agentmage-muse-runtime-") as directory:
        root = Path(directory)
        extraction = subprocess.run(
            ["tar", "-xzf", str(package), "-C", str(root)],
            check=False,
            capture_output=True,
            text=True,
            timeout=30,
            env={"PATH": "/usr/bin:/bin"},
        )
        if extraction.returncode != 0:
            raise runtime.MuseRuntimeError("muse-runtime.evidence-extraction-failed")
        runtime_root = root / profile["package"]["relative_install_root"]
        environment = {"LD_LIBRARY_PATH": str(runtime_root / "lib"), "PATH": "/usr/bin:/bin"}
        process = subprocess.run(
            [str(runtime_root / "bin/llama-server"), "--version"],
            check=False,
            capture_output=True,
            text=True,
            timeout=10,
            env=environment,
            cwd=runtime_root / "lib",
        )
        version_output = "\n".join(part.strip() for part in (process.stdout, process.stderr) if part.strip())
        file_modes = {
            str(path.relative_to(runtime_root)): path.lstat().st_mode & 0o777
            for path in sorted(runtime_root.rglob("*"))
        }

    destinations = [record["destination"] for record in profile["source_files"]]
    prohibited = [
        name for name in destinations
        if any(term in name.lower() for term in ("rpc", "download", "quantize", "bench", "completion", "tokenize", "imatrix"))
    ]
    checks = [
        {"id": "source-archive", "result": "PASS", "code": "source-archive-exact"},
        {
            "id": "deterministic-rebuild",
            "result": "PASS" if first == second else "FAIL",
            "code": "package-rebuild-exact" if first == second else "package-rebuild-drift",
        },
        {"id": "package-closure", "result": "PASS", "code": "package-members-exact"},
        {
            "id": "prohibited-entrypoints",
            "result": "PASS" if not prohibited else "FAIL",
            "code": "prohibited-entrypoints-absent" if not prohibited else "prohibited-entrypoints-present",
        },
        {
            "id": "unprivileged-extraction",
            "result": "PASS" if file_modes.get("bin/llama-server") == 0o555 else "FAIL",
            "code": "unprivileged-extraction-pass" if file_modes.get("bin/llama-server") == 0o555 else "unprivileged-extraction-fail",
        },
        {
            "id": "binary-self-check",
            "result": "PASS" if process.returncode == 0 and "build 10423" in version_output and "a94d563ed" in version_output else "FAIL",
            "code": "binary-identity-exact" if process.returncode == 0 and "build 10423" in version_output and "a94d563ed" in version_output else "binary-identity-fail",
        },
    ]
    return {
        "schema_version": 1,
        "record_type": "muse_native_runtime_package_evidence",
        "source_revision": source_revision,
        "profile_path": str(runtime.PROFILE_PATH.relative_to(ROOT)),
        "profile_sha256": profile_sha256,
        "source_archive": {
            "sha256": runtime.sha256_file(archive),
            "bytes": archive.stat().st_size,
            "source_commit": profile["source_commit"],
        },
        "package": verification,
        "member_count": len(profile["source_files"]) + 1,
        "file_modes": file_modes,
        "prohibited_entrypoints": prohibited,
        "self_check": {
            "operation": "version-only",
            "exit_code": process.returncode,
            "reported_identity": version_output,
            "model_loaded": False,
            "listener_started": False,
        },
        "checks": checks,
        "disposition": {
            "status": "PACKAGE-VERIFIED-NOT-ADMITTED" if all(check["result"] == "PASS" for check in checks) else "BLOCKED",
            "enabled_models": 0,
            "inference_implemented": False,
            "adapter_implemented": False,
            "release_approval": False,
            "automatic_fallback": False,
        },
        "limitations": [
            "No model artifact was present or loaded.",
            "The self-check did not start a listener or inference request.",
            "Package verification is not adapter, zero-egress, model-quality, or release evidence.",
        ],
    }


def validate_report(report: dict[str, Any]) -> list[str]:
    failures: list[str] = []
    if set(report) != {
        "schema_version", "record_type", "source_revision", "profile_path", "profile_sha256",
        "source_archive", "package", "member_count", "file_modes", "prohibited_entrypoints",
        "self_check", "checks", "disposition", "limitations",
    }:
        return ["runtime evidence fields are not closed"]
    checks = report.get("checks", [])
    if len(checks) != 6 or len({check.get("id") for check in checks}) != 6:
        failures.append("runtime evidence check closure changed")
    disposition = report.get("disposition", {})
    expected = "PACKAGE-VERIFIED-NOT-ADMITTED" if all(check.get("result") == "PASS" for check in checks) else "BLOCKED"
    if disposition.get("status") != expected:
        failures.append("runtime evidence disposition overstates checks")
    for field in ("inference_implemented", "adapter_implemented", "release_approval", "automatic_fallback"):
        if disposition.get(field) is not False:
            failures.append(f"runtime evidence overstates {field}")
    if disposition.get("enabled_models") != 0:
        failures.append("runtime evidence enables a model")
    self_check = report.get("self_check", {})
    if self_check.get("model_loaded") is not False or self_check.get("listener_started") is not False:
        failures.append("runtime self-check overstates execution")
    return failures


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--archive", required=True, type=Path)
    parser.add_argument("--package", type=Path, default=runtime.DEFAULT_OUTPUT)
    parser.add_argument("--source-revision", required=True)
    parser.add_argument("--write", action="store_true")
    args = parser.parse_args()
    report = build_report(args.archive, args.package, args.source_revision)
    failures = validate_report(report)
    if failures:
        for failure in failures:
            print(f"- {failure}")
        return 1
    if args.write:
        OUTPUT.parent.mkdir(parents=True, exist_ok=True)
        OUTPUT.write_text(json.dumps(report, indent=2, sort_keys=True) + "\n", encoding="utf-8")
    print(json.dumps(report["disposition"], indent=2, sort_keys=True))
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
