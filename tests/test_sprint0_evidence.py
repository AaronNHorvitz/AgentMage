from __future__ import annotations

import json
import tempfile
import unittest
from pathlib import Path

from scripts.sprint0_evidence import (
    ARTIFACT_NAMES,
    DEFAULT_OUTPUT,
    REQUIRED_CONTROLS,
    check_bundle,
    load_json,
    summarize_raw_checks,
)


class Sprint0EvidenceTests(unittest.TestCase):
    def test_committed_bundle_hashes_and_reconciles(self) -> None:
        self.assertEqual(check_bundle(), [])
        manifest = load_json(DEFAULT_OUTPUT / "evidence-manifest.json")

        self.assertEqual(manifest["schema_version"], 1)
        self.assertEqual([item["path"] for item in manifest["files"]], list(ARTIFACT_NAMES))

    def test_raw_summary_is_derived_from_unmodified_results(self) -> None:
        raw = load_json(DEFAULT_OUTPUT / "raw-checker-output.json")

        self.assertEqual(raw["summary"], summarize_raw_checks(raw["checks"]))
        self.assertEqual(raw["summary"]["failed"], 0)
        self.assertEqual(raw["summary"]["skipped"], 0)

        changed = json.loads(json.dumps(raw["checks"]))
        changed[0]["exit_code"] = 1
        recomputed = summarize_raw_checks(changed)
        self.assertEqual(recomputed["failed"], 1)
        self.assertNotEqual(recomputed, raw["summary"])

    def test_all_required_controls_are_mapped_without_release_overclaim(self) -> None:
        controls = load_json(DEFAULT_OUTPUT / "control-map.json")["controls"]
        disposition = load_json(DEFAULT_OUTPUT / "reviewer-disposition.json")

        self.assertEqual([item["id"] for item in controls], list(REQUIRED_CONTROLS))
        self.assertTrue(all(item["evidence"] for item in controls))
        self.assertTrue(all(item["release_control_satisfied"] is False for item in controls))
        self.assertFalse(disposition["independent_review_performed"])
        self.assertFalse(disposition["release_approval"])
        self.assertTrue(disposition["limitations"])

    def test_conflict_and_exclusion_evidence_preserve_blocking_results(self) -> None:
        conflict = load_json(DEFAULT_OUTPUT / "conflict-report.json")
        exclusion = load_json(DEFAULT_OUTPUT / "exclusion-diff.json")

        self.assertTrue(conflict["order_invariant"])
        self.assertEqual(conflict["forward"], conflict["reverse"])
        self.assertEqual(conflict["forward"]["status"], "provisional")
        self.assertTrue(exclusion["blocked"])
        self.assertTrue(exclusion["requirement_diagnostics"])
        self.assertTrue(exclusion["exclusion_diagnostics"])

    def test_hash_tampering_is_detected_without_mutating_evidence(self) -> None:
        with tempfile.TemporaryDirectory() as temp_dir:
            copied = Path(temp_dir) / "story-0.1"
            copied.mkdir()
            for source in DEFAULT_OUTPUT.iterdir():
                if source.is_file():
                    (copied / source.name).write_bytes(source.read_bytes())
            target = copied / "conflict-report.json"
            target.write_bytes(target.read_bytes() + b"\n")
            before = target.read_bytes()

            failures = check_bundle(copied)

            self.assertTrue(any("hash mismatch" in failure for failure in failures))
            self.assertEqual(target.read_bytes(), before)


if __name__ == "__main__":
    unittest.main()
