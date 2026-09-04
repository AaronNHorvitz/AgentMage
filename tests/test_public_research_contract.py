import unittest

from scripts import public_research_contract as contract


class PublicResearchContractTests(unittest.TestCase):
    def test_current_contract_is_bounded(self) -> None:
        self.assertEqual(contract.validate(), [])

    def test_missing_boundary_is_rejected(self) -> None:
        original = contract.REQUIRED
        try:
            contract.REQUIRED = (*original, "MissingPublicResearchBoundary")
            self.assertIn(
                "public research boundary absent: MissingPublicResearchBoundary",
                contract.validate(),
            )
        finally:
            contract.REQUIRED = original


if __name__ == "__main__":
    unittest.main()
