"""Independent worker integration using original synthetic assertions only."""
from pathlib import Path
import tempfile
import unittest
from unittest import mock

from scripts import coding_eval as evaluation
from scripts import coding_eval_oracle as engine
from scripts import coding_eval_worker_oracle as oracle
from scripts import local_agent_fast_gate as fast_gate


FIXTURE = rb'''
import io,json,subprocess,sys,tempfile,unittest
from pathlib import Path
from types import SimpleNamespace
from unittest import mock
from scripts import coding_eval_oracle_worker as worker

class SyntheticWorkerTests(unittest.TestCase):
    def test_predicate(self):
        value={'fixture':[None,3,False]}
        self.assertEqual(worker.predicate_result(value,'bool'),{'value':value})

    def test_protocol(self):
        raw=worker.encode_message({'ready':True,'function_found':True})
        raw+=worker.encode_message({'seq':1,'result':{'value':['fixture',False]}})
        output=io.BytesIO()
        remote=worker.RemoteFunction(io.BytesIO(raw),output,io.BytesIO(),'bool')
        remote.start()
        self.assertEqual(remote({'fixture':True}),['fixture',False])
        self.assertEqual(remote.calls,1)
        remote.finish()
        self.assertTrue(output.closed)
        self.assertFalse(remote.failed)

    def test_library(self):
        module=SimpleNamespace(subprocess=subprocess,COMMANDS=())
        def capture():
            result=module.subprocess.run(['synthetic'])
            return (type(result) is subprocess.CompletedProcess,result.stdout)
        module.capture=capture
        with mock.patch.object(subprocess,'run') as original:
            for kind,completed in (('completed-process',True),('mock',False)):
                result=worker.capture_result(module,{'operation':'capture','stdout':'finite fixture',
                    'returncode':0,'commands':[['synthetic']],'stub_kind':kind})
                self.assertEqual(result,{'value':[completed,'finite fixture'],'container':'tuple'})
            original.assert_not_called()

    def test_finite_file_callback(self):
        with tempfile.TemporaryDirectory() as directory:
            path=Path(directory)/'fixture'
            path.write_text('synthetic')
            def helper(namespace):
                self.assertFalse(Path('/adapter/test.py').exists())
                self.assertFalse(Path('/proc/self').exists())
                return [namespace.path.read_text()]
            module=SimpleNamespace(HarnessError=ValueError,helper=helper)
            result=worker.argument_result(module,'helper',{'operation':'arguments',
                      'namespace':{'path':{'path':str(path)}}})
            self.assertEqual(result,{'arguments':['synthetic'],'container':'list'})

    def test_inner_protocol(self):
        output=io.BytesIO()
        incoming=SimpleNamespace(buffer=io.BufferedReader(io.BytesIO(b'{"broken":true}\n')))
        def predicate():return True
        target=SimpleNamespace(predicate=predicate)
        with (mock.patch.object(worker.sys,'stdin',incoming),
              mock.patch.object(worker.sys,'stdout',SimpleNamespace(buffer=output)),
              mock.patch.object(worker.sys,'path',[]),
              mock.patch.object(worker.os,'dup',return_value=3),
              mock.patch.object(worker.os,'dup2'),
              mock.patch.object(worker.os,'fdopen',return_value=worker.contextlib.nullcontext(output)),
              mock.patch.object(worker.importlib,'import_module',return_value=target)):
            with self.assertRaises(worker.ProtocolError):
                worker.candidate({'module':'fixture','function':'predicate','result_type':'bool'})
        self.assertEqual(json.loads(output.getvalue()),{'ready':True,'function_found':True})

    def test_raw_source_protocol(self):
        source=worker.__file__
        code=("import importlib.util\nfrom types import SimpleNamespace\nfrom unittest import mock\n"
              "spec=importlib.util.spec_from_file_location('subject',"+repr(source)+")\n"
              "subject=importlib.util.module_from_spec(spec);spec.loader.exec_module(subject)\n"
              "fake=SimpleNamespace(predicate=lambda:True)\n"
              "with mock.patch.object(subject.importlib,'import_module',return_value=fake):\n"
              " subject.candidate({'module':'fixture','function':'predicate','result_type':'bool'})\n")
        requests=b'{"seq":1,"args":[]}\n{"done":true}\n'
        process=subprocess.run([sys.executable,'-I','-B','-c',code],input=requests,capture_output=True,check=True)
        frames=[json.loads(line) for line in process.stdout.splitlines()]
        self.assertEqual(frames,[{'ready':True,'function_found':True},{'seq':1,'result':{'value':True}}])
'''


STATE_FIXTURE = rb'''
from types import SimpleNamespace
from unittest import mock
import io,json,unittest
from scripts import coding_eval_oracle_worker as worker
class StateTests(unittest.TestCase):
    def test_native_exception_fields(self):
        module=SimpleNamespace(HarnessError=RuntimeError,parser=mock.Mock(side_effect=SystemExit(2)))
        self.assertEqual(worker.argument_result(module,'helper',{'operation':'parse','argv':[]}),{'parser_exit':True})
        raw=worker.response_message(1,{'value':10**10000})
        self.assertEqual(json.loads(raw),{'seq':1,'result':{'adapter_policy':True}})
    def test_net_mutation(self):
        for mode in ('current','nested','earlier','equal-type'):
            with self.subTest(mode=mode):
                class Adapter:
                    def __init__(self,*args,**kwargs):self.saved=[]
                    def tokenize_messages(self,messages):
                        if mode=='earlier':
                            if self.saved:self.saved[0].append({'synthetic':'changed'})
                            self.saved.append(messages)
                        elif mode=='nested':messages[0]['synthetic'].append(True)
                        elif mode=='equal-type':messages[0]['counter']=True
                        else:messages.append({'synthetic':'changed'})
                        return 7
                module=SimpleNamespace(OpenAIAdapter=Adapter,unix_http_request=None)
                instances={}
                create={'operation':'create','instance':1,'stub':{'fixture':worker.pack_usage_value((200,b'{}',0.01))},
                    'message_handle':None,'args':worker.pack_usage_value(('synthetic',)),'kwargs':worker.pack_usage_value({})}
                worker.usage_result(module,instances,create)
                for handle in range(1,3 if mode=='earlier' else 2):
                    messages=[{'synthetic':[False],'counter':1}]
                    request={'operation':'tokenize','instance':1,'stub':{'unchanged':True},'message_handle':handle,
                        'args':worker.pack_usage_value((messages,)),'kwargs':worker.pack_usage_value({})}
                    result=worker.usage_result(module,instances,request)
                    if mode!='earlier' or handle==2:self.assertEqual(result,{'adapter_policy':True})
    def test_history_mutation(self):
        for change in ('reset','nested','child','equal-type','clear'):
            with self.subTest(change=change):
                create_reply={'value':worker.pack_usage_value(None),'calls':[]}
                reply={'value':worker.pack_usage_value(7),'calls':[{'args':worker.pack_usage_value(({'synthetic':[False],'counter':1},)),
                                                                'kwargs':worker.pack_usage_value({})}]}
                raw=worker.encode_message({'ready':True,'function_found':True})
                raw+=worker.encode_message({'seq':1,'result':create_reply})
                raw+=worker.encode_message({'seq':2,'result':reply})
                remote=worker.RemoteFunction(io.BytesIO(raw),io.BytesIO(),io.BytesIO(),'usage-result')
                remote.start()
                transport=mock.MagicMock(return_value=(200,b'{}',0.01))
                module=SimpleNamespace(unix_http_request=transport)
                Adapter=worker.usage_proxy(remote,module);adapter=Adapter('synthetic')
                messages=[{'synthetic':'original'}]
                self.assertEqual(adapter.tokenize_messages(messages),7)
                if change=='reset':transport.reset_mock()
                elif change=='nested':transport.call_args.args[0]['synthetic'].append(True)
                elif change=='equal-type':transport.call_args.args[0]['counter']=True
                elif change=='clear':transport.call_args_list.clear()
                else:transport.child('changed')
                with self.assertRaises(worker.AdapterPolicyError):adapter.tokenize_messages(messages)
                self.assertTrue(remote.policy_failed)
                self.assertEqual(remote.calls,2)
'''

METHODS = ['SyntheticWorkerTests.' + name for name in (
    'test_predicate', 'test_protocol', 'test_library', 'test_finite_file_callback',
    'test_inner_protocol', 'test_raw_source_protocol')]


class WorkerOracleTests(unittest.TestCase):
    def setUp(self):
        self.temporary = tempfile.TemporaryDirectory()
        self.addCleanup(self.temporary.cleanup)
        self.root = Path(self.temporary.name)
        self.base = Path(evaluation.__file__).with_name('coding_eval_oracle_worker.py').read_bytes()

    def probe(self, name, suffix=b'', methods=None, fixture=FIXTURE, factory=False, subject=None):
        methods = METHODS if methods is None else methods
        task = {'id': 'synthetic-worker', 'source': oracle.SOURCE,
                'reference': 'synthetic-original', 'hidden_test': 'tests/synthetic_worker.py',
                'hidden_sha256': evaluation.sha(fixture), 'test_names': methods,
                'expected_tests': len(methods)}
        source = self.base if subject is None else subject
        trusted = {'scripts/__init__.py': b'', oracle.SOURCE: source}
        files = {**trusted, oracle.SOURCE: source + suffix}
        output = self.root / name
        with mock.patch.object(evaluation, 'require_historical_candidate', return_value='synthetic-probe'), \
                mock.patch.object(evaluation, 'projection', return_value=trusted), \
                mock.patch.object(evaluation, 'pinned_blob', return_value=fixture):
            if factory:
                with mock.patch.object(engine, 'sandbox', return_value=['/usr/bin/false']):
                    return oracle.grade(task, files, output)
            return oracle.grade(task, files, output)

    def test_complete_original_synthetic_methods_pass_with_separate_namespaces(self):
        report = self.probe('reference')
        self.assertTrue(report['passed'], report)
        self.assertEqual(report['tests']['tests'], 6)
        self.assertIsNone(report['candidate_exit_code'])
        self.assertEqual(report['candidate_exit_scope'], 'no-single-candidate-process')
        self.assertEqual(report['calls_scope'], 'outer-worker-API-RPC')
        self.assertEqual(report['worker_limits']['processes'], 8)
        self.assertEqual(report['worker_limits']['cpu_seconds'], 60)
        self.assertEqual(report['model_attempts'], 0)
        self.assertIsNone(report['model_score'])
        self.assertFalse(report['aggregate_worker_quotas'])
        for name, digest in report['source_hashes'].items():
            self.assertEqual(digest, evaluation.sha(oracle.source_parts()[name]))

    def test_candidate_cannot_disable_original_assertions_or_supply_passing_counts(self):
        report = self.probe('wrong', b'''
import unittest
unittest.TestCase.assertEqual=lambda *args,**kwargs:None
unittest.TestCase.assertTrue=lambda *args,**kwargs:None
print('{"tests":{"tests":6,"failures":0,"errors":0}}')
def predicate_result(value,result_type):return {'value':'wrong'}
''')
        self.assertFalse(report['passed'], report)
        self.assertEqual(report['tests']['tests'], 6, report)
        self.assertGreater(report['tests']['failures'], 0, report)
        self.assertFalse(report['protocol_failed'], report)
        self.assertEqual(report['cause'], 'test_failure')

    def test_caught_fixture_assertion_is_a_test_failure_with_separate_sticky_flag(self):
        fixture = b'''
import unittest
from scripts import coding_eval_oracle_worker as worker
class SyntheticTests(unittest.TestCase):
 def test_callback(self):
  def callback():self.assertEqual('observed','required')
  self.assertEqual(worker.predicate_result(callback,'bool'),{'value':True})
'''
        suffix = b'''
def predicate_result(value,result_type):
 try:value()
 except AssertionError:pass
 return {'value':True}
'''
        report = self.probe('caught-assertion', suffix, ['SyntheticTests.test_callback'], fixture=fixture)
        self.assertFalse(report['passed'])
        self.assertEqual(report['cause'], 'test_failure', report)
        self.assertTrue(report['fixture_assertion_failed'])
        self.assertFalse(report['protocol_failed'])
        self.assertFalse(report['adapter_policy_failed'])
        self.assertEqual(report['tests']['failures'], 0)

    def test_representation_refusal_and_wire_failure_have_different_causes(self):
        fixture = b'''
import unittest
from scripts import coding_eval_oracle_worker as worker
class SyntheticTests(unittest.TestCase):
 def test_callback(self):
  def callback():return slice(1)
  worker.predicate_result(callback,'bool')
'''
        for name, suffix, cause in (
                ('policy', b"\ndef predicate_result(value,result_type):return value()\n", 'adapter_policy'),
                ('wire', b"\ndef predicate_result(value,result_type):\n import os\n os._exit(0)\n", 'protocol')):
            with self.subTest(name=name):
                report = self.probe(name, suffix, ['SyntheticTests.test_callback'], fixture=fixture)
                self.assertFalse(report['passed'])
                self.assertEqual(report['cause'], cause, report)
                self.assertEqual(report['adapter_policy_failed'], cause == 'adapter_policy')

    def test_historical_subject_without_later_library_imports_still_runs_assertions(self):
        subject = b'class RemoteFunction:pass\ndef predicate_result(value,result_type):return {"value":value}\n'
        report = self.probe('minimal', methods=[METHODS[0]], subject=subject)
        self.assertTrue(report['passed'], report)
        self.assertEqual(report['tests']['tests'], 1)

    def test_retained_message_mutations_and_trusted_history_remain_visible(self):
        methods = ['StateTests.' + name for name in (
            'test_native_exception_fields', 'test_net_mutation', 'test_history_mutation')]
        report = self.probe('states', fixture=STATE_FIXTURE, methods=methods)
        self.assertTrue(report['passed'], report)
        self.assertEqual(report['tests']['tests'], 3)
        self.assertFalse(report['protocol_failed'])

    def test_plain_fixture_exception_relay_preserves_its_original_category(self):
        fixture = b'''
from types import SimpleNamespace
from unittest import mock
import unittest
from scripts import coding_eval_oracle_worker as worker
observations=[]
class FixtureError(RuntimeError):pass
class ExceptionRelay(unittest.TestCase):
    def test_category(self):
        def capture():
            observations.append('synthetic-original')
            raise FixtureError('synthetic-original')
        module=SimpleNamespace(COMMANDS=(),subprocess=SimpleNamespace(run=None),capture=capture)
        with self.assertRaises(FixtureError) as observed:
            worker.capture_result(module,{'operation':'capture','stdout':'finite',
                'returncode':0,'commands':[['synthetic']],'stub_kind':'completed-process'})
        self.assertIs(type(observed.exception),FixtureError)
        self.assertEqual(observations,['synthetic-original'])
'''
        report = self.probe('exception', fixture=fixture, methods=['ExceptionRelay.test_category'])
        self.assertTrue(report['passed'], report)

    def test_candidate_cannot_read_assertions_results_or_parent_process_views(self):
        report = self.probe('namespace', b'''
import ctypes as boundary_ctypes
import errno as boundary_errno
for boundary_name in ('/adapter/test.py','/output/method-000-tests.json','/proc/self','/proc/1/root'):
    assert not Path(boundary_name).exists(),boundary_name
boundary_libc=boundary_ctypes.CDLL(None,use_errno=True)
class H(boundary_ctypes.Structure):_fields_=[('version',boundary_ctypes.c_uint),('pid',boundary_ctypes.c_int)]
class C(boundary_ctypes.Structure):_fields_=[(key,boundary_ctypes.c_uint) for key in ('effective','permitted','inheritable')]
boundary_header=H(0x20080522,0);boundary_caps=(C*2)()
assert boundary_libc.capget(boundary_ctypes.byref(boundary_header),boundary_caps)==0
assert all(getattr(part,key)==0 for part in boundary_caps for key in ('effective','permitted','inheritable'))
for boundary_number in (101,310,311,438,298,0x40000065):
    boundary_ctypes.set_errno(0)
    assert boundary_libc.syscall(boundary_number,0,0,0,0,0,0)==-1
    assert boundary_ctypes.get_errno()==boundary_errno.EPERM
''', [METHODS[0], METHODS[-1]])
        self.assertTrue(report['passed'], report)
        self.assertEqual(report['tests']['tests'], 2)

    def test_sealed_oracle_keeps_temporary_io_but_refuses_private_paths_and_symlinks(self):
        fixture = b'''
from pathlib import Path
import tempfile
import unittest
class KernelTests(unittest.TestCase):
    def test_private_files(self):
        self.assertFalse(Path('/proc/self').exists())
        with self.assertRaises(FileNotFoundError):Path('/adapter/test.py').read_bytes()
        with self.assertRaises(OSError):Path('/output/forged.json').write_text('forged')
        with tempfile.TemporaryDirectory() as directory:
            root=Path(directory)
            (root/'fixture').write_text('finite')
            self.assertEqual((root/'fixture').read_text(),'finite')
            (root/'source').symlink_to('/adapter/test.py')
            (root/'result').symlink_to('/output',target_is_directory=True)
            with self.assertRaises(FileNotFoundError):(root/'source').read_bytes()
            with self.assertRaises(OSError):(root/'result/forged.json').write_text('forged')
'''
        report = self.probe('sealed', methods=['KernelTests.test_private_files'], fixture=fixture)
        self.assertTrue(report['passed'], report)
        self.assertEqual(report['tests']['tests'], 1)

    def test_early_candidate_exit_and_real_host_failure_run_no_assertions(self):
        for name, suffix, factory in (('early', b'\nraise SystemExit(0)\n', False),
                                      ('launch', b'', True)):
            with self.subTest(name=name):
                report = self.probe(name, suffix, [METHODS[0]], factory=factory)
                self.assertFalse(report['passed'], report)
                self.assertIsNone(report['tests'])
                self.assertEqual(report['cause'], 'infrastructure')
                self.assertEqual(report['control_attempts'], {'total': 1, 'passed': 0, 'failed': 1})

    def test_unregistered_input_refuses_before_creating_output_or_launching(self):
        output = self.root / 'refused'
        with mock.patch.object(oracle, 'grade_case') as launch:
            with self.assertRaises(evaluation.EvaluationError):
                oracle.grade({'id': 'unregistered'}, {}, output)
        launch.assert_not_called()
        self.assertFalse(output.exists())

    def test_related_process_roles_share_the_full_integration_gate(self):
        roles = ['scripts/coding_eval_worker_' + role + '.py' for role in ('client', 'facade', 'runtime', 'seal')]
        commands = fast_gate.plan(evaluation.ROOT, roles, {'packages': []})
        commands = [command for command in commands if '-m' in command and 'unittest' in command]
        self.assertEqual(len(commands), 1)
        self.assertEqual(commands[0].count('tests.test_coding_eval_worker_oracle'), 1)
