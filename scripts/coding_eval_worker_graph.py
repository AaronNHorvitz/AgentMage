"""Bounded worker API observations for independent frozen assertions.

Mutable fixture values retain identity. The wire carries typed data and explicit
fixture handles, never source, bytecode or pickle. Explicit source symbols resolve
only in the candidate namespace. Original callbacks execute under an audit guard
and, when file fixtures are enabled, a sealed filesystem.
"""
from __future__ import annotations

import argparse
import contextlib
import io
import importlib
import json
import math
import signal
import subprocess
import sys
import types
import unittest
from pathlib import Path
from unittest import mock


FRAME_BYTES = 1024 * 1024
NODES = 65536
DEPTH = 96
EXCHANGES = 4096
TRANSCRIPT_BYTES = 16 * 1024 * 1024
IO_METHODS = frozenset({'read', 'readline', 'write', 'flush', 'close', 'peek', 'seek', 'tell', 'getvalue'})
IO_PROPERTIES = frozenset({'buffer', 'raw', 'closed'})
MEMORY_METHODS = {list: frozenset({'append', 'extend', 'insert', 'pop', 'clear', 'reverse', 'copy'}),
                  dict: frozenset({'get', 'pop', 'clear', 'update', 'setdefault', 'copy'}),
                  set: frozenset({'add', 'discard', 'remove', 'pop', 'clear', 'update', 'copy'})}
ITERATORS = {type(iter([])): 'list-iterator', type(iter(())): 'tuple-iterator'}
MOCK_FIELDS = ('_mock_return_value', '_mock_side_effect', '_mock_wraps', '_mock_called',
               '_mock_call_count', '_mock_call_args', '_mock_call_args_list', '_mock_mock_calls',
               'method_calls', '_mock_children', '_mock_parent', '_mock_new_parent',
               '_mock_name', '_mock_new_name', '_mock_methods', '_spec_set', '_spec_asyncs',
               '_mock_unsafe', '_mock_sealed')
MOCK_INTERNALS = frozenset({'_mock_delegate', '_mock_methods', '_mock_unsafe', '_mock_sealed',
                            '_spec_asyncs', '_spec_class', '_spec_set', '_spec_signature'})
MOCK_PROTECTED = frozenset({'reset_mock','mock_add_spec','configure_mock','attach_mock'})
EXCEPTIONS = {value.__name__: value for value in (
    ValueError, TypeError, RuntimeError, OSError, EOFError, AttributeError,
    AssertionError, KeyError, SystemExit, BrokenPipeError, NotImplementedError,
    argparse.ArgumentTypeError, StopIteration, Exception, BaseException)}


class BoundaryError(ValueError):
    pass


class CandidateFailure(RuntimeError):
    pass


EXCEPTIONS.update(BoundaryError=BoundaryError, CandidateFailure=CandidateFailure)
API_EXCEPTIONS = {'ProtocolError': ValueError, 'AdapterPolicyError': ValueError,
                  'CandidateExecutionError': RuntimeError}
UNCOPYABLE = object()
BUILTIN_TYPES = {value.__name__: value for value in (
    object, type, bool, int, float, complex, str, bytes, list, tuple, dict, set, frozenset)}
BUILTIN_FUNCTIONS = {'print': print}


class ListSubclass(list):
    """Inert list-derived category; custom subclass behavior is ungraded."""


def native_exception(exception_type, args):
    """Preserve a validated category without executing a fixture constructor."""
    if type(args) is not tuple:
        raise BoundaryError('graph.exception-arguments')
    if issubclass(exception_type, OSError):
        return OSError.__new__(exception_type, *args)
    value = BaseException.__new__(exception_type)
    initializer = (SystemExit.__init__ if issubclass(exception_type, SystemExit) else
                   StopIteration.__init__ if issubclass(exception_type, StopIteration) else BaseException.__init__)
    initializer(value, *args)
    return value


def unique(pairs):
    result = {}
    for key, value in pairs:
        if key in result:
            raise BoundaryError('graph.duplicate-key')
        result[key] = value
    return result


def encode(value):
    raw = (json.dumps(value, ensure_ascii=True, allow_nan=False, separators=(',', ':')) + '\n').encode()
    if len(raw) > FRAME_BYTES:
        raise BoundaryError('graph.frame-limit')
    return raw


def receive(reader):
    raw = reader.readline(FRAME_BYTES + 1)
    if len(raw) > FRAME_BYTES or not raw.endswith(b'\n'):
        raise BoundaryError('graph.frame')
    try:
        return json.loads(raw, object_pairs_hook=unique,
                          parse_constant=lambda value: (_ for _ in ()).throw(BoundaryError('graph.number')))
    except (ValueError, RecursionError):
        raise BoundaryError('graph.json') from None


class Handle:
    """A candidate-owned object exposes explicit operations, never host Python."""
    def __init__(self, peer, identifier):
        object.__setattr__(self, '_peer', peer)
        object.__setattr__(self, '_identifier', identifier)

    def __bool__(self):
        return self._peer.request('truth', self._identifier, (), {})

    def __len__(self):
        return self._peer.request('length', self._identifier, (), {})

    def __getattr__(self, name):
        if name.startswith('_'):
            raise AttributeError(name)
        return self._peer.request('get', self._identifier, (name,), {})

    def __setattr__(self, name, value):
        if name.startswith('_'):
            raise AttributeError(name)
        self._peer.request('set', self._identifier, (name, value), {})

    def __delattr__(self, name):
        if name.startswith('_'):
            raise AttributeError(name)
        self._peer.request('delete', self._identifier, (name,), {})


class Remote(Handle):
    def __call__(self, *args, **kwargs):
        return self._peer.request('call', self._identifier, args, kwargs)


class GuardedFixture:
    def __init__(self, graph, identifier):
        object.__setattr__(self, '_graph', graph)
        object.__setattr__(self, '_identifier', identifier)

    def __getattr__(self, name):
        if name.startswith('_'):
            raise AttributeError(name)
        return self._graph.guarded_call('get', self._identifier, (name,), {})

    def __setattr__(self, name, value):
        if name.startswith('_'):
            raise AttributeError(name)
        self._graph.guarded_call('set', self._identifier, (name, value), {})


class GuardedCallableFixture(GuardedFixture):
    def __call__(self, *args, **kwargs):
        return self._graph.guarded_call('call', self._identifier, args, kwargs)


class Stream:
    def __init__(self, peer, identifier):
        self.peer, self.identifier = peer, identifier

    def __getattr__(self, name):
        if name in IO_PROPERTIES:
            return self.peer.request('io-get', self.identifier, (name,), {})
        if name not in IO_METHODS:
            raise AttributeError(name)
        return lambda *args, **kwargs: self.peer.request('io', self.identifier, (name, args, kwargs), {})


class Graph:
    def __init__(self, peer, prefix, fixture_file=None):
        self.peer, self.prefix = peer, prefix
        self.fixture_file = fixture_file
        self.objects, self.identities, self.kinds = {}, {}, {}
        self.guarded = {}
        self.wire_state = {}
        self.handle_capabilities = {}
        self.symbols = {}
        self.exception_metadata = {}
        self.fixture_classes = set()
        self.guarded_methods = set()
        self.libraries = {}
        self.library_overlays = {}
        self.overlay_depth = 0
        self.count = 0
        self.exceptions = dict(EXCEPTIONS)

    def remember(self, value, kind):
        identity = id(value)
        if identity in self.identities:
            identifier = self.identities[identity]
            if self.objects[identifier] is not value:
                raise BoundaryError('graph.identity-reuse')
            return identifier
        self.count += 1
        if len(self.objects) >= NODES:
            raise BoundaryError('graph.node-limit')
        identifier = f'{self.prefix}{self.count}'
        self.objects[identifier] = value
        self.identities[identity] = identifier
        self.kinds[identifier] = kind
        return identifier

    def fixture(self, value):
        if type(value) is types.FunctionType and value in self.guarded_methods:
            return True
        if type(value) is types.MethodType:
            return (type(value.__self__).__module__ == 'hidden_eval'
                    and not isinstance(value.__self__, unittest.TestCase)
                    and self.fixture(value.__func__))
        return (type(value) in (types.FunctionType, type)
                and self.fixture_file is not None
                and ((type(value) is types.FunctionType and value.__code__.co_filename == self.fixture_file)
                     or (type(value) is type and value.__module__ == 'hidden_eval')))

    def guard_class(self, cls):
        if cls in self.fixture_classes:
            return
        if cls.__bases__ != (object,):
            raise BoundaryError('graph.fixture-class-bases')
        self.fixture_classes.add(cls)

        def guarded(function):
            if not self.fixture(function):
                raise BoundaryError('graph.fixture-class-method')
            def call(*args, **kwargs):
                global _CALLBACK_DEPTH
                if _SEALED_BOUNDARY is not None and (_SEALED_BOUNDARY.proc_depth or Path('/proc/self').exists()):
                    raise BoundaryError('graph.callback-proc-window-refused')
                _CALLBACK_DEPTH += 1
                try:
                    return function(*args, **kwargs)
                except (BoundaryError, AssertionError):
                    self.peer.failed = True
                    raise
                finally:
                    _CALLBACK_DEPTH -= 1
            call.__dict__ = function.__dict__
            call.__name__ = function.__name__
            call.__qualname__ = function.__qualname__
            self.guarded_methods.add(call)
            return call

        for name, value in list(vars(cls).items()):
            if type(value) is types.FunctionType:
                setattr(cls, name, guarded(value))
            elif type(value) in (classmethod, staticmethod):
                setattr(cls, name, type(value)(guarded(value.__func__)))
            elif type(value) is property:
                setattr(cls, name, property(*(guarded(function) if function is not None else None
                                             for function in (value.fget, value.fset, value.fdel)), doc=value.__doc__))

    def memory_stream(self, value, depth=0):
        if depth > 4:
            return False
        if isinstance(value, (io.BytesIO, io.StringIO)):
            return type(value) in (io.BytesIO, io.StringIO) or self.fixture(type(value))
        if type(value) in (io.BufferedReader, io.BufferedWriter, io.BufferedRandom):
            return self.memory_stream(value.raw, depth + 1)
        if type(value) is io.TextIOWrapper:
            return self.memory_stream(value.buffer, depth + 1)
        return False

    def token(self, value, depth=0):
        if depth > DEPTH:
            raise BoundaryError('graph.depth')
        if (isinstance(value, (unittest.TestCase, unittest.TestResult))
                or (type(value) is type and issubclass(value, (unittest.TestCase, unittest.TestResult)))):
            raise BoundaryError('graph.verdict-authority')
        if type(value) is bool:
            # Native Python equality aliases True/1 and False/0. Distinct
            # tokens keep the fast whole-record delta comparison type-aware.
            return {'boolean': value}
        if type(value) is int and value.bit_length() > 4096:
            if value.bit_length() > 4 * FRAME_BYTES:
                raise BoundaryError('graph.integer-limit')
            return {'bigint': ('-' if value < 0 else '') + format(abs(value), 'x')}
        if value is None or type(value) in (str, int, bool):
            return value
        if type(value) is float:
            return {'float': repr(value)}
        if type(value) is bytes:
            return {'bytes': value.hex()}
        if type(value) is complex:
            return {'complex': [self.token(value.real, depth + 1), self.token(value.imag, depth + 1)]}
        if value is BUILTIN_FUNCTIONS['print']:
            return {'builtin-function': 'print'}
        if type(value) is type and BUILTIN_TYPES.get(value.__name__) is value:
            return {'builtin-type': value.__name__}
        if value is mock.DEFAULT:
            return {'sentinel': 'DEFAULT'}
        if type(value) is signal.Signals:
            return {'signal': int(value)}
        if isinstance(value, Path):
            return {'path': str(value)}
        if type(value) is types.BuiltinMethodType and type(value.__self__) in MEMORY_METHODS:
            if value.__name__ not in MEMORY_METHODS[type(value.__self__)]:
                raise BoundaryError('graph.memory-method')
            return {'memory-method': [self.token(value.__self__, depth + 1), value.__name__]}
        if isinstance(value, BaseException):
            exception_type = type(value)
            if exception_type not in self.exceptions.values() and not self.fixture(exception_type):
                raise BoundaryError('graph.exception-instance')
            return {'exception': [self.token(exception_type, depth + 1), self.token(value.args, depth + 1)]}
        if type(value) in (tuple, set, frozenset):
            return {type(value).__name__: [self.token(item, depth + 1) for item in value]}
        if type(value) is mock._Call:
            return {'mock-call': self.token(tuple(value), depth + 1)}
        known = self.identities.get(id(value))
        if known is not None and (self.objects[known] is value or self.guarded.get(known) is value):
            return {'ref': known}
        if type(value) is mock._CallList:
            kind = 'call-list'
        elif type(value) is list:
            kind = 'list'
        elif isinstance(value, list):
            kind = 'list-subclass'
        elif type(value) is dict:
            kind = 'dict'
        elif type(value) in (argparse.Namespace, types.SimpleNamespace):
            kind = 'argument-namespace' if type(value) is argparse.Namespace else 'namespace'
        elif isinstance(value, mock.MagicMock):
            kind = 'magic-mock'
        elif isinstance(value, mock.Mock):
            kind = 'mock'
        elif type(value) is subprocess.CompletedProcess:
            kind = 'completed'
        elif value is subprocess:
            identifier = self.remember(value, 'fixture-library')
            self.libraries[identifier] = value.run
            return {'ref': identifier}
        elif self.memory_stream(value):
            kind = 'stream'
        elif type(value) is object:
            kind = 'object'
        elif type(value) in ITERATORS:
            kind = ITERATORS[type(value)]
        elif ((type(value) in (types.FunctionType, type))
              and type(getattr(value, '__module__', None)) is str
              and value.__module__.startswith('scripts.') and value.__qualname__.isidentifier()
              and value not in self.exceptions.values() and self.prefix == 't'):
            if type(value) is type and issubclass(value, BaseException):
                if value.__init__ is not RuntimeError.__init__ or value.__new__ is not RuntimeError.__new__:
                    raise BoundaryError('graph.source-exception-constructor')
                self.exceptions[value.__name__] = value
            identifier = self.remember(value, 'source-symbol')
            self.symbols[identifier] = {'module': value.__module__, 'name': value.__qualname__,
                                        'exception': type(value) is type and issubclass(value, BaseException)}
            return {'ref': identifier}
        elif type(value) is type and issubclass(value, BaseException):
            if value not in self.exceptions.values() and not self.fixture(value):
                raise BoundaryError('graph.exception-class')
            if value not in self.exceptions.values() and (
                    value.__init__ is not RuntimeError.__init__ or value.__new__ is not RuntimeError.__new__):
                raise BoundaryError('graph.fixture-exception-constructor')
            kind = 'exception-class'
        elif self.fixture(value):
            if type(value) is type:
                self.guard_class(value)
                kind = 'fixture-class'
            else:
                kind = 'callback'
        elif self.prefix == 't' and self.fixture(type(value)):
            self.guard_class(type(value))
            kind = 'fixture-object'
        elif self.prefix == 'c':
            # Candidate-owned object handles never instantiate their classes
            # in the oracle or expose their implementation/bytecode there.
            kind = 'remote'
        else:
            raise BoundaryError('graph.unsupported-type')
        return {'ref': self.remember(value, kind)}

    def copy_token(self, value, seen=None, depth=0):
        """Closed return observations use value semantics, not object handles.

        Existing graph roots and repeated mutable aliases remain shared. This
        avoids retaining every temporary JSON encoder object in each cumulative
        history, while keeping all previously shared input/fixture identities.
        """
        if depth > DEPTH:
            raise BoundaryError('graph.depth')
        if seen is None:
            seen = set()
        if value is None or type(value) in (str, int, bool, float, bytes) or isinstance(value, Path):
            return self.token(value, depth)
        if id(value) in self.identities or id(value) in seen:
            return UNCOPYABLE
        if type(value) in (list, dict):
            seen.add(id(value))
        if type(value) in (list, tuple, frozenset):
            values = [self.copy_token(item, seen, depth + 1) for item in value]
            if any(item is UNCOPYABLE for item in values):
                return UNCOPYABLE
            return {'copy-list' if type(value) is list else type(value).__name__: values}
        if type(value) is dict and all(type(key) is str for key in value):
            values = [[key, self.copy_token(item, seen, depth + 1)] for key, item in value.items()]
            if any(item is UNCOPYABLE for _, item in values):
                return UNCOPYABLE
            return {'copy-dict': values}
        return UNCOPYABLE

    def snapshot(self, root, copy_values=False):
        token = self.copy_token(root) if copy_values else UNCOPYABLE
        if token is UNCOPYABLE:
            token = self.token(root)
        nodes = []
        seen = set()
        # Traverse until all newly encountered nodes have been included. Bound
        # total nodes; existing object identities are strongly retained.
        while len(seen) < len(self.objects):
            for identifier in list(self.objects):
                if identifier in seen:
                    continue
                seen.add(identifier)
                value, kind = self.objects[identifier], self.kinds[identifier]
                if kind in ('list', 'call-list', 'list-subclass'):
                    data = [self.token(item) for item in list.__iter__(value)]
                elif kind == 'dict':
                    data = [[self.token(key), self.token(item)] for key, item in value.items()]
                elif kind in ('namespace', 'argument-namespace'):
                    data = {key: self.token(item) for key, item in vars(value).items()}
                elif kind in ('mock', 'magic-mock'):
                    if any(vars(value).get(key) is not None for key in ('_spec_class','_spec_signature','_mock_delegate')):
                        raise BoundaryError('graph.unsupported-mock-configuration')
                    attributes = {key: self.token(item) for key, item in vars(value).items()
                                  if key not in MOCK_FIELDS and key not in MOCK_INTERNALS
                                  and not key.startswith('_')}
                    if any(key.startswith('assert') or key in MOCK_PROTECTED for key in attributes):
                        raise BoundaryError('graph.mock-verdict-api-override')
                    data = {'core':{key: self.token(vars(value).get(key, None)) for key in MOCK_FIELDS},
                            'attributes':attributes}
                elif kind == 'completed':
                    data = {key: self.token(getattr(value, key)) for key in ('args', 'returncode', 'stdout', 'stderr')}
                elif kind == 'exception-class':
                    data = {'name': value.__name__, 'base': next((base.__name__ for base in value.__mro__[1:]
                                                                 if base.__name__ in EXCEPTIONS), 'RuntimeError'),
                            'category': ('builtin' if value is EXCEPTIONS.get(value.__name__) else
                                         'api' if value.__name__ in API_EXCEPTIONS
                                         and value is self.exceptions.get(value.__name__) else 'fixture')}
                    if identifier in self.exception_metadata and self.exception_metadata[identifier] != data:
                        raise BoundaryError('graph.changed-exception-metadata')
                    self.exception_metadata[identifier] = data
                elif kind == 'callback':
                    data = ({key: self.token(item) for key, item in vars(value).items()}
                            if type(value) is types.FunctionType else {})
                elif kind == 'remote':
                    data = {'callable': callable(value)}
                elif kind == 'fixture-object':
                    data = {'callable': callable(value), 'class': self.token(type(value))}
                elif kind == 'fixture-class':
                    data = {'name': value.__name__,
                            'callable_instance': any('__call__' in vars(base) for base in value.__mro__ if base is not object)}
                elif kind == 'source-symbol':
                    data = self.symbols[identifier]
                elif kind == 'fixture-library':
                    run = value.run
                    if self.prefix == 't':
                        if self.overlay_depth:
                            run = self.library_overlays.get(identifier, run)
                        elif value.run is self.libraries[identifier]:
                            run = self.library_overlays.get(identifier, run)
                        else:
                            self.libraries[identifier] = run
                            self.library_overlays.pop(identifier, None)
                    data = {'run': self.token(run) if isinstance(run, mock.Mock) else {'native-run': True}}
                elif kind in ('list-iterator', 'tuple-iterator'):
                    reduced = value.__reduce__()
                    data = ({'items': self.token(reduced[1][0]), 'position': reduced[2], 'exhausted': False}
                            if len(reduced) == 3 else {'items': None, 'position': 0, 'exhausted': True})
                else:
                    data = None
                record = {'id': identifier, 'kind': kind, 'data': data}
                # Tokens contain only inert JSON primitives, never fixture
                # objects or user-defined equality. Compare their snapshots
                # directly; repeatedly JSON-encoding every unchanged node is
                # needlessly expensive for cumulative Mock histories.
                if self.wire_state.get(identifier) != record:
                    self.wire_state[identifier] = record
                    nodes.append(record)
        return {'root': token, 'nodes': nodes}

    def value(self, token, depth=0):
        if depth > DEPTH:
            raise BoundaryError('graph.depth')
        if token is None or type(token) in (str, int):
            return token
        if type(token) is not dict or len(token) != 1:
            raise BoundaryError('graph.value')
        kind, value = next(iter(token.items()))
        if kind == 'boolean' and type(value) is bool:
            return value
        if kind == 'bigint' and type(value) is str and 0 < len(value) <= FRAME_BYTES:
            digits = value[1:] if value.startswith('-') else value
            if digits and all(character in '0123456789abcdef' for character in digits):
                return int(value, 16)
        if kind == 'complex' and type(value) is list and len(value) == 2:
            real, imaginary = (self.value(item, depth + 1) for item in value)
            if type(real) is float and type(imaginary) is float:
                return complex(real, imaginary)
        if kind == 'builtin-function' and value == 'print':
            return BUILTIN_FUNCTIONS['print']
        if kind == 'builtin-type' and type(value) is str and value in BUILTIN_TYPES:
            return BUILTIN_TYPES[value]
        if kind == 'ref' and type(value) is str and value in self.objects:
            if self.prefix == 't' and self.kinds[value] == 'fixture-object' and type(self.objects[value]) in self.fixture_classes:
                return self.objects[value]
            if self.prefix == 't' and value.startswith('t') and self.kinds[value] in ('callback', 'fixture-object', 'source-symbol'):
                return self.guard(value)
            return self.objects[value]
        if kind == 'bytes' and type(value) is str:
            try:
                return bytes.fromhex(value)
            except ValueError:
                pass
        if kind == 'float' and type(value) is str:
            try:
                return float(value)
            except ValueError:
                pass
        if kind == 'sentinel' and value == 'DEFAULT':
            return mock.DEFAULT
        if kind == 'path' and type(value) is str:
            return Path(value)
        if kind == 'signal' and type(value) is int:
            return signal.Signals(value)
        if kind in ('tuple', 'set', 'frozenset') and type(value) is list:
            items = [self.value(item, depth + 1) for item in value]
            return tuple(items) if kind == 'tuple' else set(items) if kind == 'set' else frozenset(items)
        if kind == 'copy-list' and type(value) is list:
            return [self.value(item, depth + 1) for item in value]
        if kind == 'copy-dict' and type(value) is list:
            result = {}
            for pair in value:
                if type(pair) is not list or len(pair) != 2 or type(pair[0]) is not str or pair[0] in result:
                    raise BoundaryError('graph.copy-mapping')
                result[pair[0]] = self.value(pair[1], depth + 1)
            return result
        if kind == 'mock-call':
            parts = self.value(value, depth + 1)
            if (type(parts) is tuple and len(parts) in (2, 3)
                    and (len(parts) == 2 or type(parts[0]) is str)
                    and type(parts[-2]) is tuple and type(parts[-1]) is dict
                    and all(type(key) is str for key in parts[-1])):
                return mock._Call(parts, two=len(parts) == 2)
        if kind == 'memory-method' and type(value) is list and len(value) == 2:
            owner, name = self.value(value[0], depth + 1), value[1]
            if type(owner) in MEMORY_METHODS and name in MEMORY_METHODS[type(owner)]:
                return getattr(owner, name)
        if kind == 'exception' and type(value) is list and len(value) == 2:
            exception_type, args = self.value(value[0], depth + 1), self.value(value[1], depth + 1)
            if (type(exception_type) is type and issubclass(exception_type, BaseException)
                    and type(args) is tuple
                    and (exception_type in self.exceptions.values() or self.fixture(exception_type))):
                # The host never invokes a candidate-supplied constructor or
                # fixture __init__. Only fixed native exception allocation.
                return native_exception(exception_type, args)
        raise BoundaryError('graph.value')

    def mapping_key(self, key, depth=0):
        if depth > DEPTH:
            return False
        if type(key) in (str, int, bool, float, bytes, type(None), object, signal.Signals, type(Path())):
            return True
        if type(key) is type and any(key is value for value in BUILTIN_TYPES.values()):
            return True
        return (type(key) in (tuple, frozenset)
                and all(self.mapping_key(item, depth + 1) for item in key))

    def guarded_call(self, operation, identifier, args, kwargs):
        try:
            result = trusted_dispatch(self.peer, operation, identifier, args, kwargs)
            # Guard nested callables too. A returned bound method or a fixture
            # stored in a Mock must not escape into a later unguarded host call.
            wire_state = self.wire_state.copy()
            try:
                return self.peer.load(self.peer.snapshot(result))
            finally:
                # Local guarding sends no frame. Newly registered objects must
                # still be announced the next time this peer writes the wire.
                self.wire_state = wire_state
        except (BoundaryError, AssertionError):
            self.peer.failed = True
            raise

    def guard(self, identifier):
        if identifier not in self.guarded:
            original = self.objects[identifier]
            if self.kinds[identifier] == 'source-symbol':
                if self.symbols[identifier]['exception']:
                    return original
                def wrapper(*args, **kwargs):
                    return self.peer.request('source-call', identifier, args, kwargs)
            elif self.kinds[identifier] == 'callback':
                def wrapper(*args, **kwargs):
                    return self.guarded_call('callback', identifier, args, kwargs)
                if type(original) is types.FunctionType:
                    wrapper.__dict__ = original.__dict__
            else:
                wrapper = (GuardedCallableFixture if callable(original) else GuardedFixture)(self, identifier)
            self.guarded[identifier] = wrapper
            self.identities[id(wrapper)] = identifier
        return self.guarded[identifier]

    def load(self, frame):
        if type(frame) is not dict or frame.keys() != {'root', 'nodes'} or type(frame['nodes']) is not list:
            raise BoundaryError('graph.shape')
        if len(frame['nodes']) > NODES:
            raise BoundaryError('graph.node-limit')
        records = {}
        new_iterators = []
        new_fixture_objects = []
        for node in frame['nodes']:
            if type(node) is not dict or node.keys() != {'id', 'kind', 'data'}:
                raise BoundaryError('graph.node')
            identifier, kind, data = node['id'], node['kind'], node['data']
            if (type(identifier) is not str or len(identifier) > 16 or len(identifier) < 2
                    or identifier[0] not in 'tc' or not identifier[1:].isdigit() or identifier in records):
                raise BoundaryError('graph.identifier')
            records[identifier] = node
            if kind == 'exception-class':
                if identifier in self.exception_metadata and self.exception_metadata[identifier] != data:
                    raise BoundaryError('graph.changed-exception-metadata')
                self.exception_metadata[identifier] = data
            if kind == 'source-symbol':
                if identifier in self.symbols:
                    if data != self.symbols[identifier]:
                        raise BoundaryError('graph.changed-source-symbol')
                elif self.prefix != 'c' or not identifier.startswith('t'):
                    raise BoundaryError('graph.source-symbol-authority')
            if kind in ('remote', 'fixture-object'):
                fields = {'callable', 'class'} if kind == 'fixture-object' else {'callable'}
                if type(data) is not dict or data.keys() != fields or type(data['callable']) is not bool:
                    raise BoundaryError('graph.handle-capability')
                if identifier in self.handle_capabilities and self.handle_capabilities[identifier] is not data['callable']:
                    raise BoundaryError('graph.changed-handle-capability')
                self.handle_capabilities[identifier] = data['callable']
            if identifier in self.objects:
                if self.kinds[identifier] != kind:
                    raise BoundaryError('graph.kind-change')
                continue
            if identifier.startswith(self.prefix):
                raise BoundaryError('graph.unowned-node')
            if len(self.objects) >= NODES:
                raise BoundaryError('graph.node-limit')
            if kind == 'list': value = []
            elif kind == 'list-subclass': value = ListSubclass()
            elif kind == 'call-list': value = mock._CallList()
            elif kind == 'dict': value = {}
            elif kind == 'namespace': value = types.SimpleNamespace()
            elif kind == 'argument-namespace': value = argparse.Namespace()
            elif kind == 'mock': value = mock.Mock()
            elif kind == 'magic-mock': value = mock.MagicMock()
            elif kind == 'object': value = object()
            elif kind == 'completed': value = subprocess.CompletedProcess([], 0)
            elif kind == 'stream': value = Stream(self.peer, identifier)
            elif kind == 'callback' and self.prefix == 'c':
                def callback_factory(identifier):
                    def invoke(*args, **kwargs):
                        return self.peer.request('callback', identifier, args, kwargs)
                    return invoke
                value=callback_factory(identifier)
            elif kind == 'remote' and self.prefix == 't':
                if type(data) is not dict or data.keys() != {'callable'} or type(data['callable']) is not bool:
                    raise BoundaryError('graph.handle-capability')
                value = (Remote if data['callable'] else Handle)(self.peer, identifier)
            elif kind == 'fixture-class' and self.prefix == 'c' and identifier.startswith('t'):
                if (type(data) is not dict or data.keys() != {'name','callable_instance'}
                        or type(data['name']) is not str or not data['name'].isidentifier()
                        or type(data['callable_instance']) is not bool):
                    raise BoundaryError('graph.fixture-class')
                def constructor_factory(identifier):
                    def constructor(cls, *args, **kwargs):
                        return self.peer.request('callback-class', identifier, args, kwargs)
                    return constructor
                def initialize(self, *args, **kwargs):pass
                value = type(data['name'], (Remote if data['callable_instance'] else Handle,), {
                    '__new__': constructor_factory(identifier), '__init__': initialize,
                    '__module__': 'fixture', '__qualname__': data['name']})
            elif kind == 'fixture-object' and self.prefix == 'c' and identifier.startswith('t'):
                value = object()
                new_fixture_objects.append(identifier)
            elif kind == 'exception-class':
                if (type(data) is not dict or data.keys() != {'name', 'base', 'category'}
                        or type(data['name']) is not str or not data['name'].isidentifier()
                        or data['base'] not in EXCEPTIONS):
                    raise BoundaryError('graph.exception')
                if data['category'] == 'builtin' and data['name'] in EXCEPTIONS:
                    value = EXCEPTIONS[data['name']]
                elif data['category'] == 'api' and data['name'] in API_EXCEPTIONS:
                    if data['base'] != API_EXCEPTIONS[data['name']].__name__:
                        raise BoundaryError('graph.exception-hierarchy')
                    value = self.exceptions[data['name']]
                elif data['category'] == 'fixture' and identifier.startswith('t') and self.prefix == 'c':
                    value = type(data['name'], (EXCEPTIONS[data['base']],), {})
                else:
                    raise BoundaryError('graph.exception-category')
            elif kind in ('list-iterator', 'tuple-iterator'):
                value = object()
                new_iterators.append(identifier)
            elif kind == 'source-symbol' and self.prefix == 'c':
                if (type(data) is not dict or data.keys() != {'module', 'name', 'exception'}
                        or type(data['module']) is not str or not data['module'].startswith('scripts.')
                        or any(not part.isidentifier() for part in data['module'].split('.'))
                        or type(data['name']) is not str or not data['name'].isidentifier()
                        or type(data['exception']) is not bool):
                    raise BoundaryError('graph.source-symbol')
                value = getattr(importlib.import_module(data['module']), data['name'])
                if not callable(value):
                    raise BoundaryError('graph.source-symbol-not-callable')
                if data['exception']:
                    if type(value) is not type or not issubclass(value, BaseException):
                        raise BoundaryError('graph.source-exception')
                    self.exceptions[value.__name__] = value
                self.symbols[identifier] = data
            elif kind == 'fixture-library' and self.prefix == 'c':
                value = types.SimpleNamespace(run=subprocess.run, CompletedProcess=subprocess.CompletedProcess,
                                              PIPE=subprocess.PIPE, DEVNULL=subprocess.DEVNULL,
                                              TimeoutExpired=subprocess.TimeoutExpired)
                self.libraries[identifier] = subprocess.run
            else:
                raise BoundaryError('graph.node-kind')
            self.objects[identifier] = value
            self.identities[id(value)] = identifier
            self.kinds[identifier] = kind
            if kind == 'exception-class':
                # Fixture names may repeat. Identity, not the textual name,
                # owns each already-validated exception category.
                self.exceptions['graph:' + identifier] = value
        for identifier in new_fixture_objects:
            data = records[identifier]['data']
            cls = self.value(data['class'])
            if type(cls) is not type or not issubclass(cls, Handle):
                raise BoundaryError('graph.fixture-instance-class')
            value = object.__new__(cls)
            Handle.__init__(value, self.peer, identifier)
            self.identities.pop(id(self.objects[identifier]), None)
            self.objects[identifier] = value
            self.identities[id(value)] = identifier
        for identifier in new_iterators:
            data = records[identifier]['data']
            if type(data) is not dict or data.keys() != {'items', 'position', 'exhausted'} or type(data['exhausted']) is not bool:
                raise BoundaryError('graph.iterator')
            items = ([] if self.kinds[identifier] == 'list-iterator' else ()) if data['exhausted'] else self.value(data['items'])
            if type(items) is not (list if self.kinds[identifier] == 'list-iterator' else tuple):
                raise BoundaryError('graph.iterator')
            self.identities.pop(id(self.objects[identifier]), None)
            value = iter(items)
            self.objects[identifier] = value
            self.identities[id(value)] = identifier
        # Allocate everything before edges, so cycles and aliases are stable.
        for identifier, node in records.items():
            kind, data, value = node['kind'], node['data'], self.objects[identifier]
            if kind in ('list', 'call-list', 'list-subclass'):
                if type(data) is not list: raise BoundaryError('graph.list')
                list.__setitem__(value, slice(None), [self.value(item) for item in data])
            elif kind == 'dict':
                if type(data) is not list: raise BoundaryError('graph.dict')
                decoded = []
                for pair in data:
                    if type(pair) is not list or len(pair) != 2: raise BoundaryError('graph.pair')
                    key = self.value(pair[0])
                    if not self.mapping_key(key):
                        raise BoundaryError('graph.key')
                    if any(type(old) is type(key) and old == key for old, _ in decoded):
                        raise BoundaryError('graph.duplicate-mapping-key')
                    decoded.append((key, self.value(pair[1])))
                value.clear(); value.update(decoded)
            elif kind in ('namespace', 'argument-namespace', 'completed', 'mock', 'magic-mock'):
                if type(data) is not dict or any(type(key) is not str for key in data):
                    raise BoundaryError('graph.fields')
                if kind in ('mock', 'magic-mock'):
                    if (data.keys() != {'core','attributes'} or type(data['core']) is not dict
                            or data['core'].keys() != set(MOCK_FIELDS) or type(data['attributes']) is not dict
                            or any(type(key) is not str or key.startswith('_') or key.startswith('assert')
                                   or key in MOCK_PROTECTED or key in MOCK_FIELDS for key in data['attributes'])):
                        raise BoundaryError('graph.mock-fields')
                    data={**data['core'],**data['attributes']}
                if kind == 'completed' and data.keys() != {'args', 'returncode', 'stdout', 'stderr'}:
                    raise BoundaryError('graph.completed-fields')
                decoded = {key: self.value(item) for key, item in data.items()}
                if kind in ('mock', 'magic-mock'):
                    for key in list(vars(value)):
                        if key not in MOCK_FIELDS and key not in MOCK_INTERNALS and not key.startswith('_') and key not in decoded:
                            del vars(value)[key]
                    vars(value).update(decoded)
                else:
                    vars(value).clear(); vars(value).update(decoded)
            elif kind == 'callback':
                if (type(data) is not dict or any(type(key) is not str or key.startswith('_') for key in data)
                        or (type(value) is not types.FunctionType and data)):
                    raise BoundaryError('graph.callback-fields')
                if type(value) is types.FunctionType:
                    vars(value).clear(); vars(value).update({key: self.value(item) for key, item in data.items()})
            elif kind in ('list-iterator', 'tuple-iterator'):
                pass  # Cursor updates follow all backing-container updates.
            elif kind == 'fixture-library':
                if type(data) is not dict or data.keys() != {'run'}:
                    raise BoundaryError('graph.fixture-library')
                if data['run'] == {'native-run': True}:
                    run = self.libraries[identifier]
                    if self.prefix == 't':
                        self.library_overlays.pop(identifier, None)
                    else:
                        value.run = run
                else:
                    run = self.value(data['run'])
                    if not isinstance(run, mock.Mock):
                        raise BoundaryError('graph.fixture-library-run')
                    if self.prefix == 't':
                        self.library_overlays[identifier] = run
                    else:
                        value.run = run
        for identifier, node in records.items():
            if node['kind'] not in ('list-iterator', 'tuple-iterator'):
                continue
            data, value = node['data'], self.objects[identifier]
            if (type(data) is not dict or data.keys() != {'items', 'position', 'exhausted'}
                    or type(data['exhausted']) is not bool or type(data['position']) is not int):
                raise BoundaryError('graph.iterator')
            reduced = value.__reduce__()
            if data['exhausted']:
                if data['items'] is not None or data['position'] != 0:
                    raise BoundaryError('graph.exhausted-iterator')
                if len(reduced) == 3:
                    value.__setstate__(len(reduced[1][0]))
                    try:next(value)
                    except StopIteration:pass
                    else:raise BoundaryError('graph.iterator-exhaustion')
                continue
            items = self.value(data['items'])
            expected = list if node['kind'] == 'list-iterator' else tuple
            if type(items) is not expected or not 0 <= data['position'] <= len(items):
                raise BoundaryError('graph.iterator')
            if len(reduced) != 3 or (node['kind'] == 'list-iterator' and reduced[1][0] is not items):
                raise BoundaryError('graph.iterator-backing-change')
            value.__setstate__(data['position'])
        for identifier, node in records.items():
            self.wire_state[identifier] = node
        return self.value(frame['root'])


class Peer:
    def __init__(self, reader, writer, prefix, fixture_file=None, transcript=None):
        self.reader, self.writer = reader, writer
        self.graph = Graph(self, prefix, fixture_file)
        self.sequence, self.events, self.bytes = 0, 0, 0
        self.failed = False
        self.boundary_errors = []
        self.fixture_assertions = []
        self.transcript = transcript
        self.dispatch = None

    def send(self, frame):
        raw = encode(frame)
        self.observe(raw)
        self.writer.write(raw); self.writer.flush()

    def observe(self, raw):
        self.events += 1; self.bytes += len(raw)
        if self.events > EXCHANGES or self.bytes > TRANSCRIPT_BYTES:
            self.failed = True
            raise BoundaryError('graph.transcript-limit')
        if self.transcript is not None:
            self.transcript.write(raw); self.transcript.flush()

    def read(self):
        frame = receive(self.reader)
        self.observe(encode(frame))
        return frame

    def load(self, values):
        try:
            return self.graph.load(values)
        except Exception as error:
            self.failure('load', error)
            raise BoundaryError('graph.invalid-value-graph') from None

    def snapshot(self, values, copy_values=False):
        try:
            return self.graph.snapshot(values, copy_values)
        except Exception as error:
            self.failure('snapshot', error)
            raise BoundaryError('graph.unrepresentable-value-graph') from None

    def failure(self, operation, error):
        self.failed = True
        if len(self.boundary_errors) < 16:
            # Fixed adapter diagnostics only; never assertion text or values.
            code = str(error) if type(error) is BoundaryError else type(error).__name__
            if (len(code) > 96 or not code.isascii()
                    or any(character not in 'abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789.-' for character in code)):
                code = 'graph.redacted-error'
            self.boundary_errors.append(operation + ':' + code)

    def request(self, operation, target, args, kwargs):
        self.sequence += 1
        identifier = f'{self.graph.prefix}{self.sequence}'
        try:
            self.send({'event': 'request', 'id': identifier, 'operation': operation,
                       'target': target, 'values': self.snapshot((args, kwargs))})
            while True:
                frame = self.read()
                if type(frame) is not dict:
                    raise BoundaryError('graph.response')
                if frame.get('event') == 'request':
                    self.answer(frame)
                    continue
                if frame.keys() != {'event', 'id', 'values', 'error'} or frame['event'] != 'result' or frame['id'] != identifier:
                    raise BoundaryError('graph.sequence')
                result = self.load(frame['values'])
                if frame['error'] is not None:
                    exception = self.graph.value(frame['error'])
                    if type(exception) is not type or not issubclass(exception, BaseException):
                        raise BoundaryError('graph.error')
                    raise native_exception(exception, result)
                return result
        except BoundaryError as error:
            self.failure('request', error)
            raise

    def answer(self, frame):
        if (type(frame) is not dict or frame.keys() != {'event', 'id', 'operation', 'target', 'values'}
                or frame['event'] != 'request' or type(frame['id']) is not str):
            raise BoundaryError('graph.request')
        error = None
        try:
            args, kwargs = self.load(frame['values'])
            if type(args) is not tuple or type(kwargs) is not dict:
                raise BoundaryError('graph.arguments')
            result = self.dispatch(frame['operation'], frame['target'], args, kwargs)
        except BoundaryError:
            self.failed = True
            raise
        except BaseException as exception:
            if self.graph.prefix == 't' and isinstance(exception, AssertionError):
                # An assertion in a trusted fixture remains a failed attempt,
                # even if untrusted code catches its transported exception.
                self.failed = True
                self.fixture_assertions.append(exception)
            exception_type = type(exception)
            if exception_type not in self.graph.exceptions.values() and not self.graph.fixture(exception_type):
                exception_type = CandidateFailure
                self.graph.exceptions['CandidateFailure'] = CandidateFailure
            error = self.graph.token(exception_type)
            if self.graph.prefix == 't':
                # A fixture assertion must never disclose its expected value
                # to the candidate through an exception message. Parser exit
                # codes are an explicit tested input; other fixture errors
                # carry only a fixed category and fixed text.
                result = (exception.args if isinstance(exception, SystemExit) else
                          ('graph.fixture-assertion-failed' if isinstance(exception, AssertionError)
                           else 'graph.fixture-error',))
            else:
                result = exception.args if exception_type is type(exception) else ('graph.fixed-candidate-failure',)
        values = self.snapshot(result, copy_values=True)
        self.send({'event': 'result', 'id': frame['id'], 'values': values, 'error': error})


_CALLBACK_DEPTH = 0
_SEALED_BOUNDARY = None
FILE_EVENTS = frozenset({'open', 'os.listdir', 'os.scandir', 'os.mkdir', 'os.remove', 'os.rmdir',
                         'os.rename', 'os.link', 'os.symlink', 'os.chmod', 'os.chown', 'os.utime',
                         'os.truncate', 'os.chdir'})


def enable_sealed_files(boundary):
    global _SEALED_BOUNDARY
    if (not boundary.sealed or boundary.proc_depth or Path('/proc/self').exists()
            or Path('/adapter/test.py').exists() or Path('/output/tests.json').exists()):
        raise BoundaryError('graph.unsealed-filesystem')

    def before_launch():
        if _CALLBACK_DEPTH:
            raise BoundaryError('graph.callback-process-refused')

    boundary.before_launch = before_launch
    _SEALED_BOUNDARY = boundary


def audit(event, args):
    if _CALLBACK_DEPTH and _SEALED_BOUNDARY is not None and event in FILE_EVENTS:
        if _SEALED_BOUNDARY.proc_depth or Path('/proc/self').exists():
            raise BoundaryError('graph.callback-proc-window-refused')
        return
    if _CALLBACK_DEPTH and (event == 'open' or event.startswith(('os.', 'subprocess.', 'socket.', 'ctypes.', 'mmap.'))
                            or event in ('exec', 'compile', 'sys.settrace', 'sys.setprofile', 'sys.addaudithook',
                                         'sys._getframe', 'sys._current_frames')):
        raise BoundaryError('graph.callback-authority-refused')


def trusted_dispatch(peer, operation, target, args, kwargs):
    global _CALLBACK_DEPTH
    if _SEALED_BOUNDARY is not None and (_SEALED_BOUNDARY.proc_depth or Path('/proc/self').exists()):
        raise BoundaryError('graph.callback-proc-window-refused')
    if type(target) is not str or not target.startswith('t') or target not in peer.graph.objects:
        raise BoundaryError('graph.host-target')
    value = peer.graph.objects[target]
    kind = peer.graph.kinds[target]
    if ((operation == 'callback' and kind != 'callback') or (operation == 'callback-class' and kind != 'fixture-class')
            or (operation in ('io', 'io-get') and kind != 'stream')
            or (operation in ('get', 'set', 'call') and kind != 'fixture-object')):
        raise BoundaryError('graph.host-capability')
    if operation not in ('callback', 'callback-class', 'io', 'io-get', 'get', 'set', 'call'):
        raise BoundaryError('graph.host-operation')
    _CALLBACK_DEPTH += 1
    try:
        with contextlib.ExitStack() as overlay:
            peer.graph.overlay_depth += 1
            try:
                for identifier, run in peer.graph.library_overlays.items():
                    if not isinstance(run, mock.Mock):
                        raise BoundaryError('graph.fixture-library-not-mocked')
                    overlay.enter_context(mock.patch.object(peer.graph.objects[identifier], 'run', run))
                if operation in ('callback', 'callback-class'):
                    if not peer.graph.fixture(value):
                        raise BoundaryError('graph.fixture-callback')
                    return value(*args, **kwargs)
                if operation == 'call':
                    if not callable(value) or not peer.graph.fixture(type(value)):
                        raise BoundaryError('graph.fixture-call')
                    return value(*args, **kwargs)
                if operation in ('get', 'set'):
                    if (kwargs or len(args) != (1 if operation == 'get' else 2)
                            or type(args[0]) is not str or args[0].startswith('_')):
                        raise BoundaryError('graph.fixture-attribute')
                    if operation == 'get':
                        return getattr(value, args[0])
                    setattr(value, args[0], args[1])
                    return None
                if operation == 'io-get':
                    if kwargs or len(args) != 1 or args[0] not in IO_PROPERTIES or not peer.graph.memory_stream(value):
                        raise BoundaryError('graph.io-property')
                    return getattr(value, args[0])
                if len(args) != 3 or kwargs or args[0] not in IO_METHODS or type(args[1]) is not tuple or type(args[2]) is not dict:
                    raise BoundaryError('graph.io-operation')
                return getattr(value, args[0])(*args[1], **args[2])
            finally:
                peer.graph.overlay_depth -= 1
    finally:
        _CALLBACK_DEPTH -= 1
