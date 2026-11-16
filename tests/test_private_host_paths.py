"""Prove range coverage and the byte-bound historical path exception."""

import hashlib
import json
from pathlib import Path
import subprocess
import tempfile
import unittest
from unittest.mock import patch

from scripts import private_host_paths as checker


class PrivatePathTests(unittest.TestCase):
    def setUp(self):
        self.temp = tempfile.TemporaryDirectory()
        self.addCleanup(self.temp.cleanup)
        self.root = Path(self.temp.name)
        self.git("init", "-q")
        self.username = b"fixture-owner"
        self.old = b"/home/" + self.username + b"/private\n"
        self.write("old.log", self.old)
        self.git("add", ".")
        self.commit()
        self.base = self.git("rev-parse", "HEAD").decode().strip()
        policy = {"schema_version": 1, "baseline_commit": self.base,
                  "username_sha256": hashlib.sha256(self.username).hexdigest(),
                  "preserved_files": {"old.log": hashlib.sha256(self.old).hexdigest()}}
        raw = (json.dumps(policy) + "\n").encode()
        self.write(checker.POLICY, raw)
        self.pin = patch.object(checker, "POLICY_SHA256", hashlib.sha256(raw).hexdigest())
        self.pin.start()
        self.addCleanup(self.pin.stop)

    def git(self, *args):
        return subprocess.check_output(["git", *args], cwd=self.root, stderr=subprocess.DEVNULL)

    def commit(self):
        self.git("-c", "user.name=Synthetic", "-c", "user.email=synthetic@example.invalid",
                 "commit", "-qm", "fixture")

    def write(self, name, data):
        path = self.root / name
        path.parent.mkdir(parents=True, exist_ok=True)
        path.write_bytes(data)

    def test_exact_historical_file_and_placeholder_paths_pass(self):
        self.write("safe.md", b"<home>/work and /home/agentmage/test\n")
        self.assertEqual(checker.check(self.root, self.base), [])

    def test_new_staged_committed_and_working_changes_are_checked(self):
        for phase in ("untracked", "staged", "committed", "working"):
            with self.subTest(phase=phase):
                self.write("new.md", b"/home/" + b"another-person/secret\n")
                if phase in {"staged", "committed"}:
                    self.git("add", "new.md")
                if phase == "committed":
                    self.commit()
                self.assertTrue(checker.check(self.root, self.base))

    def test_standalone_username_and_platform_home_paths_fail_without_echoing_value(self):
        for value in (self.username, b"/var/home/" + b"other-person/key", b"/Users/" + b"other-person/key",
                      b"C:" + b"\\Users\\" + b"other-person\\key"):
            with self.subTest(value=value):
                self.write("new.bin", b"\x00" + value)
                errors = checker.check(self.root, self.base)
                self.assertTrue(errors)
                self.assertNotIn(value.decode(), "\n".join(errors))

    def test_modified_deleted_or_renamed_historical_blob_fails(self):
        for content in (self.old + b"new\n", b"redacted\n"):
            self.write("old.log", content)
            self.assertTrue(checker.check(self.root, self.base))
        (self.root / "old.log").unlink()
        self.assertTrue(checker.check(self.root, self.base))
        self.write("renamed.log", self.old)
        self.assertGreaterEqual(len(checker.check(self.root, self.base)), 2)

    def test_exception_policy_tampering_fails(self):
        self.write(checker.POLICY, b"{}\n")
        with self.assertRaises(ValueError):
            checker.check(self.root, self.base)

    def test_staged_private_content_cannot_hide_behind_clean_working_file(self):
        self.write("safe.md", b"safe\n")
        self.git("add", "safe.md")
        self.commit()
        self.write("safe.md", self.username + b"\n")
        self.git("add", "safe.md")
        self.write("safe.md", b"safe\n")
        self.assertTrue(checker.check(self.root, self.base))

    def test_staged_exception_modification_cannot_hide_behind_original_bytes(self):
        self.write("old.log", b"redacted\n")
        self.git("add", "old.log")
        self.write("old.log", self.old)
        self.assertTrue(checker.check(self.root, self.base))

    def test_symlink_target_and_filename_are_checked(self):
        (self.root / "link").symlink_to("/home/" + "another-person/file")
        self.write(self.username.decode() + ".md", self.username + b"\n")
        errors = checker.check(self.root, self.base)
        self.assertTrue(errors)
        self.assertNotIn(self.username.decode(), "\n".join(errors))


if __name__ == "__main__":
    unittest.main()
