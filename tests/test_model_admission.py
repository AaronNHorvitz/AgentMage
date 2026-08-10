from __future__ import annotations

import copy
import unittest

from scripts.model_admission import load_record, validate_record


class ModelAdmissionTests(unittest.TestCase):
    def setUp(self) -> None:
        self.record = load_record()

    def test_committed_source_admission_is_valid_and_blocked(self) -> None:
        self.assertEqual(validate_record(self.record), [])
        self.assertEqual(self.record["decision"]["status"], "BLOCKED")
        self.assertFalse(
            self.record["ownership_and_origin"]["lineage_complete_under_policy"]
        )

    def test_missing_unknown_and_mutable_identity_fields_are_rejected(self) -> None:
        missing = copy.deepcopy(self.record)
        del missing["tokenizer"]
        self.assertTrue(any("missing top-level" in item for item in validate_record(missing)))

        unknown = copy.deepcopy(self.record)
        unknown["silent_override"] = True
        self.assertTrue(any("unknown top-level" in item for item in validate_record(unknown)))

        mutable = copy.deepcopy(self.record)
        mutable["identity"]["upstream_revision"] = "latest"
        self.assertTrue(any("immutable revision" in item for item in validate_record(mutable)))

    def test_license_origin_tokenizer_and_context_drift_are_rejected(self) -> None:
        mutations = {
            "Apache-2.0": lambda value: value["license"].update({"spdx": "unknown"}),
            "supplier control": lambda value: value["ownership_and_origin"].update(
                {"control_disposition": "unknown"}
            ),
            "tokenizer": lambda value: value["tokenizer"].update(
                {"tokenizer_json_sha256": "0" * 64}
            ),
            "context": lambda value: value["context_contract"].update(
                {"declared_tokens": 262144}
            ),
        }
        for expected, mutate in mutations.items():
            with self.subTest(field=expected):
                changed = copy.deepcopy(self.record)
                mutate(changed)
                failures = validate_record(changed)
                self.assertTrue(any(expected in item for item in failures), failures)

    def test_incomplete_lineage_cannot_be_changed_to_pass_or_activation(self) -> None:
        changed = copy.deepcopy(self.record)
        changed["decision"]["status"] = "PASS"
        changed["decision"]["prohibited_actions"].remove("AgentMage profile activation")

        failures = validate_record(changed)

        self.assertTrue(any("BLOCKED decision" in item for item in failures))
        self.assertTrue(any("prohibit activation" in item for item in failures))


if __name__ == "__main__":
    unittest.main()
