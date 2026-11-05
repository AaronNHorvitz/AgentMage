"""Independent observations for development-only source evidence fixtures.

Original adapter. Only finite values and inert mock observations cross the
boundary; candidate functions and exception classes never enter the oracle.
"""
from __future__ import annotations

import contextlib
import itertools
import os
from pathlib import Path
import re
import shutil
import stat
import subprocess
from unittest import mock


OPERATIONS = {
    "identity": ("untracked_file_identities", "implementation_identity"),
    "diagnostics": ("_model_request_diagnosis", "diagnose"),
    "closure": ("complete_source_closure", "validate_report", "validate_current_source"),
    "retention": ("retain", "load_stage"),
}
MOCKS = {"identity": ("binary", "subprocess.run"),
         "closure": ("source_records", "complete_source_closure", "subprocess.run"),
         "retention": ("private_names",), "diagnostics": ("binary", "confinement_diagnosis")}
CONSTANTS = {"identity": (), "closure": ("MAX_SOURCE_BYTES", "MAX_SOURCE_FILES"), "retention": (), "diagnostics": ()}
METADATA = {"identity": (), "closure": CONSTANTS['closure'], "retention": ('TOOL',), "diagnostics": ('MODEL_CHOICES',)}
MAX_ARTIFACTS = 128
MAX_ARTIFACT_BYTES = 16384
# The frozen retention reference introduces this constant; its base lacks it.
# Keep the admitted fixture path in trusted code instead of accepting a path
# supplied by the candidate or requiring the older base to define the field.
RETENTION_TOOL = "scripts/verification_batch.py"


def attribute(module, name):
    parent = module
    parts = name.split('.')
    for part in parts[:-1]:
        parent = getattr(parent, part)
    return parent, parts[-1]


def pack_stub(value, codec):
    if isinstance(value, BaseException):
        if type(value) is not ValueError:
            raise codec.AdapterPolicyError("oracle.source-stub-exception-refused")
        return {"kind": "value-error", "message": str(value)}
    if type(value) is subprocess.CompletedProcess:
        return {"kind": "completed", "returncode": value.returncode,
                "stdout": codec.pack_usage_value(value.stdout),
                "stderr": codec.pack_usage_value(value.stderr)}
    return {"kind": "value", "value": codec.pack_usage_value(value)}


def unpack_stub(value, codec):
    if type(value) is not dict:
        raise codec.ProtocolError("oracle.source-stub-refused")
    if value.keys() == {"kind", "value"} and value['kind'] == 'value':
        return codec.unpack_usage_value(value['value'])
    if (value.keys() == {"kind", "message"} and value['kind'] == 'value-error'
            and type(value['message']) is str):
        return ValueError(value['message'])
    if (value.keys() == {"kind", "returncode", "stdout", "stderr"} and value['kind'] == 'completed'
            and type(value['returncode']) is int):
        return subprocess.CompletedProcess([], value['returncode'],
                                           stdout=codec.unpack_usage_value(value['stdout']),
                                           stderr=codec.unpack_usage_value(value['stderr']))
    raise codec.ProtocolError("oracle.source-stub-refused")


def mock_snapshot(value, codec):
    if not isinstance(value, (mock.MagicMock, mock.Mock)):
        raise codec.AdapterPolicyError("oracle.source-mock-refused")
    if value._mock_wraps is not None:
        raise codec.AdapterPolicyError("oracle.source-mock-callable-refused")
    effect = value.side_effect
    if effect is None:
        return {"mode": "return", "value": pack_stub(value.return_value, codec)}
    if isinstance(effect, BaseException):
        return {"mode": "raise", "value": pack_stub(effect, codec)}
    if callable(effect):
        raise codec.AdapterPolicyError("oracle.source-mock-callable-refused")
    # tee retains the test's iterator and its remaining sequence. A fixed cap
    # prevents an infinite or oversized fixture from blocking the trusted side.
    retained, observer = itertools.tee(effect)
    value.side_effect = retained
    entries = list(itertools.islice(observer, 129))
    if len(entries) > 128:
        raise codec.AdapterPolicyError("oracle.source-mock-limit")
    return {"mode": "sequence", "values": [pack_stub(item, codec) for item in entries]}


def install_mock(stack, parent, name, spec, codec):
    if type(spec) is not dict:
        raise codec.ProtocolError("oracle.source-mock-refused")
    value = stack.enter_context(mock.patch.object(parent, name, create=True))
    if spec.keys() == {'mode', 'values'} and spec['mode'] == 'sequence' and type(spec['values']) is list:
        if len(spec['values']) > 128:
            raise codec.ProtocolError("oracle.source-mock-limit")
        value.side_effect = [unpack_stub(item, codec) for item in spec['values']]
    elif spec.keys() == {'mode', 'value'} and spec['mode'] in ('return', 'raise'):
        decoded = unpack_stub(spec['value'], codec)
        if spec['mode'] == 'raise':
            if not isinstance(decoded, ValueError):
                raise codec.ProtocolError("oracle.source-mock-exception-refused")
            value.side_effect = decoded
        else:
            value.return_value = decoded
    else:
        raise codec.ProtocolError("oracle.source-mock-refused")
    return value


def temporary_root(path, codec):
    text = str(path)
    parts = text.split('/')
    if (len(parts) < 3 or parts[:2] != ['', 'tmp'] or not parts[2]
            or any(part in ('', '.', '..') or '\x00' in part for part in parts[2:])):
        raise codec.AdapterPolicyError("oracle.source-fixture-path-refused")
    return Path('/tmp') / parts[2]


def record_bytes(root, relative, codec):
    """Inspect actual candidate fixture bytes without following its aliases."""
    if relative.is_absolute() or not relative.parts or any(part in ('.', '..') for part in relative.parts):
        raise codec.ProtocolError("oracle.source-record-path-refused")
    descriptor = os.open(root, os.O_RDONLY | os.O_DIRECTORY | os.O_NOFOLLOW | os.O_CLOEXEC)
    try:
        for part in relative.parts[:-1]:
            child = os.open(part, os.O_RDONLY | os.O_DIRECTORY | os.O_NOFOLLOW | os.O_CLOEXEC,
                            dir_fd=descriptor)
            os.close(descriptor)
            descriptor = child
        try:
            child = os.open(relative.name, os.O_RDONLY | os.O_NOFOLLOW | os.O_NONBLOCK | os.O_CLOEXEC,
                            dir_fd=descriptor)
        except FileNotFoundError:
            return None
        with os.fdopen(child, 'rb') as stream:
            info = os.fstat(stream.fileno())
            if not stat.S_ISREG(info.st_mode):
                raise codec.AdapterPolicyError("oracle.source-record-type-refused")
            data = stream.read(16385)
        if len(data) > 16384:
            raise codec.AdapterPolicyError("oracle.source-record-limit")
        return data
    finally:
        os.close(descriptor)


def output_snapshot(destination, codec, *, root=Path('/candidate-tmp')):
    """Observe real files through held directories on the read-only mount."""
    temporary_root(destination, codec)
    relative = destination.relative_to('/tmp')
    descriptor = os.open(root, os.O_RDONLY | os.O_DIRECTORY | os.O_NOFOLLOW | os.O_CLOEXEC)
    result = []
    total = 0
    def observe(directory, prefix):
        nonlocal total
        result.append({'path': prefix or '.', 'data': None})
        with os.scandir(directory) as entries:
            for entry in entries:
                name = f'{prefix}/{entry.name}' if prefix else entry.name
                if len(result) >= MAX_ARTIFACTS:
                    raise codec.AdapterPolicyError("oracle.source-output-limit")
                is_directory = not prefix
                pattern = r'[a-z0-9][a-z0-9-]{0,63}' if is_directory else r'(?:stage\.json|[0-9]{2}\.log)'
                if not re.fullmatch(pattern, entry.name):
                    raise codec.AdapterPolicyError("oracle.source-output-path-refused")
                flags = os.O_RDONLY | os.O_NOFOLLOW | os.O_NONBLOCK | os.O_CLOEXEC
                if is_directory:
                    flags |= os.O_DIRECTORY
                child = os.open(entry.name, flags, dir_fd=directory)
                try:
                    info = os.fstat(child)
                    if is_directory:
                        observe(child, name)
                    else:
                        if not stat.S_ISREG(info.st_mode) or info.st_size > MAX_ARTIFACT_BYTES:
                            raise codec.AdapterPolicyError("oracle.source-output-type-refused")
                        with os.fdopen(os.dup(child), 'rb') as stream:
                            data = stream.read(MAX_ARTIFACT_BYTES + 1)
                        total += len(data)
                        if total > MAX_ARTIFACT_BYTES:
                            raise codec.AdapterPolicyError("oracle.source-output-limit")
                        after = os.fstat(child)
                        if any(getattr(info, name) != getattr(after, name)
                               for name in ('st_size', 'st_mtime_ns', 'st_ctime_ns')):
                            raise codec.AdapterPolicyError("oracle.source-output-changed")
                        result.append({'path': name, 'data': data.hex()})
                finally:
                    os.close(child)
    try:
        for part in relative.parts:
            try:
                child = os.open(part, os.O_RDONLY | os.O_DIRECTORY | os.O_NOFOLLOW | os.O_CLOEXEC,
                                dir_fd=descriptor)
            except FileNotFoundError:
                return None
            os.close(descriptor)
            descriptor = child
        observe(descriptor, '')
        return result
    except OSError:
        raise codec.AdapterPolicyError("oracle.source-output-observation-refused") from None
    finally:
        os.close(descriptor)


def restore_output(destination, entries, codec):
    """Validate the whole observation before writing any disposable test data."""
    if entries is None:
        return
    if type(entries) is not list or len(entries) > MAX_ARTIFACTS:
        raise codec.ProtocolError("oracle.source-output-refused")
    decoded = {}
    total = 0
    for row in entries:
        if (type(row) is not dict or row.keys() != {'path', 'data'} or type(row['path']) is not str
                or row['path'] in decoded):
            raise codec.ProtocolError("oracle.source-output-refused")
        name = row['path']
        # These frozen retain fixtures have no attachments. A candidate may
        # report only a stage directory, its stage JSON and numbered text logs.
        if name != '.' and not re.fullmatch(r'[a-z0-9][a-z0-9-]{0,63}(?:/(?:stage\.json|[0-9]{2}\.log))?', name):
            raise codec.ProtocolError("oracle.source-output-path-refused")
        data = row['data']
        if data is not None:
            if type(data) is not str or not re.fullmatch(r'(?:[0-9a-f]{2})*', data):
                raise codec.ProtocolError("oracle.source-output-refused")
            data = bytes.fromhex(data)
            total += len(data)
        decoded[name] = data
    if total > MAX_ARTIFACT_BYTES or decoded.get('.', False) is not None:
        raise codec.ProtocolError("oracle.source-output-limit")
    for name in decoded:
        if name != '.':
            for parent in Path(name).parents:
                if decoded.get(str(parent), False) is not None:
                    raise codec.ProtocolError("oracle.source-output-parent-refused")
    # The destination comes from trusted test arguments. Never overwrite an
    # alias or a special fixture entry, including one observed on an earlier RPC.
    for path in (destination, *destination.parents):
        if path.is_symlink() or path.exists() and not path.is_dir():
            raise codec.ProtocolError("oracle.source-output-target-refused")
    for name, data in sorted(decoded.items(), key=lambda row: len(Path(row[0]).parts)):
        path = destination / name
        if path.is_symlink():
            raise codec.ProtocolError("oracle.source-output-target-refused")
        if data is None:
            path.mkdir(exist_ok=True, parents=True)
        else:
            if path.is_symlink() or path.exists() and not path.is_file():
                raise codec.ProtocolError("oracle.source-output-target-refused")
            path.write_bytes(data)


def copy_fixture_file(source, destination):
    # A frozen inventory fixture deliberately includes a FIFO. Recreate that
    # inert node instead of opening it or changing the original hidden method.
    if stat.S_ISFIFO(os.lstat(source).st_mode):
        os.mkfifo(destination, stat.S_IMODE(os.lstat(source).st_mode))
        return destination
    return shutil.copy2(source, destination, follow_symlinks=False)


def result(module, request, codec, family):
    if request == {'operation': 'constants'}:
        names = METADATA[family]
        values = {name: getattr(module, name) for name in names if hasattr(module, name)}
        return {'observation': {'kind': 'value', 'value': codec.pack_usage_value(values)},
                'calls': {}, 'artifacts': None}
    if (type(request) is not dict
            or request.keys() != {'operation', 'args', 'kwargs', 'root', 'constants', 'mocks',
                                  *(('process_arguments',) if family == 'diagnostics' else ())}
            or request['operation'] not in OPERATIONS[family]
            or type(request['mocks']) is not dict or not set(request['mocks']).issubset(MOCKS[family])
            or type(request['constants']) is not dict
            or not set(request['constants']).issubset(CONSTANTS[family])):
        raise codec.ProtocolError("oracle.source-request-refused")
    args = codec.unpack_usage_value(request['args'])
    kwargs = codec.unpack_usage_value(request['kwargs'])
    if type(args) is not tuple or type(kwargs) is not dict:
        raise codec.ProtocolError("oracle.source-arguments-refused")
    with contextlib.ExitStack() as stack:
        root = Path(request['root'])
        argument_fixture = ((family == 'diagnostics' and request['operation'] == 'diagnose')
                            or (family == 'identity' and request['operation'] == 'untracked_file_identities'))
        fixture_path = args[0] if argument_fixture else root
        if ((family != 'diagnostics' and fixture_path != Path('/workspace'))
                or (family == 'diagnostics' and request['operation'] == 'diagnose')):
            fixture = temporary_root(fixture_path, codec)
            # Only this candidate-private copy is writable. The oracle fixture
            # is mounted read-only and contains data, never hidden test source.
            if fixture.exists():
                shutil.rmtree(fixture)
            shutil.copytree(Path('/trusted-tmp') / fixture.name, fixture, symlinks=True,
                            copy_function=copy_fixture_file)
        stack.enter_context(mock.patch.object(module, 'ROOT', root))
        if family == 'diagnostics':
            arguments = codec.unpack_usage_value(request['process_arguments'])
            if arguments is not None and (type(arguments) is not list or any(type(item) is not bytes for item in arguments)):
                raise codec.ProtocolError("oracle.source-process-arguments-refused")
            stack.enter_context(mock.patch.object(module, '_owned_process_arguments', return_value=arguments, create=True))
            stack.enter_context(mock.patch.object(module, 'exact_running_process', return_value=arguments is not None))
        for name, value in request['constants'].items():
            if type(value) is not int or value < 0:
                raise codec.ProtocolError("oracle.source-constant-refused")
            stack.enter_context(mock.patch.object(module, name, value, create=True))
        mocks = {name: install_mock(stack, *attribute(module, name), spec, codec)
                 for name, spec in request['mocks'].items()}
        try:
            value = getattr(module, request['operation'])(*args, **kwargs)
            observed = {'kind': 'value', 'value': codec.pack_usage_value(value)}
        except ValueError as exception:
            observed = {'kind': 'value-error', 'message': str(exception)[:4096]}
        except subprocess.CalledProcessError:
            observed = {'kind': 'git-error'}
        except RuntimeError as exception:
            if family != 'identity' or not isinstance(exception, module.HarnessError):
                raise
            observed = {'kind': 'harness-error', 'message': str(exception)[:4096]}
        # subprocess kwargs can contain bytes. Keep the wire codec typed.
        calls = {name: [{'args': codec.pack_usage_value(call.args), 'kwargs': codec.pack_usage_value(call.kwargs)}
                        for call in value.call_args_list] for name, value in mocks.items()}
        return {'observation': observed, 'calls': calls, 'artifacts': None}


def proxies(remote, module, codec, family):
    constants = remote({'operation': 'constants'})
    if (type(constants) is not dict or constants.keys() != {'observation', 'calls', 'artifacts'}
            or constants['calls'] != {} or constants['artifacts'] is not None
            or type(constants['observation']) is not dict
            or constants['observation'].keys() != {'kind', 'value'}
            or constants['observation']['kind'] != 'value'):
        raise codec.ProtocolError("oracle.source-constants-refused")
    values = codec.unpack_usage_value(constants['observation']['value'])
    names = METADATA[family]
    if type(values) is not dict or not set(values).issubset(names):
        raise codec.ProtocolError("oracle.source-constants-refused")
    for name, value in values.items():
        if (type(value) is not (str if name == 'TOOL' else tuple if name == 'MODEL_CHOICES' else int)
                or name == 'MODEL_CHOICES' and any(type(item) is not str for item in value)):
            raise codec.ProtocolError("oracle.source-constant-type-refused")
        # This path is used by trusted fixture builders. Candidate metadata
        # cannot redirect their writes, even within the oracle's private mounts.
        if name == 'TOOL' and value != RETENTION_TOOL:
            raise codec.ProtocolError("oracle.source-tool-constant-refused")
        setattr(module, name, value)

    def invoke_unchecked(operation, *args, **kwargs):
        mocks = {}
        originals = {}
        for name in MOCKS[family]:
            parent, key = attribute(module, name)
            value = getattr(parent, key, None)
            if isinstance(value, (mock.MagicMock, mock.Mock)):
                mocks[name] = mock_snapshot(value, codec)
                originals[name] = (parent, key, value, list(value.call_args_list))
        request = {'operation': operation, 'args': codec.pack_usage_value(args),
                   'kwargs': codec.pack_usage_value(kwargs), 'root': str(module.ROOT),
                   'constants': {name: getattr(module, name) for name in CONSTANTS[family]
                                 if hasattr(module, name)}, 'mocks': mocks}
        if family == 'diagnostics':
            arguments = None
            record_before = relative_record = None
            if operation == 'diagnose':
                temporary_root(args[0], codec)
                state, _, _ = module.paths(args[0])
                relative_record = (state / module.RUN_RECORD).relative_to('/tmp')
                record_before = record_bytes(Path('/tmp'), relative_record, codec)
                record = module.read_run_record(state)
                if record and module.exact_running_process(record, args[0]):
                    arguments = (Path('/proc') / str(record['pid']) / 'cmdline').read_bytes().split(b'\x00')
            request['process_arguments'] = codec.pack_usage_value(arguments)
        if module.ROOT != Path('/workspace'):
            temporary_root(module.ROOT, codec)
        destination = None
        if family == 'retention' and operation == 'retain':
            destination = args[2]
            if temporary_root(destination, codec) != temporary_root(module.ROOT, codec):
                raise codec.ProtocolError("oracle.source-output-target-refused")
        response = remote(request)
        if family == 'diagnostics' and relative_record is not None:
            try:
                unchanged = record_bytes(Path('/candidate-tmp'), relative_record, codec) == record_before
            except OSError:
                unchanged = False
            if not unchanged:
                raise codec.AdapterPolicyError("oracle.source-run-record-mutated")
        if (type(response) is not dict or response.keys() != {'observation', 'calls', 'artifacts'}
                or type(response['calls']) is not dict or response['calls'].keys() != originals.keys()):
            raise codec.ProtocolError("oracle.source-result-refused")
        decoded = {}
        for name, calls in response['calls'].items():
            if type(calls) is not list or len(calls) > 128:
                raise codec.ProtocolError("oracle.source-call-limit")
            decoded[name] = []
            for call in calls:
                if type(call) is not dict or call.keys() != {'args', 'kwargs'}:
                    raise codec.ProtocolError("oracle.source-call-refused")
                call_args = codec.unpack_usage_value(call['args'])
                call_kwargs = codec.unpack_usage_value(call['kwargs'])
                if type(call_args) is not tuple or type(call_kwargs) is not dict:
                    raise codec.ProtocolError("oracle.source-call-refused")
                decoded[name].append((call_args, call_kwargs))
        observed = response['observation']
        if type(observed) is not dict:
            raise codec.ProtocolError("oracle.source-observation-refused")
        if observed.keys() == {'kind', 'value'} and observed['kind'] == 'value':
            value = codec.unpack_usage_value(observed['value'])
            error = None
        elif (observed.keys() == {'kind', 'message'} and observed['kind'] == 'value-error'
              and type(observed['message']) is str and len(observed['message']) <= 4096):
            value, error = None, getattr(module, 'RecordError', ValueError)(observed['message'])
        elif observed == {'kind': 'git-error'}:
            value, error = None, subprocess.CalledProcessError(1, ['git'])
        elif (family == 'identity' and observed.keys() == {'kind', 'message'}
              and observed['kind'] == 'harness-error' and type(observed['message']) is str
              and len(observed['message']) <= 4096):
            value, error = None, module.HarnessError(observed['message'])
        else:
            raise codec.ProtocolError("oracle.source-observation-refused")
        for name, (parent, key, original, history) in originals.items():
            if getattr(parent, key) is not original or original.call_args_list != history:
                remote.policy_failed = True
                raise codec.AdapterPolicyError("oracle.source-mock-history-changed")
        if response['artifacts'] is not None:
            raise codec.ProtocolError("oracle.source-unexpected-output")
        if destination is not None:
            restore_output(destination, output_snapshot(destination, codec), codec)
        for name, calls in decoded.items():
            original = originals[name][2]
            for call_args, call_kwargs in calls:
                try:
                    original(*call_args, **call_kwargs)
                except (ValueError, StopIteration):
                    pass
        if error is not None:
            raise error
        return value

    def invoke(operation, *args, **kwargs):
        try:
            return invoke_unchecked(operation, *args, **kwargs)
        except codec.ProtocolError:
            remote.failed = True
            raise
        except codec.AdapterPolicyError:
            remote.policy_failed = True
            raise

    for operation in OPERATIONS[family]:
        setattr(module, operation, lambda *args, _operation=operation, **kwargs: invoke(_operation, *args, **kwargs))
    return getattr(module, OPERATIONS[family][0])
