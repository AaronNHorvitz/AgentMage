#!/usr/bin/env python3
"""Check or regenerate the runtime producer contract fixtures (Decision 0135).

The fixtures of the current contract version (2, Decision 0143) are built by
the host's own types in the ignored Rust test
`print_runtime_producer_fixtures`, which prints them and writes nothing. In
check mode this script compares the printed files with the committed ones;
with --write it replaces the current version's directory's files with them.
Version 1 is never written.
"""

from __future__ import annotations

import argparse
import json
import subprocess
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
FIXTURES = ROOT / "fixtures/runtime-producer/v2"
BEGIN = "BEGIN-RUNTIME-PRODUCER-FIXTURES"
END = "END-RUNTIME-PRODUCER-FIXTURES"
TEST = "runtime_producer_contract_tests::print_runtime_producer_fixtures"


def printed_fixtures(output: str) -> dict[str, str]:
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
    for name, text in files.items():
        if not isinstance(name, str) or "/" in name or not name.endswith(".json"):
            raise ValueError(f"unexpected fixture name {name!r}")
        if not isinstance(text, str):
            raise ValueError(f"fixture {name} is not text")
    return files


def build() -> dict[str, str]:
    completed = subprocess.run(
        ["cargo", "test", "--locked", "--offline", "-p", "agentmage-host", "--lib", "--",
         "--ignored", "--exact", "--nocapture", TEST],
        cwd=ROOT, check=True, capture_output=True, text=True)
    return printed_fixtures(completed.stdout)


def committed() -> dict[str, str]:
    return {path.name: path.read_text(encoding="utf-8") for path in sorted(FIXTURES.iterdir())}


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
        for name, text in files.items():
            (FIXTURES / name).write_text(text, encoding="utf-8")
        print(f"wrote {len(files)} runtime producer fixtures")
        return 0
    if committed() != files:
        print("runtime producer fixtures differ from what the runtime builds", file=sys.stderr)
        return 1
    print(f"runtime producer fixtures match ({len(files)} files)")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
