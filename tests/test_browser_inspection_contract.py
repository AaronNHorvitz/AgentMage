import unittest
from scripts import browser_inspection_contract as contract


class BrowserInspectionContractTests(unittest.TestCase):
    def test_current_contract_is_bounded(self) -> None:
        self.assertEqual(contract.validate(), [])

    def test_missing_boundary_is_rejected(self) -> None:
        original = contract.REQUIRED
        try:
            contract.REQUIRED = (*original, "MissingBrowserBoundary")
            self.assertIn("browser inspection boundary absent: MissingBrowserBoundary", contract.validate())
        finally:
            contract.REQUIRED = original


if __name__ == "__main__":
    unittest.main()
