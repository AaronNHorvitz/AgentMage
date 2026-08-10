from __future__ import annotations

import unittest

from scripts.kernel_contract_reference import (
    boundary_error_codes,
    exported_symbols,
    validate_reference_text,
    versioned_contracts,
)


class KernelContractReferenceTests(unittest.TestCase):
    def test_public_export_and_versioned_contract_parsers_are_closed(self) -> None:
        exports = exported_symbols(
            "pub use task::{Task, Plan};\npub const COMPONENT_ID: &str = \"x\";"
        )
        self.assertEqual(exports, ("COMPONENT_ID", "Plan", "Task"))
        versioned = versioned_contracts(
            "impl_versioned_contract!(crate::Task, crate::Plan,);"
        )
        self.assertEqual(versioned, ("Plan", "Task"))

    def test_error_code_parser_deduplicates_and_sorts(self) -> None:
        codes = boundary_error_codes(
            '"contract.zeta"; "contract.alpha"; "contract.zeta";'
        )
        self.assertEqual(codes, ("contract.alpha", "contract.zeta"))

    def test_reference_validation_reports_missing_coverage_and_overclaim(self) -> None:
        failures = validate_reference_text(
            "## Authority Boundary\nmacOS verification has passed\n",
            ("Task",),
            ("Task",),
            ("contract.parse.syntax",),
        )
        self.assertIn("missing public symbol: Task", failures)
        self.assertIn("missing boundary error code: contract.parse.syntax", failures)
        self.assertIn(
            "unsupported reference claim: macOS verification has passed", failures
        )


if __name__ == "__main__":
    unittest.main()
