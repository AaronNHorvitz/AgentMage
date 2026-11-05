"""Separate frozen Rust corpus assertions from candidate path construction.

Original evaluation adapter. Only bounded constructor observations cross pipes;
the original corpus and test run in a namespace containing no candidate source.
This diagnostic adapter grants no generated-candidate admission.
"""
from __future__ import annotations

import contextlib
import ctypes
import json
import os
from pathlib import Path
import re
import shutil
import socket
import subprocess
import sys


MAX_CALLS = 640
REQUEST_PREFIX = b"coding-path-request:"

CANDIDATE = r'''
use agentmage_kernel_contracts::{WorkspaceId, WorkspacePath};
use std::io::{BufRead, Write};
fn main() {
    for line in std::io::stdin().lock().lines() {
        let input: serde_json::Value = serde_json::from_str(&line.unwrap()).unwrap();
        let components: Vec<String> = serde_json::from_value(input["components"].clone()).unwrap();
        let result = WorkspacePath::new(WorkspaceId::from_raw(input["workspace"].as_str().unwrap()), components);
        let observation = match result {
            Ok(path) => serde_json::json!({"components": path.components().iter().map(|c| c.as_str()).collect::<Vec<_>>() }),
            Err(error) => serde_json::json!({"code": error.kind().code(), "index": error.component_index()}),
        };
        println!("{}", observation);
        std::io::stdout().flush().unwrap();
    }
}
'''.encode()

# The trusted shim carries observations only. In particular it does not run
# baseline validation over returned components, which could conceal a defect.
# Original corpus/test bytes and assertions are never rewritten.
CONSTRUCTOR = r'''
        use std::io::{BufRead, Write};
        let candidates: Vec<String> = components.into_iter().map(Into::into).collect();
        let request = serde_json::json!({"workspace": workspace_id.as_str(), "components": candidates});
        println!("coding-path-request:{}", request);
        std::io::stdout().flush().expect("observation request");
        let mut line = String::new();
        std::io::stdin().lock().read_line(&mut line).expect("observation reply");
        let observation: serde_json::Value = serde_json::from_str(&line).expect("bounded observation");
        if let Some(values) = observation.get("components") {
            let components: Vec<String> = serde_json::from_value(values.clone()).expect("component values");
            return Ok(Self { workspace_id, components: components.into_iter().map(WorkspacePathComponent).collect() });
        }
        let code = observation["code"].as_str().expect("error code").to_owned();
        let index = observation["index"].as_u64().map(|v| v as usize);
        Err(WorkspacePathError { kind: WorkspacePathErrorKind::OracleObservation(Box::leak(code.into_boxed_str())), component_index: index })
'''.encode()


def trusted_source(source):
    """Replace only the pinned baseline constructor with our transport shim."""
    source = source.replace(b"pub enum WorkspacePathErrorKind {", b'''pub enum WorkspacePathErrorKind {
    /// An independently observed, bounded constructor error.
    OracleObservation(&'static str),''', 1)
    source = source.replace(b"match self {", b"match self {\n            Self::OracleObservation(code) => code,", 1)
    start = source.index(b"        validate_workspace_id(&workspace_id)?;")
    end = source.index(b"\n    }\n", start)
    return source[:start] + CONSTRUCTOR + source[end:]


def bounded_string(value, limit):
    if type(value) is not str:
        return False
    try:
        return len(value.encode('utf-8')) <= limit
    except UnicodeError:
        return False


def valid_request(value):
    return (type(value) is dict and value.keys() == {"workspace", "components"}
            and bounded_string(value['workspace'], 128)
            and type(value['components']) is list and len(value['components']) <= 256
            and all(bounded_string(item, 4096) for item in value['components']))


def valid_observation(value):
    return (type(value) is dict and (
        value.keys() == {'components'} and type(value['components']) is list
        and len(value['components']) <= 256
        and all(bounded_string(item, 4096) for item in value['components'])
        or value.keys() == {'code', 'index'} and type(value['code']) is str
        and re.fullmatch(r'[a-z][a-z0-9_.]{0,127}', value['code']) is not None
        and (value['index'] is None or type(value['index']) is int and 0 <= value['index'] <= 256)))


def compile_binary(role):
    """Compile only original installed dependencies, offline, with one job."""
    command = ['/toolchain/bin/cargo', 'build' if role == 'candidate' else 'test',
               '--locked', '--offline', '--jobs', '1', '--target-dir', '/tmp/target',
               '--manifest-path', '/workspace/kernel/contracts/Cargo.toml',
               *(['--example', 'eval_path'] if role == 'candidate' else ['--test', 'eval_hidden', '--no-run']),
               '--message-format=json']
    with Path('/tmp/compiler.jsonl').open('xb') as stream:
        result = subprocess.run(command, stdout=stream, check=False)
    if result.returncode:
        raise ValueError('oracle.rust-compile-failed')
    with Path('/tmp/compiler.jsonl').open('rb') as stream:
        raw = stream.read(2 * 1024 * 1024 + 1)
    if len(raw) > 2 * 1024 * 1024:
        raise ValueError('oracle.rust-compile-output-limit')
    executables = [message['executable'] for message in map(json.loads, raw.splitlines())
                   if message.get('reason') == 'compiler-artifact'
                   and message.get('target', {}).get('name') == ('eval_path' if role == 'candidate' else 'eval_hidden')
                   and message.get('executable')]
    if len(executables) != 1 or not executables[0].startswith('/tmp/target/'):
        raise ValueError('oracle.rust-executable-refused')
    return executables[0]


def candidate(codec, writer):
    executable = compile_binary('candidate')
    process = subprocess.Popen([executable], stdin=subprocess.PIPE, stdout=subprocess.PIPE)
    try:
        writer.write(codec.encode_message({'ready': True, 'function_found': True}))
        writer.flush()
        for sequence in range(1, MAX_CALLS + 2):
            request, _ = codec.read_message(sys.stdin.buffer)
            if request == {'done': True}:
                return 0
            if (request.keys() != {'seq', 'args'} or type(request['seq']) is not int
                    or request['seq'] != sequence or sequence > MAX_CALLS
                    or type(request['args']) is not list or len(request['args']) != 1
                    or not valid_request(request['args'][0])):
                raise codec.ProtocolError('oracle.rust-request-refused')
            process.stdin.write(codec.encode_message(request['args'][0]))
            process.stdin.flush()
            observation, _ = codec.read_message(process.stdout)
            writer.write(codec.response_message(sequence, {'value': observation}))
            writer.flush()
    finally:
        process.kill()
        process.wait()
        process.stdin.close()
        process.stdout.close()


def oracle(codec, writer):
    executable = compile_binary('oracle')
    counts = {key: 0 for key in ('tests', 'failures', 'errors', 'skipped', 'expected_failures', 'unexpected_successes')}
    with Path('/output/wire.log').open('xb') as transcript:
        # This copy is loaded only in these Rust workers. Python limits stay 128.
        codec.MAX_CALLS = MAX_CALLS
        remote = codec.RemoteFunction(sys.stdin.buffer, writer, transcript, 'bool-map')
        process = None
        trusted_output = bytearray()
        try:
            remote.start()
            process = subprocess.Popen([executable, '--nocapture', '--test-threads=1'],
                                       stdin=subprocess.PIPE, stdout=subprocess.PIPE)
            while True:
                line = process.stdout.readline(codec.MAX_MESSAGE + 1)
                if not line:
                    break
                if len(line) > codec.MAX_MESSAGE:
                    raise codec.ProtocolError('oracle.rust-trusted-line-limit')
                # Rust's test harness can prefix a constructor's first line.
                position = line.find(REQUEST_PREFIX)
                if position >= 0:
                    request, _ = codec.decode_message(line[position + len(REQUEST_PREFIX):])
                    if not valid_request(request):
                        raise codec.ProtocolError('oracle.rust-trusted-request-refused')
                    response = remote(request)
                    if not valid_observation(response):
                        remote.policy_failed = True
                        raise codec.AdapterPolicyError('oracle.rust-observation-refused')
                    process.stdin.write(codec.encode_message(response))
                    process.stdin.flush()
                else:
                    trusted_output.extend(line)
                    if len(trusted_output) > codec.MAX_TRANSCRIPT:
                        raise codec.ProtocolError('oracle.rust-trusted-output-limit')
                    sys.stderr.buffer.write(line)
            code = process.wait()
            summaries = re.findall(rb'^test result: (ok|FAILED)\. ([0-9]+) passed; ([0-9]+) failed; ([0-9]+) ignored; ([0-9]+) measured; ([0-9]+) filtered out;[^\n]*$', trusted_output, re.M)
            if len(summaries) != 1:
                raise codec.ProtocolError('oracle.rust-trusted-summary-refused')
            status, passed, failed, skipped, measured, filtered = summaries[0]
            passed, failed, skipped, measured, filtered = map(int, (passed, failed, skipped, measured, filtered))
            if (measured or filtered or passed + failed + skipped != 1
                    or (status == b'ok') != (code == 0 and failed == 0)):
                raise codec.ProtocolError('oracle.rust-trusted-summary-refused')
            counts.update(tests=1, failures=failed, skipped=skipped)
        except (codec.ProtocolError, codec.AdapterPolicyError, codec.CandidateExecutionError):
            # Failed observations cannot disappear when the Rust test panics.
            if remote.candidate_ready:
                counts.update(tests=1, errors=1)
            remote.failed = remote.failed or not (remote.policy_failed or remote.exception_failed)
        finally:
            if process is not None:
                if process.poll() is None:
                    process.kill()
                process.wait()
                process.stdin.close()
                process.stdout.close()
        try:
            remote.finish()
        except codec.ProtocolError:
            pass
        Path('/output/tests.json').write_text(json.dumps({
            'tests': counts, 'calls': remote.calls, 'protocol_failed': remote.failed,
            'adapter_policy_failed': remote.policy_failed, 'candidate_exception_failed': remote.exception_failed,
            'candidate_ready': remote.candidate_ready, 'function_found': remote.function_found}) + '\n')
        return int(not (counts['tests'] == 1 and not any(value for key, value in counts.items() if key != 'tests')
                       and remote.calls > 0 and not remote.failed and not remote.policy_failed and not remote.exception_failed))


def worker_main():
    import resource
    resource.setrlimit(resource.RLIMIT_NPROC, (16, 16))
    resource.setrlimit(resource.RLIMIT_AS, (768 * 1024 * 1024,) * 2)
    os.sched_setaffinity(0, sorted(os.sched_getaffinity(0))[:4])
    sys.path.insert(0, '/adapter')
    import worker as codec
    with os.fdopen(os.dup(1), 'wb') as writer, contextlib.redirect_stdout(sys.stderr):
        os.dup2(2, 1)
        return (candidate if sys.argv[1] == 'candidate' else oracle)(codec, writer)


def holder_request_valid(request):
    if (type(request) is not dict or request.keys() != {'mode', 'timeout', 'candidate', 'oracle'}
            or request['mode'] != 'independent-rust' or type(request['timeout']) not in (int, float)
            or not 0 < request['timeout'] <= 90):
        return False
    for command in (request['candidate'], request['oracle']):
        if (type(command) is not list or not 1 <= len(command) <= 256
                or any(type(value) is not str or len(value) > 4096 or '\0' in value for value in command)):
            return False
        # Match the existing evaluator's effect-free launch-failure probe.
        if command == ['/usr/bin/false']:
            continue
        if command[:9] != ['/usr/bin/bwrap', '--unshare-all', '--unshare-user', '--die-with-parent',
                           '--new-session', '--disable-userns', '--clearenv', '--cap-drop', 'ALL']:
            return False
    return True


def sandbox(root, role, independent):
    """Build on the host so existing lifecycle fault injection also covers Rust."""
    result = independent.sandbox(root / ('trusted' if role == 'oracle' else 'candidate'),
                                 root / (role + '-adapter'), root / 'results', role,
                                 private_tmp=root / ('private-' + role))
    if result == ['/usr/bin/false']:
        return result
    boundary = result.index('--remount-ro')
    result[boundary:boundary] = [
        '--ro-bind', '/runtime/toolchain', '/toolchain',
        '--bind', str(root / ('private-' + role) / 'cargo'), '/cargo',
        '--ro-bind', '/runtime/registry', '/cargo/registry',
        '--setenv', 'CARGO_HOME', '/cargo', '--setenv', 'CARGO_NET_OFFLINE', 'true',
        '--setenv', 'PATH', '/toolchain/bin:/usr/bin',
        '--setenv', 'RUSTFLAGS', '-Ccodegen-units=1 -Clink-arg=-Wl,--threads=2']
    return result


def run_request(request):
    """Trusted holder: bounded host commands and fixed runtime/scratch mounts."""
    import coding_eval_control as control
    import coding_eval_oracle as independent
    import coding_eval_scratch as scratch
    if not holder_request_valid(request):
        raise scratch.ScratchError('oracle.rust-holder-request-refused')
    root = Path('/scratch')
    libc = ctypes.CDLL(None, use_errno=True)
    libc.mount.argtypes = [ctypes.c_char_p, ctypes.c_char_p, ctypes.c_char_p, ctypes.c_ulong, ctypes.c_char_p]
    for name, size, inodes in (('private-candidate', control.RUST_TMP_BYTES, control.RUST_TMP_INODES),
                               ('private-oracle', control.RUST_TMP_BYTES, control.RUST_TMP_INODES)):
        directory = root / name
        directory.mkdir(mode=0o700)
        if libc.mount(b'tmpfs', os.fsencode(directory), b'tmpfs', 6,
                      f'size={size},nr_inodes={inodes},mode=0700'.encode()) != 0:
            raise scratch.ScratchError('oracle.rust-private-mount-refused')
        (directory / 'cargo').mkdir(mode=0o700)
    if libc.mount(b'tmpfs', b'/runtime', b'tmpfs', 6,
                  f'size={control.RUST_RUNTIME_BYTES},nr_inodes={control.RUST_RUNTIME_INODES},mode=0700'.encode()) != 0:
        raise scratch.ScratchError('oracle.rust-runtime-mount-refused')
    for name in ('toolchain', 'registry'):
        shutil.copytree(Path('/runtime-input') / name, Path('/runtime') / name, symlinks=True)
    return independent.exchange(request['candidate'], request['oracle'], Path('/logs'), request['timeout'], rust=True)


def grade_case(task, files, output, timeout=90):
    """The caller owns historical admission and resource observation."""
    if __package__:
        from . import coding_eval as evaluation, coding_eval_oracle as independent, coding_eval_scratch as scratch
    else:
        import coding_eval as evaluation, coding_eval_oracle as independent, coding_eval_scratch as scratch
    runtime = evaluation.coding_eval_rust.runtime_paths(evaluation)
    with independent.case_workspace(output), scratch.scratch_mount(output, runtime_paths=runtime) as mount:
        root = mount.root
        for role, content in (('candidate', files), ('oracle', evaluation.projection(task))):
            workspace = root / ('candidate' if role == 'candidate' else 'trusted')
            adapter = root / (role + '-adapter')
            workspace.mkdir(mode=0o700)
            adapter.mkdir(mode=0o700)
            for name, data in content.items():
                if role == 'oracle' and name == task['source']:
                    data = trusted_source(data)
                evaluation.write_file(workspace / name, data)
            if role == 'candidate':
                evaluation.write_file(workspace / 'kernel/contracts/examples/eval_path.rs', CANDIDATE)
            else:
                evaluation.coding_eval_rust.materialize(workspace, task, evaluation)
            evaluation.write_file(adapter / 'run.py', Path(__file__).read_bytes())
            evaluation.write_file(adapter / 'worker.py', Path(independent.worker.__file__).read_bytes())
        results = root / 'results'
        results.mkdir(mode=0o700)
        mount.used = True
        mount.control.settimeout(timeout + 5)
        prefix = str(root) + '/'
        def local(command):
            return ['/scratch/' + value[len(prefix):] if value.startswith(prefix) else value for value in command]
        request = {'mode': 'independent-rust', 'timeout': timeout,
                   **{role: local(sandbox(root, role, independent)) for role in ('candidate', 'oracle')}}
        if not holder_request_valid(request):
            raise evaluation.EvaluationError('oracle.rust-holder-request-refused')
        encoded = json.dumps(request, allow_nan=False).encode()
        if len(encoded) > 65536:
            raise evaluation.EvaluationError('oracle.rust-holder-request-refused')
        mount.control.send(encoded)
        response, ancillary, flags, _ = mount.control.recvmsg(4096)
        if ancillary or flags & (socket.MSG_TRUNC | socket.MSG_CTRUNC):
            raise evaluation.EvaluationError('oracle.rust-outcome-refused')
        outcome = json.loads(response)
        if (type(outcome) is not list or len(outcome) != 4
                or any(value is not None and type(value) is not int for value in outcome[:2])
                or any(type(value) is not bool for value in outcome[2:])):
            raise evaluation.EvaluationError('oracle.rust-outcome-refused')
        try:
            record = independent.result_record(results / 'tests.json', task)
        except (OSError, ValueError):
            record = None
        transcript = evaluation.read_regular(results / 'wire.log', independent.worker.MAX_TRANSCRIPT) if (results / 'wire.log').exists() else b''
    evaluation.write_file(output / 'wire.log', transcript)
    if record is not None:
        evaluation.publish(output / 'tests.json', record)
    return (*outcome, record)


if __name__ == '__main__':
    raise SystemExit(worker_main())
