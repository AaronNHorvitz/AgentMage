from __future__ import annotations

import json
import os
import subprocess
import tempfile
import unittest
from pathlib import Path
from unittest import mock

from scripts.evidence_core import (
    EvidenceError,
    atomic_write,
    bounded_read,
    canonical_json_bytes,
    git_blob,
    git_source_identity,
    sha256_bytes,
)


class EvidenceCoreTests(unittest.TestCase):
    def test_canonical_json_is_deterministic_ascii_and_newline_terminated(self) -> None:
        first = canonical_json_bytes({"z": "caf\u00e9", "a": [2, 1]})
        second = canonical_json_bytes({"a": [2, 1], "z": "caf\u00e9"})
        self.assertEqual(first, second)
        self.assertTrue(first.endswith(b"\n"))
        first.decode("ascii")

    def test_bounded_read_rejects_oversize_symlink_and_escape(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            (root / "value.txt").write_bytes(b"1234")
            (root / "link.txt").symlink_to("value.txt")
            self.assertEqual(bounded_read(root, "value.txt", maximum_bytes=4), b"1234")
            with self.assertRaisesRegex(EvidenceError, "size_exceeded"):
                bounded_read(root, "value.txt", maximum_bytes=3)
            with self.assertRaisesRegex(EvidenceError, "unavailable"):
                bounded_read(root, "link.txt")
            with self.assertRaisesRegex(EvidenceError, "invalid"):
                bounded_read(root, "../value.txt")

    def test_atomic_write_replaces_regular_file_and_cleans_failed_temporary(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            path = root / "report.json"
            atomic_write(path, b"first\n")
            atomic_write(path, b"second\n")
            self.assertEqual(path.read_bytes(), b"second\n")
            with mock.patch("scripts.evidence_core.os.replace", side_effect=OSError("fixture")):
                with self.assertRaises(OSError):
                    atomic_write(path, b"third\n")
            self.assertEqual(path.read_bytes(), b"second\n")
            self.assertEqual(list(root.glob(".agentmage-evidence-*")), [])

    def test_atomic_write_refuses_to_replace_a_symbolic_link(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            target = root / "target.txt"
            target.write_text("retained", encoding="utf-8")
            link = root / "report.json"
            link.symlink_to(target)
            with self.assertRaisesRegex(EvidenceError, "symbolic_link"):
                atomic_write(link, b"changed\n")
            self.assertEqual(target.read_text(encoding="utf-8"), "retained")

    def test_git_identity_and_blob_replay_ignore_later_worktree_change(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            subprocess.run(["git", "init", "-q"], cwd=root, check=True)
            subprocess.run(["git", "config", "user.name", "Fixture"], cwd=root, check=True)
            subprocess.run(
                ["git", "config", "user.email", "fixture@example.invalid"],
                cwd=root,
                check=True,
            )
            (root / "input.txt").write_text("historical\n", encoding="utf-8")
            subprocess.run(["git", "add", "input.txt"], cwd=root, check=True)
            subprocess.run(["git", "commit", "-qm", "fixture"], cwd=root, check=True)
            identity = git_source_identity(root, "HEAD")
            original = git_blob(root, identity["revision"], "input.txt")
            (root / "input.txt").write_text("current\n", encoding="utf-8")
            self.assertEqual(original, b"historical\n")
            self.assertEqual(sha256_bytes(original), sha256_bytes(b"historical\n"))
            self.assertNotEqual(original, (root / "input.txt").read_bytes())


if __name__ == "__main__":
    unittest.main()
