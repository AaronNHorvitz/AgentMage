#!/usr/bin/env python3
"""Check or regenerate the public research worker contract fixtures (Decision 0141).

The engine's own types build the fixtures in the ignored Rust test
`print_public_research_worker_fixtures`, which prints each file's bytes as hex
and writes nothing. In check mode this script compares the printed files with
the committed ones; with --write it replaces the committed directory's files.
"""

from __future__ import annotations

import argparse
import json
import subprocess
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
FIXTURES = ROOT / "fixtures/public-research-worker/v1"
BEGIN = "BEGIN-PUBLIC-RESEARCH-WORKER-FIXTURES"
END = "END-PUBLIC-RESEARCH-WORKER-FIXTURES"
TEST = "research_response::contract_tests::print_public_research_worker_fixtures"
SUFFIXES = (".json", ".frame")


def printed_fixtures(output: str) -> dict[str, bytes]:
    """Returns the files between the marker lines of the test's output."""
    lines = output.splitlines()
    if lines.count(BEGIN) != 1 or lines.count(END) != 1:
        raise ValueError("the fixture printer's markers are missing or repeated")
    start, end = lines.index(BEGIN), lines.index(END)
    if end != start + 2:
        raise ValueError("the fixture printer must print exactly one line")
    files = json.loads(lines[start + 1])
    if not isinstance(files, dict) or not files:
        raise ValueError("the fixture printer printed no files")
    result = {}
    for name, text in files.items():
        if not isinstance(name, str) or "/" in name or not name.endswith(SUFFIXES):
            raise ValueError(f"unexpected fixture name {name!r}")
        if not isinstance(text, str):
            raise ValueError(f"fixture {name} is not hex text")
        result[name] = bytes.fromhex(text)
    return result


def build() -> dict[str, bytes]:
    completed = subprocess.run(
        ["cargo", "test", "--locked", "--offline", "-p", "agentmage-kernel-engine", "--lib", "--",
         "--ignored", "--exact", "--nocapture", TEST],
        cwd=ROOT, check=True, capture_output=True, text=True)
    return printed_fixtures(completed.stdout)


def committed() -> dict[str, bytes]:
    if not FIXTURES.is_dir():
        return {}
    return {path.name: path.read_bytes() for path in sorted(FIXTURES.iterdir())}


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--write", action="store_true", help="replace the committed fixtures")
    arguments = parser.parse_args()
    files = build()
    if arguments.write:
        FIXTURES.mkdir(parents=True, exist_ok=True)
        for path in FIXTURES.iterdir():
            if path.name not in files:
                path.unlink()
        for name, data in files.items():
            (FIXTURES / name).write_bytes(data)
        print(f"wrote {len(files)} public research worker fixtures")
        return 0
    if committed() != files:
        print("public research worker fixtures differ from what the engine builds", file=sys.stderr)
        return 1
    print(f"public research worker fixtures match ({len(files)} files)")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
