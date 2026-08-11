import copy
import unittest

from scripts import authority_escalation_matrix as authority


class AuthorityEscalationMatrixTests(unittest.TestCase):
    def test_all_sources_and_escalations_are_attributed_and_denied(self) -> None:
        coverage = authority.validate_receipts(authority.run_receipts())
        self.assertEqual(coverage["receipt_count"], 28)
        self.assertEqual(coverage["source_count"], 7)
        self.assertEqual(coverage["escalation_kind_count"], 4)
        self.assertEqual(coverage["admitted_authority_count"], 0)
        self.assertEqual(coverage["actor_identity_count"], 1)
        self.assertEqual(coverage["redacted_error_code_count"], 1)

    def test_missing_source_or_escalation_and_success_are_rejected(self) -> None:
        receipts = authority.run_receipts()
        with self.assertRaises(authority.AuthorityEscalationError):
            authority.validate_receipts(receipts[:-1])
        changed = copy.deepcopy(receipts)
        changed[0]["authority_admitted"] = True
        with self.assertRaises(authority.AuthorityEscalationError):
            authority.validate_receipts(changed)

    def test_actor_artifact_error_and_authority_material_mutations_are_rejected(self) -> None:
        receipts = authority.run_receipts()
        for field, value in (
            ("actor_id", "actor-other"),
            ("artifact_kind", "approval_request"),
        ):
            changed = copy.deepcopy(receipts)
            changed[0][field] = value
            with self.assertRaises(authority.AuthorityEscalationError):
                authority.validate_receipts(changed)
        changed = copy.deepcopy(receipts)
        changed[0]["error"]["code"] = "policy.allow"
        with self.assertRaises(authority.AuthorityEscalationError):
            authority.validate_receipts(changed)
        changed = copy.deepcopy(receipts)
        changed[0]["nonce"] = "forged"
        with self.assertRaises(authority.AuthorityEscalationError):
            authority.validate_receipts(changed)


if __name__ == "__main__":
    unittest.main()
