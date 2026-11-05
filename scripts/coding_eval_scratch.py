"""Private, quota-backed scratch mounts for the independent coding evaluator.

Only the trusted mount helper runs here. Candidate code never enters its
namespace; it receives selected subdirectories through separate bind mounts.
"""
from __future__ import annotations

from array import array
from contextlib import contextmanager
import json
import os
from pathlib import Path
import signal
import socket
import stat
import subprocess
import tempfile


SCRATCH_BYTES = 128 * 1024 * 1024
# Accommodates the unchanged 24,000-directory branching lifecycle control.
SCRATCH_INODES = 32768
PRIVATE_BYTES = 64 * 1024 * 1024
PRIVATE_INODES = 8192
SETUP_SECONDS = 5
TOKEN = b"coding-scratch-v1"


class ScratchError(ValueError):
    """Fixed, content-minimized scratch setup/refusal diagnostic."""


# Writable host mounts in this helper are the private socket and bounded logs. The
# mount capability belongs only to this fixed helper, never to a task worker.
# Mount before exporting the descriptor so both data and inode limits apply
# before the first input is written. Disconnect/parent death ends the holder.
HELPER = b'''
from array import array
import ctypes
import os
import json
import socket
import sys
libc = ctypes.CDLL(None, use_errno=True)
libc.mount.argtypes = [ctypes.c_char_p, ctypes.c_char_p, ctypes.c_char_p,
                       ctypes.c_ulong, ctypes.c_char_p]
if libc.mount(b"tmpfs", b"/scratch", b"tmpfs", 2 | 4,
              b"size=134217728,nr_inodes=32768,mode=0700") != 0:
    raise OSError(ctypes.get_errno(), "scratch-mount-refused")
with socket.socket(socket.AF_UNIX, socket.SOCK_SEQPACKET) as control:
    control.connect("/control")
    descriptor = os.open("/scratch", os.O_RDONLY | os.O_DIRECTORY | os.O_CLOEXEC)
    try:
        control.sendmsg([b"coding-scratch-v1"],
                        [(socket.SOL_SOCKET, socket.SCM_RIGHTS, array("i", [descriptor]))])
    finally:
        os.close(descriptor)
    # Mount paths must be resolved in the namespace that owns the mount. The
    # fixed trusted exchange supervises both nested task namespaces here.
    sys.path.insert(0, "/runner")
    import coding_eval_oracle as oracle
    request = json.loads(control.recv(65537))
    if type(request) is dict and request.get("mode") == "independent-rust":
        import coding_eval_rust_oracle
        result = coding_eval_rust_oracle.run_request(request)
        control.send(json.dumps(result, allow_nan=False).encode("ascii"))
        control.recv(1)
        raise SystemExit(0)
    if type(request) is dict and request.get("mode") == "historical-control":
        import coding_eval_control
        result = coding_eval_control.run_request(request)
        control.send(json.dumps(result, allow_nan=False).encode("ascii"))
        control.recv(1)
        raise SystemExit(0)
    if (type(request) is not dict or request.keys() != {"candidate", "oracle", "timeout", "identity"}
            or type(request["timeout"]) not in (int, float) or not 0 < request["timeout"] <= 90
            or type(request["identity"]) is not bool):
        raise ValueError("scratch.request-refused")
    for command in (request["candidate"], request["oracle"]):
        if (type(command) is not list or not 1 <= len(command) <= 256
                or any(type(value) is not str or len(value) > 4096 or "\\0" in value for value in command)):
            raise ValueError("scratch.command-refused")
        # The one effect-free fault command preserves host lifecycle probes.
        if command != ["/usr/bin/false"] and (len(command) < 2 or command[0] != "/usr/bin/bwrap"):
            raise ValueError("scratch.command-refused")
    for name in ("private-candidate", "private-oracle"):
        path = oracle.Path("/scratch") / name
        path.mkdir(mode=0o700, exist_ok=True)
        if libc.mount(b"tmpfs", os.fsencode(path), b"tmpfs", 2 | 4,
                      b"size=67108864,nr_inodes=8192,mode=0700") != 0:
            raise OSError(ctypes.get_errno(), "scratch-private-mount-refused")
    if request["identity"]:
        for name in ("candidate", "trusted"):
            oracle.evaluation.initialize_grading_repository(oracle.Path("/scratch") / name)
    result = oracle.exchange(request["candidate"], request["oracle"], oracle.Path("/logs"), request["timeout"])
    control.send(json.dumps(result, allow_nan=False).encode("ascii"))
    control.recv(1)
'''


def stop_holder(process):
    """Signal before reaping, preserving the group identity on every outcome."""
    try:
        os.killpg(process.pid, signal.SIGKILL)
    except ProcessLookupError:
        pass
    process.wait()


def receive_root(control):
    """Accept exactly one directory capability; close every fd on refusal."""
    descriptors = array("i")
    accepted = False
    try:
        data, ancillary, flags, _ = control.recvmsg(
            len(TOKEN) + 1, socket.CMSG_SPACE(16 * descriptors.itemsize), socket.MSG_CMSG_CLOEXEC)
        invalid = False
        for level, kind, payload in ancillary:
            if level != socket.SOL_SOCKET or kind != socket.SCM_RIGHTS:
                invalid = True
                continue
            descriptors.frombytes(payload[:len(payload) - len(payload) % descriptors.itemsize])
        if invalid or data != TOKEN or flags & (socket.MSG_TRUNC | socket.MSG_CTRUNC) or len(descriptors) != 1:
            raise ScratchError("scratch.descriptor-refused")
        descriptor = descriptors[0]
        info, filesystem = os.fstat(descriptor), os.fstatvfs(descriptor)
        if (not stat.S_ISDIR(info.st_mode) or stat.S_IMODE(info.st_mode) != 0o700
                or filesystem.f_blocks * filesystem.f_frsize != SCRATCH_BYTES
                or filesystem.f_files != SCRATCH_INODES or os.get_inheritable(descriptor)):
            raise ScratchError("scratch.filesystem-refused")
        accepted = True
        return descriptor
    finally:
        if not accepted:
            for descriptor in descriptors:
                os.close(descriptor)


class ScratchMount:
    def __init__(self, descriptor, control):
        self.root = Path("/proc/self/fd") / str(descriptor)
        self.control = control
        self.used = False

    def exchange(self, candidate, oracle, timeout, *, identity=False):
        """One host-owned exchange, with no scratch capability in task workers."""
        if self.used or type(timeout) not in (int, float) or not 0 < timeout <= 90 or type(identity) is not bool:
            raise ScratchError("scratch.exchange-refused")
        self.used = True
        prefix = str(self.root) + "/"

        def local(command):
            return ["/scratch/" + value[len(prefix):] if value.startswith(prefix) else value for value in command]

        data = json.dumps({"candidate": local(candidate), "oracle": local(oracle),
                           "timeout": timeout, "identity": identity}, allow_nan=False).encode("ascii")
        if len(data) > 65536:
            raise ScratchError("scratch.exchange-refused")
        self.control.settimeout(timeout + SETUP_SECONDS)
        self.control.send(data)
        response, ancillary, flags, _ = self.control.recvmsg(4096)
        if ancillary or flags & (socket.MSG_TRUNC | socket.MSG_CTRUNC):
            raise ScratchError("scratch.outcome-refused")
        try:
            result = json.loads(response)
        except ValueError:
            raise ScratchError("scratch.outcome-refused") from None
        if (type(result) is not list or len(result) != 4
                or any(value is not None and type(value) is not int for value in result[:2])
                or any(type(value) is not bool for value in result[2:])):
            raise ScratchError("scratch.outcome-refused")
        return tuple(result)


@contextmanager
def control_directory():
    """Remove only the fixed trusted control entries, never task-owned trees."""
    directory = Path(tempfile.mkdtemp(prefix="agentmage-scratch-"))
    failed = False
    try:
        yield directory
    except BaseException:
        failed = True
        raise
    finally:
        try:
            for name in ("candidate.log", "oracle.log"):
                (directory / "logs" / name).unlink(missing_ok=True)
            try:
                (directory / "logs").rmdir()
            except FileNotFoundError:
                pass
            (directory / "control").unlink(missing_ok=True)
            directory.rmdir()
        except OSError:
            if not failed:
                raise


@contextmanager
def scratch_mount(output=None, *, runtime_paths=None):
    """Export a private anonymous tmpfs; teardown needs no candidate traversal.

    No fallback to host disk exists. Setup failure refuses the case. Workers
    must be killed/reaped before leaving this context. Closing the capability
    and killing/reaping the holder releases our references even after errors.
    """
    process = control = None
    descriptor = None
    with control_directory() as directory:
        with socket.socket(socket.AF_UNIX, socket.SOCK_SEQPACKET) as listener, \
                tempfile.TemporaryFile() as helper:
            endpoint = Path(directory) / "control"
            logs = Path(output) if output is not None else Path(directory) / "logs"
            if output is None:
                logs.mkdir(mode=0o700)
            # AF_UNIX addresses are much shorter than filesystem paths. Bind
            # through a pinned parent; the socket inode remains at endpoint.
            parent = os.open(directory, os.O_RDONLY | os.O_DIRECTORY | os.O_NOFOLLOW | os.O_CLOEXEC)
            try:
                listener.bind(f"/proc/self/fd/{parent}/control")
            finally:
                os.close(parent)
            listener.listen(1)
            listener.settimeout(SETUP_SECONDS)
            helper.write(HELPER)
            helper.seek(0)
            command = [
                "/usr/bin/bwrap", "--unshare-all", "--die-with-parent", "--new-session",
                "--uid", "0", "--gid", "0",
                "--cap-drop", "ALL", "--cap-add", "CAP_SYS_ADMIN",
                "--cap-add", "CAP_SETFCAP",
                "--ro-bind", "/usr", "/usr", "--symlink", "usr/bin", "/bin",
                "--symlink", "usr/lib", "/lib", "--symlink", "usr/lib64", "/lib64",
                "--size", str(16 * 1024 * 1024), "--tmpfs", "/tmp",
                "--proc", "/proc", "--dev", "/dev", "--size", str(SCRATCH_BYTES), "--tmpfs", "/scratch",
                "--ro-bind", str(Path(__file__).resolve().parent), "/runner", "--bind", str(logs), "/logs",
                "--bind", str(endpoint), "/control", "--file", str(helper.fileno()), "/holder.py",
                "--", "/usr/bin/python3", "-I", "-B", "/holder.py",
            ]
            if runtime_paths is not None:
                toolchain, registry = runtime_paths
                boundary = command.index("--")
                command[boundary:boundary] = [
                    "--ro-bind", str(toolchain), "/runtime-input/toolchain",
                    "--ro-bind", str(registry), "/runtime-input/registry", "--dir", "/runtime",
                ]
            try:
                process = subprocess.Popen(command, stdin=subprocess.DEVNULL, stdout=subprocess.DEVNULL,
                                           stderr=subprocess.DEVNULL, env={}, start_new_session=True,
                                           pass_fds=(helper.fileno(),))
                control, _ = listener.accept()
                control.settimeout(SETUP_SECONDS)
                descriptor = receive_root(control)
                yield ScratchMount(descriptor, control)
            except (OSError, subprocess.SubprocessError) as error:
                raise ScratchError("scratch.setup-or-io-refused") from error
            finally:
                if descriptor is not None:
                    os.close(descriptor)
                if control is not None:
                    control.close()
                if process is not None:
                    stop_holder(process)
