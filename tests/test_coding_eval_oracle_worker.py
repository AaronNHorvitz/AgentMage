"""Hostile wire data must remain data, never test results or executable objects."""
import io
import json
from pathlib import Path
import unittest
from unittest import mock

from scripts import coding_eval_oracle_worker as worker


class OracleProtocolTests(unittest.TestCase):
    def remote(self, raw, result_type="bool"):
        return worker.RemoteFunction(io.BytesIO(raw), io.BytesIO(), io.BytesIO(), result_type)

    def test_startup_requires_one_exact_frame_before_assertions(self):
        for found in (True, False):
            raw = worker.encode_message({"ready": True, "function_found": found})
            remote = self.remote(raw)
            remote.start()
            self.assertTrue(remote.candidate_ready)
            self.assertIs(remote.function_found, found)
            self.assertEqual(remote.calls, 0)
            self.assertEqual(remote.writer.getvalue(), b'')
            self.assertEqual(remote.transcript.getvalue(), raw)
            with self.assertRaises(worker.ProtocolError):
                remote.start()
            self.assertTrue(remote.failed)

    def test_missing_malformed_or_forged_startup_is_sticky_failure(self):
        for raw in (b'', b'{"ready":true}\n', b'{"ready":1,"function_found":true}\n',
                    b'{"ready":true,"function_found":1}\n',
                    b'{"ready":true,"function_found":true,"tests":1}\n',
                    b'{"ready":true,"ready":true,"function_found":true}\n',
                    b'{"seq":1,"result":true}\n', b'x' * (worker.MAX_MESSAGE + 2)):
            remote = self.remote(raw)
            with self.subTest(raw=raw[:80]), self.assertRaises(worker.ProtocolError):
                remote.start()
            self.assertFalse(remote.candidate_ready)
            self.assertIsNone(remote.function_found)
            self.assertTrue(remote.failed)
            self.assertEqual(remote.calls, 0)
            remote.reader = io.BytesIO(b'{"ready":true,"function_found":true}\n')
            with self.assertRaises(worker.ProtocolError):
                remote.start()
            self.assertFalse(remote.candidate_ready)

    def test_path_calls_rebuild_values_and_only_the_fixed_trusted_exception(self):
        class Ambiguous(ValueError):
            pass

        raw = (b'{"seq":1,"result":{"ambiguous":true}}\n'
               b'{"seq":2,"result":{"paths":["crate/src/test.rs"]}}\n')
        remote = self.remote(raw, "path-result")
        call = worker.path_proxy(remote, Ambiguous)
        sources = {Path("crate/src/test.rs"): "fn test() {}"}
        with self.assertRaisesRegex(Ambiguous, "oracle.remote-ambiguity"):
            call(sources)
        self.assertFalse(remote.failed)
        self.assertEqual(call(sources), frozenset(sources))
        self.assertEqual(remote.calls, 2)
        requests = [json.loads(line) for line in remote.writer.getvalue().splitlines()]
        self.assertEqual(requests[0]["args"], [{"crate/src/test.rs": "fn test() {}"}])
        self.assertEqual(requests[1]["args"], requests[0]["args"])

    def test_path_results_refuse_foreign_paths_duplicates_and_exception_payloads(self):
        for result in ({"paths": ["../outside.rs"]}, {"paths": ["/output/tests.json"]},
                       {"paths": ["x.rs", "x.rs"]}, {"paths": [1]}, {"paths": {}},
                       {"ambiguous": False}, {"ambiguous": "ValueError"},
                       {"ambiguous": True, "message": "executable expression"},
                       {"exception": "AmbiguousReference", "args": []}, {"__reduce__": []}):
            remote = self.remote(worker.encode_message({"seq": 1, "result": result}), "path-result")
            with self.subTest(result=result), self.assertRaises(worker.ProtocolError):
                remote({"x.rs": "source"})
            self.assertTrue(remote.failed)

    def test_candidate_path_adapter_transports_no_other_exception_or_message(self):
        class Ambiguous(ValueError):
            pass

        def uncertain(sources):
            raise Ambiguous("candidate-selected payload")

        def failed(sources):
            raise RuntimeError("candidate-selected payload")

        self.assertEqual(worker.path_result(lambda sources: frozenset(sources), Ambiguous,
                                            {"a.rs": "text"}), {"paths": ["a.rs"]})
        self.assertEqual(worker.path_result(uncertain, Ambiguous, {}), {"ambiguous": True})
        with self.assertRaises(RuntimeError):
            worker.path_result(failed, Ambiguous, {})

    def test_json_call_preserves_arguments_and_requires_exact_sequence_and_type(self):
        remote = self.remote(b'{"seq":1,"result":false}\n')
        self.assertIs(remote({"measured": [0, True, "text"]}), False)
        self.assertEqual(json.loads(remote.writer.getvalue()),
                         {"seq": 1, "args": [{"measured": [0, True, "text"]}]})
        self.assertEqual(remote.calls, 1)
        self.assertFalse(remote.failed)
        self.assertIn(b'"result":false', remote.transcript.getvalue())
        for reply in ({"seq": True, "result": True}, {"seq": 2, "result": True},
                      {"seq": 1, "result": 1}, {"seq": 1, "result": []},
                      {"seq": 1, "result": True, "tests": {"failures": 0}},
                      {"seq": 1, "error": "forged"}):
            with self.subTest(reply=reply), self.assertRaises(worker.ProtocolError):
                self.remote(worker.encode_message(reply))({})

    def test_rejects_duplicate_keys_nonfinite_deep_truncated_and_non_json_frames(self):
        for raw in (b'{"seq":1,"seq":1,"result":true}\n',
                    b'{"seq":1,"result":NaN}\n', b'{"seq":1,"result":Infinity}\n',
                    b'\xff\n', b'{"seq":1,"result":true}', b'', b'Ran 1 test: OK\n',
                    b'[' * 2000 + b']' * 2000 + b'\n', b'{}\n' * 2):
            remote = self.remote(raw)
            with self.subTest(raw=raw[:50]), self.assertRaises(worker.ProtocolError):
                remote({})
            self.assertTrue(remote.failed)
            with self.assertRaises(worker.ProtocolError):
                remote({})

    def test_oversize_input_is_bounded_and_retained_before_refusal(self):
        remote = self.remote(b'x' * (worker.MAX_MESSAGE * 3) + b'\n')
        with self.assertRaises(worker.ProtocolError):
            remote({})
        self.assertLessEqual(len(remote.transcript.getvalue()), worker.MAX_MESSAGE + 100)
        self.assertIn(b'x' * 100, remote.transcript.getvalue())
        self.assertEqual(remote.reader.tell(), worker.MAX_MESSAGE + 1)

    def test_bool_map_is_nonempty_with_only_boolean_values(self):
        remote = self.remote(b'{"seq":1,"result":{"answers":true,"filesystem":false}}\n', "bool-map")
        self.assertEqual(remote(0, [], []), {"answers": True, "filesystem": False})
        for result in ({}, {"answer": 1}, {"answer": None}, [], True):
            with self.subTest(result=result), self.assertRaises(worker.ProtocolError):
                self.remote(worker.encode_message({"seq": 1, "result": result}), "bool-map")()

    def test_call_and_transcript_budgets_fail_sticky_without_unbounded_logs(self):
        remote = self.remote(b'{"seq":1,"result":true}\n')
        remote.calls = worker.MAX_CALLS
        with self.assertRaises(worker.ProtocolError):
            remote()
        self.assertTrue(remote.failed)
        remote = self.remote(b'{"seq":1,"result":true}\n')
        with mock.patch.object(worker, "MAX_TRANSCRIPT", 8), self.assertRaises(worker.ProtocolError):
            remote({})
        self.assertEqual(len(remote.transcript.getvalue()), 8)
        self.assertEqual(remote.writer.getvalue(), b'')
        self.assertTrue(remote.failed)

    def test_outgoing_objects_and_nonfinite_values_cannot_escape_json(self):
        for value in (object(), float("nan"), float("inf"), "x" * worker.MAX_MESSAGE):
            remote = self.remote(b'{"seq":1,"result":true}\n')
            with self.subTest(value=type(value).__name__), self.assertRaises(worker.ProtocolError):
                remote(value)
            self.assertTrue(remote.failed)
            self.assertEqual(remote.writer.getvalue(), b'')


if __name__ == "__main__":
    unittest.main()
