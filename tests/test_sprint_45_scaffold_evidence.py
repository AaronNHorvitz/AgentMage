"""Mutation tests for the Sprint 45 scaffold application artifact."""

import copy
import unittest
from unittest.mock import patch

from scripts import sprint_45_scaffold_evidence as evidence


class Sprint45ScaffoldEvidenceTests(unittest.TestCase):
    def test_valid_shape_and_mutations(self) -> None:
        with patch.object(evidence, "git_bytes", return_value=b"source"):
            value = evidence.expected(
                "a" * 40,
                0,
                b"test result: ok. 5 passed; 0 failed; 0 ignored; 0 measured; 43 filtered out;",
            )
            self.assertEqual(evidence.validate(value), [])
            mutations = (
                lambda changed: changed.update({"command_exit_code": 1}),
                lambda changed: changed.update({"application_manifest_count": 4}),
                lambda changed: changed.update({"mutation_case_count": 6}),
                lambda changed: changed.update({"ignored_test_count": 1}),
                lambda changed: changed.update({"mutation_authority": True}),
                lambda changed: changed["source_sha256"].pop(next(iter(changed["source_sha256"]))),
            )
            for mutate in mutations:
                changed = copy.deepcopy(value)
                mutate(changed)
                self.assertTrue(evidence.validate(changed))


if __name__ == "__main__":
    unittest.main()
