"""Synthetic historical-worker resource checks, never held-out assertions."""
from __future__ import annotations

import os
from pathlib import Path
import resource
import subprocess
import tempfile
import unittest
from unittest import mock

from scripts import coding_eval as evaluation
from scripts import coding_eval_control as control
from scripts import coding_eval_scratch as scratch


class CodingControlTests(unittest.TestCase):
    def run_driver(self, driver, *, rust=False, timeout=10):
        with tempfile.TemporaryDirectory() as temporary:
            output = Path(temporary)
            runtime = evaluation.coding_eval_rust.runtime_paths(evaluation) if rust else None
            with scratch.scratch_mount(output, runtime_paths=runtime) as mount:
                root = mount.root
                for name in ("workspace", "grader", "results"):
                    (root / name).mkdir(mode=0o700)
                evaluation.write_file(root / "workspace/scripts/fixture.py", b"original = True\n")
                evaluation.write_file(root / "grader/run.py", control.limited_driver(driver, rust))
                result = control.run(mount, rust, timeout)
                self.assertTrue(mount.used)
            return result, (output / "stdout.log").read_bytes(), (output / "stderr.log").read_bytes()

    def test_python_worker_has_hard_limits_private_mounts_and_no_host_metadata(self):
        before = resource.getrlimit(resource.RLIMIT_NPROC)
        driver = b'''
import os
import resource
from pathlib import Path
assert resource.getrlimit(resource.RLIMIT_NPROC) == (8, 8)
assert resource.getrlimit(resource.RLIMIT_AS) == (512 * 1024 * 1024,) * 2
assert resource.getrlimit(resource.RLIMIT_CPU) == (60, 60)
info = os.statvfs('/tmp')
assert info.f_blocks * info.f_frsize == 64 * 1024 * 1024
assert info.f_files == 8192
assert Path('/proc/1/environ').read_bytes() == b''
for line in Path('/proc/self/status').read_text().splitlines():
 if line.startswith(('CapInh:', 'CapPrm:', 'CapEff:', 'CapBnd:', 'CapAmb:')):
  assert int(line.split()[1], 16) == 0, line
for directory in ('/', '/dev', '/dev/shm', '/grader', '/workspace'):
 try: Path(directory, 'write-probe').write_bytes(b'forbidden')
 except OSError: pass
 else: raise AssertionError('writable mount: ' + directory)
for name in ('cmdline', 'environ', 'mountinfo'):
 print(name, Path('/proc/1', name).read_bytes())
 print(name, Path('/proc/self', name).read_bytes())
assert not Path('/control').exists()
assert not Path('/runtime').exists()
assert not Path('/logs').exists()
assert not Path('/scratch').exists()
print('private-worker-verified')
'''
        result, stdout, stderr = self.run_driver(driver)
        self.assertEqual(result, (0, False, False), stderr)
        self.assertIn(b"private-worker-verified", stdout)
        for forbidden in (os.fsencode(evaluation.ROOT), os.fsencode(Path.home()),
                          b"agentmage-scratch-", b"/proc/self/fd/", b"/lane/"):
            self.assertNotIn(forbidden, stdout)
        self.assertEqual(resource.getrlimit(resource.RLIMIT_NPROC), before)

    def test_python_multifile_scratch_and_inode_exhaustion_are_refused(self):
        result, stdout, stderr = self.run_driver(b'''
import errno
from pathlib import Path
payload = b'x' * (1024 * 1024)
count = 0
try:
 for index in range(128):
  Path('/tmp', str(index)).write_bytes(payload)
  count += 1
except OSError as error: assert error.errno == errno.ENOSPC
else: raise AssertionError('missing byte quota')
assert 0 < count <= 64
for path in Path('/tmp').iterdir(): path.unlink()
count = 0
try:
 for index in range(8193):
  Path('/tmp', str(index)).touch()
  count += 1
except OSError as error: assert error.errno == errno.ENOSPC
else: raise AssertionError('missing inode quota')
assert 0 < count < 8192
print('both-quotas-refused')
''')
        self.assertEqual(result, (0, False, False), stderr)
        self.assertIn(b"both-quotas-refused", stdout)

    def test_process_quota_refuses_forks_and_cannot_be_raised(self):
        result, stdout, stderr = self.run_driver(b'''
import errno
import os
import resource
import signal
import time
try: resource.setrlimit(resource.RLIMIT_NPROC, (32, 32))
except (ValueError, OSError): pass
else: raise AssertionError('raised hard quota')
children = []
try:
 for index in range(32):
  try: child = os.fork()
  except OSError as error:
   assert error.errno == errno.EAGAIN
   break
  if child == 0:
   time.sleep(20)
   os._exit(0)
  children.append(child)
 else: raise AssertionError('missing process quota')
 assert 0 < len(children) < 8
 print('process-quota-refused')
finally:
 for child in children: os.kill(child, signal.SIGKILL)
 for child in children: os.waitpid(child, 0)
''')
        self.assertEqual(result, (0, False, False), stderr)
        self.assertIn(b"process-quota-refused", stdout)

    def test_timeout_and_output_exhaustion_are_failures_with_bounded_logs(self):
        result, stdout, stderr = self.run_driver(b"import time\ntime.sleep(20)\n", timeout=0.1)
        self.assertEqual(result, (None, True, False))
        result, stdout, stderr = self.run_driver(b"import os\nos.write(1, b'x' * (3 * 1024 * 1024))\n")
        self.assertTrue(result[2])
        self.assertEqual(len(stdout), 2 * evaluation.MAX_FILE)

    def test_legacy_tree_is_discarded_without_host_traversal_even_when_locked(self):
        processes = []
        real = subprocess.Popen
        with tempfile.TemporaryDirectory() as temporary:
            output = Path(temporary)
            with mock.patch.object(scratch.subprocess, "Popen", side_effect=lambda *a, **k: (
                    processes.append(real(*a, **k)) or processes[-1])):
                with self.assertRaises(KeyboardInterrupt):
                    with scratch.scratch_mount(output) as mount:
                        root = mount.root
                        for name in ("workspace", "grader", "results"):
                            (root / name).mkdir(mode=0o700)
                        evaluation.write_file(root / "workspace/scripts/example.py", b"pass\n")
                        evaluation.write_file(root / "grader/run.py", control.limited_driver(b'''
import os
from pathlib import Path
fd = os.open('/tmp', os.O_RDONLY | os.O_DIRECTORY)
for index in range(1200):
 os.mkdir('nested', dir_fd=fd)
 child = os.open('nested', os.O_RDONLY | os.O_DIRECTORY, dir_fd=fd)
 os.fchmod(fd, 0)
 os.close(fd)
 fd = child
os.fchmod(fd, 0)
os.close(fd)
print('locked-tree-ready')
''', False))
                        with mock.patch.object(evaluation.shutil, "rmtree", side_effect=AssertionError("host traversal")), \
                                mock.patch.object(control.oracle, "remove_fixture_tree", side_effect=AssertionError("host traversal")):
                            self.assertEqual(control.run(mount, False, 10), (0, False, False))
                            self.assertIn(b"locked-tree-ready", (output / "stdout.log").read_bytes())
                            raise KeyboardInterrupt
            self.assertEqual(len(processes), 1)
            self.assertIsNotNone(processes[0].returncode)
            with self.assertRaises(ProcessLookupError):
                os.kill(processes[0].pid, 0)

    def test_rust_worker_has_its_own_bounded_tmp_and_cannot_write_runtime(self):
        result, stdout, stderr = self.run_driver(b'''
import os
import resource
from pathlib import Path
assert resource.getrlimit(resource.RLIMIT_NPROC) == (16, 16)
assert resource.getrlimit(resource.RLIMIT_AS) == (768 * 1024 * 1024,) * 2
assert len(os.sched_getaffinity(0)) <= 4
info = os.statvfs('/tmp')
assert info.f_blocks * info.f_frsize == 768 * 1024 * 1024
assert info.f_files == 32768
assert os.statvfs('/cargo').f_fsid == info.f_fsid
for directory in ('/', '/dev', '/dev/shm', '/cargo/registry', '/toolchain'):
 try: Path(directory, 'write-probe').touch()
 except OSError: pass
 else: raise AssertionError('writable mount: ' + directory)
print('rust-private-worker-verified')
print(Path('/proc/self/mountinfo').read_text())
print(Path('/proc/1/cmdline').read_bytes())
''', rust=True)
        self.assertEqual(result, (0, False, False), stderr)
        self.assertIn(b"rust-private-worker-verified", stdout)
        for forbidden in (os.fsencode(evaluation.ROOT), os.fsencode(Path.home()), b"agentmage-scratch-", b"/lane/"):
            self.assertNotIn(forbidden, stdout)

    def test_invalid_requests_refuse_before_mounting_or_sending(self):
        invalid = ({}, {"mode": "historical-control", "rust": 1, "timeout": 1},
                   {"mode": "historical-control", "rust": False, "timeout": True},
                   {"mode": "historical-control", "rust": False, "timeout": float("nan")},
                   {"mode": "historical-control", "rust": False, "timeout": 91},
                   {"mode": "historical-control", "rust": False, "timeout": 1, "command": []})
        for request in invalid:
            with self.subTest(request=request), self.assertRaisesRegex(scratch.ScratchError, "control.request-refused"):
                control.run_request(request)
        connection = mock.Mock()
        mount = scratch.ScratchMount(7, connection)
        for timeout in (0, -1, True, 91, float("nan")):
            with self.subTest(timeout=timeout), self.assertRaises(scratch.ScratchError):
                control.run(mount, False, timeout)
        connection.send.assert_not_called()

    def test_forged_outcomes_and_reused_holder_are_refused(self):
        for payload in (b"", b"{}", b"[0, false]", b"[true, false, false]", b"[0, 0, false]"):
            connection = mock.Mock()
            connection.recvmsg.return_value = payload, [], 0, None
            mount = scratch.ScratchMount(7, connection)
            with self.subTest(payload=payload), self.assertRaisesRegex(scratch.ScratchError, "control.outcome-refused"):
                control.run(mount, False, 1)
            self.assertTrue(mount.used)
            with self.assertRaisesRegex(scratch.ScratchError, "control.request-refused"):
                control.run(mount, False, 1)

    def test_generated_candidate_refusal_precedes_supervisor_and_output(self):
        task = evaluation.collection()[0]
        files = evaluation.projection(task)
        files[task["source"]] += b"\n# generated probe\n"
        with tempfile.TemporaryDirectory() as temporary:
            output = Path(temporary) / "refused"
            with mock.patch.object(scratch, "scratch_mount") as holder, \
                    self.assertRaisesRegex(evaluation.EvaluationError, "untrusted-candidate-disabled"):
                evaluation.grade(task, files, output)
            holder.assert_not_called()
            self.assertFalse(output.exists())

    def test_mount_failure_is_infrastructure_with_no_counts_or_private_diagnostic(self):
        task = next(task for task in evaluation.collection() if task["id"] == "dev-005")
        files = evaluation.projection(task)
        with tempfile.TemporaryDirectory() as temporary:
            output = Path(temporary) / "failed"
            with mock.patch.object(scratch, "scratch_mount", side_effect=scratch.ScratchError("scratch.setup-or-io-refused")):
                report = evaluation.grade(task, files, output)
            self.assertFalse(report["passed"])
            self.assertEqual(report["cause"], "infrastructure")
            self.assertEqual(report["infrastructure_error"], "code=scratch.setup-or-io-refused type=EvaluationError")
            self.assertIsNone(report["tests"])
            self.assertFalse((output / "tests.json").exists())
            self.assertIsNone(report["model_score"])
            self.assertEqual(report["declared_worker_policy"], control.policy(False))

    def test_interrupt_during_control_keeps_no_counts_and_reaps_mount_holder(self):
        task = next(task for task in evaluation.collection() if task["id"] == "dev-005")
        files = evaluation.projection(task)
        processes = []
        real = subprocess.Popen

        def launch(*arguments, **options):
            process = real(*arguments, **options)
            if arguments[0][0] == "/usr/bin/bwrap":
                processes.append(process)
            return process

        with tempfile.TemporaryDirectory() as temporary:
            output = Path(temporary) / "interrupted"
            with mock.patch.object(scratch.subprocess, "Popen", side_effect=launch), \
                    mock.patch.object(control, "run", side_effect=KeyboardInterrupt), \
                    self.assertRaises(KeyboardInterrupt):
                evaluation.grade(task, files, output)
            self.assertFalse((output / "tests.json").exists())
            self.assertFalse((output / "report.json").exists())
            self.assertEqual(len(processes), 1)
            self.assertIsNotNone(processes[0].returncode)
            with self.assertRaises(ProcessLookupError):
                os.kill(processes[0].pid, 0)


if __name__ == "__main__":
    unittest.main()
