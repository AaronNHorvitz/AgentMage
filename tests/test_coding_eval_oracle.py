"""Measure frozen controls and result-forgery probes in separate real sandboxes."""
import json
import os
from pathlib import Path
import subprocess
import sys
import tempfile
import unittest
from unittest import mock

from scripts import coding_eval as evaluation
from scripts import coding_eval_oracle as oracle


class IndependentOracleTests(unittest.TestCase):
    def setUp(self):
        self.temporary = tempfile.TemporaryDirectory(prefix="agentmage-oracle-test-")
        self.addCleanup(self.temporary.cleanup)
        self.root = Path(self.temporary.name)
        self.task = next(t for t in evaluation.collection() if t["id"] == "dev-015")

    def files(self, task=None, fixed=True):
        task = task or self.task
        files = evaluation.projection(task)
        if fixed:
            files[task["source"]] = evaluation.pinned_blob(
                task["reference"], task["source"], task["reference_sha256"])
        return files

    def probe(self, source, name):
        # Builder-authored diagnostics only. Real CLI/grade admission is tested
        # separately, and never accepts these modified snapshots.
        files = self.files()
        files[self.task["source"]] += source
        with mock.patch.object(evaluation, "require_historical_candidate", return_value="synthetic-probe"):
            return oracle.grade(self.task, files, self.root / name)

    def test_real_two_sandbox_controls_keep_original_hidden_tests_and_counts(self):
        summary = oracle.verify(self.root / "controls")
        self.assertTrue(summary["passed"], summary)
        self.assertEqual(len(summary["tasks"]), 12)
        self.assertEqual(summary["control_attempts"], {"total": 24, "passed": 12, "failed": 12})
        self.assertFalse(summary["generated_scoring"])
        self.assertIsNone(summary["model_score"])
        self.assertEqual(summary["collection_sha256"],
                         "0c2a0624df93085540f61c4677260490a9810a0325efd6bcb75db0dce1461570")
        for row in summary["tasks"]:
            self.assertFalse(row["base"]["passed"])
            task = next(t for t in evaluation.collection() if t["id"] == row["task"])
            self.assertEqual(row["reference"]["tests"]["tests"], task["expected_tests"])
            self.assertEqual(row["reference"]["tests"]["errors"], 0)
            self.assertGreater(row["reference"]["calls"], 0)
            self.assertTrue(row["reference"]["candidate_ready"])
            self.assertTrue(row["reference"]["function_found"])
            self.assertTrue(oracle.expected_base(
                next(t for t in evaluation.collection() if t["id"] == row["task"]), row["base"]))
            self.assertFalse(row["reference"]["aggregate_worker_quotas"])

    def test_candidate_launch_failure_cannot_validate_either_failing_base(self):
        real_sandbox, real_grade = oracle.sandbox, oracle.grade

        def fail_candidate(workspace, adapter, output, role):
            if role == "candidate":
                return ["/usr/bin/false"]
            return real_sandbox(workspace, adapter, output, role)

        def fail_base(task, files, output):
            if evaluation.require_historical_candidate(task, files) == "frozen-base":
                with mock.patch.object(oracle, "sandbox", side_effect=fail_candidate):
                    return real_grade(task, files, output)
            return real_grade(task, files, output)

        with mock.patch.object(oracle, "grade", side_effect=fail_base):
            summary = oracle.verify(self.root / "failed-launches")
        self.assertFalse(summary["passed"])
        self.assertFalse(summary["input_refused"])
        for row in summary["tasks"]:
            self.assertFalse(row["valid"])
            self.assertTrue(row["reference"]["passed"], row)
            base = row["base"]
            self.assertEqual(base["cause"], "infrastructure")
            self.assertFalse(base["candidate_ready"])
            self.assertIsNone(base["function_found"])
            self.assertEqual(base["calls"], 0)
            self.assertEqual(base["tests"]["tests"], 0)
            self.assertEqual(base["logs"]["wire.log"], evaluation.sha(b""))
            self.assertEqual(base["control_attempts"], {"total": 1, "passed": 0, "failed": 1})

    def test_capture_controls_refuse_generated_inputs_before_launch(self):
        for task in (t for t in evaluation.collection() if t["id"] in {"dev-005", "dev-018"}):
            files = self.files(task)
            files[task["source"]] += b"\n# synthetic candidate\n"
            output = self.root / task["id"]
            with self.subTest(task=task["id"]), mock.patch.object(oracle, "exchange") as launch:
                with self.assertRaisesRegex(evaluation.EvaluationError, "untrusted-candidate-disabled"):
                    oracle.grade(task, files, output)
                launch.assert_not_called()
                self.assertFalse(output.exists())

    def test_capture_assertions_stay_trusted_and_wrong_values_fail(self):
        variants = (
            ("redaction", b"\ndef capture(): return ('unredacted output', 0)\n", "test_failure"),
            ("validation", b"\ndef validate_raw(text): return []\n", "test_failure"),
            ("forgery", b'''
import unittest
from pathlib import Path
assert not Path('/adapter/test.py').exists()
assert not Path('/output').exists()
unittest.TestCase.assertIn = lambda *a, **kw: None
unittest.TestCase.assertNotIn = lambda *a, **kw: None
def capture(): return ('unredacted output', 0)
def validate_raw(text): return []
''', "test_failure"),
            ("object", b"\ndef validate_raw(text): return object()\n", "adapter_policy"),
            ("exception", b"\ndef validate_raw(text): raise ValueError('private-candidate-payload')\n",
             "candidate_exception"),
        )
        for task in (t for t in evaluation.collection() if t["id"] in {"dev-005", "dev-018"}):
            for name, suffix, cause in variants:
                files = self.files(task)
                files[task["source"]] += suffix
                output = self.root / (task["id"] + "-" + name)
                with self.subTest(task=task["id"], variant=name), mock.patch.object(
                        evaluation, "require_historical_candidate", return_value="synthetic-probe"):
                    report = oracle.grade(task, files, output)
                self.assertFalse(report["passed"], report)
                self.assertEqual(report["cause"], cause, report)
                self.assertEqual(report["tests"]["tests"], 1)
                self.assertEqual(report["control_attempts"], {"total": 1, "passed": 0, "failed": 1})
                self.assertIn("real-command-execution", report["coverage"]["not_graded"])
                self.assertNotIn(b"private-candidate-payload", (output / "wire.log").read_bytes())
                self.assertNotIn(b"private-candidate-payload", (output / "oracle.log").read_bytes())

    def test_response_memory_exhaustion_is_counted_adapter_policy(self):
        report = self.probe(b"\ndef case_passed(v): return 'x' * (150 * 1024 * 1024) if v else ''\n",
                            "memory-exhaustion")
        self.assertFalse(report["passed"])
        self.assertEqual(report["cause"], "adapter_policy", report)
        self.assertEqual(report["candidate_exit_code"], 0)
        self.assertEqual(report["control_attempts"], {"total": 1, "passed": 0, "failed": 1})

    def test_import_failure_and_false_startup_do_not_run_hidden_assertions(self):
        for name, code in (
                ("import-failure", b'\nraise RuntimeError("synthetic import failure")\n'),
                ("false-startup", b'\nimport os\nos.write(1, b\'{"ready":false,"function_found":true}\\n\')\n')):
            with self.subTest(name=name):
                report = self.probe(code, name)
                self.assertFalse(report["passed"])
                self.assertEqual(report["cause"], "infrastructure")
                self.assertFalse(report["candidate_ready"])
                self.assertEqual(report["calls"], 0)
                self.assertEqual(report["tests"]["tests"], 0)
                self.assertEqual(report["control_attempts"], {"total": 1, "passed": 0, "failed": 1})

    def test_verification_fixtures_cannot_patch_or_write_trusted_assertions(self):
        for ident in ("dev-020", "dev-021", "dev-022"):
            task = next(t for t in evaluation.collection() if t["id"] == ident)
            files = self.files(task)
            files[task["source"]] += b'''
import unittest
from pathlib import Path
unittest.TestCase.assertFalse = lambda *a, **kw: None
assert not Path('/adapter/test.py').exists()
assert not Path('/output').exists()
assert not Path('/lane').exists()
def verify_case(*args):
 return {'passed': True, 'checks': {'retained-muse-read-arguments': True,
  'no-rejection-authority-or-effects': True, 'bounded-tools': True,
  'terminal-rejection-code': True}, 'tests': {'failures': 0}}
'''
            with self.subTest(task=ident), mock.patch.object(
                    evaluation, "require_historical_candidate", return_value="synthetic-probe"):
                report = oracle.grade(task, files, self.root / ident)
            self.assertFalse(report["passed"], report)
            self.assertEqual(report["cause"], "test_failure")
            self.assertTrue(report["candidate_ready"])
            self.assertEqual(report["tests"]["tests"], 1)
            self.assertEqual(report["control_attempts"], {"total": 1, "passed": 0, "failed": 1})
            self.assertIn("other-report-fields", report["coverage"]["not_graded"])

    def test_new_verification_controls_refuse_generated_snapshots_and_hide_exception_data(self):
        for ident in ("dev-020", "dev-021", "dev-022"):
            task = next(t for t in evaluation.collection() if t["id"] == ident)
            files = self.files(task)
            files[task["source"]] += b"\ndef verify_case(*args): raise KeyError('untrusted exception payload')\n"
            output = self.root / ident
            with self.subTest(task=ident), self.assertRaisesRegex(
                    evaluation.EvaluationError, "untrusted-candidate-disabled"):
                oracle.grade(task, files, output)
            self.assertFalse(output.exists())
            with mock.patch.object(evaluation, "require_historical_candidate", return_value="synthetic-probe"):
                report = oracle.grade(task, files, output)
            self.assertFalse(report["passed"])
            self.assertEqual(report["tests"]["errors"], 1)
            self.assertFalse(report["protocol_failed"])
            self.assertNotIn("untrusted exception payload", (output / "wire.log").read_text())
            self.assertNotIn("untrusted exception payload", (output / "oracle.log").read_text())

    def test_argument_controls_refuse_generated_sources_before_output_or_launch(self):
        for task in (t for t in evaluation.collection() if t["id"] in {"dev-006", "dev-007", "dev-027"}):
            files = self.files(task)
            files[task["source"]] += b"\n# synthetic candidate\n"
            output = self.root / task["id"]
            with self.subTest(task=task["id"]), mock.patch.object(oracle, "exchange") as launch:
                with self.assertRaisesRegex(evaluation.EvaluationError, "untrusted-candidate-disabled"):
                    oracle.grade(task, files, output)
                launch.assert_not_called()
                self.assertFalse(output.exists())

    def test_argument_assertions_survive_candidate_patching_and_wrong_order(self):
        for task in (t for t in evaluation.collection() if t["id"] in {"dev-006", "dev-007", "dev-027"}):
            name = oracle.SUPPORTED[task["id"]]["function"]
            files = self.files(task)
            files[task["source"]] += ('''
import unittest
unittest.TestCase.assertEqual = lambda *args, **kwargs: None
unittest.TestCase.assertRaises = lambda *args, **kwargs: None
assert not Path('/adapter/test.py').exists()
assert not Path('/output').exists()
original_arguments = %s
def %s(arguments):
 return list(reversed(original_arguments(arguments)))
''' % (name, name)).encode()
            with self.subTest(task=task["id"]), mock.patch.object(
                    evaluation, "require_historical_candidate", return_value="synthetic-probe"):
                report = oracle.grade(task, files, self.root / task["id"])
            self.assertFalse(report["passed"], report)
            self.assertEqual(report["cause"], "test_failure")
            self.assertEqual(report["tests"]["failures"], 1)
            self.assertFalse(report["protocol_failed"])

    def test_argument_exception_messages_stay_out_of_trusted_logs(self):
        task = next(t for t in evaluation.collection() if t["id"] == "dev-006")
        files = self.files(task)
        files[task["source"]] += b'''
original_arguments = route_grant_arguments
def route_grant_arguments(arguments):
 try:
  return original_arguments(arguments)
 except HarnessError:
  raise HarnessError('synthetic-private-exception')
'''
        output = self.root / "exception-tag"
        with mock.patch.object(evaluation, "require_historical_candidate", return_value="synthetic-probe"):
            report = oracle.grade(task, files, output)
        self.assertTrue(report["passed"], report)
        for name in ("wire.log", "oracle.log"):
            self.assertNotIn("synthetic-private-exception", (output / name).read_text())

    def test_argument_containers_are_graded_and_integer_truthiness_reaches_assertions(self):
        task = next(t for t in evaluation.collection() if t["id"] == "dev-006")
        files = self.files(task)
        files[task["source"]] += b'''
original_arguments = route_grant_arguments
def route_grant_arguments(arguments):
 return tuple(original_arguments(arguments))
'''
        with mock.patch.object(evaluation, "require_historical_candidate", return_value="synthetic-probe"):
            report = oracle.grade(task, files, self.root / "container-observation")
        self.assertFalse(report["passed"], report)
        self.assertEqual(report["cause"], "test_failure")
        self.assertIn("argument-return-container-type", report["coverage"]["graded"])
        task = next(t for t in evaluation.collection() if t["id"] == "dev-022")
        files = self.files(task)
        files[task["source"]] += b'''
original_verify_case = verify_case
def verify_case(*args):
 result = original_verify_case(*args)
 return {'passed': int(result['passed']), 'checks': {key: int(value) for key, value in result['checks'].items()}}
'''
        with mock.patch.object(evaluation, "require_historical_candidate", return_value="synthetic-probe"):
            report = oracle.grade(task, files, self.root / "integer-observation")
        self.assertTrue(report["passed"], report)
        self.assertFalse(report["adapter_policy_failed"])
        self.assertFalse(report["protocol_failed"])
        self.assertIn("exact-boolean-result-type", report["coverage"]["not_graded"])
        self.assertIn("passed-truthiness", report["coverage"]["graded"])

    def test_unsupported_parser_namespace_is_a_policy_failure_without_reconstruction(self):
        task = next(t for t in evaluation.collection() if t["id"] == "dev-006")
        files = self.files(task)
        files[task["source"]] += b'''
class HostileParser:
 def parse_args(self, arguments):
  return argparse.Namespace(root={'__reduce__': 'payload'})
def parser(): return HostileParser()
'''
        with mock.patch.object(evaluation, "require_historical_candidate", return_value="synthetic-probe"):
            report = oracle.grade(task, files, self.root / "hostile-namespace")
        self.assertFalse(report["passed"])
        self.assertEqual(report["cause"], "adapter_policy")
        self.assertEqual(report["tests"]["errors"], 1)

    def test_verification_preamble_grades_candidate_constants_with_original_assertion(self):
        task = next(t for t in evaluation.collection() if t["id"] == "dev-022")
        files = self.files(task)
        files[task["source"]] += b"\nCASE_ROOTS = {name: 'duplicate' for name in CASE_ROOTS}\n"
        with mock.patch.object(evaluation, "require_historical_candidate", return_value="synthetic-probe"):
            report = oracle.grade(task, files, self.root / "candidate-case-roots")
        self.assertFalse(report["passed"], report)
        self.assertEqual(report["cause"], "test_failure")
        self.assertEqual(report["calls"], 1)
        self.assertEqual(report["tests"]["failures"], 1)
        self.assertIn("candidate-case-root-uniqueness", report["coverage"]["graded"])

    def test_parser_policies_are_separate_from_malformed_wire_and_still_count_failures(self):
        for name, ident, source, cause in (
                ('callable-default', 'dev-007', b'''
original_parser = parser
def parser():
 result = original_parser()
 result.set_defaults(handler=print)
 return result
''', 'adapter_policy'),
                ('nonstandard-exit', 'dev-027', b'''
original_parser = parser
class ParserProxy:
 def parse_args(self, argv):
  try: return original_parser().parse_args(argv)
  except SystemExit: raise SystemExit(1)
def parser(): return ParserProxy()
''', 'adapter_policy'),
                ('forged-policy-payload', 'dev-006', b'''
import __main__
__main__.argument_result = lambda *a: {'adapter_policy': True, 'message': 'untrusted-payload'}
''', 'protocol'),
                ('forged-container', 'dev-006', b'''
import __main__
__main__.argument_result = lambda *a: {'arguments': [], 'container': '__reduce__'}
''', 'protocol')):
            task = next(t for t in evaluation.collection() if t['id'] == ident)
            files = self.files(task)
            files[task['source']] += source
            with self.subTest(name=name), mock.patch.object(
                    evaluation, 'require_historical_candidate', return_value='synthetic-probe'):
                report = oracle.grade(task, files, self.root / name)
            self.assertFalse(report['passed'], report)
            self.assertEqual(report['cause'], cause)
            self.assertEqual(report['adapter_policy_failed'], cause == 'adapter_policy')
            self.assertEqual(report['protocol_failed'], cause == 'protocol')
            self.assertEqual(report['control_attempts'], {'total': 1, 'passed': 0, 'failed': 1})
            self.assertNotIn('untrusted-payload', (self.root / name / 'oracle.log').read_text())

    def test_unsupported_case_roots_stop_before_hidden_assertions_and_count_the_attempt(self):
        task = next(t for t in evaluation.collection() if t['id'] == 'dev-022')
        for name, source, cause in (
                ('unsupported-roots', b"\nCASE_ROOTS = {'case': print}\n", 'adapter_policy'),
                ('malformed-roots', b'''
import os
os.write(1, b'{"ready":true,"function_found":true}\\n{"seq":1,"result":{"case_roots":{"a":1}}}\\n')
os._exit(0)
''', 'protocol')):
            files = self.files(task)
            files[task['source']] += source
            with self.subTest(name=name), mock.patch.object(
                    evaluation, 'require_historical_candidate', return_value='synthetic-probe'):
                report = oracle.grade(task, files, self.root / name)
            self.assertEqual(report['cause'], cause, report)
            self.assertTrue(report['candidate_ready'])
            self.assertEqual(report['calls'], 1)
            self.assertEqual(report['tests']['tests'], 0)
            self.assertEqual(report['control_attempts'], {'total': 1, 'passed': 0, 'failed': 1})

    def test_declared_coverage_matches_allowed_policy_types(self):
        for task in (t for t in evaluation.collection() if t['id'] in oracle.SUPPORTED):
            kind = oracle.SUPPORTED[task['id']]['result_type']
            self.assertEqual(bool(oracle.coverage(task).get('adapter_policy')),
                             kind in oracle.worker.POLICY_RESULT_TYPES)

    def predicate_contrast(self, ident, transform, name, expected):
        task = next(t for t in evaluation.collection() if t['id'] == ident)
        function = oracle.SUPPORTED[ident]['function']
        files = self.files(task)
        result = (transform if ident == 'dev-015' else
                  '{key: ' + transform + ' for key, value in original(*args).items()}')
        files[task['source']] += ('\noriginal = ' + function + '\ndef ' + function + '(*args):\n'
                                  + (' value = original(*args)\n' if ident == 'dev-015' else '')
                                  + ' return ' + result + '\n').encode()
        # Independent and legacy diagnostics use the same candidate projection.
        # The production admission guard and every hidden assertion stay unchanged.
        with mock.patch.object(evaluation, 'require_historical_candidate', return_value='synthetic-probe'):
            report = oracle.grade(task, files, self.root / name)
            legacy = evaluation.grade(task, files, self.root / (name + '-legacy'))
        self.assertEqual(report['passed'], expected, report)
        self.assertEqual(legacy['passed'], expected, legacy)
        self.assertEqual(report['cause'], 'none' if expected else 'test_failure', report)
        self.assertEqual(report['tests']['tests'], task['expected_tests'])
        self.assertFalse(report['protocol_failed'] or report['adapter_policy_failed']
                         or report['candidate_exception_failed'])
        self.assertEqual(report['control_attempts'], {'total': 1, 'passed': int(expected),
                                                    'failed': int(not expected)})
        self.assertIn('exact-boolean-result-type', report['coverage']['not_graded'])
        self.assertIn('selected-predicate-truthiness', report['coverage']['graded'])

    def test_boolean_predicate_truthiness_matches_frozen_assertions_and_legacy(self):
        for ident in ('dev-015', 'dev-029'):
            for name, transform in (('integer', 'int(value)'), ('null', 'value or None')):
                with self.subTest(task=ident, transform=name):
                    self.predicate_contrast(ident, transform, ident + '-' + name, True)
        for name, transform in (('text', "'yes' if value else ''"), ('float', 'float(value)')):
            with self.subTest(transform=name):
                self.predicate_contrast('dev-015', transform, name, True)
        self.predicate_contrast('dev-029', '[1] if value else []', 'map-containers', True)
        for name, transform in (('list', '[1] if value else []'),
                                ('dict', "{'ok': 1} if value else {}")):
            self.predicate_contrast('dev-015', transform, 'predicate-' + name, True)

    def test_wrong_boolean_predicate_values_fail_original_assertions_and_legacy(self):
        for ident in ('dev-015', 'dev-029'):
            with self.subTest(task=ident):
                self.predicate_contrast(ident, 'not value', ident + '-inverted', False)

    def test_case_root_exception_records_no_assertions_but_one_failed_attempt(self):
        task = next(t for t in evaluation.collection() if t['id'] == 'dev-022')
        files = self.files(task)
        files[task['source']] += b'\ndel CASE_ROOTS\ndef __getattr__(name): raise RuntimeError("private-payload")\n'
        output = self.root / 'case-root-exception'
        with mock.patch.object(evaluation, 'require_historical_candidate', return_value='synthetic-probe'):
            report = oracle.grade(task, files, output)
        self.assertEqual(report['cause'], 'candidate_exception', report)
        self.assertEqual(report['calls'], 1)
        self.assertEqual(report['tests']['tests'], 0)
        self.assertEqual(report['control_attempts'], {'total': 1, 'passed': 0, 'failed': 1})
        self.assertNotIn('private-payload', (output / 'wire.log').read_text())

    def test_wrong_json_helper_values_fail_original_assertions_without_policy_or_protocol(self):
        task = next(t for t in evaluation.collection() if t['id'] == 'dev-006')
        for index, value in enumerate(('None', '"wrong"', '1', 'False', '[1]', '[None]', '{}', '[{}]')):
            files = self.files(task)
            files[task['source']] += ('\ndef route_grant_arguments(args): return ' + value + '\n').encode()
            with self.subTest(value=value), mock.patch.object(
                    evaluation, 'require_historical_candidate', return_value='synthetic-probe'):
                report = oracle.grade(task, files, self.root / ('wrong-helper-' + str(index)))
            self.assertEqual(report['cause'], 'test_failure', report)
            self.assertEqual(report['tests']['failures'], 1)
            self.assertFalse(report['protocol_failed'] or report['adapter_policy_failed']
                             or report['candidate_exception_failed'])

    def test_wrong_verification_values_reach_original_truthiness_assertions(self):
        task = next(t for t in evaluation.collection() if t['id'] == 'dev-020')
        for index, value in enumerate(('None', '"wrong"', '1', '0')):
            files = self.files(task)
            files[task['source']] += ("\ndef verify_case(*args): return {'passed': " + value
                                      + ", 'checks': {'validation': True}}\n").encode()
            with self.subTest(value=value), mock.patch.object(
                    evaluation, 'require_historical_candidate', return_value='synthetic-probe'):
                report = oracle.grade(task, files, self.root / ('wrong-predicate-' + str(index)))
            self.assertEqual(report['cause'], 'test_failure', report)
            self.assertGreater(report['tests']['failures'], 0)
            self.assertFalse(report['protocol_failed'] or report['adapter_policy_failed']
                             or report['candidate_exception_failed'])

    def test_candidate_exceptions_have_one_fixed_cause_without_payloads_and_still_count(self):
        probes = [
            ('helper-raises', 'dev-006', 'def route_grant_arguments(args): raise RuntimeError("private-payload")'),
            ('parser-raises', 'dev-007', 'def parser(): raise RuntimeError("private-payload")'),
            ('predicate-raises', 'dev-020', 'def verify_case(*args): raise RuntimeError("private-payload")'),
            ('missing-checks', 'dev-022', 'def verify_case(*args): return {"passed": True}'),
            ('bool-raises', 'dev-015', 'def case_passed(*args): raise RuntimeError("private-payload")'),
            ('path-raises', 'dev-010', 'def test_only_files(*args): raise RuntimeError("private-payload")'),
            ('map-raises', 'dev-029', 'def suspend_resume_checks(*args): raise RuntimeError("private-payload")'),
            ('bool-systemexit', 'dev-015', 'def case_passed(*args): raise SystemExit("private-payload")'),
        ]
        for name, ident, source in probes:
            task = next(t for t in evaluation.collection() if t['id'] == ident)
            files = self.files(task)
            files[task['source']] += ('\n' + source + '\n').encode()
            output = self.root / name
            with self.subTest(name=name), mock.patch.object(
                    evaluation, 'require_historical_candidate', return_value='synthetic-probe'):
                report = oracle.grade(task, files, output)
            self.assertEqual(report['cause'], 'candidate_exception', report)
            self.assertTrue(report['candidate_exception_failed'])
            self.assertFalse(report['protocol_failed'] or report['adapter_policy_failed'])
            self.assertEqual(report['candidate_exit_code'], 0)
            self.assertEqual(report['tests']['tests'], task['expected_tests'])
            self.assertEqual(report['control_attempts'], {'total': 1, 'passed': 0, 'failed': 1})
            for log in ('wire.log', 'oracle.log'):
                self.assertNotIn('private-payload', (output / log).read_text())

    def test_predicate_data_keys_cannot_forge_control_tags_or_verdicts(self):
        for name, value in (('policy-data', '{"adapter_policy": True}'),
                            ('exception-data', '{"candidate_exception": True}'),
                            ('fake-verdict-data', '{"tests": 1, "passed": True}')):
            report = self.probe(('\ndef case_passed(case): return ' + value + '\n').encode(), name)
            self.assertEqual(report['cause'], 'test_failure', report)
            self.assertFalse(report['candidate_exception_failed'] or report['adapter_policy_failed']
                             or report['protocol_failed'])
            self.assertEqual(report['control_attempts'], {'total': 1, 'passed': 0, 'failed': 1})

    def test_raw_forged_control_tags_still_only_fail_and_are_counted(self):
        for name, result, cause in (('raw-policy', {'adapter_policy': True}, 'adapter_policy'),
                                   ('raw-exception', {'candidate_exception': True}, 'candidate_exception'),
                                   ('raw-payload', {'candidate_exception': True, 'message': 'untrusted'}, 'protocol')):
            frame = oracle.worker.encode_message({'seq': 1, 'result': result})
            source = ('\nimport os\ndef case_passed(case):\n os.write(1, ' + repr(frame)
                      + ')\n os._exit(0)\n').encode()
            report = self.probe(source, name)
            self.assertEqual(report['cause'], cause, report)
            self.assertEqual(report['control_attempts'], {'total': 1, 'passed': 0, 'failed': 1})

    def test_reserved_map_keys_reach_unchanged_assertions_and_agree_with_legacy(self):
        task = next(t for t in evaluation.collection() if t['id'] == 'dev-029')
        for key in ('adapter_policy', 'candidate_exception', 'value', 'tests', 'passed'):
            files = self.files(task)
            files[task['source']] += ('\noriginal = suspend_resume_checks\n'
                'def suspend_resume_checks(*args):\n result = original(*args)\n'
                ' result[' + repr(key) + '] = True\n return result\n').encode()
            with mock.patch.object(evaluation, 'require_historical_candidate', return_value='synthetic-probe'):
                report = oracle.grade(task, files, self.root / ('key-' + key))
                legacy = evaluation.grade(task, files, self.root / ('legacy-key-' + key))
            self.assertTrue(report['passed'], report)
            self.assertTrue(legacy['passed'], legacy)
            self.assertEqual(report['cause'], 'none')

    def test_unrepresentable_observations_have_policy_cause_and_clean_candidate_exit(self):
        cases = [('scalar-oversize', 'dev-015', "def case_passed(*a): return 'x' * 70000"),
                 ('scalar-bigint', 'dev-015', 'def case_passed(*a): return 10**5000'),
                 ('scalar-nonfinite', 'dev-015', 'def case_passed(*a): return float("nan")'),
                 ('scalar-object', 'dev-015', 'def case_passed(*a): return object()'),
                 ('scalar-deep', 'dev-015', 'def case_passed(*a):\n v = []\n for _ in range(17): v = [v]\n return v'),
                 ('map-bigint', 'dev-029', 'def suspend_resume_checks(*a): return {"x": 10**5000}'),
                 ('map-oversize', 'dev-029', 'def suspend_resume_checks(*a): return {"x": "a" * 70000}'),
                 ('map-deep', 'dev-029', 'def suspend_resume_checks(*a):\n v = []\n for _ in range(17): v = [v]\n return {"x": v}'),
                 ('argument-bigint', 'dev-006', 'def route_grant_arguments(*a): return [10**5000]'),
                 ('argument-oversize', 'dev-007', 'def memory_arguments(*a): return ["x" * 70000]'),
                 ('verification-oversize', 'dev-020', 'def verify_case(*a): return {"passed": "x" * 70000, "checks": {}}')]
        for name, ident, source in cases:
            task = next(t for t in evaluation.collection() if t['id'] == ident)
            files = self.files(task)
            files[task['source']] += ('\n' + source + '\n').encode()
            with self.subTest(name=name), mock.patch.object(
                    evaluation, 'require_historical_candidate', return_value='synthetic-probe'):
                report = oracle.grade(task, files, self.root / name)
            self.assertEqual(report['cause'], 'adapter_policy', report)
            self.assertTrue(report['adapter_policy_failed'])
            self.assertFalse(report['protocol_failed'] or report['candidate_exception_failed'])
            self.assertEqual(report['candidate_exit_code'], 0)
            self.assertEqual(report['control_attempts'], {'total': 1, 'passed': 0, 'failed': 1})
            self.assertLess((self.root / name / 'wire.log').stat().st_size, oracle.worker.MAX_TRANSCRIPT)

    def test_path_return_types_are_explicitly_outside_oracle_coverage(self):
        task = next(t for t in evaluation.collection() if t["id"] == "dev-010")
        files = self.files(task)
        files[task["source"]] += b'''
original_test_only_files = test_only_files
def test_only_files(sources):
 return [str(path) for path in original_test_only_files(sources)]
'''
        with mock.patch.object(evaluation, "require_historical_candidate", return_value="synthetic-probe"):
            report = oracle.grade(task, files, self.root / "ungraded-return-types")
        self.assertTrue(report["passed"], report)
        self.assertEqual(report["coverage"]["not_graded"], [
            "return-container-type", "return-element-type", "unselected-statement-clauses"])

    def test_only_the_expected_regression_outcome_validates_a_base(self):
        for task in (t for t in evaluation.collection() if t["id"] in oracle.SUPPORTED):
            base = oracle.grade(task, evaluation.projection(task), self.root / task["id"])
            self.assertTrue(oracle.expected_base(task, base), base)
            for update in ({"cause": "infrastructure"}, {"cause": "candidate_exit"},
                           {"candidate_ready": False}, {"function_found": None},
                           {"candidate_exception_failed": not base["candidate_exception_failed"]},
                           {"calls": base["calls"] + 1}, {"exit_code": -9},
                           {"candidate_exit_code": -9}, {"candidate_kind": "frozen-reference"},
                           {"tests": {**base["tests"], "errors": 5}}, {"passed": True}):
                with self.subTest(task=task["id"], update=update):
                    self.assertFalse(oracle.expected_base(task, {**base, **update}))

    def test_candidate_cannot_patch_assertions_or_write_the_oracle_result(self):
        report = self.probe(b'''
import unittest
from pathlib import Path
unittest.TestCase.assertFalse = lambda *args, **kw: None
unittest.TestCase.assertTrue = lambda *args, **kw: None
try:
 Path('/output/tests.json').write_text('{"tests":1,"failures":0}')
except OSError:
 pass
print('Ran 1 test in 0s\\nOK')
def case_passed(case): return True
''', "forgery")
        self.assertFalse(report["passed"], report)
        self.assertEqual(report["cause"], "test_failure")
        self.assertEqual(report["tests"]["failures"], 1)
        self.assertIn("OK", (self.root / "forgery/candidate.log").read_text())
        self.assertIn("FAILED", (self.root / "forgery/oracle.log").read_text())

    def test_path_function_assertions_and_exception_checks_stay_in_the_oracle(self):
        task = next(t for t in evaluation.collection() if t["id"] == "dev-010")
        for name, source in (
                ("path-assertion-patch", b'''
import unittest
unittest.TestCase.assertEqual = lambda *args, **kw: None
def test_only_files(sources): return frozenset()
'''),
                ("foreign-path", b"\ndef test_only_files(sources): return frozenset([Path('/output/tests.json')])\n"),
                ("always-ambiguous", b"\ndef test_only_files(sources): raise AmbiguousReference('forged payload')\n")):
            files = self.files(task)
            files[task["source"]] += source
            with self.subTest(probe=name), mock.patch.object(
                    evaluation, "require_historical_candidate", return_value="synthetic-probe"):
                report = oracle.grade(task, files, self.root / name)
            self.assertFalse(report["passed"], report)
            self.assertTrue(report["candidate_ready"])
            self.assertEqual(report["tests"]["tests"], 1)
            self.assertGreater(report["tests"]["failures"] + report["tests"]["errors"], 0)
            self.assertNotIn("forged payload", (self.root / name / "oracle.log").read_text())

    def test_new_path_controls_still_refuse_generated_source_before_launch(self):
        for task in (t for t in evaluation.collection() if t["id"] in {"dev-010", "dev-012"}):
            files = self.files(task)
            files[task["source"]] += b"\n# generated\n"
            output = self.root / task["id"]
            with self.subTest(task=task["id"]), self.assertRaisesRegex(
                    evaluation.EvaluationError, "untrusted-candidate-disabled"):
                oracle.grade(task, files, output)
            self.assertFalse(output.exists())

    def test_candidate_has_no_hidden_tests_result_mount_or_host_home(self):
        report = self.probe(b'''
import json
from pathlib import Path
assert not Path('/adapter/test.py').exists()
assert not Path('/output').exists()
assert not Path('/grader').exists()
assert not Path('/lane').exists()
assert not Path('/var/home').exists()
assert set(json.loads(Path('/adapter/config.json').read_text())) == {'module','function','result_type'}
original_case_passed = case_passed
def case_passed(case):
 return original_case_passed(case)
''', "mounts")
        self.assertTrue(report["passed"], report)
        self.assertIsNone(report["model_score"])

    def test_forged_frames_early_exit_and_oversize_results_cannot_pass(self):
        for name, source in (
            ("early-exit", b"\nimport os\nos._exit(0)\n"),
            ("forged-counts", b"\ndef case_passed(case): return {'tests':1,'failures':0}\n"),
            ("oversize", b"\ndef case_passed(case): return 'x' * (128 * 1024)\n"),
            ("duplicate-key", b'''
import os
def case_passed(case):
 os.write(1, b'{"seq":1,"seq":1,"result":true}\\n')
 return True
'''),
        ):
            with self.subTest(probe=name):
                report = self.probe(source, name)
                self.assertFalse(report["passed"], report)
                self.assertNotEqual(report["cause"], "none")
        self.assertIn(b'"seq":1,"seq":1', (self.root / "duplicate-key/wire.log").read_bytes())

    def test_generated_and_unsupported_tasks_are_refused_before_output_creation(self):
        files = self.files()
        files[self.task["source"]] += b"\n# generated snapshot\n"
        with self.assertRaisesRegex(evaluation.EvaluationError, "untrusted-candidate-disabled"):
            oracle.grade(self.task, files, self.root / "generated")
        self.assertFalse((self.root / "generated").exists())
        other = next(t for t in evaluation.collection() if t["id"] == "dev-001")
        with self.assertRaisesRegex(evaluation.EvaluationError, "oracle.task-refused"):
            oracle.grade(other, {}, self.root / "unsupported")
        self.assertFalse((self.root / "unsupported").exists())

    def test_correct_replies_followed_by_candidate_failure_cannot_pass(self):
        report = self.probe(b'''
import json
import os
original_case_passed = case_passed
calls = 0
def case_passed(case):
 global calls
 calls += 1
 result = original_case_passed(case)
 if calls == 4:
  os.write(1, (json.dumps({'seq':calls,'result':{'value':result}})+'\\n').encode())
  os._exit(7)
 return result
''', "late-failure")
        self.assertFalse(report["passed"], report)
        self.assertEqual(report["exit_code"], 0)
        self.assertEqual(report["candidate_exit_code"], 7)
        self.assertEqual(report["cause"], "candidate_exit")
        self.assertEqual(report["tests"]["failures"], 0)

    def test_public_cli_exposes_no_generated_scoring_operation(self):
        result = subprocess.run([sys.executable, str(Path(oracle.__file__)), "score",
                                 "--output", str(self.root / "score")], capture_output=True, timeout=10)
        self.assertEqual(result.returncode, 2)
        self.assertFalse((self.root / "score").exists())

    def test_sandboxes_have_disjoint_code_and_only_oracle_has_result_mount(self):
        candidate = oracle.sandbox(Path("candidate"), Path("candidate-adapter"), Path("result"), "candidate")
        trusted = oracle.sandbox(Path("trusted"), Path("oracle-adapter"), Path("result"), "oracle")
        self.assertNotIn("--bind", candidate)
        self.assertIn("--bind", trusted)
        for command in (candidate, trusted):
            self.assertIn("--unshare-all", command)
            self.assertIn("--disable-userns", command)
            self.assertIn("--clearenv", command)
        self.assertNotIn("oracle-adapter", candidate)
        self.assertNotIn("candidate", trusted)

    def test_timeout_and_log_budget_stop_both_owned_process_groups(self):
        for name, code, timeout in (
            ("timeout", "import time; time.sleep(20)", 0.2),
            ("output", "import os; os.write(2,b'x'*20000)", 5),
        ):
            output = self.root / name
            output.mkdir()
            commands = [sys.executable, "-c", code]
            spawned = []
            real_popen = subprocess.Popen

            def spawn(*args, **kwargs):
                process = real_popen(*args, **kwargs)
                spawned.append(process)
                return process

            with mock.patch.object(oracle, "LOG_LIMIT", 4096), mock.patch.object(
                    oracle.subprocess, "Popen", side_effect=spawn):
                _, _, timed_out, limited = oracle.exchange(commands, commands, output, timeout)
            self.assertEqual(timed_out, name == "timeout")
            self.assertEqual(limited, name == "output")
            self.assertEqual(len(spawned), 2)
            for process in spawned:
                self.assertIsNotNone(process.returncode)
                with self.assertRaises(ProcessLookupError):
                    os.killpg(process.pid, 0)
            for path in output.iterdir():
                self.assertLessEqual(path.stat().st_size, 4096)

    def test_second_launch_failure_and_interruption_clean_first_process(self):
        for name, error in (("launch", OSError("synthetic launch failure")),
                            ("interrupt", KeyboardInterrupt())):
            spawned = []
            output = self.root / name
            output.mkdir()
            real_popen = subprocess.Popen

            def spawn(*args, **kwargs):
                if spawned:
                    raise error
                process = real_popen(*args, **kwargs)
                spawned.append(process)
                return process

            with mock.patch.object(oracle.subprocess, "Popen", side_effect=spawn):
                with self.assertRaises(type(error)):
                    oracle.exchange([sys.executable, "-c", "import time; time.sleep(20)"],
                                    [sys.executable, "-c", "pass"], output, 5)
            self.assertIsNotNone(spawned[0].returncode)
            with self.assertRaises(ProcessLookupError):
                os.killpg(spawned[0].pid, 0)

    def test_interruption_during_exchange_cleans_both_processes(self):
        output = self.root / "active-interruption"
        output.mkdir()
        spawned = []
        real_popen = subprocess.Popen

        def spawn(*args, **kwargs):
            process = real_popen(*args, **kwargs)
            spawned.append(process)
            return process

        selector = oracle.selectors.DefaultSelector
        with mock.patch.object(oracle.subprocess, "Popen", side_effect=spawn), mock.patch.object(
                selector, "select", side_effect=KeyboardInterrupt()):
            with self.assertRaises(KeyboardInterrupt):
                command = [sys.executable, "-c", "import time; time.sleep(20)"]
                oracle.exchange(command, command, output, 5)
        self.assertEqual(len(spawned), 2)
        for process in spawned:
            self.assertIsNotNone(process.returncode)
            with self.assertRaises(ProcessLookupError):
                os.killpg(process.pid, 0)

    def test_cleanup_failure_never_promotes_counts_or_success(self):
        files = self.files()
        remove = evaluation.remove_case_workspace

        def cleanup_failure(root, output):
            remove(root, output)
            raise evaluation.EvaluationError("evaluation.cleanup-failed")

        with mock.patch.object(evaluation, "remove_case_workspace", side_effect=cleanup_failure):
            report = oracle.grade(self.task, files, self.root / "cleanup")
        self.assertFalse(report["passed"])
        self.assertIsNone(report["tests"])
        self.assertEqual(report["cause"], "infrastructure")
        self.assertNotIn("private failure", json.dumps(report))
        self.assertFalse((self.root / "cleanup/tests.json").exists())
        self.assertFalse((self.root / "cleanup/wire.log").exists())

    def test_input_integrity_failure_is_not_a_failed_test_control(self):
        files = self.files()
        with mock.patch.object(oracle, "grade_case", side_effect=evaluation.InputIntegrityError(
                "evaluation.input-hash-mismatch")):
            report = oracle.grade(self.task, files, self.root / "input")
        self.assertEqual(report["cause"], "input_integrity")
        self.assertIsNone(report["tests"])
        self.assertFalse(report["passed"])

    def test_missing_counts_skips_and_wrong_count_cannot_be_results(self):
        path = self.root / "tests.json"
        valid = {"tests": {"tests": 1, "failures": 0, "errors": 0, "skipped": 0,
                           "expected_failures": 0, "unexpected_successes": 0},
                 "calls": 4, "protocol_failed": False, "adapter_policy_failed": False,
                 "candidate_exception_failed": False,
                 "candidate_ready": True, "function_found": True}
        for update in ({"candidate_exception_failed": 1}, {"adapter_policy_failed": 1}, {"tests": {}}, {"calls": True}, {"calls": 0},
                       {"protocol_failed": "false"}, {"tests": {**valid["tests"], "tests": 0}},
                       {"tests": {**valid["tests"], "failures": False}},
                       {"candidate_ready": 1}, {"function_found": 1},
                       {"candidate_ready": False}, {"function_found": None}):
            path.write_text(json.dumps({**valid, **update}))
            with self.subTest(update=update), self.assertRaises(evaluation.EvaluationError):
                oracle.result_record(path, self.task)
        for field in ("skipped", "expected_failures", "unexpected_successes"):
            record = {**valid, "tests": {**valid["tests"], field: 1}}
            with mock.patch.object(oracle, "grade_case", return_value=(0, 0, False, False, record)):
                report = oracle.grade(self.task, self.files(), self.root / field)
            self.assertFalse(report["passed"], report)

    def test_startup_failure_record_requires_zero_assertions_and_no_function(self):
        counts = {name: 0 for name in ("tests", "failures", "errors", "skipped",
                                       "expected_failures", "unexpected_successes")}
        valid = {"tests": counts, "calls": 0, "protocol_failed": True, "adapter_policy_failed": False,
                 "candidate_exception_failed": False,
                 "candidate_ready": False, "function_found": None}
        path = self.root / "startup.json"
        path.write_text(json.dumps(valid))
        self.assertEqual(oracle.result_record(path, self.task), valid)
        for update in ({"calls": 1}, {"protocol_failed": False}, {"function_found": False},
                       {"candidate_exception_failed": True},
                       {"tests": {**counts, "tests": 1}}, {"tests": {**counts, "errors": 1}}):
            path.write_text(json.dumps({**valid, **update}))
            with self.subTest(update=update), self.assertRaises(evaluation.EvaluationError):
                oracle.result_record(path, self.task)

    def test_zero_assertions_require_a_failed_case_root_observation(self):
        task = next(t for t in evaluation.collection() if t['id'] == 'dev-022')
        counts = {name: 0 for name in ('tests', 'failures', 'errors', 'skipped',
                                      'expected_failures', 'unexpected_successes')}
        valid = {'tests': counts, 'calls': 1, 'protocol_failed': False,
                 'adapter_policy_failed': True, 'candidate_exception_failed': False,
                 'candidate_ready': True, 'function_found': True}
        path = self.root / 'observation.json'
        for field in ('protocol_failed', 'adapter_policy_failed', 'candidate_exception_failed'):
            value = {**valid, 'protocol_failed': False, 'adapter_policy_failed': False,
                     'candidate_exception_failed': False, field: True}
            path.write_text(json.dumps(value))
            self.assertEqual(oracle.result_record(path, task), value)
            with self.assertRaises(evaluation.EvaluationError):
                oracle.result_record(path, self.task)
        for update in ({'calls': 0}, {'calls': 2}, {'adapter_policy_failed': False},
                       {'tests': {**counts, 'errors': 1}}, {'candidate_ready': False}):
            path.write_text(json.dumps({**valid, **update}))
            with self.subTest(update=update), self.assertRaises(evaluation.EvaluationError):
                oracle.result_record(path, task)

    def test_every_reported_failure_stays_in_control_attempt_counts(self):
        counts = {name: 0 for name in ("failures", "errors", "skipped",
                                       "expected_failures", "unexpected_successes")}
        ready = {"tests": {**counts, "tests": 1}, "calls": 4, "protocol_failed": False, "adapter_policy_failed": False,
                 "candidate_exception_failed": False,
                 "candidate_ready": True, "function_found": True}
        not_ready = {"tests": {**counts, "tests": 0}, "calls": 0, "protocol_failed": True, "adapter_policy_failed": False,
                 "candidate_exception_failed": False,
                     "candidate_ready": False, "function_found": None}
        for cause, result in (
                ("none", (0, 0, False, False, ready)),
                ("infrastructure", (1, 1, False, False, not_ready)),
                ("infrastructure", (1, 1, False, False, None)),
                ("timeout", (None, None, True, False, None)),
                ("output_limit", (None, None, False, True, None)),
                ("protocol", (1, 1, False, False, {**ready, "protocol_failed": True})),
                ("adapter_policy", (1, 0, False, False, {**ready, "adapter_policy_failed": True})),
                ("candidate_exception", (1, 0, False, False, {**ready, "candidate_exception_failed": True})),
                ("candidate_exit", (0, 7, False, False, ready)),
                ("test_failure", (1, 0, False, False, {
                    **ready, "tests": {**ready["tests"], "failures": 1}})),
                ("input_integrity", evaluation.InputIntegrityError("evaluation.input-hash-mismatch")),
                ("infrastructure", OSError("synthetic launch failure"))):
            path = self.root / str(len(list(self.root.iterdir())))
            with self.subTest(cause=cause), mock.patch.object(oracle, "grade_case") as grade_case:
                if isinstance(result, Exception):
                    grade_case.side_effect = result
                else:
                    grade_case.return_value = result
                report = oracle.grade(self.task, self.files(), path)
            self.assertEqual(report["cause"], cause)
            self.assertEqual(report["control_attempts"], {
                "total": 1, "passed": int(cause == "none"), "failed": int(cause != "none")})


if __name__ == "__main__":
    unittest.main()
