#!/usr/bin/env python3
"""Validate the reviewable classification and retention decision table."""

from __future__ import annotations

import hashlib
import json
import re
from pathlib import Path
from typing import Final

ROOT: Final = Path(__file__).resolve().parents[1]
TABLE_PATH: Final = ROOT / "docs/architecture/data-classification-retention-table.md"
PERSISTENCE_PATH: Final = ROOT / "kernel/engine/src/persistence.rs"
STORE_PATH: Final = ROOT / "kernel/engine/src/operational_store.rs"
EXPECTED_IDS: Final = (
    "DC-01", "DC-02", "DC-03", "DC-04", "DC-05", "DC-06", "DC-07", "DC-08",
    "RT-01", "RT-02", "RT-03", "RT-04", "RT-05",
    "LC-01", "LC-02", "LC-03", "LC-04", "LC-05", "LC-06",
    "DE-01", "BK-01", "RS-01", "ER-01", "ER-02",
)
TABLE_FRAGMENTS: Final = (
    "Restricted persistence remains unavailable",
    "Application-host wiring remains open",
    "Never read as startup authority",
    "Not per-record; separately keyed backups require their own erasure",
    "No physical-overwrite claim",
)
PERSISTENCE_FRAGMENTS: Final = (
    "pub enum PersistenceSensitivity {",
    "pub enum PersistenceRetentionIntent {",
    "pub enum PersistenceFieldHandling {",
    "pub enum EphemeralContentClass {",
    "Some(PersistenceOutcome::DeniedRestricted)",
    "let encryption = if prepared.is_some() {",
)
STORE_FRAGMENTS: Final = (
    "pub enum RetentionHoldKind {",
    "pub fn apply_retention_hold(",
    "pub fn release_retention_hold(",
    "pub fn expire_due(",
    "pub fn export_json_lines(",
    "pub fn restore_to_fresh_candidate<",
    "pub fn cryptographic_erase<L: OperationalStoreKeyLifecycle>(",
)
ID = re.compile(r"`([A-Z]{2}-\d{2})`")


def validate(table: str, persistence: str, store: str) -> list[str]:
    failures = []
    if tuple(ID.findall(table)) != EXPECTED_IDS:
        failures.append("classification-retention row closure changed")
    for label, value, fragments in (
        ("table", table, TABLE_FRAGMENTS),
        ("persistence", persistence, PERSISTENCE_FRAGMENTS),
        ("store", store, STORE_FRAGMENTS),
    ):
        failures.extend(
            f"classification-retention {label} fragment changed: {index}"
            for index, fragment in enumerate(fragments, 1)
            if value.count(fragment) != 1
        )
    return failures


def report() -> dict[str, object]:
    values = {
        "table": TABLE_PATH.read_text(),
        "persistence": PERSISTENCE_PATH.read_text(),
        "store": STORE_PATH.read_text(),
    }
    failures = validate(values["table"], values["persistence"], values["store"])
    if failures:
        raise ValueError("; ".join(failures))
    return {
        "artifact_id": "classification-retention-decision-table",
        "contract_version": 1,
        "row_count": len(EXPECTED_IDS),
        "row_ids": list(EXPECTED_IDS),
        "sources": {
            "decision_table_sha256": hashlib.sha256(values["table"].encode()).hexdigest(),
            "persistence_source_sha256": hashlib.sha256(values["persistence"].encode()).hexdigest(),
            "store_source_sha256": hashlib.sha256(values["store"].encode()).hexdigest(),
        },
        "limitations_preserved": True,
        "private_user_data_used": False,
        "external_network_used": False,
        "release_support": False,
    }


def main() -> int:
    print(json.dumps(report(), sort_keys=True))
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
