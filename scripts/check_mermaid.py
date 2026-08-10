#!/usr/bin/env python3
"""Parse every Mermaid block with the pinned Mermaid CLI."""

from __future__ import annotations

import json
import re
import subprocess
import sys
import tempfile
from pathlib import Path


ROOT = Path(__file__).resolve().parents[1]
MERMAID_BLOCK = re.compile(r"```mermaid\s*\n(.*?)```", re.DOTALL)


def markdown_files() -> list[Path]:
    return sorted(
        path
        for path in ROOT.rglob("*.md")
        if ".git" not in path.parts and "node_modules" not in path.parts
    )


def main() -> int:
    cli = ROOT / "node_modules" / ".bin" / "mmdc"
    if not cli.exists():
        print("Mermaid CLI is missing; run npm ci --ignore-scripts.", file=sys.stderr)
        return 2

    failures: list[str] = []
    count = 0
    with tempfile.TemporaryDirectory(prefix="agentmage-mermaid-") as temp_dir:
        temp = Path(temp_dir)
        browser_config = temp / "puppeteer.json"
        browser_config.write_text(
            json.dumps({"args": ["--no-sandbox", "--disable-setuid-sandbox"]}),
            encoding="utf-8",
        )

        for path in markdown_files():
            source = path.read_text(encoding="utf-8")
            for block_number, block in enumerate(MERMAID_BLOCK.findall(source), start=1):
                count += 1
                input_path = temp / f"diagram-{count}.mmd"
                output_path = temp / f"diagram-{count}.svg"
                input_path.write_text(block.strip() + "\n", encoding="utf-8")
                result = subprocess.run(
                    [
                        str(cli),
                        "--input",
                        str(input_path),
                        "--output",
                        str(output_path),
                        "--puppeteerConfigFile",
                        str(browser_config),
                        "--quiet",
                    ],
                    cwd=ROOT,
                    capture_output=True,
                    text=True,
                    check=False,
                )
                if result.returncode != 0:
                    relative = path.relative_to(ROOT)
                    detail = (result.stderr or result.stdout).strip()
                    failures.append(f"{relative}: Mermaid block {block_number}: {detail}")

    if failures:
        print("\n".join(failures), file=sys.stderr)
        return 1

    print(f"Validated {count} Mermaid block(s).")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
