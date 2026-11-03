"""Hostile wire data must remain data, never test results or executable objects."""
import io
import json
import unittest
from unittest import mock

from scripts import coding_eval_oracle_worker as worker


class OracleProtocolTests(unittest.TestCase):
    def remote(self, raw, result_type="bool"):
        return worker.RemoteFunction(io.BytesIO(raw), io.BytesIO(), io.BytesIO(), result_type)

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
