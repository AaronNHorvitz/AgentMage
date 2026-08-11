from __future__ import annotations

import copy
import tempfile
import unittest
from pathlib import Path

from scripts.story_5_1_security_evidence import (
    EVIDENCE_PATHS,
    EXPECTED_REQUIREMENTS,
    MAPPINGS,
    build_map,
    validate_map,
)


class Story51SecurityEvidenceTests(unittest.TestCase):
    def test_requirement_mappings_are_exact_and_product_incomplete(self) -> None:
        evidence_map = build_map("0" * 40)
        self.assertEqual(
            [item["requirement_id"] for item in evidence_map["requirements"]],
            list(EXPECTED_REQUIREMENTS),
        )
        self.assertEqual(evidence_map["summary"]["product_requirements_complete"], 0)
        self.assertEqual(evidence_map["summary"]["demonstrated_story_scope_count"], 1)
        self.assertEqual(evidence_map["summary"]["partial_story_evidence_count"], 7)
        for item in evidence_map["requirements"]:
            self.assertEqual(item["product_requirement_status"], "not-complete")
            self.assertEqual(
                item,
                {
                    "requirement_id": item["requirement_id"],
                    "product_requirement_status": "not-complete",
                    **MAPPINGS[item["requirement_id"]],
                },
            )
            self.assertTrue(item["remaining"])

    def test_seed_race_state_and_independent_review_evidence_is_retained(self) -> None:
        paths = set(EVIDENCE_PATHS)
        self.assertIn("fixtures/grants/adversarial/v1/corpus.json", paths)
        self.assertIn(
            "artifacts/sprints/sprint-5/story-5.1/grant-race-replay-report.json",
            paths,
        )
        self.assertIn("docs/architecture/grant-state-transitions.md", paths)
        self.assertIn(
            "artifacts/sprints/sprint-5/story-5.1/grant-boundary-review.json",
            paths,
        )

    def test_requirement_omission_reordering_and_completion_fail_closed(self) -> None:
        evidence_map = build_map("0" * 40)
        omitted = copy.deepcopy(evidence_map)
        omitted["requirements"].pop()
        reordered = copy.deepcopy(evidence_map)
        reordered["requirements"].reverse()
        completed = copy.deepcopy(evidence_map)
        completed["requirements"][0]["product_requirement_status"] = "complete"
        no_remaining = copy.deepcopy(evidence_map)
        no_remaining["requirements"][0]["remaining"] = ""
        for changed in (omitted, reordered, completed, no_remaining):
            with self.subTest(changed=changed):
                self.assertTrue(validate_map(changed))

    def test_artifact_hash_omission_and_unknown_path_fail_closed(self) -> None:
        evidence_map = build_map("0" * 40)
        changed_hash = copy.deepcopy(evidence_map)
        changed_hash["artifacts"][0]["sha256"] = "0" * 64
        omitted = copy.deepcopy(evidence_map)
        omitted["artifacts"].pop()
        unknown = copy.deepcopy(evidence_map)
        unknown["requirements"][0]["evidence"][0] = "unreviewed/evidence.json"
        for changed in (changed_hash, omitted, unknown):
            with self.subTest(changed=changed):
                self.assertTrue(validate_map(changed))

    def test_review_audit_fuzzing_release_network_and_macos_overclaims_fail_closed(self) -> None:
        evidence_map = build_map("0" * 40)
        changes = []
        for key, value in (
            ("independent_review_type", "external-human-review"),
            ("external_human_review_status", "pass"),
            ("product_requirement_completion_claim", "complete"),
            ("fuzzing_claim", "complete"),
            ("durable_audit_claim", "pass"),
            ("release_claim", "pass"),
            ("network_used", True),
            ("macos_execution_status", "pass"),
            ("macos_evidence_substituted", True),
        ):
            changed = copy.deepcopy(evidence_map)
            changed[key] = value
            changes.append(changed)
        for changed in changes:
            with self.subTest(changed=changed):
                self.assertTrue(validate_map(changed))

    def test_missing_evidence_prevents_map_validation(self) -> None:
        evidence_map = build_map("0" * 40)
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            for relative in EVIDENCE_PATHS[:-1]:
                path = root / relative
                path.parent.mkdir(parents=True, exist_ok=True)
                path.write_bytes(b"fixture")
            self.assertTrue(validate_map(evidence_map, root))


if __name__ == "__main__":
    unittest.main()
