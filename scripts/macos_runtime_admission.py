#!/usr/bin/env python3
"""Validate the exact Apple Silicon llama.cpp runtime admitted for evaluation."""

from __future__ import annotations

import argparse
import json
import re
import sys
from pathlib import Path
from typing import Final


ROOT: Final = Path(__file__).resolve().parents[1]
DEFAULT_RECORD: Final = (
    ROOT / "model-profiles" / "runtimes" / "llama-cpp-b10333-macos-arm64.json"
)
SHA256: Final = re.compile(r"[0-9a-f]{64}")
COMMIT: Final = re.compile(r"[0-9a-f]{40}")
EXPECTED_ARTIFACT: Final = {
    "download_url": (
        "https://github.com/ggml-org/llama.cpp/releases/download/b10333/"
        "llama-b10333-bin-macos-arm64.tar.gz"
    ),
    "name": "llama-b10333-bin-macos-arm64.tar.gz",
    "sha256": "e5d67c5264107e3c14d3bf2aee349365bb2b85ae99bb077a0cb974a1c4c2741a",
    "size_bytes": 11015270,
}
EXPECTED_FILES: Final = {
    "libggml_metal": {
        "relative_path": "llama-b10333/libggml-metal.0.19.0.dylib",
        "sha256": "5cac42dbd02198b55bd03365132b757f87553462bf07b2273116b2226b5e4670",
        "size_bytes": 884456,
    },
    "llama_cli": {
        "relative_path": "llama-b10333/llama-cli",
        "sha256": "b4cd2feb131d1f96895a9551e2f47e6510d90f63c3b998ee974eda9d6e132a13",
        "size_bytes": 49960,
    },
    "llama_server": {
        "relative_path": "llama-b10333/llama-server",
        "sha256": "d3bce60d45758268a90e0fca82ce5a22d5c35ecb92a06d1c544ed68ec2efa769",
        "size_bytes": 33472,
    },
}


def load_record(path: Path = DEFAULT_RECORD) -> dict[str, object]:
    value = json.loads(path.read_text(encoding="utf-8"))
    if not isinstance(value, dict):
        raise ValueError("macOS runtime admission must be a JSON object")
    return value


def validate_record(record: dict[str, object]) -> list[str]:
    failures: list[str] = []
    required = {
        "schema_version",
        "record_type",
        "adapter_id",
        "platform",
        "architecture",
        "project",
        "release",
        "source_commit",
        "artifact",
        "critical_files",
        "decision",
    }
    if set(record) != required:
        return ["macOS runtime admission top-level fields do not match the schema"]
    expected_identity = {
        "schema_version": 1,
        "record_type": "macos_native_runtime_admission",
        "adapter_id": "macos-native-metal",
        "platform": "macOS",
        "architecture": "arm64",
        "project": "ggml-org/llama.cpp",
        "release": "b10333",
        "source_commit": "08659901c43b51de735740f1cf61bb82fbe0c4e4",
    }
    for field, expected in expected_identity.items():
        if record.get(field) != expected:
            failures.append(f"macOS runtime identity mismatch: {field}")
    if not COMMIT.fullmatch(str(record.get("source_commit", ""))):
        failures.append("macOS runtime source commit is not immutable")

    artifact = record.get("artifact")
    if artifact != EXPECTED_ARTIFACT:
        failures.append("macOS runtime archive identity mismatch")
    elif not SHA256.fullmatch(str(artifact.get("sha256", ""))):
        failures.append("macOS runtime archive hash is malformed")

    files = record.get("critical_files")
    if files != EXPECTED_FILES:
        failures.append("macOS runtime critical-file identities mismatch")
    elif any(
        not SHA256.fullmatch(str(item.get("sha256", "")))
        for item in files.values()
        if isinstance(item, dict)
    ):
        failures.append("macOS runtime critical-file hash is malformed")

    decision = record.get("decision")
    expected_decision = {
        "execution_status": "NOT_RUN",
        "hardware_status": "BLOCKED_REQUIRED_MACBOOK_PRO_M5",
        "linux_evidence_substituted": False,
        "release_approval": False,
    }
    if decision != expected_decision:
        failures.append("macOS runtime pre-execution decision is overstated or malformed")
    return failures


def main(argv: list[str] | None = None) -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--record", type=Path, default=DEFAULT_RECORD)
    args = parser.parse_args(argv)
    try:
        failures = validate_record(load_record(args.record))
    except (OSError, json.JSONDecodeError, ValueError) as error:
        print(f"macOS runtime admission validation failed: {error}", file=sys.stderr)
        return 1
    if failures:
        for failure in failures:
            print(f"- {failure}", file=sys.stderr)
        return 1
    print(f"Validated blocked macOS runtime admission at {args.record}.")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
