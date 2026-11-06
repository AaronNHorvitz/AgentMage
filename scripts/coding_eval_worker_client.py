"""Confined worker API subject for independent frozen assertions."""
import contextlib
import importlib
import importlib.util
import os
import sys
import types
from pathlib import Path

spec = importlib.util.spec_from_file_location('graph', str(Path(__file__).with_name('graph.py')))
graph = importlib.util.module_from_spec(spec)
spec.loader.exec_module(graph)

FUNCTIONS = frozenset({
    'scalar_value', 'observation_value', 'decode_message', 'read_message', 'encode_message',
    'predicate_result', 'response_message', 'namespace_values', 'pack_namespace',
    'valid_argument_result', 'argument_proxies', 'argument_result', 'valid_case_roots',
    'verification_result', 'capture_proxies', 'capture_result', 'job_proxies', 'job_result',
    'usage_proxy', 'usage_result', 'pack_call_value', 'unpack_call_value', 'pack_usage_value',
    'unpack_usage_value', 'unpack_calls', 'unpack_usage_result', 'valid_job_result',
    'RemoteFunction', 'candidate',
})
FIELDS = FUNCTIONS | frozenset({'MAX_MESSAGE', 'MAX_TRANSCRIPT', 'MAX_CALLS', 'POLICY_RESULT_TYPES',
                                'ProtocolError', 'AdapterPolicyError', 'CandidateExecutionError',
                                'sys', 'os', 'contextlib', 'importlib'})


class Library:
    def __init__(self, source):
        vars(self).update(vars(source))
        if source is sys:
            self.path = list(sys.path)

with os.fdopen(os.dup(1), 'wb') as channel, contextlib.redirect_stdout(sys.stderr):
    os.dup2(2, 1)
    sys.path.insert(0, '/workspace')
    subject = importlib.import_module('scripts.coding_eval_oracle_worker')
    # Unit fixtures may patch the subject's imported library attributes. Such
    # patches stay local to the subject; the bridge retains its own stdio and
    # native modules, especially while candidate() runs its inner protocol.
    for name in ('sys', 'os', 'contextlib', 'importlib'):
        if hasattr(subject, name):
            setattr(subject, name, Library(getattr(subject, name)))
    peer = graph.Peer(sys.stdin.buffer, channel, 'c')
    for name, base in graph.API_EXCEPTIONS.items():
        value = getattr(subject, name, None)
        # A transport category exists even when an older subject lacks that
        # public API. Do not add the absent attribute to the subject itself.
        if type(value) is not type or not issubclass(value, BaseException):
            value = type(name, (base,), {})
        peer.graph.exceptions[name] = value

    def dispatch_body(operation, target, args, kwargs):
        if operation == 'source-call':
            if (type(target) is not str or not target.startswith('t')
                    or peer.graph.kinds.get(target) != 'source-symbol'):
                raise graph.BoundaryError('graph.source-call-authority')
            return peer.graph.objects[target](*args, **kwargs)
        if operation in ('get-subject', 'set-subject', 'delete-subject'):
            if target not in FIELDS or kwargs or len(args) != (1 if operation == 'set-subject' else 0):
                raise graph.BoundaryError('graph.subject-attribute')
            if operation == 'get-subject':
                return getattr(subject, target)
            if operation == 'delete-subject':
                delattr(subject, target); return None
            setattr(subject, target, args[0]); return None
        if operation == 'function':
            if target not in FUNCTIONS:
                raise graph.BoundaryError('graph.subject-function')
            return getattr(subject, target)(*args, **kwargs)
        if type(target) is not str or not target.startswith('c') or target not in peer.graph.objects:
            raise graph.BoundaryError('graph.subject-target')
        value = peer.graph.objects[target]
        if peer.graph.kinds[target] != 'remote':
            raise graph.BoundaryError('graph.subject-kind')
        if operation == 'call':
            return value(*args, **kwargs)
        if operation in ('truth', 'length'):
            if args or kwargs:
                raise graph.BoundaryError('graph.subject-observation')
            return bool(value) if operation == 'truth' else len(value)
        if operation in ('get', 'set', 'delete'):
            if (kwargs or len(args) != (2 if operation == 'set' else 1)
                    or type(args[0]) is not str or args[0].startswith('_')):
                raise graph.BoundaryError('graph.subject-field')
            if operation == 'get':
                return getattr(value, args[0])
            if operation == 'delete':
                delattr(value, args[0]); return None
            setattr(value, args[0], args[1]); return None
        raise graph.BoundaryError('graph.subject-operation')

    def dispatch(operation, target, args, kwargs):
        # This descriptor never grants the candidate authority to replace a
        # native library function in the oracle. Temporary finite patches must
        # restore their original target before the API observation returns.
        libraries = [(value, value.run) for identifier, value in peer.graph.objects.items()
                     if peer.graph.kinds[identifier] == 'fixture-library']
        try:
            return dispatch_body(operation, target, args, kwargs)
        finally:
            if any(value.run is not run for value, run in libraries):
                raise graph.BoundaryError('graph.fixture-library-mutation')

    peer.dispatch = dispatch
    peer.send({'event': 'ready'})
    try:
        while True:
            request = peer.read()
            if request == {'event': 'stop'}:
                break
            peer.answer(request)
    except graph.BoundaryError:
        print('worker-adapter-boundary ' + repr(peer.boundary_errors), file=sys.stderr)
        raise
