#!/usr/bin/env python3
"""Run non-acquiring reference-machine preflight against the frozen Sprint 14 inventory."""

from __future__ import annotations

import argparse
import hashlib
import json
from pathlib import Path
from typing import Any, Final

ROOT: Final = Path(__file__).resolve().parents[1]
INVENTORY: Final = ROOT / "model-profiles/catalogs/2026-08-14/candidate-inventory.json"
ROLE_MATRIX: Final = ROOT / "model-profiles/catalogs/2026-08-14/candidate-role-suite-matrix.json"
ENVELOPES: Final = ROOT / "evidence/model-inventory-preflight/reference-machine-envelopes.json"
REPORT: Final = ROOT / "evidence/model-inventory-preflight/preflight-report.json"

PRESERVED_INVENTORY_STATUSES: Final = frozenset({"INELIGIBLE"})
ACCEPTABLE_AXIS_STATUSES: Final = frozenset({"PASS", "BLOCKED-HARDWARE", "UNRESOLVED"})
ACCEPTABLE_ENTRY_STATUSES: Final = frozenset({"BLOCKED", "BLOCKED-HARDWARE", "INELIGIBLE"})
AXES: Final = (
    "architecture",
    "runtime",
    "format",
    "acceleration",
    "disk",
    "memory",
    "context",
    "modality",
    "expected_working_set",
)


def canonical_bytes(value: object) -> bytes:
    return (json.dumps(value, sort_keys=True, separators=(",", ":")) + "\n").encode()


def sha256_bytes(value: bytes) -> str:
    return hashlib.sha256(value).hexdigest()


def read_json(path: Path) -> dict[str, Any]:
    return json.loads(path.read_text(encoding="utf-8"))


def _preflight_axes(entry: dict[str, Any], envelope: dict[str, Any]) -> dict[str, dict[str, Any]]:
    formats = list(entry.get("artifact_formats", []))
    modalities = [m for m in entry.get("modalities", []) if m]
    supported_formats = set(envelope["supported_artifact_formats"])
    supported_modalities = set(envelope["modality_support"])
    runtimes = envelope["supported_runtimes"]

    axes: dict[str, dict[str, Any]] = {}

    axes["architecture"] = {
        "status": "PASS",
        "declared_architecture": envelope["cpu"]["architecture"],
        "reason": "artifact formats are architecture-neutral at the source-catalog layer",
    }

    if not formats:
        axes["format"] = {
            "status": "UNRESOLVED",
            "declared_formats": sorted(supported_formats),
            "candidate_formats": [],
            "reason": "exact-artifact-format-not-selected-in-source-entry",
        }
        axes["runtime"] = {
            "status": "UNRESOLVED",
            "declared_runtimes": list(runtimes),
            "reason": "no artifact format known, so no runtime can be matched without acquisition",
        }
    else:
        overlap = sorted(supported_formats.intersection(formats))
        if overlap:
            axes["format"] = {
                "status": "PASS",
                "declared_formats": sorted(supported_formats),
                "candidate_formats": sorted(formats),
                "reason": "candidate advertises at least one envelope-supported artifact format",
            }
            axes["runtime"] = {
                "status": "PASS",
                "declared_runtimes": list(runtimes),
                "reason": "envelope publishes a runtime capable of loading the overlapping artifact format after exact profile admission",
            }
        else:
            axes["format"] = {
                "status": "BLOCKED-HARDWARE",
                "declared_formats": sorted(supported_formats),
                "candidate_formats": sorted(formats),
                "reason": "no candidate artifact format is served by any envelope runtime",
            }
            axes["runtime"] = {
                "status": "BLOCKED-HARDWARE",
                "declared_runtimes": list(runtimes),
                "reason": "envelope has no runtime for the candidate artifact formats",
            }

    if modalities:
        unsupported = sorted(set(modalities) - supported_modalities)
        if unsupported:
            axes["modality"] = {
                "status": "BLOCKED-HARDWARE",
                "declared_modalities": sorted(supported_modalities),
                "candidate_modalities": sorted(modalities),
                "unsupported": unsupported,
                "reason": "candidate modality is not covered by envelope",
            }
        else:
            axes["modality"] = {
                "status": "PASS",
                "declared_modalities": sorted(supported_modalities),
                "candidate_modalities": sorted(modalities),
                "unsupported": [],
                "reason": "every candidate modality is covered by envelope",
            }
    else:
        axes["modality"] = {
            "status": "UNRESOLVED",
            "declared_modalities": sorted(supported_modalities),
            "candidate_modalities": [],
            "unsupported": [],
            "reason": "candidate modality is not declared at source-entry layer",
        }

    axes["acceleration"] = {
        "status": "UNRESOLVED",
        "declared_class": envelope["accelerator"]["class"],
        "declared_device_memory_bytes": envelope["accelerator"]["device_memory_bytes"],
        "reason": "exact-artifact-and-profile-required-to-measure-accelerator-fit",
    }
    axes["disk"] = {
        "status": "UNRESOLVED",
        "declared_bytes": envelope["storage_bytes"],
        "candidate_bytes": None,
        "reason": "exact-artifact-byte-size-not-selected",
    }
    axes["memory"] = {
        "status": "UNRESOLVED",
        "declared_bytes": envelope["memory_bytes"],
        "candidate_bytes": None,
        "reason": "exact-runtime-and-quantization-required-to-measure-resident-memory",
    }
    axes["context"] = {
        "status": "UNRESOLVED",
        "declared_ceiling_tokens": envelope["context_token_ceiling"],
        "candidate_tokens": entry.get("context_tokens"),
        "reason": "context ceiling not declared at source-entry layer",
    }
    axes["expected_working_set"] = {
        "status": "UNRESOLVED",
        "declared_ceiling_bytes": envelope["expected_working_set_ceiling_bytes"],
        "candidate_bytes": None,
        "reason": "expected working set requires measured runtime profile",
    }

    for axis_name in AXES:
        if axis_name not in axes:
            raise ValueError(f"missing preflight axis: {axis_name}")
        if axes[axis_name]["status"] not in ACCEPTABLE_AXIS_STATUSES:
            raise ValueError(f"invalid axis status for {axis_name}: {axes[axis_name]['status']}")
    return axes


def _entry_status(entry: dict[str, Any], axes: dict[str, dict[str, Any]]) -> tuple[str, list[str]]:
    if entry["disposition"] == "INELIGIBLE":
        return "INELIGIBLE", ["source-ineligible-remains-ineligible-per-envelope"]
    hardware_axes = [name for name in AXES if axes[name]["status"] == "BLOCKED-HARDWARE"]
    if hardware_axes:
        return "BLOCKED-HARDWARE", sorted(f"{name}-blocked-hardware" for name in hardware_axes)
    return "BLOCKED", [
        "exact-artifact-bytes-and-digest-not-selected",
        "expected-working-set-not-measured",
    ]


def build() -> dict[str, Any]:
    inventory = read_json(INVENTORY)
    matrix = read_json(ROLE_MATRIX)
    envelopes = read_json(ENVELOPES)

    if matrix.get("inventory_sha256") != inventory.get("inventory_sha256"):
        raise ValueError("role matrix does not bind the current inventory")

    envelope_by_id = {env["envelope_id"]: env for env in envelopes["envelopes"]}
    if len(envelope_by_id) != len(envelopes["envelopes"]):
        raise ValueError("envelope declaration contains duplicate identity")

    matrix_entries = {item["entry_id"]: item for item in matrix["entries"]}

    results: list[dict[str, Any]] = []
    for entry in inventory["entries"]:
        entry_id = entry["entry_id"]
        matrix_entry = matrix_entries.get(entry_id)
        if matrix_entry is None:
            raise ValueError(f"inventory entry missing from role matrix: {entry_id}")
        if matrix_entry["repository"] != entry["repository"]:
            raise ValueError(f"role-matrix repository disagrees with inventory: {entry_id}")
        per_envelope: list[dict[str, Any]] = []
        for envelope_id in sorted(envelope_by_id):
            envelope = envelope_by_id[envelope_id]
            axes = _preflight_axes(entry, envelope)
            entry_status, reasons = _entry_status(entry, axes)
            per_envelope.append(
                {
                    "envelope_id": envelope_id,
                    "acquisition_started": False,
                    "runtime_started": False,
                    "network_egress_opened": False,
                    "role_borrowed_from_family": False,
                    "result_borrowed_from_sibling": False,
                    "preflight_status": entry_status,
                    "reason_codes": reasons,
                    "axes": axes,
                }
            )
        results.append(
            {
                "entry_id": entry_id,
                "repository": entry["repository"],
                "revision": entry["revision"],
                "developer": entry["developer"],
                "publisher": entry["publisher"],
                "source_disposition": entry["disposition"],
                "roles": list(entry.get("roles", [])),
                "prohibited_roles": list(entry.get("prohibited_roles", [])),
                "applicable_suites": list(entry.get("applicable_suites", [])),
                "artifact_formats": list(entry.get("artifact_formats", [])),
                "modalities": [m for m in entry.get("modalities", []) if m],
                "per_envelope": per_envelope,
            }
        )

    counts_by_envelope: dict[str, dict[str, int]] = {}
    for result in results:
        for entry in result["per_envelope"]:
            envelope_id = entry["envelope_id"]
            bucket = counts_by_envelope.setdefault(
                envelope_id, {status: 0 for status in sorted(ACCEPTABLE_ENTRY_STATUSES)}
            )
            bucket[entry["preflight_status"]] += 1

    frozen_inputs = {
        str(path.relative_to(ROOT)): sha256_bytes(path.read_bytes())
        for path in (INVENTORY, ROLE_MATRIX, ENVELOPES)
    }

    report: dict[str, Any] = {
        "schema_version": 1,
        "record_type": "reference_machine_preflight_report",
        "frozen_on": inventory["frozen_on"],
        "frozen_inputs": frozen_inputs,
        "envelope_ids": sorted(envelope_by_id),
        "entry_count": len(results),
        "envelope_count": len(envelope_by_id),
        "decision_count": len(results) * len(envelope_by_id),
        "counts_by_envelope": counts_by_envelope,
        "product_state": {
            "acquisitions_started": 0,
            "runtimes_started": 0,
            "network_egress_opened": 0,
            "results_borrowed_across_family": 0,
        },
        "non_acquisition_rule": envelopes["non_acquisition_rule"],
        "role_isolation_rule": envelopes["role_isolation_rule"],
        "hardware_admission_rule": envelopes["hardware_admission_rule"],
        "entries": results,
    }
    report["report_sha256"] = sha256_bytes(canonical_bytes(report))
    return report


def validate(report: dict[str, Any]) -> list[str]:
    failures: list[str] = []
    rebuilt = build()
    copy_report = {k: v for k, v in report.items() if k != "report_sha256"}
    copy_rebuilt = {k: v for k, v in rebuilt.items() if k != "report_sha256"}
    if copy_report != copy_rebuilt:
        failures.append("report is not reproducible from frozen inputs")
    if report.get("report_sha256") != sha256_bytes(canonical_bytes(copy_report)):
        failures.append("report digest mismatch")

    inventory = read_json(INVENTORY)
    envelopes = read_json(ENVELOPES)
    if report["entry_count"] != len(inventory["entries"]):
        failures.append("preflight entry count does not reconcile with inventory")
    if report["envelope_count"] != len(envelopes["envelopes"]):
        failures.append("preflight envelope count does not reconcile with declaration")
    if report["decision_count"] != report["entry_count"] * report["envelope_count"]:
        failures.append("preflight decision count is not the exact cartesian product")

    inventory_ids = [item["entry_id"] for item in inventory["entries"]]
    report_ids = [item["entry_id"] for item in report["entries"]]
    if inventory_ids != report_ids:
        failures.append("preflight order does not follow inventory order exactly")
    if len(report_ids) != len(set(report_ids)):
        failures.append("preflight contains duplicate entry identity")

    envelope_id_set = {env["envelope_id"] for env in envelopes["envelopes"]}
    for item in report["entries"]:
        seen_envelopes = {row["envelope_id"] for row in item["per_envelope"]}
        if seen_envelopes != envelope_id_set:
            failures.append(f"preflight envelope coverage mismatch: {item['entry_id']}")
        for row in item["per_envelope"]:
            if row["acquisition_started"] or row["runtime_started"] or row["network_egress_opened"]:
                failures.append(f"non-acquisition rule violated: {item['entry_id']}")
            if row["role_borrowed_from_family"] or row["result_borrowed_from_sibling"]:
                failures.append(f"family borrowing detected: {item['entry_id']}")
            if row["preflight_status"] not in ACCEPTABLE_ENTRY_STATUSES:
                failures.append(f"invalid preflight status: {item['entry_id']}")
            if item["source_disposition"] == "INELIGIBLE" and row["preflight_status"] != "INELIGIBLE":
                failures.append(f"ineligible source silently promoted: {item['entry_id']}")
            if item["source_disposition"] != "INELIGIBLE" and row["preflight_status"] == "INELIGIBLE":
                failures.append(f"unexpected ineligibility inferred: {item['entry_id']}")
        if "coding_planner" in item["prohibited_roles"] and "coding_planner" in item["roles"]:
            failures.append(f"specialist role escalation: {item['entry_id']}")

    product_state = report["product_state"]
    if any(value != 0 for value in product_state.values()):
        failures.append("preflight changed zero-authority product state")
    return failures


def write_report(report: dict[str, Any]) -> None:
    REPORT.parent.mkdir(parents=True, exist_ok=True)
    REPORT.write_text(json.dumps(report, indent=2, sort_keys=True) + "\n", encoding="utf-8")


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--write", action="store_true")
    args = parser.parse_args()
    report = build()
    if args.write:
        write_report(report)
    stored = read_json(REPORT) if REPORT.exists() else report
    failures = validate(stored)
    if failures:
        print("Reference-machine preflight: invalid")
        for failure in failures:
            print(f"- {failure}")
        return 1
    print(
        "Reference-machine preflight: valid "
        f"({report['entry_count']} exact source entries "
        f"across {report['envelope_count']} declared envelopes; "
        "zero acquisition, zero borrowing)"
    )
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
