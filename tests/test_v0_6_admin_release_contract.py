"""Tests for the blocked v0.6 local aggregate."""

import unittest
from scripts.v0_6_admin_release_contract import GUIDES, SPRINTS, validate


class V06AdminReleaseContractTests(unittest.TestCase):
    def test_contract_is_current_and_blocked(self) -> None:
        self.assertEqual(validate(), [])

    def test_inventory_is_explicit(self) -> None:
        self.assertEqual((len(SPRINTS), len(GUIDES)), (15, 14))


if __name__ == "__main__": unittest.main()
