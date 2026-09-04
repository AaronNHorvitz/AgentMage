import unittest

from scripts import mcp_artifact_adapter_contract as contract


class McpArtifactAdapterContractTests(unittest.TestCase):
    def test_current_contract_is_bounded(self) -> None:
        self.assertEqual(contract.validate(), [])

    def test_missing_boundary_is_rejected(self) -> None:
        original = contract.REQUIRED_ADAPTER
        try:
            contract.REQUIRED_ADAPTER = (*original, "MissingMcpArtifactBoundary")
            self.assertIn(
                "MCP artifact adapter boundary absent: MissingMcpArtifactBoundary",
                contract.validate(),
            )
        finally:
            contract.REQUIRED_ADAPTER = original


if __name__ == "__main__":
    unittest.main()
