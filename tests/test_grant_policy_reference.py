import unittest

from scripts import grant_policy_reference as reference


class GrantPolicyReferenceTests(unittest.TestCase):
    def test_source_contract_and_document_are_complete(self) -> None:
        coverage = reference.validate_sources()
        self.assertEqual(coverage["grant_field_count"], 29)
        self.assertEqual(coverage["grant_operation_count"], 15)
        self.assertEqual(coverage["policy_denial_scope_count"], 15)
        self.assertEqual(coverage["strict_explicit_denial_count"], 12)

    def test_missing_field_scope_code_and_operation_are_detected(self) -> None:
        text = reference.DOC_PATH.read_text(encoding="utf-8")
        for marker in (
            "`parent_grant_sha256`",
            "`Preimage`",
            "`policy.deny.preimage`",
            "`credential_access`",
        ):
            failures = reference.validate_reference_text(text.replace(marker, "removed"))
            self.assertTrue(failures, marker)

    def test_rust_parsers_preserve_security_relevant_order(self) -> None:
        grant = reference.GRANT_PATH.read_text(encoding="utf-8")
        policy = reference.POLICY_PATH.read_text(encoding="utf-8")
        self.assertEqual(
            reference.rust_struct_fields(grant, "CapabilityGrant"),
            reference.GRANT_FIELDS,
        )
        self.assertEqual(
            reference.rust_enum_variants(policy, "PolicyDenialScope"),
            reference.DENIAL_SCOPES,
        )
        self.assertEqual(
            reference.strict_denied_operations(policy),
            reference.STRICT_DENIED,
        )


if __name__ == "__main__":
    unittest.main()
