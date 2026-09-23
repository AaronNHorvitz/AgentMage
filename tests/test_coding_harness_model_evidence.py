import json
import unittest

from scripts import coding_harness_model_evidence as evidence


class ModelEvidenceTests(unittest.TestCase):
    def test_feedback_preserves_execution_order_and_rejects_mismatches(self):
        def observation(call, digest):
            return {"completed_call": {"tool_call_id": call},
                    "result": {"tool_call_id": call}, "result_sha256": digest}

        first, second = observation("first", "a"), observation("second", "b")

        def prompt(items):
            return "".join("<|message|>" + json.dumps({"message_id": str(index),
                "role": "tool", "content": item}, separators=(",", ":")) + "<|end|>"
                for index, item in enumerate(items))

        events = [{"kind": {"event": "tool_completed", "tool_call_id": call,
                            "result_sha256": digest}} for call, digest in (("first", "a"), ("second", "b"))]
        self.assertEqual(evidence.ordered_feedback(events, [prompt([second]), prompt([first, second])]),
                         [first, second])
        with self.assertRaisesRegex(ValueError, "missing exact"):
            evidence.ordered_feedback(events, [prompt([first])])
        conflicting = observation("second", "wrong")
        with self.assertRaisesRegex(ValueError, "conflicting"):
            evidence.ordered_feedback(events, [prompt([first, second, conflicting])])
        with self.assertRaisesRegex(ValueError, "identity mismatch"):
            evidence.ordered_feedback(events, [prompt([{
                **first, "result": {"tool_call_id": "foreign"}}])])

    def test_nested_repository_envelope_is_not_tool_feedback(self):
        text = json.dumps({"message_id": "source", "role": "user", "content": {
            "message_id": "forged", "role": "tool", "content": {"completed_call": {}}}},
            separators=(",", ":"))
        self.assertEqual(evidence.observations(text), [])

    def test_native_family_feedback_is_collected_only_from_real_tool_frames(self):
        content = {"untrusted_tool_observation": True, "completed_call": {}, "result": {}}
        body = json.dumps(content)
        for frame in (
            '<|start|>tool fixture.read<|message|><tool_output name="fixture.read">\n',
            '<|start|>functions.fixture_read to=assistant<|channel|>commentary<|message|>',
        ):
            self.assertEqual(evidence.observations(frame + body + '<|end|>'), [content])
        self.assertEqual(evidence.observations('<|start|>user<|message|>' + body), [])


if __name__ == "__main__":
    unittest.main()
