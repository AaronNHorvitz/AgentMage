from __future__ import annotations

import hashlib
import json
import unittest
from pathlib import Path


ROOT = Path(__file__).resolve().parents[1]


class PublicPolicyBaselineTests(unittest.TestCase):
    def test_apache_2_license_is_complete_pinned_and_consistent(self) -> None:
        license_bytes = (ROOT / "LICENSE").read_bytes()
        license_text = license_bytes.decode("utf-8")
        package = json.loads((ROOT / "package.json").read_text(encoding="utf-8"))

        self.assertEqual(
            hashlib.sha256(license_bytes).hexdigest(),
            "02f41e321c6eabad29b0f412b9aaa710dfcff9df7fa563a8db0a408e35e5ba6f",
        )
        self.assertIn("Apache License", license_text)
        self.assertIn("Version 2.0, January 2004", license_text)
        for section in range(1, 10):
            self.assertIn(f"   {section}.", license_text)
        self.assertIn("END OF TERMS AND CONDITIONS", license_text)
        self.assertEqual(package["license"], "Apache-2.0")

        expected_references = {
            "README.md": "[Apache License 2.0](./LICENSE)",
            "PRD.md": "| **License** | Apache License 2.0 |",
            "IMPLEMENTATION-PLAN.md": "Apache-2.0 licensing",
            "Agent-Scaffolding-Inventory.md": "Apache-2.0 license",
            "TASKS.md": "Publish the Apache License 2.0",
            "docs/decisions/0001-product-security-and-runtime-baseline.md": (
                "distributed under the Apache License 2.0"
            ),
        }
        for relative, expected in expected_references.items():
            with self.subTest(document=relative):
                self.assertIn(
                    expected,
                    (ROOT / relative).read_text(encoding="utf-8"),
                )


if __name__ == "__main__":
    unittest.main()
