#!/usr/bin/env python3
"""Build a deterministic signable AgentMage Linux release bundle."""

from __future__ import annotations

import argparse
import os
import subprocess
import zipfile
from pathlib import Path

from package_candidate import (
    ROOT,
    PackageCandidateError,
    build_release_bundle,
    canonical_json,
)


def main(argv: list[str] | None = None) -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--output", type=Path, default=ROOT / "release-output")
    parser.add_argument("--version", required=True)
    parser.add_argument("--release-sequence", required=True, type=int)
    args = parser.parse_args(argv)
    try:
        artifacts = build_release_bundle(
            args.output.resolve(), args.version, args.release_sequence
        )
    except (OSError, ValueError, subprocess.SubprocessError, zipfile.BadZipFile) as error:
        print(f"package release bundle failed: {error}", file=os.sys.stderr)
        return 1
    print(
        canonical_json({name: str(path) for name, path in sorted(artifacts.items())}).decode(),
        end="",
    )
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
