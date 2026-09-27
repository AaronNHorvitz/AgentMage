"""Developer-wrapper components, including one owned Linux child; no CLI qualification."""
from contextlib import ExitStack, contextmanager
import copy
import fcntl
import io
import json
import os
from pathlib import Path
import select
import signal
import subprocess
import sys
import tempfile
import unittest
from unittest import mock

from scripts import coding_harness as harness


IDENTITY = {"start_ticks": 100, "uid": os.getuid(),
            "boot_id": "12345678-1234-1234-1234-123456789abc"}


class HarnessOwnershipTests(unittest.TestCase):
    def setUp(self):
        self.temporary = tempfile.TemporaryDirectory()
        self.addCleanup(self.temporary.cleanup)
        self.base = Path(self.temporary.name) / "fixture"
        self.state = self.base / "state"
        self.state.mkdir(parents=True, mode=0o700)
        self.path = self.state / harness.RUN_RECORD
        self.executable = Path(sys.executable).resolve()
        self.logs = Path(self.temporary.name) / "logs"

    def owner(self):
        return harness.RunRecordReservation(self.state, self.base, self.executable)

    @contextmanager
    def start_environment(self):
        with ExitStack() as stack:
            stack.enter_context(mock.patch.object(harness, "diagnose", return_value={
                "lifecycle": "ready", "git_clean": True, "transport_path": True,
            }))
            stack.enter_context(mock.patch.object(harness, "binary", return_value=self.executable))
            stack.enter_context(mock.patch.object(harness, "implementation_identity",
                                                  return_value={"binaries": []}))
            stack.enter_context(mock.patch.object(harness, "linux_process_identity", return_value=IDENTITY))
            stack.enter_context(mock.patch("sys.stdout", new=io.StringIO()))
            child = stack.enter_context(mock.patch.object(harness.subprocess, "Popen")).return_value
            child.pid = 12345
            child.poll.return_value = 0
            child.wait.return_value = 0
            yield child

    def start(self, **kwargs):
        return harness.start(self.base, "failed-test-repair", "synthetic objective",
                             False, False, self.logs, **kwargs)

    def test_competing_record_after_preflight_survives_rejected_start(self):
        with self.start_environment() as child:
            owner = self.owner()
            before = self.path.read_bytes()
            self.logs.mkdir()
            try:
                with self.assertRaises(FileExistsError):
                    self.start()
                self.assertEqual(self.path.read_bytes(), before)
                child.wait.assert_not_called()
                harness.subprocess.Popen.assert_not_called()
            finally:
                owner.close()

    def test_prelaunch_log_failure_removes_only_own_reservation(self):
        self.logs.mkdir()
        with self.start_environment():
            with self.assertRaisesRegex(harness.HarnessError, "log-target-exists"):
                self.start()
            harness.subprocess.Popen.assert_not_called()
        self.assertFalse(self.path.exists())

    def test_exclusive_reservation_precedes_child_and_second_owner_is_refused(self):
        owner = self.owner()
        try:
            before = self.path.read_bytes()
            self.assertIsNone(harness.read_run_record(self.state)["pid"])
            with self.assertRaises(FileExistsError):
                self.owner()
            self.assertEqual(self.path.read_bytes(), before)
        finally:
            owner.close()
        self.assertFalse(self.path.exists())

    def test_launch_failure_cleans_reservation_and_closes_streams(self):
        with self.start_environment():
            harness.subprocess.Popen.side_effect = OSError("synthetic launch refusal")
            with self.assertRaisesRegex(OSError, "synthetic launch refusal"):
                self.start()
            self.assertTrue(harness.subprocess.Popen.call_args.kwargs["stdout"].closed)
            self.assertTrue(harness.subprocess.Popen.call_args.kwargs["stderr"].closed)
        self.assertFalse(self.path.exists())
        self.assertFalse((self.logs / "result.json").exists())

    def test_success_publishes_result_only_after_owned_cleanup(self):
        with self.start_environment() as child:
            def wait():
                record = harness.read_run_record(self.state)
                self.assertEqual(record["pid"], child.pid)
                self.assertEqual(record["process_identity"], IDENTITY)
                return 7
            child.wait.side_effect = wait
            original = harness.private_file
            def write(path, content):
                if path.name == "result.json":
                    self.assertFalse(self.path.exists())
                    self.assertTrue(harness.subprocess.Popen.call_args.kwargs["stdout"].closed)
                original(path, content)
            with mock.patch.object(harness, "private_file", side_effect=write):
                self.assertEqual(self.start(), 7)
        self.assertEqual(json.loads((self.logs / "result.json").read_text())["exit_code"], 7)

    def test_cleanup_replacement_preserved_and_success_result_suppressed(self):
        with self.start_environment() as child:
            def wait():
                self.path.unlink()
                harness.private_file(self.path, "competing record\n")
                return 0
            child.wait.side_effect = wait
            with self.assertRaisesRegex(harness.HarnessError, "ownership-lost"):
                self.start()
        self.assertEqual(self.path.read_text(), "competing record\n")
        self.assertFalse((self.logs / "result.json").exists())

    def test_publication_failure_cleans_owned_child_and_releases_resource_guard(self):
        with self.start_environment() as child:
            child.poll.side_effect = [None, 0]
            guard = mock.Mock()
            with mock.patch.object(harness.CandidateResourceGuard, "acquire", return_value=guard), \
                    mock.patch.object(harness.RunRecordReservation, "identify",
                                      side_effect=harness.HarnessError("synthetic publication failure")):
                with self.assertRaisesRegex(harness.HarnessError, "synthetic publication failure"):
                    self.start(model="synthetic-resource-guard")
            child.send_signal.assert_called_once_with(signal.SIGINT)
            child.wait.assert_called_once_with(timeout=30)
            guard.close.assert_called_once()
        self.assertFalse(self.path.exists())
        self.assertFalse((self.logs / "result.json").exists())

    def test_unreaped_child_preserves_record_and_releases_guard_on_cleanup_failure(self):
        with self.start_environment() as child:
            child.poll.return_value = None
            child.wait.side_effect = subprocess.TimeoutExpired("owned synthetic child", 10)
            guard = mock.Mock()
            with mock.patch.object(harness.CandidateResourceGuard, "acquire", return_value=guard), \
                    mock.patch.object(harness.RunRecordReservation, "identify",
                                      side_effect=harness.HarnessError("synthetic publication failure")):
                with self.assertRaisesRegex(harness.HarnessError, "child-cleanup-incomplete"):
                    self.start(model="synthetic-resource-guard")
            child.kill.assert_called_once()
            guard.close.assert_called_once()
        self.assertIsNotNone(harness.read_run_record(self.state))
        self.assertFalse((self.logs / "result.json").exists())

    def test_cleanup_refuses_missing_modified_aliased_and_replaced_records(self):
        for change in ("missing", "modified", "symlink", "hardlink", "replacement"):
            with self.subTest(change=change):
                owner = self.owner()
                descriptors = owner.descriptor, owner.directory
                other = self.state / "other"
                if change == "missing":
                    self.path.unlink()
                elif change == "modified":
                    self.path.write_text("modified")
                elif change == "hardlink":
                    os.link(self.path, other)
                else:
                    self.path.unlink()
                    if change == "symlink":
                        other.write_text("other")
                        self.path.symlink_to(other)
                    else:
                        harness.private_file(self.path, "replacement")
                with self.assertRaises((harness.HarnessError, OSError)):
                    owner.close()
                self.assertEqual(self.path.exists(), change != "missing")
                for descriptor in descriptors:
                    with self.assertRaises(OSError):
                        os.fstat(descriptor)
                self.path.unlink(missing_ok=True)
                other.unlink(missing_ok=True)

    def test_legacy_malformed_duplicate_and_oversized_records_are_preserved(self):
        owner = self.owner()
        record = owner.record.copy()
        owner.close()
        invalid = [
            json.dumps({"pid": 123, "base": str(self.base), "binary": str(self.executable),
                        "started_at_epoch_ms": 1}).encode(),
            b"[]", b"null", b"", b"{", b"\xff", b" " * (harness.MAX_RECORD_BYTES + 1),
            json.dumps(record).replace('"schema_version": 2', '"schema_version": 2, "schema_version": 2').encode(),
        ]
        for value in invalid:
            with self.subTest(length=len(value)):
                self.path.write_bytes(value)
                self.path.chmod(0o600)
                with self.assertRaises(harness.HarnessError):
                    harness.read_run_record(self.state)
                self.assertEqual(self.path.read_bytes(), value)
                self.path.unlink()

    def test_closed_types_and_process_identity_required(self):
        owner = self.owner()
        record = {**owner.record, "pid": 123, "process_identity": IDENTITY.copy()}
        owner.close()
        self.assertTrue(harness.valid_run_record(record))
        for key, value in [("pid", True), ("pid", 1), ("pid", "123"), ("pid", 2**31),
                           ("schema_version", True), ("base", "relative"), ("binary", "/a\0b"),
                           ("process_identity", None), ("started_at_epoch_ms", True)]:
            self.assertFalse(harness.valid_run_record({**record, key: value}), key)
        for key, value in [("uid", os.getuid() + 1), ("uid", False), ("start_ticks", True),
                           ("start_ticks", 0), ("boot_id", "unknown")]:
            changed = copy.deepcopy(record)
            changed["process_identity"][key] = value
            self.assertFalse(harness.valid_run_record(changed), key)

    def test_reader_refuses_symlinks_hardlinks_fifo_public_mode_and_writer_lock(self):
        owner = self.owner()
        try:
            fcntl.flock(owner.descriptor, fcntl.LOCK_EX | fcntl.LOCK_NB)
            with self.assertRaises(harness.HarnessError):
                harness.read_run_record(self.state)
            fcntl.flock(owner.descriptor, fcntl.LOCK_UN)
        finally:
            owner.close()
        other = self.state / "other"
        harness.private_file(other, "private")
        self.path.symlink_to(other)
        with self.assertRaises(OSError):
            harness.read_run_record(self.state)
        self.path.unlink()
        os.link(other, self.path)
        with self.assertRaises(harness.HarnessError):
            harness.read_run_record(self.state)
        self.path.unlink()
        os.mkfifo(self.path, 0o600)
        with self.assertRaises(harness.HarnessError):
            harness.read_run_record(self.state)
        self.path.unlink()
        self.path.write_text("{}")
        self.path.chmod(0o644)
        with self.assertRaises(harness.HarnessError):
            harness.read_run_record(self.state)

    def test_reader_detects_mutation_during_held_descriptor_read(self):
        owner = self.owner()
        original = os.pread
        def mutate(*args):
            content = original(*args)
            self.path.write_bytes(content + b" ")
            return content
        with mock.patch.object(os, "pread", side_effect=mutate):
            with self.assertRaisesRegex(harness.HarnessError, "run-record-changed"):
                harness.read_run_record(self.state)
        with self.assertRaises(harness.HarnessError):
            owner.close()

    def test_partial_initial_publication_preserves_uncertain_record(self):
        with mock.patch.object(os, "pwrite", return_value=0):
            with self.assertRaisesRegex(harness.HarnessError, "write-failed"):
                self.owner()
        self.assertEqual(self.path.read_bytes(), b"")
        with self.assertRaises(harness.HarnessError):
            harness.read_run_record(self.state)

    def test_brief_reader_contention_retries_but_persistent_lock_is_bounded(self):
        owner = self.owner()
        try:
            with mock.patch.object(fcntl, "flock", side_effect=[BlockingIOError, None]) as lock, \
                    mock.patch.object(harness.time, "sleep") as sleep:
                owner.lock_for_update()
                self.assertEqual(lock.call_count, 2)
                sleep.assert_called_once_with(0.01)
            with mock.patch.object(fcntl, "flock", side_effect=BlockingIOError), \
                    mock.patch.object(harness.time, "monotonic", side_effect=[0, 1]), \
                    mock.patch.object(harness.time, "sleep") as sleep:
                with self.assertRaisesRegex(harness.HarnessError, "run-record-busy"):
                    owner.lock_for_update()
                sleep.assert_not_called()
        finally:
            owner.close()

    def test_diagnosis_distinguishes_reserved_stale_and_ready_without_adopting_record(self):
        base = Path(self.temporary.name) / "diagnosis"
        with mock.patch.object(harness, "binary", return_value=self.executable), \
                mock.patch.object(harness, "confinement_diagnosis", return_value={"ready": False}), \
                mock.patch("sys.stdout", new=io.StringIO()):
            harness.setup(base)
            owner = harness.RunRecordReservation(base / "state", base, self.executable)
            try:
                self.assertEqual(harness.diagnose(base)["lifecycle"], "reserved")
                owner.record.update(pid=12345, process_identity=IDENTITY)
                owner.publish()
                with mock.patch.object(harness, "exact_running_process", return_value=False):
                    self.assertEqual(harness.diagnose(base)["lifecycle"], "stale-record")
                with mock.patch.object(harness, "exact_running_process", return_value=True):
                    self.assertEqual(harness.diagnose(base)["lifecycle"], "starting")
                    harness.private_file(base / "state/operational-store-development-v1.key", "fixture")
                    self.assertEqual(harness.diagnose(base)["lifecycle"], "running")
            finally:
                owner.close()
            self.assertEqual(harness.diagnose(base)["lifecycle"], "ready")

    def test_fast_exited_child_retains_actual_exit_without_invented_identity(self):
        with self.start_environment() as child:
            child.poll.return_value = 9
            child.wait.return_value = 9
            harness.linux_process_identity.side_effect = FileNotFoundError
            self.assertEqual(self.start(), 9)
            child.send_signal.assert_not_called()
        self.assertEqual(json.loads((self.logs / "result.json").read_text())["exit_code"], 9)
        self.assertFalse(self.path.exists())

    def test_exact_process_requires_unchanged_start_boot_uid_executable_and_each_root(self):
        owner = self.owner()
        record = {**owner.record, "pid": 12345, "process_identity": IDENTITY}
        owner.close()
        arguments = [b"synthetic-cli", b"--development"]
        for flag, path in zip((b"--state-root", b"--disposable-root", b"--workspace-root"),
                              harness.paths(self.base)):
            arguments.extend((flag, os.fsencode(path)))
        resolve = Path.resolve
        def resolve_executable(path, **kwargs):
            return self.executable if str(path) == "/proc/12345/exe" else resolve(path, **kwargs)
        with mock.patch.object(harness, "linux_process_identity", return_value=IDENTITY) as identity, \
                mock.patch.object(Path, "resolve", resolve_executable), \
                mock.patch.object(Path, "read_bytes", return_value=b"\0".join(arguments)) as read:
            self.assertTrue(harness.exact_running_process(record, self.base))
            for key, value in [("start_ticks", 101), ("uid", os.getuid() + 1),
                               ("boot_id", "aaaaaaaa-aaaa-aaaa-aaaa-aaaaaaaaaaaa")]:
                identity.return_value = {**IDENTITY, key: value}
                self.assertFalse(harness.exact_running_process(record, self.base))
            identity.side_effect = [IDENTITY, {**IDENTITY, "start_ticks": 101}]
            self.assertFalse(harness.exact_running_process(record, self.base))
            identity.side_effect = None
            identity.return_value = IDENTITY
            for changed in [arguments[:-2], arguments + arguments[-2:],
                            [*arguments[:-1], os.fsencode(self.base / "wrong")],
                            [*arguments, b"--development"]]:
                read.return_value = b"\0".join(changed)
                self.assertFalse(harness.exact_running_process(record, self.base))
            read.return_value = b"\0".join(arguments)
            record["binary"] = "/different/executable"
            self.assertFalse(harness.exact_running_process(record, self.base))

    def test_stop_pins_before_revalidation_closes_descriptor_and_never_uses_pid_signal(self):
        owner = self.owner()
        owner.record.update(pid=12345, process_identity=IDENTITY)
        owner.publish()
        harness.private_file(self.state / "operational-store-development-v1.key", "synthetic fixture")
        try:
            for failure in ("identity", "record", "exited", "unsupported"):
                with self.subTest(failure=failure), ExitStack() as stack:
                    calls = []
                    stack.enter_context(mock.patch.object(os, "kill", side_effect=AssertionError("bare PID")))
                    opened = stack.enter_context(mock.patch.object(os, "pidfd_open", side_effect=lambda pid: (
                        calls.append("pin") or os.open(os.devnull, os.O_RDONLY))))
                    exact = stack.enter_context(mock.patch.object(harness, "exact_running_process",
                        side_effect=lambda *args: calls.append("identity") or failure != "identity"))
                    send = stack.enter_context(mock.patch.object(signal, "pidfd_send_signal"))
                    closed = stack.enter_context(mock.patch.object(os, "close", wraps=os.close))
                    if failure == "record":
                        stack.enter_context(mock.patch.object(harness, "read_run_record",
                            side_effect=[owner.record, {**owner.record, "pid": 999}]))
                    if failure == "exited":
                        send.side_effect = ProcessLookupError
                    if failure == "unsupported":
                        opened.side_effect = OSError("unsupported")
                    with self.assertRaises((harness.HarnessError, OSError)):
                        harness.stop(self.base)
                    if failure == "unsupported":
                        exact.assert_not_called()
                    else:
                        self.assertEqual(calls[:2], ["pin", "identity"])
                        self.assertTrue(closed.called)
                    if failure != "exited":
                        send.assert_not_called()
        finally:
            owner.close()

    def test_owned_linux_child_cancellation_through_real_process_descriptor(self):
        if not hasattr(os, "pidfd_open") or not hasattr(signal, "pidfd_send_signal"):
            self.skipTest("Linux process descriptors unavailable; not a passed cancellation check")
        command = [str(self.executable), "-c",
                   "import signal,sys,time; signal.signal(signal.SIGINT, lambda *_: sys.exit(73)); "
                   "print('ready', flush=True); time.sleep(30)", "--development"]
        for flag, path in zip(("--state-root", "--disposable-root", "--workspace-root"),
                              harness.paths(self.base)):
            command.extend((flag, str(path)))
        owner = self.owner()
        child = None
        try:
            child = subprocess.Popen(command, stdin=subprocess.DEVNULL,
                                     stdout=subprocess.PIPE, stderr=subprocess.PIPE, text=True)
            self.assertTrue(select.select([child.stdout], [], [], 5)[0], "owned child did not become ready")
            self.assertEqual(child.stdout.readline(), "ready\n")
            owner.identify(child)
            self.assertTrue(harness.exact_running_process(harness.read_run_record(self.state), self.base))
            harness.private_file(self.state / "operational-store-development-v1.key", "synthetic fixture")
            with mock.patch.object(os, "kill", side_effect=AssertionError("bare PID")), \
                    mock.patch("sys.stdout", new=io.StringIO()) as output:
                harness.stop(self.base)
                self.assertEqual(child.wait(timeout=5), 73)
                self.assertIn("cancellation-requested", output.getvalue())
            self.assertFalse(harness.exact_running_process(harness.read_run_record(self.state), self.base))
        finally:
            if child is not None:
                if child.poll() is None:
                    child.kill()
                child.communicate(timeout=5)
                child.stdout.close()
                child.stderr.close()
            owner.close(child_reaped=child is None or child.poll() is not None)


if __name__ == "__main__":
    unittest.main()
