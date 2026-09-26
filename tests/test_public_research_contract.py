import unittest
from unittest.mock import Mock, patch

from scripts import public_research_contract as contract


class PublicResearchContractTests(unittest.TestCase):
    def test_missing_host_export_and_host_effects_are_rejected(self) -> None:
        for source, failure in (
            ("", "host public research compatibility boundary absent"),
            (contract.HOST_SOURCE.read_text() + "\nstd::process\n", "public research contract acquired authority: std::process"),
        ):
            # Keep the synthetic forbidden text in production, before any fixture section.
            source = source.replace("#[cfg(test)]", "// synthetic test section")
            with patch.object(contract, "HOST_SOURCE", Mock(read_text=Mock(return_value=source))):
                self.assertIn(failure, contract.validate())

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
