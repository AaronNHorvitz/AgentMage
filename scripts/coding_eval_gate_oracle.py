"""Independent frozen fast-gate assertions and confined candidate execution.

The trusted test process observes real process IDs and reservation locks. Nested
candidate mounts share its PID namespace, but have no procfs, hidden tests,
result mount or ptrace-like syscalls. This adapter does not admit model scores.
"""
from __future__ import annotations

import contextlib
import ctypes
import errno
import importlib.util
import io
import json
import os
from pathlib import Path
import platform
import selectors
import signal
import subprocess
import sys
import tempfile
import time
import unittest
from unittest import mock


VERSION = 'coding-gate-oracle-v1'
MAX_BYTES = 65536
MAX_EVENTS = 128
MAX_TRANSCRIPT = 2 * 1024 * 1024
PROCESSES = 16
KILL_GROUP = os.killpg
KILL_PROCESS = os.kill
FUNCTIONS = frozenset({'changed_paths', 'affected_packages', 'python_lint', 'untracked_whitespace',
                       'review_text_lint', 'plan', 'observe_exit', 'stop_check', 'execute', 'main'})


class GateProtocolError(ValueError):
    """Fixed refusal; candidate objects never enter trusted assertions."""


def pack(value, depth=0):
    if depth > 16:
        raise GateProtocolError('gate.value-depth')
    if value is None or type(value) in (str, int, bool):
        return value
    if type(value) is signal.Signals:
        return int(value)
    if type(value) is bytes:
        return {'bytes': value.hex()}
    if isinstance(value, Path):
        return {'path': str(value)}
    if type(value) in (list, tuple):
        return {'tuple' if type(value) is tuple else 'list': [pack(item, depth + 1) for item in value]}
    if type(value) is dict and all(type(key) is str for key in value):
        return {'dict': {key: pack(item, depth + 1) for key, item in value.items()}}
    raise GateProtocolError('gate.value-type')


def unpack(value, depth=0):
    if depth > 16:
        raise GateProtocolError('gate.value-depth')
    if value is None or type(value) in (str, int, bool):
        return value
    if type(value) is not dict or len(value) != 1:
        raise GateProtocolError('gate.value-shape')
    kind, item = next(iter(value.items()))
    if kind == 'bytes' and type(item) is str:
        try:
            return bytes.fromhex(item)
        except ValueError:
            raise GateProtocolError('gate.bytes-refused') from None
    if kind == 'path' and type(item) is str:
        return Path(item)
    if kind in ('list', 'tuple') and type(item) is list:
        items = [unpack(part, depth + 1) for part in item]
        return tuple(items) if kind == 'tuple' else items
    if kind == 'dict' and type(item) is dict:
        return {key: unpack(part, depth + 1) for key, part in item.items()}
    raise GateProtocolError('gate.value-shape')


def unique(pairs):
    value = {}
    for key, item in pairs:
        if key in value:
            raise GateProtocolError('gate.duplicate-key')
        value[key] = item
    return value


def encoded(value):
    data = json.dumps(value, allow_nan=False).encode('utf-8')
    if len(data) > MAX_BYTES:
        raise GateProtocolError('gate.frame-limit')
    return data


def decoded(data):
    if len(data) > MAX_BYTES:
        raise GateProtocolError('gate.frame-limit')
    try:
        return json.loads(data, object_pairs_hook=unique,
                          parse_constant=lambda value: (_ for _ in ()).throw(GateProtocolError('gate.number')))
    except (ValueError, RecursionError):
        raise GateProtocolError('gate.invalid-json') from None


def syscall_filter():
    """Deny process inspection without exposing the trusted namespace's procfs.

    Fixed Linux x86-64 filter; refuse other architectures rather than silently
    use wrong syscall numbers. The architecture check precedes every syscall.
    x32 calls and ptrace/process_vm/pidfd_getfd/perf are denied with EPERM.
    """
    if platform.machine() != 'x86_64':
        raise GateProtocolError('gate.architecture-refused')
    # sock_filter: load arch, require AUDIT_ARCH_X86_64, load nr, disallow x32,
    # reject the five inspection primitives; permit the remaining syscall set.
    rows = [(0x20, 0, 0, 4), (0x15, 1, 0, 0xC000003E), (0x06, 0, 0, 0x80000000),
            (0x20, 0, 0, 0), (0x45, 0, 1, 0x40000000), (0x06, 0, 0, 0x50000 | errno.EPERM)]
    for number in (101, 310, 311, 438, 298):
        rows.extend([(0x15, 0, 1, number), (0x06, 0, 0, 0x50000 | errno.EPERM)])
    rows.append((0x06, 0, 0, 0x7FFF0000))
    import struct
    return b''.join(struct.pack('HBBI', *row) for row in rows)


def nested_command(command, filter_descriptor):
    """No proc mount, parent descriptors, results, hidden input or network."""
    return [
        '/usr/bin/bwrap', '--unshare-user', '--unshare-net', '--unshare-ipc', '--unshare-uts',
        '--uid', '0', '--gid', '0', '--disable-userns', '--cap-drop', 'ALL', '--clearenv',
        '--ro-bind', '/usr', '/usr', '--symlink', 'usr/bin', '/bin',
        '--symlink', 'usr/lib', '/lib', '--symlink', 'usr/lib64', '/lib64',
        '--dev', '/dev', '--dir', '/proc', '--ro-bind', '/candidate', '/workspace',
        '--ro-bind', '/candidate-runner', '/runner', '--bind', '/tmp', '/tmp',
        '--setenv', 'PATH', '/usr/bin', '--setenv', 'HOME', '/tmp', '--setenv', 'LANG', 'C.UTF-8',
        '--chdir', os.getcwd(), '--seccomp', str(filter_descriptor),
        '--remount-ro', '/dev', '--remount-ro', '/', '--', *command,
    ]


def bootstrap_lineage(child, leader):
    """Use kernel ancestry, never a candidate-selected PID, to authorize cleanup."""
    if child == leader:
        raise GateProtocolError('gate.bootstrap-ancestry-refused')
    lineage = []
    current = child
    for _ in range(8):
        if current == leader:
            return lineage
        row = Path(f'/proc/{current}/stat').read_text().rsplit(') ', 1)[1].split()
        parent = int(row[1])
        if current <= 1 or parent <= 0 or current in lineage:
            break
        lineage.append(current)
        current = parent
    raise GateProtocolError('gate.bootstrap-ancestry-refused')


def install_launch_boundary():
    original = subprocess.Popen

    def launch(command, *args, **kwargs):
        if command[0] in ('git', '/usr/bin/git'):
            command = ['/usr/bin/git', *command[1:]]
        elif command[0] not in ('/usr/bin/python3', sys.executable):
            raise GateProtocolError('gate.fixture-command-refused')
        if kwargs.get('pass_fds') or kwargs.get('preexec_fn'):
            raise GateProtocolError('gate.inherited-descriptor-refused')
        # Launcher groups are private. The actual child creates its own session
        # in trusted bootstrap, before any candidate source is imported.
        kwargs.pop('process_group', None)
        kwargs['start_new_session'] = True
        handshake = Path(tempfile.mkdtemp(prefix='gate-launch-'))
        with tempfile_filter() as descriptor:
            bootstrap = ['/usr/bin/python3', '-I', '-B', '/runner/gate.py', 'bootstrap',
                         str(handshake), json.dumps(list(command))]
            confined = nested_command(bootstrap, descriptor)
            confined[confined.index('--chdir') + 1] = str(kwargs.get('cwd') or os.getcwd())
            kwargs['pass_fds'] = (descriptor,)
            kwargs['env'] = {}
            if ctypes.CDLL(None).prctl(36, 1, 0, 0, 0) != 0:
                raise GateProtocolError('gate.subreaper-refused')
            process = None
            child_descriptor = None
            child = None
            adopted = False
            try:
                process = original(confined, *args, **kwargs)
                deadline = time.monotonic() + 5
                while True:
                    try:
                        fd = os.open(handshake / 'ready', os.O_RDONLY | os.O_NOFOLLOW | os.O_CLOEXEC)
                    except FileNotFoundError:
                        ready = None
                    else:
                        with os.fdopen(fd, 'rb') as stream:
                            ready = decoded(stream.read(4097))
                    if type(ready) is int and ready > 1:
                        child = ready
                        lineage = bootstrap_lineage(child, process.pid)
                        if os.getpgid(child) != child or any(os.getpgid(pid) != process.pid for pid in lineage[1:]):
                            raise GateProtocolError('gate.child-group-refused')
                        child_descriptor = os.pidfd_open(child)
                        break
                    if process.poll() is not None or time.monotonic() >= deadline:
                        raise GateProtocolError('gate.bootstrap-refused')
                    time.sleep(0.001)
                # Kill every launcher ancestor, leaving the separately grouped
                # trusted bootstrap paused. Reap the intermediaries as well as
                # Popen's original leader before adopting the actual child.
                KILL_GROUP(process.pid, signal.SIGKILL)
                process.wait()
                for ancestor in reversed(lineage[1:]):
                    while True:
                        try:
                            status = os.waitid(os.P_PID, ancestor, os.WEXITED | os.WNOHANG | os.WNOWAIT)
                        except ChildProcessError:
                            break
                        if status is not None:
                            os.waitpid(ancestor, 0)
                            break
                        if time.monotonic() >= deadline:
                            raise GateProtocolError('gate.launcher-cleanup-timeout')
                        time.sleep(0.001)
                if os.waitid(os.P_PID, child, os.WEXITED | os.WNOHANG | os.WNOWAIT) is not None:
                    raise GateProtocolError('gate.adoption-refused')
                process.pid = child
                process.returncode = None
                adopted = True
                if ctypes.CDLL(None).prctl(36, 0, 0, 0, 0) != 0:
                    raise GateProtocolError('gate.subreaper-refused')
                parent = os.open(handshake, os.O_RDONLY | os.O_DIRECTORY | os.O_NOFOLLOW | os.O_CLOEXEC)
                try:
                    release = os.open('go', os.O_WRONLY | os.O_CREAT | os.O_EXCL | os.O_NOFOLLOW | os.O_CLOEXEC,
                                      0o600, dir_fd=parent)
                    os.close(release)
                finally:
                    os.close(parent)
                return process
            except BaseException:
                if process is not None:
                    try:
                        if adopted:
                            KILL_PROCESS(process.pid, signal.SIGKILL)
                        elif process.returncode is None:
                            KILL_GROUP(process.pid, signal.SIGKILL)
                    except ProcessLookupError:
                        pass
                    process.wait()
                if child_descriptor is not None and not adopted:
                    try:
                        signal.pidfd_send_signal(child_descriptor, signal.SIGKILL)
                    except ProcessLookupError:
                        pass
                    try:
                        os.waitpid(child, 0)
                    except ChildProcessError:
                        pass
                if process is not None:
                    for stream in (process.stdin, process.stdout, process.stderr):
                        if stream is not None:
                            if stream is process.stderr:
                                os.set_blocking(stream.fileno(), False)
                                try:
                                    sys.stderr.write(os.read(stream.fileno(), MAX_BYTES).decode('utf-8', errors='replace'))
                                except BlockingIOError:
                                    pass
                            stream.close()
                raise
            finally:
                if child_descriptor is not None:
                    os.close(child_descriptor)
                if ctypes.CDLL(None).prctl(36, 0, 0, 0, 0) != 0:
                    raise GateProtocolError('gate.subreaper-refused')

    subprocess.Popen = launch


@contextlib.contextmanager
def tempfile_filter():
    with tempfile.TemporaryFile() as stream:
        stream.write(syscall_filter())
        stream.seek(0)
        yield stream.fileno()


def import_subject():
    sys.path.insert(0, '/workspace')
    from scripts import local_agent_fast_gate
    return local_agent_fast_gate


def bounded_call(request):
    """Bound both pipes during execution; no unbounded communicate buffer."""
    command = ['/usr/bin/python3', '-I', '-B', '/runner/gate.py', 'candidate']
    process = subprocess.Popen(command, stdin=subprocess.PIPE, stdout=subprocess.PIPE, stderr=subprocess.PIPE,
                               start_new_session=True)
    stdout, stderr = bytearray(), bytearray()
    try:
        pending = memoryview(encoded(request))
        deadline = time.monotonic() + 75
        with selectors.DefaultSelector() as events:
            os.set_blocking(process.stdin.fileno(), False)
            events.register(process.stdin, selectors.EVENT_WRITE, None)
            events.register(process.stdout, selectors.EVENT_READ, stdout)
            events.register(process.stderr, selectors.EVENT_READ, stderr)
            while events.get_map():
                if time.monotonic() >= deadline:
                    raise GateProtocolError('gate.call-timeout')
                for key, _ in events.select(0.01):
                    if key.fileobj is process.stdin:
                        try:
                            pending = pending[os.write(process.stdin.fileno(), pending):]
                        except BrokenPipeError:
                            pending = pending[:0]
                        if not pending:
                            events.unregister(process.stdin)
                            process.stdin.close()
                        continue
                    chunk = os.read(key.fileobj.fileno(), 16384)
                    if not chunk:
                        events.unregister(key.fileobj)
                    else:
                        if len(key.data) + len(chunk) > MAX_BYTES:
                            raise GateProtocolError('gate.call-output-limit')
                        key.data.extend(chunk)
        while True:
            status = os.waitid(os.P_PID, process.pid, os.WEXITED | os.WNOHANG | os.WNOWAIT)
            if status is not None:
                break
            if time.monotonic() >= deadline:
                raise GateProtocolError('gate.call-timeout')
            time.sleep(0.01)
        if status.si_code != os.CLD_EXITED or status.si_status != 0:
            raise GateProtocolError('gate.call-exit')
        return decoded(stdout)
    finally:
        sys.stderr.write(stderr.decode('utf-8', errors='replace'))
        # Signal before reaping, retaining the group identity on failures.
        try:
            KILL_GROUP(process.pid, signal.SIGKILL)
        except ProcessLookupError:
            pass
        process.wait()
        for stream in (process.stdin, process.stdout, process.stderr):
            stream.close()


class ProcessObservation:
    """A finite zombie observation; trusted replay owns all real child effects."""
    def __init__(self, status, events):
        self.pid = status['pid']
        self.status = status
        self.returncode = None
        self.events = events

    def poll(self):
        self.events.append(['poll'])
        self.returncode = self.status['returncode']
        return self.returncode

    def wait(self, timeout=None):
        self.events.append(['wait'] if timeout is None else ['wait-timeout', timeout])
        self.returncode = self.status['returncode']
        return self.returncode


def candidate():
    # Enforce limits before importing any candidate module, including snippets
    # launched directly by frozen tests through the nested command boundary.
    module = import_subject()
    request = decoded(sys.stdin.buffer.read(MAX_BYTES + 1))
    if request == {'function': '__ready__'}:
        sys.stdout.buffer.write(encoded({name: callable(getattr(module, name, None)) for name in FUNCTIONS}))
        return
    if (type(request) is not dict or request.keys() != {'function', 'args', 'status', 'empty_changes'}
            or request['function'] not in FUNCTIONS):
        raise GateProtocolError('gate.request-refused')
    args = unpack(request['args'])
    if type(args) is not tuple:
        raise GateProtocolError('gate.arguments-refused')
    events = []
    out, err = io.StringIO(), io.StringIO()
    with contextlib.ExitStack() as stack, contextlib.redirect_stdout(out), contextlib.redirect_stderr(err):
        if request['empty_changes']:
            stack.enter_context(mock.patch.object(module, 'changed_paths', return_value=[]))
            stack.enter_context(mock.patch.object(sys, 'argv', ['gate']))
        if request['status'] is not None:
            process = ProcessObservation(request['status'], events)

            def observe(*arguments):
                events.append(['waitid', pack(arguments)])
                if process.returncode is not None:
                    raise ChildProcessError
                return os.waitid_result((process.pid, 0, 17, process.status['status'], process.status['code']))

            stack.enter_context(mock.patch.object(module.os, 'waitid', side_effect=observe))
            stack.enter_context(mock.patch.object(module.os, 'killpg', side_effect=
                                                lambda *arguments: events.append(['killpg', pack(arguments)])))
            args = (process,)
        try:
            result = {'kind': 'value', 'value': pack(getattr(module, request['function'])(*args))}
        except (ValueError, SyntaxError, subprocess.CalledProcessError) as error:
            result = {'kind': ('git-error' if isinstance(error, subprocess.CalledProcessError) else
                               'syntax-error' if isinstance(error, SyntaxError) else 'value-error')}
        except AttributeError:
            result = {'kind': 'missing'}
    sys.stdout.buffer.write(encoded({'result': result, 'events': events,
                                     'stdout': out.getvalue(), 'stderr': err.getvalue()}))


def replay(events, process, module):
    if type(events) is not list or len(events) > MAX_EVENTS:
        raise GateProtocolError('gate.events-refused')
    for event in events:
        if event in (['poll'], ['wait']):
            getattr(process, event[0])()
        elif (type(event) is list and len(event) == 2 and event[0] == 'wait-timeout'
              and type(event[1]) is int and 0 <= event[1] <= 60):
            process.wait(timeout=event[1])
        elif (type(event) is list and len(event) == 2 and event[0] in ('waitid', 'killpg')):
            args = unpack(event[1])
            if type(args) is not tuple or any(type(item) is not int for item in args):
                raise GateProtocolError('gate.event-arguments')
            if event[0] == 'killpg':
                if args not in ((process.pid, signal.SIGTERM), (process.pid, signal.SIGKILL)):
                    raise GateProtocolError('gate.signal-refused')
                module.os.killpg(*args)
            elif args != (os.P_PID, process.pid, os.WEXITED | os.WNOHANG | os.WNOWAIT):
                raise GateProtocolError('gate.waitid-refused')
            else:
                os.waitid(*args)
        else:
            raise GateProtocolError('gate.event-refused')


def record(stream, kind, value):
    data = encoded({'kind': kind, 'value': value}) + b'\n'
    if stream.tell() + len(data) > MAX_TRANSCRIPT:
        raise GateProtocolError('gate.transcript-limit')
    stream.write(data)
    stream.flush()


def proxy(module, name, counter, transcript):
    def call(*args):
        counter['calls'] += 1
        process = args[0] if name in ('observe_exit', 'stop_check') else None
        status = None
        if process is not None:
            observed = os.waitid(os.P_PID, process.pid, os.WEXITED | os.WNOHANG | os.WNOWAIT)
            if observed is None:
                raise GateProtocolError('gate.running-process-refused')
            status = {'pid': process.pid, 'status': observed.si_status, 'code': observed.si_code,
                      'returncode': observed.si_status if observed.si_code == os.CLD_EXITED else -observed.si_status}
        request = {'function': name, 'args': pack(()) if process else pack(args), 'status': status,
                   'empty_changes': name == 'main'}
        # Diagnostic text is data. Only original trusted assertions decide the
        # verdict; a worker's summary text can never replace their counts.
        try:
            record(transcript, 'request', request)
            response = bounded_call(request)
            record(transcript, 'response', response)
            if (type(response) is not dict or response.keys() != {'result', 'events', 'stdout', 'stderr'}
                    or type(response['stdout']) is not str or type(response['stderr']) is not str):
                raise GateProtocolError('gate.response-refused')
            if process is not None:
                replay(response['events'], process, module)
            elif response['events'] != []:
                raise GateProtocolError('gate.unexpected-events')
            sys.stdout.write(response['stdout'])
            sys.stderr.write(response['stderr'])
            observed = response['result']
            if observed == {'kind': 'value-error'}:
                raise ValueError('gate.candidate-value-error')
            if observed == {'kind': 'syntax-error'}:
                raise SyntaxError('gate.candidate-syntax-error')
            if observed == {'kind': 'git-error'}:
                raise subprocess.CalledProcessError(1, ['git'])
            if observed == {'kind': 'missing'}:
                counter['candidate_exception_failed'] = True
                raise AttributeError('gate.missing-function')
            if type(observed) is not dict or observed.keys() != {'kind', 'value'} or observed['kind'] != 'value':
                raise GateProtocolError('gate.result-refused')
            return unpack(observed['value'])
        except GateProtocolError:
            counter['protocol_failed'] = True
            raise
    return call


def oracle():
    import resource
    resource.setrlimit(resource.RLIMIT_NPROC, (PROCESSES, PROCESSES))
    # The candidate has no procfs and the syscall filter also prevents inspection
    # of trusted objects by numerical PID. Drop dumpability as defense in depth.
    if ctypes.CDLL(None).prctl(4, 0, 0, 0, 0) != 0:
        raise GateProtocolError('gate.dumpability-refused')
    module = import_subject()
    configuration = json.loads(Path('/adapter/config.json').read_bytes())
    counter = {'calls': 0, 'protocol_failed': False, 'adapter_policy_failed': False,
               'candidate_exception_failed': False, 'candidate_ready': False, 'function_found': None}
    install_launch_boundary()
    ready = bounded_call({'function': '__ready__'})
    if (type(ready) is not dict or ready.keys() != FUNCTIONS
            or any(type(value) is not bool for value in ready.values())):
        raise GateProtocolError('gate.startup-refused')
    counter.update(candidate_ready=True, function_found=ready[configuration['function']])
    with Path('/output/wire.log').open('xb') as transcript:
        record(transcript, 'startup', ready)
        for name in FUNCTIONS:
            setattr(module, name, proxy(module, name, counter, transcript))
        spec = importlib.util.spec_from_file_location('hidden_gate', '/adapter/test.py')
        hidden = importlib.util.module_from_spec(spec)
        spec.loader.exec_module(hidden)
        suite = (unittest.defaultTestLoader.loadTestsFromModule(hidden) if configuration['test_names'] is None else
                 unittest.TestSuite(unittest.defaultTestLoader.loadTestsFromName(name, hidden)
                                    for name in configuration['test_names']))
        result = unittest.TextTestRunner(stream=sys.stderr, verbosity=2).run(suite)
    counts = {'tests': result.testsRun, 'failures': len(result.failures), 'errors': len(result.errors),
              'skipped': len(result.skipped), 'expected_failures': len(result.expectedFailures),
              'unexpected_successes': len(result.unexpectedSuccesses)}
    Path('/output/tests.json').write_bytes(encoded({**counter, 'tests': counts}))
    return int(not result.wasSuccessful() or result.testsRun != configuration['expected_tests']
               or any(counter[key] for key in ('protocol_failed', 'adapter_policy_failed', 'candidate_exception_failed')))


def run_request(request):
    """One trusted holder operation; only the oracle receives assertions."""
    import coding_eval as evaluation
    import coding_eval_control as control
    if (type(request) is not dict or request.keys() != {'mode', 'startup_refused'}
            or request['mode'] != 'independent-gate' or type(request['startup_refused']) is not bool):
        raise GateProtocolError('gate.holder-request-refused')
    root = Path('/scratch')
    temporary = root / 'private-gate'
    temporary.mkdir(mode=0o700)
    libc = ctypes.CDLL(None, use_errno=True)
    libc.mount.argtypes = [ctypes.c_char_p, ctypes.c_char_p, ctypes.c_char_p, ctypes.c_ulong, ctypes.c_char_p]
    if libc.mount(b'tmpfs', os.fsencode(temporary), b'tmpfs', 2 | 4,
                  b'size=67108864,nr_inodes=8192,mode=0700') != 0:
        raise GateProtocolError('gate.scratch-mount-refused')
    # Trusted outer process may create the nested candidate user namespaces.
    command = [
        '/usr/bin/bwrap', '--unshare-all', '--unshare-user', '--die-with-parent', '--new-session',
        '--uid', '0', '--gid', '0', '--cap-drop', 'ALL', '--cap-add', 'CAP_SYS_ADMIN', '--cap-add', 'CAP_SETFCAP',
        '--clearenv', '--ro-bind', '/usr', '/usr', '--symlink', 'usr/bin', '/bin',
        '--symlink', 'usr/lib', '/lib', '--symlink', 'usr/lib64', '/lib64', '--proc', '/proc', '--dev', '/dev',
        '--ro-bind', str(root / 'trusted'), '/workspace', '--ro-bind', str(root / 'candidate'), '/candidate',
        '--ro-bind', str(root / 'candidate-runner'), '/candidate-runner',
        '--ro-bind', str(root / 'oracle-adapter'), '/adapter', '--bind', str(root / 'results'), '/output',
        '--bind', str(temporary), '/tmp', '--setenv', 'PATH', '/usr/bin', '--setenv', 'HOME', '/tmp',
        '--setenv', 'LANG', 'C.UTF-8', '--chdir', '/workspace',
        '--remount-ro', '/dev', '--remount-ro', '/', '--',
        '/usr/bin/python3', '-I', '-B', '/adapter/gate.py', 'oracle',
    ]
    code, timed_out, limited = control.run_process(
        ['/usr/bin/false'] if request['startup_refused'] else command,
        Path('/logs'), evaluation.TIMEOUT, False)
    # There is no single candidate worker: original tests also launch children
    # whose nonzero exits/signals are expected observations. Never invent an
    # aggregate exit status for those processes.
    return [code, None, timed_out, limited]


def grade_case(task, files, output, timeout):
    if __package__:
        from . import coding_eval as evaluation
        from . import coding_eval_oracle as oracle_engine
        from . import coding_eval_scratch as scratch
    else:
        import coding_eval as evaluation
        import coding_eval_oracle as oracle_engine
        import coding_eval_scratch as scratch
    if timeout != evaluation.TIMEOUT:
        raise GateProtocolError('gate.deadline-refused')
    hidden = evaluation.pinned_blob(task['reference'], task['hidden_test'], task['hidden_sha256'])
    data = transcript = None
    with oracle_engine.case_workspace(output), scratch.scratch_mount(output) as mount:
        root = mount.root
        for name in ('trusted', 'candidate', 'candidate-runner', 'oracle-adapter', 'results'):
            (root / name).mkdir(mode=0o700)
        for name, content in evaluation.projection(task).items():
            evaluation.write_file(root / 'trusted' / name, content)
        for name, content in files.items():
            evaluation.write_file(root / 'candidate' / name, content)
        source = Path(__file__).read_bytes()
        for name in ('oracle-adapter', 'candidate-runner'):
            evaluation.write_file(root / name / 'gate.py', source)
        evaluation.write_file(root / 'oracle-adapter/test.py', hidden)
        evaluation.publish(root / 'oracle-adapter/config.json', {'test_names': task.get('test_names'),
                                                               'expected_tests': task['expected_tests'],
                                                               'function': oracle_engine.SUPPORTED[task['id']]['function']})
        # Preserve the shared host readiness factory, including its established
        # effect-free failure probe. Never accept arbitrary replacement commands
        # or interpret a failed launch as a successful regression failure.
        arguments = (root / 'candidate', root / 'candidate-runner', root / 'results', 'candidate')
        candidate_factory = oracle_engine.sandbox(*arguments)
        startup_refused = candidate_factory == ['/usr/bin/false']
        if not startup_refused and candidate_factory != oracle_engine.STANDARD_SANDBOX(*arguments):
            raise GateProtocolError('gate.factory-refused')
        mount.used = True
        mount.control.settimeout(evaluation.TIMEOUT + scratch.SETUP_SECONDS)
        mount.control.send(encoded({'mode': 'independent-gate', 'startup_refused': startup_refused}))
        outcome = decoded(mount.control.recv(MAX_BYTES + 1))
        if (type(outcome) is not list or len(outcome) != 4
                or any(item is not None and type(item) is not int for item in outcome[:2])
                or any(type(item) is not bool for item in outcome[2:])):
            raise GateProtocolError('gate.outcome-refused')
        if startup_refused:
            # The observed false exit launched no candidate or assertions.
            # Publish zero counts and the compatibility shutdown marker only;
            # neither can validate a historical base or a generated attempt.
            if outcome != [1, None, False, False]:
                raise GateProtocolError('gate.failed-startup-outcome-refused')
            data = {'tests': {key: 0 for key in ('tests', 'failures', 'errors', 'skipped',
                                                'expected_failures', 'unexpected_successes')},
                    'calls': 0, 'candidate_ready': False, 'function_found': None,
                    'protocol_failed': True, 'adapter_policy_failed': False, 'candidate_exception_failed': False}
            transcript = oracle_engine.worker.encode_message({'done': True})
        else:
            try:
                data = oracle_engine.result_record(root / 'results/tests.json', task)
                transcript = evaluation.read_regular(root / 'results/wire.log', MAX_TRANSCRIPT)
            except (ValueError, OSError):
                data = None
        # The existing supervisor uses these diagnostic basenames.
        for previous, current in (('stdout.log', 'candidate.log'), ('stderr.log', 'oracle.log')):
            if (output / previous).exists():
                (output / previous).rename(output / current)
    if data is not None:
        evaluation.publish(output / 'tests.json', data)
    if transcript is not None:
        evaluation.write_file(output / 'wire.log', transcript)
    return *outcome, data


if __name__ == '__main__':
    if len(sys.argv) == 4 and sys.argv[1] == 'bootstrap':
        # Only trusted code executes until the parent owns this exact child.
        # No procfs or extra descriptors are needed inside the candidate mount.
        if os.getpgrp() != os.getpid():
            os.setsid()
        handshake = Path(sys.argv[2])
        with (handshake / 'ready.pending').open('xb') as ready:
            ready.write(encoded(os.getpid()))
        (handshake / 'ready.pending').rename(handshake / 'ready')
        while not (handshake / 'go').exists():
            time.sleep(0.001)
        command = decoded(sys.argv[3].encode())
        os.execv(command[0], command)
    elif sys.argv[1:] == ['candidate']:
        candidate()
    elif sys.argv[1:] == ['oracle']:
        raise SystemExit(oracle())
    else:
        raise SystemExit(2)
