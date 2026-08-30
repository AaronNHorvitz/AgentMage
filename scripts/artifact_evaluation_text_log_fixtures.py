#!/usr/bin/env python3
"""Build and verify Story 2.3.1.1 text, reference, and bounded-log fixtures."""

from __future__ import annotations

import argparse
import copy
import json
import sys
import zipfile
from pathlib import Path
from typing import Any, Final


ROOT: Final = Path(__file__).resolve().parents[1]
if str(ROOT) not in sys.path:
    sys.path.insert(0, str(ROOT))

from scripts.engineering_artifact_admission_fixtures import (  # noqa: E402
    ZERO_SHA256,
    build_archive,
    canonical_json,
    sealed,
    sha256_bytes,
    validate_archive,
    write_atomic,
)


FIXTURE_DIR: Final = ROOT / "fixtures" / "artifact-evaluation" / "v1"
ARCHIVE_PATH: Final = FIXTURE_DIR / "text-reference-corpus-v1.zip"
MANIFEST_PATH: Final = FIXTURE_DIR / "text-reference-manifest.json"
LOG_SIZE: Final = 25 * 1024 * 1024
LOG_SEED: Final = "agentmage-story-2.3-log-seed-v1"
LOG_BLOCK: Final = (
    "2026-08-29T00:00:00Z INFO public-synthetic-log "
    f"seed={LOG_SEED} event=bounded-replay-safe\n"
).encode("ascii")
CATEGORIES: Final = (
    "paste_999",
    "paste_1001",
    "utf8",
    "utf16",
    "invalid_encoding",
    "mixed_encoding",
    "empty",
    "duplicate_original",
    "duplicate_reference",
    "stale",
    "replaced",
    "virtual_reference",
    "remote_reference",
    "inaccessible_reference",
    "unknown_reference",
    "bounded_25_mib_log",
)


def bounded_log() -> bytes:
    repeats, remainder = divmod(LOG_SIZE, len(LOG_BLOCK))
    return LOG_BLOCK * repeats + LOG_BLOCK[:remainder]


def payloads() -> dict[str, bytes]:
    duplicate = b"public synthetic duplicate payload\n"
    entries = {
        "payloads/paste-999.txt": "p".encode() * 999,
        "payloads/paste-1001.txt": "q".encode() * 1001,
        "payloads/utf8.txt": "Public synthetic UTF-8: cafe\u0301, snowman \u2603, rocket \U0001f680.\n".encode("utf-8"),
        "payloads/utf16.txt": "Public synthetic UTF-16: alpha beta gamma.\n".encode("utf-16"),
        "payloads/invalid.bin": b"public-utf8-prefix\xff\xfe\x80invalid-tail",
        "payloads/mixed.bin": b"utf8-prefix\n" + "utf16-middle\n".encode("utf-16-le") + b"\xffsuffix",
        "payloads/empty.txt": b"",
        "payloads/duplicate-original.txt": duplicate,
        "payloads/duplicate-reference.txt": duplicate,
        "payloads/stale-observed.txt": b"public synthetic stale observed revision\n",
        "payloads/stale-current.txt": b"public synthetic stale current revision\n",
        "payloads/replaced-observed.txt": b"public synthetic replaced observed bytes\n",
        "payloads/replaced-current.txt": b"public synthetic replaced current bytes\n",
        "payloads/virtual.txt": b"public synthetic pre-staged virtual URI bytes\n",
        "payloads/remote.txt": b"public synthetic pre-staged remote URI bytes; no fetch\n",
    }
    return dict(sorted(entries.items()))


def identity(content: bytes) -> dict[str, Any]:
    return {"byte_length": len(content), "sha256": sha256_bytes(content)}


def case(
    fixture_id: str,
    category: str,
    *,
    entry: str | None,
    encoding: str,
    disposition: str,
    reference_class: str = "inline",
    current_entry: str | None = None,
    relation: str | None = None,
    character_count: int | None = None,
    generated: bool = False,
) -> dict[str, Any]:
    entries = payloads()
    content = bounded_log() if generated else (None if entry is None else entries[entry])
    current = None if current_entry is None else entries[current_entry]
    return {
        "fixture_id": fixture_id,
        "category": category,
        "reference_class": reference_class,
        "encoding_expectation": encoding,
        "expected_disposition": disposition,
        "archive_entry": entry,
        "generated_from_seed": generated,
        "byte_identity": None if content is None else identity(content),
        "character_count": character_count,
        "current_archive_entry": current_entry,
        "current_byte_identity": None if current is None else identity(current),
        "identity_relation": relation,
        "network_access": False,
        "synthetic_only": True,
    }


def cases() -> list[dict[str, Any]]:
    return [
        case("eval-paste-999-v1", "paste_999", entry="payloads/paste-999.txt", encoding="utf-8", disposition="captured", character_count=999),
        case("eval-paste-1001-v1", "paste_1001", entry="payloads/paste-1001.txt", encoding="utf-8", disposition="captured", character_count=1001),
        case("eval-utf8-v1", "utf8", entry="payloads/utf8.txt", encoding="utf-8-valid", disposition="captured"),
        case("eval-utf16-v1", "utf16", entry="payloads/utf16.txt", encoding="utf-16-with-bom", disposition="captured"),
        case("eval-invalid-v1", "invalid_encoding", entry="payloads/invalid.bin", encoding="invalid-utf-8", disposition="failed"),
        case("eval-mixed-v1", "mixed_encoding", entry="payloads/mixed.bin", encoding="mixed-utf8-utf16-invalid", disposition="partial"),
        case("eval-empty-v1", "empty", entry="payloads/empty.txt", encoding="empty", disposition="captured", character_count=0),
        case("eval-duplicate-original-v1", "duplicate_original", entry="payloads/duplicate-original.txt", encoding="utf-8", disposition="captured"),
        case("eval-duplicate-reference-v1", "duplicate_reference", entry="payloads/duplicate-reference.txt", encoding="utf-8", disposition="duplicate", relation="same-bytes-distinct-reference"),
        case("eval-stale-v1", "stale", entry="payloads/stale-observed.txt", current_entry="payloads/stale-current.txt", encoding="utf-8", disposition="stale", relation="observed-differs-from-current"),
        case("eval-replaced-v1", "replaced", entry="payloads/replaced-observed.txt", current_entry="payloads/replaced-current.txt", encoding="utf-8", disposition="failed", relation="object-replaced-after-observation"),
        case("eval-virtual-v1", "virtual_reference", entry="payloads/virtual.txt", encoding="utf-8", disposition="captured", reference_class="virtual_uri"),
        case("eval-remote-v1", "remote_reference", entry="payloads/remote.txt", encoding="utf-8", disposition="captured", reference_class="remote_uri"),
        case("eval-inaccessible-v1", "inaccessible_reference", entry=None, encoding="unavailable", disposition="unavailable", reference_class="local_file"),
        case("eval-unknown-v1", "unknown_reference", entry=None, encoding="unknown", disposition="unsupported", reference_class="unknown"),
        case("eval-log-25mib-v1", "bounded_25_mib_log", entry=None, encoding="ascii", disposition="captured", reference_class="local_file", generated=True),
    ]


def build_suite() -> tuple[bytes, dict[str, Any]]:
    entries = payloads()
    archive = build_archive(entries)
    value = sealed(
        {
            "schema_version": 1,
            "corpus_id": "artifact-evaluation-text-reference-v1",
            "task_id": "2.3.1.1",
            "generated_on": "2026-08-29",
            "status": "synthetic-fixture-contract",
            "synthetic_only": True,
            "network_access": False,
            "product_parser_support_claim": "none",
            "required_categories": list(CATEGORIES),
            "case_count": len(CATEGORIES),
            "archive": {
                "path": str(ARCHIVE_PATH.relative_to(ROOT)),
                "format": "zip-stored",
                "entry_count": len(entries),
                "byte_length": len(archive),
                "sha256": sha256_bytes(archive),
            },
            "bounded_log_recipe": {
                "seed": LOG_SEED,
                "algorithm": "repeat-ascii-block-and-prefix-remainder-v1",
                "target_byte_length": LOG_SIZE,
                "block_sha256": sha256_bytes(LOG_BLOCK),
                "generated_sha256": sha256_bytes(bounded_log()),
                "persisted_in_archive": False,
            },
            "generator": {
                "path": "scripts/artifact_evaluation_text_log_fixtures.py",
                "sha256": sha256_bytes(Path(__file__).read_bytes()),
            },
            "cases": cases(),
            "manifest_sha256": ZERO_SHA256,
        },
        "manifest_sha256",
    )
    return archive, value


def validate_manifest(value: Any) -> list[str]:
    if not isinstance(value, dict):
        return ["text/reference manifest must be an object"]
    failures: list[str] = []
    unhashed = copy.deepcopy(value)
    recorded = unhashed.get("manifest_sha256")
    unhashed["manifest_sha256"] = ZERO_SHA256
    if recorded != sha256_bytes(canonical_json(unhashed)):
        failures.append("text/reference manifest self-hash is invalid")
    observed = value.get("cases", [])
    if [item.get("category") for item in observed if isinstance(item, dict)] != list(CATEGORIES):
        failures.append("text/reference categories are incomplete or reordered")
    if value.get("case_count") != len(CATEGORIES):
        failures.append("text/reference case count is incomplete")
    if value.get("bounded_log_recipe", {}).get("target_byte_length") != LOG_SIZE or value.get("bounded_log_recipe", {}).get("generated_sha256") != sha256_bytes(bounded_log()):
        failures.append("bounded 25 MiB log recipe does not reproduce exactly")
    if any(item.get("network_access") is not False or item.get("synthetic_only") is not True for item in observed if isinstance(item, dict)):
        failures.append("a text/reference fixture widened network or data scope")
    unavailable = [item for item in observed if item.get("expected_disposition") in {"unavailable", "unsupported"}]
    if any(item.get("byte_identity") is not None or item.get("archive_entry") is not None for item in unavailable):
        failures.append("unavailable or unknown reference invented bytes")
    if value.get("product_parser_support_claim") != "none":
        failures.append("text/reference corpus makes a product parser support claim")
    return failures


def check() -> list[str]:
    try:
        expected_archive, expected_manifest = build_suite()
        actual_archive = ARCHIVE_PATH.read_bytes()
        actual_manifest = json.loads(MANIFEST_PATH.read_text(encoding="utf-8"))
    except (OSError, ValueError, json.JSONDecodeError, zipfile.BadZipFile) as error:
        return [f"cannot validate text/reference corpus: {error}"]
    failures = validate_archive(actual_archive, payloads()) + validate_manifest(actual_manifest)
    if actual_archive != expected_archive:
        failures.append("checked text/reference archive is stale or corrupt")
    if actual_manifest != expected_manifest:
        failures.append("checked text/reference manifest is stale, incomplete, or widened")
    return failures


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--write", action="store_true")
    args = parser.parse_args()
    if args.write:
        archive, manifest = build_suite()
        write_atomic(ARCHIVE_PATH, archive)
        write_atomic(MANIFEST_PATH, canonical_json(manifest))
    failures = check()
    if failures:
        for failure in failures:
            print(f"Text/reference fixture validation failed: {failure}", file=sys.stderr)
        return 1
    print(f"Validated {len(CATEGORIES)} text, reference, and bounded-log fixtures")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
