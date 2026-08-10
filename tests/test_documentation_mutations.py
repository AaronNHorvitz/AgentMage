from __future__ import annotations

import io
import tempfile
import unittest
from contextlib import redirect_stderr, redirect_stdout
from pathlib import Path
from unittest.mock import patch

from scripts.check_mermaid import main as check_mermaid
from scripts.validate_docs import (
    ROOT,
    check_claims,
    check_identifiers,
    check_links,
    check_sensitive,
    main as validate_docs,
)


class DocumentationMutationTests(unittest.TestCase):
    def test_unmodified_repository_passes(self) -> None:
        with redirect_stdout(io.StringIO()), redirect_stderr(io.StringIO()):
            self.assertEqual(validate_docs(), 0)
            self.assertEqual(check_mermaid(), 0)

    def test_broken_local_link_blocks_independently(self) -> None:
        with tempfile.TemporaryDirectory() as temp_dir:
            root = Path(temp_dir)
            document = root / "broken-link.md"
            document.write_text("[missing](does-not-exist.md)\n", encoding="utf-8")
            failures: list[str] = []

            with patch("scripts.validate_docs.ROOT", root):
                check_links([document], failures)

        self.assertEqual(len(failures), 1)
        self.assertIn("broken-link.md: broken local link", failures[0])

    def test_malformed_mermaid_blocks_independently(self) -> None:
        with tempfile.TemporaryDirectory(dir=ROOT) as temp_dir:
            document = Path(temp_dir) / "malformed-mermaid.md"
            document.write_text(
                "# Fixture\n\n```mermaid\nthis is not a diagram\n```\n",
                encoding="utf-8",
            )
            stderr = io.StringIO()

            with patch("scripts.check_mermaid.markdown_files", return_value=[document]):
                with redirect_stdout(io.StringIO()), redirect_stderr(stderr):
                    result = check_mermaid()

        self.assertEqual(result, 1)
        self.assertIn("Mermaid block 1", stderr.getvalue())

    def test_unresolved_stable_identifier_blocks_independently(self) -> None:
        with tempfile.TemporaryDirectory() as temp_dir:
            document = Path(temp_dir) / "unresolved-id.md"
            document.write_text("Requirement AM-UNKNOWN-999 applies.\n", encoding="utf-8")
            failures: list[str] = []

            check_identifiers([document], failures)

        self.assertIn("unresolved stable identifier: AM-UNKNOWN-999", failures)

    def test_prohibited_deployment_claim_blocks_independently(self) -> None:
        with tempfile.TemporaryDirectory(dir=ROOT) as temp_dir:
            document = Path(temp_dir) / "prohibited-claim.md"
            document.write_text("This product is approved for federal use.\n", encoding="utf-8")
            failures: list[str] = []

            check_claims([document], failures)

        self.assertEqual(len(failures), 1)
        self.assertIn("prohibited deployment-specific claim", failures[0])

    def test_each_synthetic_secret_signature_blocks_without_disclosure(self) -> None:
        synthetic_values = {
            "private-key marker": "-----BEGIN " + "PRIVATE KEY-----",
            "AWS access key": "AK" + "IA" + ("A" * 16),
            "GitHub token": "gh" + "p_" + ("a" * 24),
            "Slack token": "xo" + "xb-" + ("a" * 12),
            "OpenAI-style token": "s" + "k-" + ("a" * 24),
        }

        for label, synthetic_value in synthetic_values.items():
            with self.subTest(signature=label), tempfile.TemporaryDirectory() as temp_dir:
                root = Path(temp_dir)
                document = root / "synthetic-secret.txt"
                document.write_text(f"value={synthetic_value}\n", encoding="utf-8")
                failures: list[str] = []

                with patch("scripts.validate_docs.ROOT", root):
                    check_sensitive([document], failures)

                self.assertEqual(len(failures), 1)
                self.assertIn(f"possible {label}", failures[0])
                self.assertNotIn(synthetic_value, failures[0])


if __name__ == "__main__":
    unittest.main()
