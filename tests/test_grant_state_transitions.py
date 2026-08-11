import unittest

from scripts import grant_state_transitions as transitions


class GrantStateTransitionTests(unittest.TestCase):
    def test_statuses_and_transition_paths_are_complete(self) -> None:
        coverage = transitions.validate_sources()
        self.assertEqual(coverage["grant_status_count"], 6)
        self.assertEqual(coverage["implemented_transition_count"], 8)
        self.assertEqual(coverage["session_parent_transition_count"], 3)
        self.assertEqual(coverage["operation_transition_count"], 5)
        self.assertEqual(coverage["reserved_without_producer"], ["Revoked"])

    def test_missing_status_transition_and_boundary_are_detected(self) -> None:
        text = transitions.DOC_PATH.read_text(encoding="utf-8")
        for marker in (
            "`revoked`",
            "`P3`",
            "OperationIssued --> OperationExpired",
            "`ParentExpired` is currently an error classification",
        ):
            failures = transitions.validate_document(text.replace(marker, "removed"))
            self.assertTrue(failures, marker)

    def test_grant_status_parser_preserves_wire_order(self) -> None:
        source = transitions.GRANT_PATH.read_text(encoding="utf-8")
        self.assertEqual(
            transitions.rust_enum_variants(source, "GrantStatus"),
            transitions.GRANT_STATUSES,
        )


if __name__ == "__main__":
    unittest.main()
