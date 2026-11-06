"""Remove assertion/result mounts before trusted fixture execution.

Only a synchronous trusted launch may temporarily attach procfs. The confined
child still has no proc mount, assertions, results, capabilities or network.
Admission belongs to the supervising evaluator, never this helper.
"""
import contextlib
import ctypes
import importlib.util
import os
from pathlib import Path
import subprocess
import threading
import selectors


class SealError(RuntimeError):
    pass


class SealedOracle:
    def __init__(self):
        self.libc = ctypes.CDLL(None, use_errno=True)
        self.libc.mount.argtypes = [ctypes.c_char_p, ctypes.c_char_p, ctypes.c_char_p,
                                   ctypes.c_ulong, ctypes.c_char_p]
        self.libc.umount2.argtypes = [ctypes.c_char_p, ctypes.c_int]
        self.sealed = False
        self.proc_depth = 0
        self.windows = 0
        self.proc_tree = None
        self.before_launch = lambda: None
        self.raw_launch = subprocess.Popen
        self.gate = None

    def clone_proc(self):
        # A detached mount descriptor avoids a private pathname and avoids
        # trying to create a new proc superblock after hiding the old view.
        descriptor = self.libc.syscall(428, ctypes.c_long(-100), b'/proc',
                                       ctypes.c_uint(1 | os.O_CLOEXEC))
        if descriptor < 0:
            raise SealError('seal.proc-clone-refused')
        return descriptor

    def detach(self, name):
        if self.libc.umount2(os.fsencode(name), 2) != 0:
            raise SealError('seal.unmount-refused')

    def close_private_paths(self):
        if self.sealed or threading.active_count() != 1:
            raise SealError('seal.concurrent-or-repeated-seal')
        self.proc_tree = self.clone_proc()
        # Parent-side Python objects and private preopened result descriptors
        # remain in memory. Their backing files have no reachable mount here.
        for name in ('/adapter', '/output', '/proc'):
            self.detach(name)
        if (os.listdir('/adapter') or os.listdir('/output') or Path('/proc/self').exists()
                or not os.statvfs('/').f_flag & os.ST_RDONLY):
            raise SealError('seal.private-path-still-visible')
        self.sealed = True

    @contextlib.contextmanager
    def proc_window(self):
        self.before_launch()
        if not self.sealed or threading.active_count() != 1 or self.proc_depth:
            raise SealError('seal.concurrent-launch-refused')
        if Path('/proc/self').exists():
            raise SealError('seal.unexpected-proc-mount')
        if self.libc.syscall(429, ctypes.c_int(self.proc_tree), b'', ctypes.c_long(-100), b'/proc',
                             ctypes.c_uint(4)) != 0:
            raise SealError('seal.proc-attach-refused')
        self.proc_depth = 1
        self.windows += 1
        try:
            yield
        finally:
            following = None
            try:
                following = self.clone_proc()
            finally:
                self.detach('/proc')
                self.proc_depth = 0
            os.close(self.proc_tree)
            self.proc_tree = following
            if Path('/proc/self').exists():
                raise SealError('seal.proc-window-not-closed')

    def install_launch_boundary(self):
        spec = importlib.util.spec_from_file_location('native_gate', '/candidate-runner/gate.py')
        gate = importlib.util.module_from_spec(spec)
        spec.loader.exec_module(gate)
        if self.libc.prctl(4, 0, 0, 0, 0) != 0:
            raise SealError('seal.dumpability-refused')
        original_nested = gate.nested_command

        def nested(command, descriptor):
            result = original_nested(command, descriptor)
            index = result.index('--chdir')
            result[index:index] = ['--ro-bind', '/candidate', '/candidate']
            return result

        gate.nested_command = nested
        gate.install_launch_boundary()
        native_launch = subprocess.Popen

        def launch(*args, **kwargs):
            with self.proc_window():
                return native_launch(*args, **kwargs)

        subprocess.Popen = launch
        self.gate = gate
        return gate

    def launch_bridge(self):
        """Fixed bridge in its own PID namespace; no numeric-PID observations.

        Trusted bootstrap completes namespace setup before procfs disappears.
        Candidate import starts only after that private proc window is closed.
        """
        process = None
        try:
            with self.proc_window(), self.gate.tempfile_filter() as descriptor:
                command = self.gate.nested_command(
                    ['/usr/bin/python3', '-I', '-B', '/runner/bootstrap.py'], descriptor)
                index = command.index('--uid')
                # The bridge itself is namespace PID 1. Its exit destroys any
                # descendants, and an extra reaper does not consume the fixed
                # eight-process allowance needed by raw fixture launches.
                command[index:index] = ['--unshare-pid', '--as-pid-1', '--die-with-parent', '--new-session']
                process = self.raw_launch(command, stdin=subprocess.PIPE, stdout=subprocess.PIPE,
                                          stderr=None, close_fds=True, pass_fds=(descriptor,), env={},
                                          start_new_session=True)
                with selectors.DefaultSelector() as events:
                    events.register(process.stdout, selectors.EVENT_READ)
                    if not events.select(5) or process.stdout.readline(64) != b'worker-bootstrap-v1\n':
                        raise SealError('seal.bridge-bootstrap-refused')
            process.stdin.write(b'worker-go-v1\n');process.stdin.flush()
            return process
        except BaseException:
            if process is not None:
                self.stop_bridge(process)
            raise

    @staticmethod
    def stop_bridge(process):
        import signal
        # Do not reap the reserved leader before signaling its private group.
        if process.returncode is None:
            try:os.killpg(process.pid, signal.SIGKILL)
            except ProcessLookupError:pass
        process.wait(timeout=5)
        for stream in (process.stdin,process.stdout,process.stderr):
            if stream is not None:
                try:stream.close()
                except OSError:pass  # Preserve the original launch/protocol failure.
