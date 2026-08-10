from __future__ import annotations

import copy
import json
import tempfile
import unittest
from pathlib import Path

from scripts.security_references import (
    DEFAULT_REGISTER,
    DEFAULT_SECURITY_REVIEW,
    audit_reference_register,
    extract_reference_urls,
)


class SecurityReferenceTests(unittest.TestCase):
    @classmethod
    def setUpClass(cls) -> None:
        cls.baseline = json.loads(DEFAULT_REGISTER.read_text(encoding="utf-8"))
        cls.security_review = DEFAULT_SECURITY_REVIEW.read_text(encoding="utf-8")

    def audit(self, data: dict[str, object], review: str | None = None) -> dict[str, object]:
        with tempfile.TemporaryDirectory() as temp_dir:
            root = Path(temp_dir)
            register = root / "security-references.json"
            security_review = root / "SECURITY-REVIEW.md"
            register.write_text(json.dumps(data, indent=2) + "\n", encoding="utf-8")
            security_review.write_text(review or self.security_review, encoding="utf-8")
            return audit_reference_register(register, security_review)

    def test_baseline_has_exact_citation_coverage(self) -> None:
        report = audit_reference_register()

        self.assertTrue(report["ok"], report["failures"])
        self.assertEqual(self.baseline["schema_version"], 1)
        self.assertEqual(
            self.baseline["register_id"],
            "agentmage-public-product-security-references",
        )
        self.assertEqual(
            self.baseline["governance"]["decision_record"],
            "docs/decisions/0002-public-security-reference-retention.md",
        )
        self.assertEqual(report["reference_count"], 21)
        self.assertEqual(report["citation_count"], 21)
        self.assertEqual(
            [record["source_url"] for record in self.baseline["references"]],
            extract_reference_urls(),
        )

    def test_every_record_has_provenance_and_explicit_non_claims(self) -> None:
        for record in self.baseline["references"]:
            with self.subTest(reference=record["id"]):
                self.assertRegex(record["source_sha256"], r"^[a-f0-9]{64}$")
                self.assertEqual(record["retrieval_date"], "2026-08-10")
                self.assertEqual(record["status"], "current")
                self.assertIsNone(record["superseded_by"])
                self.assertEqual(record["snapshot"]["approval_status"], "accepted")
                self.assertIsNone(record["snapshot"]["local_path"])
                self.assertIs(record["claims"]["external_certification"], False)
                self.assertIs(record["claims"]["publisher_endorsement"], False)

    def test_missing_and_orphaned_records_are_rejected(self) -> None:
        missing = copy.deepcopy(self.baseline)
        missing["references"].pop()
        missing_report = self.audit(missing)
        self.assertFalse(missing_report["ok"])
        self.assertTrue(
            any("unregistered Section 5 citation" in item for item in missing_report["failures"])
        )

        orphaned = copy.deepcopy(self.baseline)
        orphaned["references"][-1]["source_url"] = "https://example.com/orphan"
        orphaned_report = self.audit(orphaned)
        self.assertFalse(orphaned_report["ok"])
        self.assertTrue(any("orphaned reference record" in item for item in orphaned_report["failures"]))

    def test_duplicate_identity_and_order_changes_are_rejected(self) -> None:
        duplicate = copy.deepcopy(self.baseline)
        duplicate["references"][1]["id"] = "PSR-001"
        duplicate["references"][1]["source_url"] = duplicate["references"][0]["source_url"]
        report = self.audit(duplicate)

        self.assertFalse(report["ok"])
        self.assertTrue(any("duplicate reference id" in item for item in report["failures"]))
        self.assertTrue(any("duplicate source_url" in item for item in report["failures"]))

        reordered = copy.deepcopy(self.baseline)
        reordered["references"][0], reordered["references"][1] = (
            reordered["references"][1],
            reordered["references"][0],
        )
        reordered_report = self.audit(reordered)
        self.assertFalse(reordered_report["ok"])
        self.assertTrue(any("expected ordered id" in item for item in reordered_report["failures"]))

    def test_malformed_dates_hash_url_and_http_status_are_rejected(self) -> None:
        malformed = copy.deepcopy(self.baseline)
        record = malformed["references"][0]
        record["publication_date"] = "2026-13"
        record["source_sha256"] = "not-a-hash"
        record["source_url"] = "http://example.com/insecure"
        record["retrieval_http_status"] = True
        report = self.audit(malformed)

        self.assertFalse(report["ok"])
        joined = "\n".join(report["failures"])
        self.assertIn("publication_date", joined)
        self.assertIn("source_sha256", joined)
        self.assertIn("source_url must use HTTPS", joined)
        self.assertIn("retrieval_http_status must be an integer", joined)

    def test_status_and_supersession_must_be_consistent(self) -> None:
        current_with_replacement = copy.deepcopy(self.baseline)
        current_with_replacement["references"][0]["superseded_by"] = "PSR-002"
        report = self.audit(current_with_replacement)
        self.assertFalse(report["ok"])
        self.assertTrue(any("current reference" in item for item in report["failures"]))

        missing_replacement = copy.deepcopy(self.baseline)
        missing_replacement["references"][0]["status"] = "superseded"
        missing_replacement["references"][0]["superseded_by"] = "PSR-999"
        report = self.audit(missing_replacement)
        self.assertFalse(report["ok"])
        self.assertTrue(any("different registered reference" in item for item in report["failures"]))

    def test_unapproved_or_locally_retained_snapshot_is_rejected(self) -> None:
        changed = copy.deepcopy(self.baseline)
        changed["references"][0]["snapshot"]["approval_status"] = "proposed"
        changed["references"][0]["snapshot"]["local_path"] = "third-party/source.html"
        report = self.audit(changed)

        self.assertFalse(report["ok"])
        self.assertTrue(any("decision is not accepted" in item for item in report["failures"]))
        self.assertTrue(any("null local_path" in item for item in report["failures"]))

    def test_certification_and_endorsement_claims_are_rejected(self) -> None:
        changed = copy.deepcopy(self.baseline)
        changed["governance"]["external_certification_claimed"] = True
        changed["references"][0]["claims"]["publisher_endorsement"] = True
        report = self.audit(changed)

        self.assertFalse(report["ok"])
        joined = "\n".join(report["failures"])
        self.assertIn("external certification claims are prohibited", joined)
        self.assertIn("publisher endorsement claims are prohibited", joined)

    def test_new_section_citation_requires_a_registered_record(self) -> None:
        changed_review = self.security_review.replace(
            "## 6. Shared-Responsibility Model",
            "- [Unregistered source](https://example.com/new-source)\n\n"
            "## 6. Shared-Responsibility Model",
            1,
        )
        report = self.audit(copy.deepcopy(self.baseline), changed_review)

        self.assertFalse(report["ok"])
        self.assertTrue(
            any("unregistered Section 5 citation" in item for item in report["failures"])
        )

    def test_validation_does_not_mutate_inputs(self) -> None:
        before = (DEFAULT_REGISTER.read_bytes(), DEFAULT_SECURITY_REVIEW.read_bytes())
        audit_reference_register()
        after = (DEFAULT_REGISTER.read_bytes(), DEFAULT_SECURITY_REVIEW.read_bytes())

        self.assertEqual(before, after)


if __name__ == "__main__":
    unittest.main()
