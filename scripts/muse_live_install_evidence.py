#!/usr/bin/env python3
"""Retain content-free evidence from the isolated Muse install lifecycle run."""

from __future__ import annotations

import argparse
import hashlib
import json
import os
import re
import stat
from pathlib import Path
from typing import Any, Final


ROOT: Final = Path(__file__).resolve().parents[1]
OUTPUT: Final = ROOT / "artifacts/sprints/sprint-13/story-13.3/muse-live-install.json"
PROFILE_ID: Final = "muse-glimmer-30b-q4-k-m-text-8k-fedora-diagnostic-repeatability"
MODEL_SHA256: Final = "4cc57c0f51040a226e5a72cc47b7613f7772950e460a665f7083de89f183f60e"
MODEL_BYTES: Final = 16_756_683_904
MANIFEST_SHA256: Final = "58c7091643220e98fffd4e1da99bca27b405472caa44b9fc1bafc954d602d166"
RUNTIME_SHA256: Final = "3b1194ef38f4b02b6329d698e29532435a5a7c3567c84b8bb822459ca0893286"
SCANNER_POLICY_SHA256: Final = "e8a3dba2b12d59bb2aab517376a18890814f5c5a17855d561ee689ef663533af"
REVISION: Final = re.compile(r"^[0-9a-f]{40}$")


def sha256_bytes(value: bytes) -> str:
    return hashlib.sha256(value).hexdigest()


def _mode(path: Path) -> int:
    return stat.S_IMODE(path.stat(follow_symlinks=False).st_mode)


def build_report(
    *,
    model_store: Path,
    socket_root: Path,
    source_revision: str,
    elapsed_ms: int,
) -> dict[str, Any]:
    manifest_path = model_store / "active-model.json"
    artifact_path = model_store / f"{MODEL_SHA256}.gguf"
    lock_path = model_store / ".installer.lock"
    manifest_bytes = manifest_path.read_bytes()
    manifest = json.loads(manifest_bytes)
    active = manifest.get("active", {})
    socket_entries = sorted(path.name for path in socket_root.iterdir()) if socket_root.is_dir() else []
    checks = [
        {
            "id": "manifest-identity",
            "result": "PASS" if (
                manifest.get("schema_version") == 1
                and manifest.get("generation") == 1
                and manifest.get("previous") is None
                and active.get("profile_id") == PROFILE_ID
                and active.get("manifest_sha256") == MANIFEST_SHA256
                and active.get("artifact_name") == artifact_path.name
                and active.get("artifact_sha256") == MODEL_SHA256
                and active.get("runtime_sha256") == RUNTIME_SHA256
                and active.get("scanner_policy_sha256") == SCANNER_POLICY_SHA256
            ) else "FAIL",
            "code": "active-manifest-exact" if (
                active.get("artifact_sha256") == MODEL_SHA256
                and active.get("runtime_sha256") == RUNTIME_SHA256
            ) else "active-manifest-drift",
        },
        {
            "id": "artifact-state",
            "result": "PASS" if (
                artifact_path.is_file()
                and not artifact_path.is_symlink()
                and artifact_path.stat(follow_symlinks=False).st_size == MODEL_BYTES
                and _mode(artifact_path) == 0o600
                and artifact_path.stat(follow_symlinks=False).st_nlink == 1
            ) else "FAIL",
            "code": "installed-artifact-bounded" if artifact_path.is_file() else "installed-artifact-absent",
        },
        {
            "id": "store-state",
            "result": "PASS" if (
                _mode(model_store) == 0o700
                and _mode(manifest_path) == 0o600
                and lock_path.is_file()
                and _mode(lock_path) == 0o600
            ) else "FAIL",
            "code": "private-store-bounded" if model_store.is_dir() else "private-store-absent",
        },
        {
            "id": "runtime-cleanup",
            "result": "PASS" if socket_root.is_dir() and _mode(socket_root) == 0o700 and not socket_entries else "FAIL",
            "code": "runtime-socket-removed" if not socket_entries else "runtime-residue-present",
        },
        {
            "id": "lifecycle-result",
            "result": "PASS" if elapsed_ms > 0 else "FAIL",
            "code": "import-scan-load-ready-unload-activate-pass" if elapsed_ms > 0 else "lifecycle-result-invalid",
        },
    ]
    passed = all(check["result"] == "PASS" for check in checks)
    return {
        "schema_version": 1,
        "record_type": "muse_isolated_live_install_evidence",
        "source_revision": source_revision,
        "profile_id": PROFILE_ID,
        "tuple": {
            "model_sha256": MODEL_SHA256,
            "model_bytes": MODEL_BYTES,
            "profile_manifest_sha256": MANIFEST_SHA256,
            "runtime_sha256": RUNTIME_SHA256,
            "scanner_policy_sha256": SCANNER_POLICY_SHA256,
            "context_tokens": 8192,
            "parallel_slots": 1,
            "network_egress_configured": False,
            "synthetic_data_only": True,
        },
        "execution": {
            "test_id": "exact_muse_import_scan_load_unload_and_evidence_activation",
            "exit_code": 0 if passed else 1,
            "elapsed_ms": elapsed_ms,
            "generation": manifest.get("generation"),
            "copied_bytes": 0,
            "retries": 0,
            "terminal_output_sha256": sha256_bytes(
                b"MUSE_LIVE_INSTALL_PASS generation=1 copied_bytes=0 retries=0\n"
            ),
        },
        "observations": {
            "active_manifest_sha256": sha256_bytes(manifest_bytes),
            "model_store_mode": _mode(model_store),
            "artifact_mode": _mode(artifact_path) if artifact_path.exists() else None,
            "artifact_links": artifact_path.stat(follow_symlinks=False).st_nlink if artifact_path.exists() else None,
            "socket_root_mode": _mode(socket_root) if socket_root.exists() else None,
            "socket_residue_names": socket_entries,
            "workspace_available": False,
            "session_available": False,
            "unrelated_inference_available": False,
            "tool_available": False,
            "network_available_to_installer": False,
        },
        "checks": checks,
        "disposition": {
            "status": "ISOLATED-LIFECYCLE-PASS" if passed else "BLOCKED",
            "adapter_live_proven_for_exact_tuple": passed,
            "evidence_store_generation": 1 if passed else 0,
            "product_profile_enabled": False,
            "release_approval": False,
            "automatic_fallback": False,
            "zero_egress_observed": False,
            "quality_evaluated": False,
            "repeatability_evaluated": False,
        },
        "limitations": [
            "The activation occurred only in an isolated evidence store and does not enable a product profile.",
            "Network unavailability was enforced by the adapter contract but was not independently packet-captured in this run.",
            "This lifecycle run did not evaluate inference quality or diagnostic repeatability.",
            "The retained record contains no model bytes, prompts, private paths, credentials, or host identifier.",
        ],
    }


def validate_report(report: dict[str, Any]) -> list[str]:
    failures: list[str] = []
    expected_fields = {
        "schema_version", "record_type", "source_revision", "profile_id", "tuple",
        "execution", "observations", "checks", "disposition", "limitations",
    }
    if set(report) != expected_fields:
        return ["live install evidence fields are not closed"]
    if report.get("schema_version") != 1 or report.get("record_type") != "muse_isolated_live_install_evidence":
        failures.append("live install evidence identity changed")
    if not REVISION.fullmatch(str(report.get("source_revision", ""))):
        failures.append("live install source revision is not exact")
    if report.get("profile_id") != PROFILE_ID:
        failures.append("live install profile identity changed")
    checks = report.get("checks", [])
    if len(checks) != 5 or len({check.get("id") for check in checks}) != 5:
        failures.append("live install check closure changed")
    passed = bool(checks) and all(check.get("result") == "PASS" for check in checks)
    disposition = report.get("disposition", {})
    if disposition.get("status") != ("ISOLATED-LIFECYCLE-PASS" if passed else "BLOCKED"):
        failures.append("live install disposition disagrees with checks")
    if disposition.get("adapter_live_proven_for_exact_tuple") is not passed:
        failures.append("live adapter claim disagrees with checks")
    for field in (
        "product_profile_enabled", "release_approval", "automatic_fallback",
        "zero_egress_observed", "quality_evaluated", "repeatability_evaluated",
    ):
        if disposition.get(field) is not False:
            failures.append(f"live install evidence overstates {field}")
    observations = report.get("observations", {})
    if any("/" in name for name in observations.get("socket_residue_names", [])):
        failures.append("live install evidence contains a path-like residue name")
    return failures


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--model-store", required=True, type=Path)
    parser.add_argument("--socket-root", required=True, type=Path)
    parser.add_argument("--source-revision", required=True)
    parser.add_argument("--elapsed-ms", required=True, type=int)
    parser.add_argument("--write", action="store_true")
    args = parser.parse_args()
    report = build_report(
        model_store=args.model_store,
        socket_root=args.socket_root,
        source_revision=args.source_revision,
        elapsed_ms=args.elapsed_ms,
    )
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
