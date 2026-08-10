#!/usr/bin/env python3
"""Build and verify immutable guarded DMR reachability evidence for Story 0.3."""

from __future__ import annotations

import argparse
import hashlib
import json
import re
import subprocess
import sys
from pathlib import Path
from typing import Final

try:
    from scripts.dmr_reachability import read_json_file, validate_result
except ModuleNotFoundError:
    from dmr_reachability import read_json_file, validate_result


ROOT: Final = Path(__file__).resolve().parents[1]
DEFAULT_OUTPUT: Final = (
    ROOT / "artifacts" / "sprints" / "sprint-0" / "story-0.3-reachability"
)
EVIDENCE_DATE: Final = "2026-08-10"
ARTIFACT_NAMES: Final = (
    "dmr-server.log",
    "raw-reachability-result.json",
    "reachability-disposition.json",
    "runtime-state.json",
    "summary.md",
)
SENSITIVE_TEXT = re.compile(
    r"(?:/(?:var/)?home/[^/\s]+|[A-Za-z0-9._%+-]+@[A-Za-z0-9.-]+\.[A-Za-z]{2,}|"
    r"-----BEGIN [A-Z ]*PRIVATE KEY-----|Authorization:\s*Bearer|"
    r"(?:password|api[_ -]?key|secret|token)\s*[:=])",
    re.IGNORECASE,
)


class ReachabilityEvidenceError(ValueError):
    """Raised when reachability evidence cannot be built or reconciled."""


def canonical_json(value: object) -> bytes:
    return (json.dumps(value, indent=2, sort_keys=True) + "\n").encode("utf-8")


def sha256(content: bytes) -> str:
    return hashlib.sha256(content).hexdigest()


def relative(path: Path) -> str:
    try:
        return str(path.relative_to(ROOT))
    except ValueError:
        return str(path)


def resolve_revision(revision: str) -> str:
    result = subprocess.run(
        ["git", "rev-parse", "--verify", f"{revision}^{{commit}}"],
        cwd=ROOT,
        check=False,
        capture_output=True,
        text=True,
    )
    if result.returncode != 0:
        raise ReachabilityEvidenceError(
            f"cannot resolve reachability verification revision {revision}"
        )
    return result.stdout.strip()


def sanitize_log(content: str) -> str:
    if SENSITIVE_TEXT.search(content):
        raise ReachabilityEvidenceError(
            "DMR reachability log contains a sensitive path or authentication pattern"
        )
    return content


def disposition(result: dict[str, object], verification_revision: str) -> dict[str, object]:
    probes = result["probes"]
    return {
        "schema_version": 1,
        "record_type": "story_0_3_dmr_reachability_disposition",
        "evidence_date": EVIDENCE_DATE,
        "result_source_revision": result["source_revision"],
        "verification_revision": verification_revision,
        "data_classification": "public_synthetic_metadata_only",
        "probe_results": {
            name: {
                "passed": value["passed"],
                "observed": value["observed"],
            }
            for name, value in probes.items()
        },
        "guarded_topology": {
            "feasibility_status": "PASS",
            "raw_dmr_transport": "private_tmpfs_unix_domain_socket",
            "exposed_transport": "mode_0600_authenticated_unix_domain_socket",
            "fresh_in_memory_token": True,
            "non_dumpable_processes": True,
            "private_network_only_loopback": True,
            "private_network_routes": 0,
            "private_network_tcp_listeners": 0,
        },
        "task_0_3_3_2_status": "COMPLETE",
        "docker_adapter_production_status": "BLOCKED",
        "production_support_claim": False,
        "release_approval": False,
        "limitations": result["limitations"],
        "required_follow_up": [
            "Replace the evaluation-only Python guard with the signed compiled kernel adapter and short-lived authenticated session contract.",
            "Re-run the matrix with the exact production OCI topology and resource limits.",
            "Run the later release probe from a separately administered physical LAN peer in addition to the non-loopback namespace peer.",
            "Keep Docker Model Runner unavailable until model, adapter, topology, and independent review gates all pass.",
        ],
    }


def summary_markdown(
    result: dict[str, object], decision: dict[str, object]
) -> bytes:
    probes = result["probes"]
    rows = "\n".join(
        f"| `{name}` | `{value['observed']}` | {'PASS' if value['passed'] else 'FAIL'} |"
        for name, value in probes.items()
    )
    text = f"""# Story 0.3 Guarded DMR Reachability Summary

| Probe | Observed | Result |
|---|---|---:|
{rows}

The exact pinned Docker Model Runner binary and filesystem ran inside private user, mount, and network namespaces. Its unauthenticated raw API existed only on a private tmpfs Unix socket. A non-dumpable evaluation guard exposed a mode `0600` Unix socket and required a fresh in-memory bearer value. Only the designated guarded request returned HTTP 200.

An unrelated same-user request received HTTP 401 and could neither traverse the supervisor's `/proc` root nor find the raw socket in the stopped image root filesystem. A tool container received HTTP 401 even with its SELinux label disabled for the probe. A separately mapped namespace received a filesystem denial. A rootless network peer connecting to the host's private non-loopback address received `ConnectionRefusedError`. The private DMR namespace contained only loopback, zero routes, zero TCP listeners, and zero network-interface bytes before and after the probes.

This is a `{decision['guarded_topology']['feasibility_status']}` feasibility result and a `{decision['docker_adapter_production_status']}` production-support result. The non-loopback peer was a separate rootless network namespace, not a second physical workstation. The exact image filesystem was used, but the DMR binary was not an OCI workload in this guarded prototype. The guard is evaluation-only Python, no rejected model was loaded, and no product support or release approval is claimed. Raw JSON remains authoritative.
"""
    return text.encode("utf-8")


def build_bundle(result_dir: Path, verification_revision: str) -> dict[str, bytes]:
    result = read_json_file(result_dir / "results.json")
    failures = validate_result(result)
    if failures:
        raise ReachabilityEvidenceError(
            "source reachability result is invalid: " + "; ".join(failures)
        )
    runtime = read_json_file(result_dir / "runtime-state.json")
    if runtime != result["runtime"]:
        raise ReachabilityEvidenceError(
            "standalone runtime state does not match the raw reachability result"
        )
    server_log = sanitize_log(
        (result_dir / "dmr-server.log").read_text(encoding="utf-8")
    )
    decision = disposition(result, verification_revision)
    return {
        "dmr-server.log": server_log.encode("utf-8"),
        "raw-reachability-result.json": (result_dir / "results.json").read_bytes(),
        "reachability-disposition.json": canonical_json(decision),
        "runtime-state.json": (result_dir / "runtime-state.json").read_bytes(),
        "summary.md": summary_markdown(result, decision),
    }


def build_manifest(bundle: dict[str, bytes], decision: dict[str, object]) -> dict[str, object]:
    return {
        "schema_version": 1,
        "bundle_id": "sprint-0-story-0.3-guarded-dmr-reachability-evidence",
        "evidence_date": EVIDENCE_DATE,
        "result_source_revision": decision["result_source_revision"],
        "verification_revision": decision["verification_revision"],
        "scope": "Sub-task 0.3.3.2 guarded DMR feasibility reachability matrix",
        "files": [
            {"path": name, "sha256": sha256(bundle[name]), "size": len(bundle[name])}
            for name in ARTIFACT_NAMES
        ],
    }


def write_bundle(output: Path, result_dir: Path, verification_revision: str) -> None:
    if output.exists():
        raise ReachabilityEvidenceError(f"refusing to overwrite existing evidence: {output}")
    revision = resolve_revision(verification_revision)
    bundle = build_bundle(result_dir, revision)
    decision = json.loads(bundle["reachability-disposition.json"])
    output.mkdir(parents=True)
    for name, content in bundle.items():
        (output / name).write_bytes(content)
    (output / "evidence-manifest.json").write_bytes(
        canonical_json(build_manifest(bundle, decision))
    )


def check_bundle(output: Path = DEFAULT_OUTPUT) -> list[str]:
    failures: list[str] = []
    try:
        manifest = read_json_file(output / "evidence-manifest.json")
        result = read_json_file(output / "raw-reachability-result.json")
        decision = read_json_file(output / "reachability-disposition.json")
        runtime = read_json_file(output / "runtime-state.json")
    except (OSError, json.JSONDecodeError, ValueError) as error:
        return [f"cannot load guarded reachability evidence: {error}"]
    entries = manifest.get("files")
    names = [item.get("path") for item in entries if isinstance(item, dict)] if isinstance(entries, list) else []
    if names != list(ARTIFACT_NAMES):
        failures.append("reachability evidence manifest membership or order is invalid")
    else:
        for item in entries:
            path = output / str(item["path"])
            try:
                content = path.read_bytes()
            except OSError as error:
                failures.append(f"cannot read reachability evidence {path.name}: {error}")
                continue
            if item.get("sha256") != sha256(content) or item.get("size") != len(content):
                failures.append(f"reachability evidence binding changed: {path.name}")
    for failure in validate_result(result):
        failures.append(f"raw guarded reachability result: {failure}")
    if runtime != result.get("runtime"):
        failures.append("retained reachability runtime state does not reconcile")
    expected_decision = disposition(
        result, str(decision.get("verification_revision", ""))
    )
    if decision != expected_decision:
        failures.append("reachability disposition does not reconcile")
    if manifest.get("result_source_revision") != result.get("source_revision"):
        failures.append("reachability result revision does not reconcile")
    if manifest.get("verification_revision") != decision.get("verification_revision"):
        failures.append("reachability verification revision does not reconcile")
    try:
        resolve_revision(str(manifest.get("verification_revision", "")))
        sanitize_log((output / "dmr-server.log").read_text(encoding="utf-8"))
    except (OSError, ReachabilityEvidenceError) as error:
        failures.append(str(error))
    return failures


def main(argv: list[str] | None = None) -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--output", type=Path, default=DEFAULT_OUTPUT)
    parser.add_argument("--result-dir", type=Path)
    parser.add_argument("--verification-revision", default="HEAD")
    parser.add_argument("--write", action="store_true")
    args = parser.parse_args(argv)
    try:
        if args.write:
            if args.result_dir is None:
                raise ReachabilityEvidenceError("--write requires --result-dir")
            write_bundle(args.output, args.result_dir, args.verification_revision)
            return 0
        failures = check_bundle(args.output)
    except (OSError, json.JSONDecodeError, ReachabilityEvidenceError, ValueError) as error:
        print(f"Reachability evidence error: {error}", file=sys.stderr)
        return 1
    if failures:
        print("Reachability evidence validation failed:", file=sys.stderr)
        for failure in failures:
            print(f"- {failure}", file=sys.stderr)
        return 1
    print(f"Validated guarded DMR reachability evidence at {relative(args.output)}.")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
