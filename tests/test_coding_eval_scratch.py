"""Synthetic scratch-quota and mount-privacy checks; no held-out imports."""
from array import array
import errno
import os
from pathlib import Path
import resource
import socket
import subprocess
import tempfile
import time
import unittest
from unittest import mock

from scripts import coding_eval as evaluation
from scripts import coding_eval_oracle as oracle
from scripts import coding_eval_scratch as scratch


class CodingScratchTests(unittest.TestCase):
    def test_received_mount_has_byte_inode_and_descriptor_limits(self):
        with scratch.scratch_mount() as mount:
            root = mount.root
            descriptor = int(root.name)
            filesystem = os.fstatvfs(descriptor)
            self.assertEqual(filesystem.f_blocks * filesystem.f_frsize, scratch.SCRATCH_BYTES)
            self.assertEqual(filesystem.f_files, scratch.SCRATCH_INODES)
            self.assertFalse(os.get_inheritable(descriptor))
            (root / "private").write_bytes(b"original")
            self.assertEqual((root / "private").read_bytes(), b"original")
        with self.assertRaises(OSError) as raised:
            os.fstat(descriptor)
        self.assertEqual(raised.exception.errno, errno.EBADF)

    def test_all_fixture_files_share_one_byte_quota(self):
        with scratch.scratch_mount() as mount:
            root = mount.root
            for name in ("first", "second"):
                (root / name).mkdir()
            payload = b"x" * (1024 * 1024)
            count = 0
            try:
                for index in range(256):
                    (root / ("first" if index % 2 else "second") / str(index)).write_bytes(payload)
                    count += 1
            except OSError as error:
                self.assertEqual(error.errno, errno.ENOSPC)
            else:
                self.fail("aggregate scratch byte quota did not refuse")
            self.assertGreater(count, 1)
            self.assertLessEqual(count * len(payload), scratch.SCRATCH_BYTES)
            self.assertEqual(os.fstatvfs(int(root.name)).f_bavail, 0)

    def test_empty_files_share_one_inode_quota(self):
        with scratch.scratch_mount() as mount:
            root = mount.root
            count = 0
            try:
                for index in range(scratch.SCRATCH_INODES + 1):
                    (root / str(index)).touch()
                    count += 1
            except OSError as error:
                self.assertEqual(error.errno, errno.ENOSPC)
            else:
                self.fail("scratch inode quota did not refuse")
            self.assertGreater(count, 1)
            self.assertLess(count, scratch.SCRATCH_INODES)
            self.assertEqual(os.fstatvfs(int(root.name)).f_favail, 0)

    def test_interruption_releases_locked_deep_tree_and_reaps_holder(self):
        processes = []
        real_launch = subprocess.Popen

        def launch(*args, **kwargs):
            process = real_launch(*args, **kwargs)
            processes.append(process)
            return process

        started = time.monotonic()
        with mock.patch.object(scratch.subprocess, "Popen", side_effect=launch):
            with self.assertRaises(KeyboardInterrupt):
                with scratch.scratch_mount() as mount:
                    root = mount.root
                    descriptor = int(root.name)
                    current = os.open(root, os.O_DIRECTORY)
                    try:
                        for _ in range(512):
                            os.mkdir("child", dir_fd=current)
                            child = os.open("child", os.O_DIRECTORY, dir_fd=current)
                            os.fchmod(current, 0)
                            os.close(current)
                            current = child
                    finally:
                        os.close(current)
                    raise KeyboardInterrupt
        self.assertLess(time.monotonic() - started, scratch.SETUP_SECONDS + 2)
        self.assertEqual(len(processes), 1)
        self.assertIsNotNone(processes[0].returncode)
        with self.assertRaises(ProcessLookupError):
            os.kill(processes[0].pid, 0)
        with self.assertRaises(ChildProcessError):
            os.waitpid(processes[0].pid, os.WNOHANG)
        with self.assertRaises(OSError):
            os.fstat(descriptor)

    def test_both_namespace_roles_hide_host_mounts_and_scratch_root_capability(self):
        script = b'''
from pathlib import Path
import json
import os
import sys
assert Path('/workspace/input').read_text() == 'fixture'
status = Path('/proc/self/status').read_text().splitlines()
for field in ('CapInh:', 'CapPrm:', 'CapEff:', 'CapBnd:', 'CapAmb:'):
 assert int(next(line.split()[1] for line in status if line.startswith(field)), 16) == 0, field
metadata = []
for process in Path('/proc').iterdir():
 if not process.name.isdigit(): continue
 for name in ('mountinfo', 'mounts'):
  try: metadata.append((process / name).read_text())
  except OSError: pass
 try: descriptors = list((process / 'fd').iterdir())
 except OSError: continue
 for descriptor in descriptors:
  try: assert not (descriptor / 'not-for-worker').exists(), descriptor
  except OSError: pass
print(json.dumps(metadata), file=sys.stderr)
'''
        with tempfile.TemporaryDirectory(prefix="agentmage-private-sentinel-") as host:
            output = Path(host) / "logs"
            output.mkdir()
            with scratch.scratch_mount(output) as mount:
                root = mount.root
                (root / "not-for-worker").write_text("hidden")
                for name in ("workspace", "adapter", "results"):
                    (root / name).mkdir()
                (root / "workspace/input").write_text("fixture")
                (root / "adapter/run.py").write_bytes(oracle.limited_source(script))
                commands = [oracle.sandbox(root / "workspace", root / "adapter", root / "results", role)
                            for role in ("candidate", "oracle")]
                result = mount.exchange(*commands, 10)
                self.assertEqual(result, (0, 0, False, False))
                with self.assertRaisesRegex(scratch.ScratchError, "^scratch.exchange-refused$"):
                    mount.exchange(*commands, 10)
                for name in ("candidate.log", "oracle.log"):
                    text = (output / name).read_bytes()
                    self.assertNotIn(os.fsencode(evaluation.ROOT), text)
                    self.assertNotIn(os.fsencode(Path.home()), text)
                    self.assertNotIn(os.fsencode(host), text)
                    self.assertNotIn(b"agentmage-scratch-", text)
                    self.assertNotIn(b"/proc/self/fd/", text)
                    self.assertIn(b" /workspace ", text)

    def test_setup_failure_refuses_without_host_disk_fallback(self):
        with mock.patch.object(scratch.subprocess, "Popen", side_effect=OSError("private detail")):
            with self.assertRaisesRegex(scratch.ScratchError, "^scratch.setup-or-io-refused$"):
                with scratch.scratch_mount():
                    self.fail("failed setup yielded scratch")

    def test_candidate_shared_fixture_uses_the_mount_quota(self):
        script = b'''
import errno
import sys
from pathlib import Path
if sys.argv[1] == 'candidate':
 payload = b'x' * (1024 * 1024)
 count = 0
 try:
  for index in range(256):
   Path('/tmp', str(index)).write_bytes(payload)
   count += 1
 except OSError as error:
  assert error.errno == errno.ENOSPC, error
 else: raise AssertionError('missing shared scratch quota')
 assert 1 < count <= 128, count
 print('shared-fixture-quota-refused', file=sys.stderr)
'''
        with tempfile.TemporaryDirectory() as directory:
            output = Path(directory)
            with scratch.scratch_mount(output) as mount:
                root = mount.root
                for name in ("workspace", "adapter", "results", "shared"):
                    (root / name).mkdir()
                (root / "adapter/run.py").write_bytes(oracle.limited_source(script))
                commands = [oracle.sandbox(root / "workspace", root / "adapter", root / "results", role,
                                           candidate_tmp=root / "shared") for role in ("candidate", "oracle")]
                self.assertEqual(mount.exchange(*commands, 10), (0, 0, False, False))
                self.assertIn(b"shared-fixture-quota-refused", (output / "candidate.log").read_bytes())
                self.assertEqual(os.fstatvfs(int(root.name)).f_bavail, 0)

    def test_namespace_process_limit_bounds_forks_without_changing_host_limits(self):
        script = b'''
import errno
import os
import resource
import signal
import sys
import time
from pathlib import Path
assert int(next(line.split()[1] for line in Path('/proc/self/status').read_text().splitlines()
                if line.startswith('CapEff:')), 16) == 0
assert resource.getrlimit(resource.RLIMIT_NPROC) == (8, 8)
assert resource.getrlimit(resource.RLIMIT_AS) == (512 * 1024 * 1024,) * 2
assert resource.getrlimit(resource.RLIMIT_CPU) == (60, 60)
try:
 resource.setrlimit(resource.RLIMIT_NPROC, (64, 64))
except (ValueError, OSError): pass
else: raise AssertionError('process quota raised')
resource.setrlimit(resource.RLIMIT_NPROC, (8, 8))
children = []
refused = False
try:
 for _ in range(32):
  try: child = os.fork()
  except OSError as error:
   assert error.errno == errno.EAGAIN, error
   refused = True
   break
  if child == 0:
   time.sleep(20)
   os._exit(0)
  children.append(child)
 assert refused and 0 < len(children) <= 7, len(children)
 print('namespace-process-quota-refused', len(children), file=sys.stderr)
finally:
 for child in children:
  os.kill(child, signal.SIGKILL)
 for child in children:
  os.waitpid(child, 0)
'''
        before = resource.getrlimit(resource.RLIMIT_NPROC)
        with tempfile.TemporaryDirectory() as directory:
            output = Path(directory)
            with scratch.scratch_mount(output) as mount:
                root = mount.root
                for name in ("workspace", "adapter", "results"):
                    (root / name).mkdir()
                (root / "adapter/run.py").write_bytes(oracle.limited_source(script))
                commands = [oracle.sandbox(root / "workspace", root / "adapter", root / "results", role)
                            for role in ("candidate", "oracle")]
                self.assertEqual(mount.exchange(*commands, 10), (0, 0, False, False))
                for name in ("candidate.log", "oracle.log"):
                    self.assertIn(b"namespace-process-quota-refused", (output / name).read_bytes())
        self.assertEqual(resource.getrlimit(resource.RLIMIT_NPROC), before)

    def test_case_private_tmp_mounts_have_separate_byte_and_inode_quotas(self):
        script = b'''
import os
import sys
from pathlib import Path
info = os.statvfs('/tmp')
assert info.f_blocks * info.f_frsize == 64 * 1024 * 1024
assert info.f_files == 8192
role = sys.argv[1]
Path('/tmp', role).write_text('private')
print('ready', flush=True)
assert sys.stdin.readline() == 'ready\\n'
other = 'candidate' if role == 'oracle' else 'oracle'
assert not Path('/tmp', other).exists()
print('private-tmp-quotas-observed', file=sys.stderr)
'''
        with tempfile.TemporaryDirectory() as directory:
            output = Path(directory)
            with scratch.scratch_mount(output) as mount:
                root = mount.root
                for name in ("workspace", "adapter", "results"):
                    (root / name).mkdir()
                (root / "adapter/run.py").write_bytes(oracle.limited_source(script))
                commands = [oracle.sandbox(root / "workspace", root / "adapter", root / "results", role,
                                           private_tmp=root / ("private-" + role))
                            for role in ("candidate", "oracle")]
                self.assertEqual(mount.exchange(*commands, 10), (0, 0, False, False))
                for name in ("candidate.log", "oracle.log"):
                    self.assertIn(b"private-tmp-quotas-observed", (output / name).read_bytes())

    def test_wrong_message_and_extra_descriptors_close_received_capabilities(self):
        for token, count in ((b"wrong", 1), (scratch.TOKEN, 2)):
            with self.subTest(token=token, count=count), \
                    tempfile.TemporaryFile() as stream:
                # Use separate endpoints with deterministic closure of both.
                sender, receiver = socket.socketpair(socket.AF_UNIX, socket.SOCK_SEQPACKET)
                with sender, receiver:
                    before = set(os.listdir("/proc/self/fd"))
                    sender.sendmsg([token], [(socket.SOL_SOCKET, socket.SCM_RIGHTS,
                                             array("i", [stream.fileno()] * count))])
                    with self.assertRaises(scratch.ScratchError):
                        scratch.receive_root(receiver)
                    self.assertEqual(set(os.listdir("/proc/self/fd")), before)
                    os.fstat(stream.fileno())

    def test_non_directory_capability_is_closed_on_refusal(self):
        with tempfile.TemporaryFile() as stream:
            sender, receiver = socket.socketpair(socket.AF_UNIX, socket.SOCK_SEQPACKET)
            with sender, receiver:
                before = set(os.listdir("/proc/self/fd"))
                sender.sendmsg([scratch.TOKEN], [(socket.SOL_SOCKET, socket.SCM_RIGHTS,
                                                array("i", [stream.fileno()]))])
                with self.assertRaisesRegex(scratch.ScratchError, "^scratch.filesystem-refused$"):
                    scratch.receive_root(receiver)
                self.assertEqual(set(os.listdir("/proc/self/fd")), before)

    def test_refused_exchange_does_not_send_a_request(self):
        control = mock.Mock()
        mount = scratch.ScratchMount(7, control)
        for timeout in (0, -1, True, 91, float("nan")):
            with self.subTest(timeout=timeout), self.assertRaises(scratch.ScratchError):
                mount.exchange([], [], timeout)
        control.send.assert_not_called()

    def test_control_cleanup_refuses_unexpected_entries_without_traversing_them(self):
        with tempfile.TemporaryDirectory() as parent:
            root = Path(parent) / "control"
            root.mkdir()
            with mock.patch.object(scratch.tempfile, "mkdtemp", return_value=str(root)):
                with self.assertRaises(OSError):
                    with scratch.control_directory():
                        (root / "unexpected").mkdir()
                        (root / "unexpected/payload").write_text("retained")
            self.assertEqual((root / "unexpected/payload").read_text(), "retained")

    def test_control_cleanup_does_not_mask_original_interruption(self):
        with tempfile.TemporaryDirectory() as parent:
            root = Path(parent) / "control"
            root.mkdir()
            with mock.patch.object(scratch.tempfile, "mkdtemp", return_value=str(root)):
                with self.assertRaises(KeyboardInterrupt):
                    with scratch.control_directory():
                        (root / "unexpected").touch()
                        raise KeyboardInterrupt
            self.assertTrue((root / "unexpected").is_file())

    def test_generated_candidate_is_refused_before_new_mount_holder(self):
        task = next(task for task in evaluation.collection() if task["id"] == "dev-015")
        files = evaluation.projection(task)
        files[task["source"]] += b"\n# generated admission probe\n"
        with tempfile.TemporaryDirectory() as temporary:
            output = Path(temporary) / "refused"
            with mock.patch.object(scratch, "scratch_mount") as holder:
                with self.assertRaisesRegex(evaluation.EvaluationError, "^evaluation\\.untrusted-candidate-disabled$"):
                    oracle.grade(task, files, output)
                holder.assert_not_called()
            self.assertFalse(output.exists())

    def test_long_operator_temporary_path_uses_a_pinned_socket_parent(self):
        with tempfile.TemporaryDirectory() as temporary:
            parent = Path(temporary) / ("long-" + "x" * 80)
            parent.mkdir()
            original = tempfile.mkdtemp
            paths = []

            def create(**options):
                directory = original(**options, dir=parent)
                paths.append(Path(directory))
                return directory

            with mock.patch.object(scratch.tempfile, "mkdtemp", side_effect=create):
                with scratch.scratch_mount() as mount:
                    self.assertGreater(len(os.fsencode(paths[0] / "control")), 108)
                    (mount.root / "probe").write_text("reachable")
                    self.assertEqual((mount.root / "probe").read_text(), "reachable")
            self.assertEqual(list(parent.iterdir()), [])



if __name__ == "__main__":
    unittest.main()
