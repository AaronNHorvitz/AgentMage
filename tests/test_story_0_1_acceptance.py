from __future__ import annotations

import copy
import json
import unittest

from scripts.requirement_conflicts import resolve_conflict
from scripts.requirement_coverage import (
    DEFAULT_NORMATIVE_MAP,
    DEFAULT_REGISTRY,
    ROOT,
    build_coverage_report,
    load_json,
)
from scripts.requirement_registry import DEFAULT_SOURCE, build_registry, render_registry
from scripts.security_references import (
    DEFAULT_REGISTER as DEFAULT_REFERENCE_REGISTER,
    audit_reference_register,
)


class Story01AcceptanceTests(unittest.TestCase):
    def setUp(self) -> None:
        self.registry = load_json(DEFAULT_REGISTRY)
        self.normative_map = load_json(DEFAULT_NORMATIVE_MAP)

    @staticmethod
    def categories(report: dict[str, object]) -> set[str]:
        return {item["category"] for item in report["diagnostics"]}

    def test_ac1_is_deterministic_and_all_seeded_failures_block(self) -> None:
        first = render_registry(build_registry(DEFAULT_SOURCE))
        second = render_registry(build_registry(DEFAULT_SOURCE))
        self.assertEqual(first, second)
        self.assertTrue(
            build_coverage_report(self.registry, self.normative_map, ROOT)["ok"]
        )

        cases: dict[str, tuple[str, object]] = {}
        duplicate = copy.deepcopy(self.registry)
        duplicate["requirements"].append(copy.deepcopy(duplicate["requirements"][0]))
        cases["duplicate_identifier"] = ("registry", duplicate)

        orphan = copy.deepcopy(self.registry)
        product = next(
            item for item in orphan["requirements"]
            if item["kind"] == "product_requirement"
        )
        product["dependencies"] = ["AM-ORPHAN-001"]
        cases["unresolved_dependency"] = ("registry", orphan)

        missing_test = copy.deepcopy(self.registry)
        product = next(
            item for item in missing_test["requirements"]
            if item["kind"] == "product_requirement"
        )
        product["acceptance_tests"] = []
        cases["missing_acceptance_test"] = ("registry", missing_test)

        release_mismatch = copy.deepcopy(self.registry)
        product = next(
            item for item in release_mismatch["requirements"]
            if item["kind"] == "product_requirement" and item["dependencies"]
        )
        by_id = {item["id"]: item for item in release_mismatch["requirements"]}
        by_id[product["dependencies"][0]]["release"] = "v9.0"
        cases["release_mismatch"] = ("registry", release_mismatch)

        for expected_category, (_, changed) in cases.items():
            with self.subTest(category=expected_category):
                report = build_coverage_report(changed, self.normative_map, ROOT)
                self.assertFalse(report["ok"])
                self.assertIn(expected_category, self.categories(report))

        left = {
            "id": "STORY-LEFT",
            "allow": {"actions": ["read"]},
            "deny": {"capabilities": []},
            "required_controls": ["receipt"],
            "ceilings": {"max_bytes": 1024},
            "exact": {"authority_model": "capability_grant"},
        }
        right = copy.deepcopy(left)
        right["id"] = "STORY-RIGHT"
        right["ceilings"]["max_bytes"] = 512
        conflict = resolve_conflict(left, right)
        self.assertEqual(conflict["status"], "provisional")
        self.assertTrue(conflict["requires_approved_decision"])
        self.assertEqual(conflict["boundary"]["ceilings"]["max_bytes"], 512)

    def test_ac2_every_v01_product_requirement_has_complete_navigation(self) -> None:
        report = json.loads(
            (ROOT / "requirements" / "traceability-report.json").read_text(
                encoding="utf-8"
            )
        )
        records = [
            record for record in report["requirements"]
            if record["kind"] == "product_requirement" and record["release"] == "v0.1"
        ]

        self.assertEqual(len(records), 27)
        for record in records:
            with self.subTest(requirement=record["id"]):
                self.assertTrue(record["source"]["heading"])
                self.assertGreater(record["source"]["line"], 0)
                self.assertTrue(record["implementation"]["planning_items"])
                self.assertTrue(record["acceptance_tests"])
                self.assertEqual(record["exclusion"]["state"], "not_excluded")
                self.assertEqual(record["status"], "planned")
                self.assertEqual(record["evidence"]["status"], "not_yet_produced")
                self.assertTrue(record["evidence"]["expected_roots"])

    def test_ac3_every_public_reference_is_current_or_approved_and_never_overclaims(self) -> None:
        audit = audit_reference_register()
        register = json.loads(DEFAULT_REFERENCE_REGISTER.read_text(encoding="utf-8"))

        self.assertTrue(audit["ok"], audit["failures"])
        self.assertEqual(audit["reference_count"], audit["citation_count"])
        self.assertEqual(audit["impact_reviews"], [])
        self.assertFalse(register["governance"]["external_certification_claimed"])
        self.assertFalse(register["governance"]["publisher_endorsement_claimed"])
        for reference in register["references"]:
            with self.subTest(reference=reference["id"]):
                if reference["status"] == "current":
                    self.assertTrue(reference["source_url"].startswith("https://"))
                else:
                    self.assertEqual(reference["snapshot"]["approval_status"], "accepted")
                    self.assertTrue(reference["snapshot"]["local_path"])
                self.assertFalse(reference["claims"]["external_certification"])
                self.assertFalse(reference["claims"]["publisher_endorsement"])


if __name__ == "__main__":
    unittest.main()
