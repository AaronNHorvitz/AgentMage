"""Synthetic resource/lifecycle probes, never frozen held-out assertions."""
from __future__ import annotations

import json
import os
from pathlib import Path
import resource
import signal
import subprocess
import tempfile
import time
import unittest
from unittest import mock

from scripts import coding_eval as evaluation
from scripts import coding_eval_oracle as oracle


class CodingBoundaryTests(unittest.TestCase):
    def setUp(self):
        self.temporary = tempfile.TemporaryDirectory(prefix="agentmage-boundary-test-")
        self.addCleanup(self.temporary.cleanup)
        self.root = Path(self.temporary.name)

    def test_cleanup_repairs_locked_tree_without_following_external_alias(self):
        outside = self.root / "outside"
        outside.mkdir()
        outside.chmod(0o750)
        payload = outside / "payload"
        payload.write_text("untouched")
        fixture = self.root / "case"
        child = fixture / "locked"
        child.mkdir(parents=True)
        (child / "alias").symlink_to(outside, target_is_directory=True)
        child.chmod(0)
        oracle.bounded_cleanup(fixture)
        self.assertFalse(fixture.exists())
        self.assertEqual(payload.read_text(), "untouched")
        self.assertEqual(outside.stat().st_mode & 0o777, 0o750)

    def test_cleanup_has_private_limits_and_closes_inherited_reservation(self):
        observed = self.root / "observed.json"
        descriptor = os.open(self.root / "reservation", os.O_CREAT | os.O_RDWR, 0o600)
        self.addCleanup(os.close, descriptor)

        def inspect(root):
            try:
                os.fstat(descriptor)
            except OSError:
                closed = True
            else:
                closed = False
            observed.write_text(json.dumps({
                "closed": closed, "as": resource.getrlimit(resource.RLIMIT_AS),
                "cpu": resource.getrlimit(resource.RLIMIT_CPU),
                "fds": resource.getrlimit(resource.RLIMIT_NOFILE),
                "core": resource.getrlimit(resource.RLIMIT_CORE),
                "own_group": os.getpid() == os.getpgrp(),
            }))

        with mock.patch.object(oracle, "repair_fixture_permissions", side_effect=inspect), \
                mock.patch.object(oracle, "remove_fixture_tree"):
            oracle.bounded_cleanup(self.root / "unused")
        value = json.loads(observed.read_text())
        self.assertTrue(value["closed"])
        self.assertTrue(value["own_group"])
        self.assertEqual(value["as"], [oracle.CLEANUP_ADDRESS_BYTES] * 2)
        self.assertEqual(value["cpu"], [oracle.CLEANUP_SECONDS] * 2)
        self.assertEqual(value["fds"], [64, 64])
        self.assertEqual(value["core"], [0, 0])
        os.fstat(descriptor)  # The parent still owns its reservation.

    def test_cleanup_timeout_kills_and_reaps_without_deleting_recovery_tree(self):
        fixture = self.root / "case"
        fixture.mkdir()
        identity = self.root / "child"

        def stuck(root):
            identity.write_text(str(os.getpid()))
            while True:
                time.sleep(10)

        started = time.monotonic()
        with mock.patch.object(oracle, "repair_fixture_permissions", side_effect=stuck):
            with self.assertRaisesRegex(evaluation.EvaluationError, "oracle.cleanup-timeout"):
                oracle.bounded_cleanup(fixture, timeout=0.2)
        self.assertLess(time.monotonic() - started, 2)
        self.assertTrue(fixture.is_dir())
        pid = int(identity.read_text())
        with self.assertRaises(ProcessLookupError):
            os.kill(pid, 0)
        with self.assertRaises(ChildProcessError):
            os.waitpid(pid, os.WNOHANG)

    def test_cleanup_error_has_fixed_diagnostic_and_no_recovery_tree_traversal(self):
        fixture = self.root / "case"
        fixture.mkdir()
        removed = self.root / "removal-ran"
        with mock.patch.object(oracle, "repair_fixture_permissions", side_effect=OSError("private-detail")), \
                mock.patch.object(oracle, "remove_fixture_tree", side_effect=lambda root: removed.touch()):
            with self.assertRaisesRegex(evaluation.EvaluationError, "^oracle.cleanup-failed$"):
                oracle.bounded_cleanup(fixture)
        self.assertFalse(removed.exists())
        self.assertTrue(fixture.is_dir())

    def test_interruption_kills_cleanup_and_preserves_original_interruption(self):
        identity = self.root / "child"

        def stuck(root):
            identity.write_text(str(os.getpid()))
            while True:
                time.sleep(10)

        def interrupted(number, frame):
            raise KeyboardInterrupt()

        old = signal.signal(signal.SIGALRM, interrupted)
        try:
            signal.setitimer(signal.ITIMER_REAL, 0.2)
            with mock.patch.object(oracle, "repair_fixture_permissions", side_effect=stuck):
                with self.assertRaises(KeyboardInterrupt):
                    oracle.bounded_cleanup(self.root / "unused")
        finally:
            signal.setitimer(signal.ITIMER_REAL, 0)
            signal.signal(signal.SIGALRM, old)
        with self.assertRaises(ProcessLookupError):
            os.kill(int(identity.read_text()), 0)

    def test_case_failure_keeps_staged_fragments_under_recorded_private_container(self):
        output = self.root / "output"
        output.mkdir()
        container = self.root / "container"
        container.mkdir()

        def failed(root):
            (root.parent / "detached-fragment").mkdir()
            raise evaluation.EvaluationError("oracle.cleanup-timeout")

        with mock.patch.object(oracle.tempfile, "mkdtemp", return_value=str(container)), \
                mock.patch.object(oracle, "bounded_cleanup", side_effect=failed), \
                mock.patch.object(evaluation.shutil, "rmtree") as fallback:
            with self.assertRaisesRegex(evaluation.EvaluationError, "oracle.cleanup-timeout"):
                with oracle.case_workspace(output):
                    pass
            fallback.assert_not_called()
        record = json.loads((output / "cleanup.json").read_text())
        self.assertFalse(record["removed"])
        self.assertEqual(record["private_workspace"], str(container))
        self.assertTrue((container / "detached-fragment").is_dir())
        self.assertFalse((output / "tests.json").exists())

    def test_case_preserves_interruption_even_if_cleanup_fails(self):
        output = self.root / "output"
        output.mkdir()
        container = self.root / "container"
        container.mkdir()
        with mock.patch.object(oracle.tempfile, "mkdtemp", return_value=str(container)), \
                mock.patch.object(oracle, "bounded_cleanup", side_effect=evaluation.EvaluationError("refused")):
            with self.assertRaises(KeyboardInterrupt):
                with oracle.case_workspace(output):
                    raise KeyboardInterrupt()
        self.assertEqual(json.loads((output / "cleanup.json").read_text())["private_workspace"], str(container))

    def test_recovery_write_failure_does_not_replace_original_interruption(self):
        output = self.root / "output"
        output.mkdir()
        container = self.root / "container"
        container.mkdir()
        with mock.patch.object(oracle.tempfile, "mkdtemp", return_value=str(container)), \
                mock.patch.object(oracle, "bounded_cleanup", side_effect=evaluation.EvaluationError("refused")), \
                mock.patch.object(evaluation, "publish", side_effect=OSError("private-detail")):
            with self.assertRaises(KeyboardInterrupt):
                with oracle.case_workspace(output):
                    raise KeyboardInterrupt()

    def test_private_tmpfs_rejects_aggregate_writes_across_multiple_files(self):
        adapter = self.root / "adapter"
        adapter.mkdir()
        (adapter / "run.py").write_text('''
import errno
from pathlib import Path
payload = b'x' * (1024 * 1024)
written = 0
try:
 for index in range(128):
  with open('/tmp/file-' + str(index), 'wb') as stream:
   written += stream.write(payload)
except OSError as error:
 assert error.errno == errno.ENOSPC, error
else:
 raise AssertionError('missing tmpfs byte limit')
assert 0 < written <= 64 * 1024 * 1024, written
assert Path('/workspace/run.py').is_file()
for directory in ('/', '/dev', '/dev/shm'):
 try:
  Path(directory, 'quota-bypass').write_bytes(b'forbidden')
 except OSError: pass
 else: raise AssertionError('writable scratch bypass: ' + directory)
''')
        for role in ("candidate", "oracle"):
            with self.subTest(role=role):
                process = oracle.launch(oracle.sandbox(adapter, adapter, self.root, role),
                                        stdout=subprocess.PIPE, stderr=subprocess.PIPE,
                                        start_new_session=True, preexec_fn=evaluation.limits)
                try:
                    stdout, stderr = process.communicate(timeout=10)
                finally:
                    if process.returncode is None:
                        oracle.terminate(process)
                self.assertEqual(process.returncode, 0, stderr.decode(errors="replace"))
                self.assertEqual(stdout, b"")
        self.assertFalse(any(self.root.glob("file-*")))


if __name__ == "__main__":
    unittest.main()
