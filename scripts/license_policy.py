#!/usr/bin/env python3
"""Enforce the owner-selected dual licence without relabelling dependencies."""

from __future__ import annotations

import hashlib
import json
from pathlib import Path
import re
import subprocess
import sys
import tomllib

ROOT = Path(__file__).resolve().parents[1]
EXPRESSION = "MIT OR Apache-2.0"
PINNED_TEXTS = {'LICENSE': '59ebbb74beebe1ceec69999f18f094d730742206c40a02b51ff7036d5c8b233e', 'LICENSE-MIT': '49b6046baf54cc1167ef68b364f3da580cde27aeae6b900263a2bc3ed90278b6', 'LICENSE-APACHE': 'f04bf757ffe8ac0a6cd4ff8403b33f5f657b9b4ed049834f0b5444ab81eb862e', 'docs/decisions/0051-business-source-license.md': 'a0cc37465a553c9413b2d14b60c4c7d815345f22a534152557673f942bbed0cd'}
INVENTORY_SHA256 = '71ea8e693134e43a735c4f2dd3ad80dbbbd1533147528ce47dbc4dada51ae4df'


def check(root: Path = ROOT) -> list[str]:
    failures = []
    for name, digest in PINNED_TEXTS.items():
        if hashlib.sha256((root / name).read_bytes()).hexdigest() != digest:
            failures.append(f"{name}: complete pinned licence text changed")
    if (root / "kernel/contracts/LICENSE").read_bytes() != (root / "LICENSE").read_bytes():
        failures.append("contract crate must carry both complete licence alternatives")
    inventory = (root / "Agent-Scaffolding-Inventory.md").read_bytes()
    _, separator, original = inventory.partition(b"\n\n")
    if not separator or hashlib.sha256(original).hexdigest() != INVENTORY_SHA256:
        failures.append("historical inventory original text changed")
    paths = subprocess.check_output(["git", "ls-files", "-z"], cwd=root).decode().split("\0")
    workspace = tomllib.loads((root / "Cargo.toml").read_text())["workspace"]["package"]
    if workspace["license"] != EXPRESSION:
        failures.append("workspace licence must be MIT OR Apache-2.0")
    for name in paths:
        if name.endswith("Cargo.toml") and name != "Cargo.toml":
            package = tomllib.loads((root / name).read_text())["package"]
            value = package.get("license")
            if value == {"workspace": True}:
                value = workspace["license"]
            if value != EXPRESSION:
                failures.append(f"{name}: crate licence must resolve to {EXPRESSION}")
        if name.endswith((".rs", ".py", ".ts", ".swift")) and not name.startswith(("artifacts/", "docs/reviews/")):
            text = (root / name).read_text()
            for header in re.findall(r"^(?://|#) SPDX-License-Identifier: (.+)$", text, re.MULTILINE):
                if header != EXPRESSION:
                    failures.append(f"{name}: inconsistent source SPDX header")
    for name in ("package.json", "shells/vscode/package.json"):
        if json.loads((root / name).read_text())["license"] != EXPRESSION:
            failures.append(f"{name}: package licence must be {EXPRESSION}")
    lock = json.loads((root / "package-lock.json").read_text())["packages"]
    for name in ("", "shells/vscode"):
        if lock[name]["license"] != EXPRESSION:
            failures.append("workspace npm lock licence differs from its package")
    for name in ("packaging/linux/agentmage.spec.in", "packaging/linux/agentmage-release.spec.in"):
        if f"License:        ({EXPRESSION})" not in (root / name).read_text():
            failures.append(f"{name}: package licence differs")
    for name in ("NOTICE", "README.md"):
        text = (root / name).read_text()
        for marker in (EXPRESSION, "2026-09-07", "Business Source License", "published", "relicensing commit"):
            if marker not in text:
                failures.append(f"{name}: missing current or historical licence marker: {marker}")
    contribution = (root / "CONTRIBUTING.md").read_text()
    for marker in (EXPRESSION, "Signed-off-by:", "no right to the project's name"):
        if marker not in contribution:
            failures.append("contribution licence, sign-off or name boundary missing")
    return failures


def main() -> int:
    try:
        failures = check()
    except (KeyError, ValueError, OSError, subprocess.CalledProcessError) as error:
        print(f"LICENCE POLICY: FAIL ({type(error).__name__})", file=sys.stderr)
        return 1
    if failures:
        print("\n".join(failures), file=sys.stderr)
        return 1
    print("LICENCE POLICY: PASS")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
