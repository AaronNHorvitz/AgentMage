"""Tests for the Sprint 67 common artifact receipt contract."""

from __future__ import annotations

import unittest

from scripts.common_artifact_receipt_contract import OPERATION_KINDS, validate


class CommonArtifactReceiptContractTests(unittest.TestCase):
    def test_contract_is_current(self) -> None:
        self.assertEqual(validate(), [])

    def test_operation_inventory_is_closed(self) -> None:
        self.assertEqual(len(OPERATION_KINDS), 7)


if __name__ == "__main__":
    unittest.main()
