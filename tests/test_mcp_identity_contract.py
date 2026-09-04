import unittest

from scripts import mcp_identity_contract as contract


class McpIdentityContractTests(unittest.TestCase):
    def test_current_contract_is_inert_and_complete(self) -> None:
        self.assertEqual(contract.validate(), [])

    def test_missing_contract_type_is_rejected(self) -> None:
        original = contract.CONTRACT_TYPES
        try:
            contract.CONTRACT_TYPES = (*original, "MissingMcpRecord")
            self.assertIn("MCP contract token absent: MissingMcpRecord", contract.validate())
        finally:
            contract.CONTRACT_TYPES = original


if __name__ == "__main__":
    unittest.main()
