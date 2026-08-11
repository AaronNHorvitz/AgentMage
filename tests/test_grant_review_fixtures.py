import json
import unittest

from scripts import grant_review_fixtures as fixtures


class GrantReviewFixtureTests(unittest.TestCase):
    def test_typed_generator_and_retained_fixtures_are_exact(self) -> None:
        coverage = fixtures.check_fixtures()
        self.assertEqual(coverage["fixture_count"], 3)
        self.assertEqual(coverage["approval_forbidden_authority_field_count"], 0)
        self.assertEqual(coverage["denial_scope"], "argument")
        self.assertEqual(coverage["denial_resulting_status"], "invalidated")
        self.assertEqual(coverage["denial_resulting_use_count"], 0)
        self.assertEqual(coverage["receipt_outcome"], "denied")

    def test_bundle_parser_rejects_duplicates_and_incomplete_sets(self) -> None:
        duplicate = [
            {"name": fixtures.EXPECTED_NAMES[0], "canonical_json": "{}"},
            {"name": fixtures.EXPECTED_NAMES[0], "canonical_json": "{}"},
        ]
        with self.assertRaises(fixtures.GrantReviewFixtureError):
            fixtures.parse_bundle(
                (fixtures.MARKER + json.dumps(duplicate) + "\n").encode("utf-8")
            )
        incomplete = [
            {"name": name, "canonical_json": "{}"}
            for name in fixtures.EXPECTED_NAMES[:-1]
        ]
        with self.assertRaises(fixtures.GrantReviewFixtureError):
            fixtures.parse_bundle(
                (fixtures.MARKER + json.dumps(incomplete) + "\n").encode("utf-8")
            )

    def test_authority_hash_and_denial_mutations_are_rejected(self) -> None:
        retained = {
            name: (fixtures.FIXTURE_ROOT / name).read_bytes()
            for name in fixtures.EXPECTED_NAMES
        }
        approval = json.loads(retained[fixtures.EXPECTED_NAMES[0]])
        approval["nonce"] = "forged-authority"
        changed = dict(retained)
        changed[fixtures.EXPECTED_NAMES[0]] = fixtures.compact_json(approval)
        with self.assertRaises(fixtures.GrantReviewFixtureError):
            fixtures.validate_fixture_values(changed)

        denial = json.loads(retained[fixtures.EXPECTED_NAMES[2]])
        denial["resulting_use_count"] = 1
        changed = dict(retained)
        changed[fixtures.EXPECTED_NAMES[2]] = fixtures.compact_json(denial)
        with self.assertRaises(fixtures.GrantReviewFixtureError):
            fixtures.validate_fixture_values(changed)


if __name__ == "__main__":
    unittest.main()
