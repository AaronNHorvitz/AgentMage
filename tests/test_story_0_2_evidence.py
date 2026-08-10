from __future__ import annotations

import json
import tempfile
import unittest
from pathlib import Path

from scripts.story_0_2_evidence import (
    ARTIFACT_NAMES,
    DEFAULT_OUTPUT,
    POLICY_FILES,
    REQUIRED_CONTROLS,
    ROOT,
    Story02EvidenceError,
    control_map,
    check_bundle,
    decision_record,
    policy_hashes,
    reviewer_disposition,
    workflow_identity,
    write_bundle,
)


class Story02EvidenceTests(unittest.TestCase):
    def test_committed_bundle_hashes_and_reconciles(self) -> None:
        self.assertEqual(check_bundle(), [])
        manifest = json.loads(
            (DEFAULT_OUTPUT / "evidence-manifest.json").read_text(encoding="utf-8")
        )

        self.assertEqual(manifest["schema_version"], 1)
        self.assertRegex(manifest["source_revision"], r"^[0-9a-f]{40}$")
        self.assertEqual(
            [item["path"] for item in manifest["files"]],
            list(ARTIFACT_NAMES),
        )

    def test_policy_hashes_cover_every_governing_artifact(self) -> None:
        hashes = policy_hashes(ROOT, "fixture-revision")

        self.assertEqual(
            [item["path"] for item in hashes["files"]],
            list(POLICY_FILES),
        )
        for item in hashes["files"]:
            self.assertRegex(item["sha256"], r"^[0-9a-f]{64}$")
            self.assertGreater(item["size"], 0)

    def test_workflow_identity_is_pinned_and_uses_the_clean_gate(self) -> None:
        identity = workflow_identity(ROOT, "fixture-revision")

        self.assertTrue(identity["actions_immutable"])
        self.assertTrue(identity["actions"])
        self.assertEqual(identity["permissions"], {"contents": "read"})
        self.assertEqual(identity["gate_command"], "npm run docs:clean-check")
        self.assertEqual(
            identity["gate_script"],
            "npm ci --ignore-scripts && npm run docs:check",
        )

    def test_decision_record_retains_accepted_twelve_item_baseline(self) -> None:
        decision = decision_record(ROOT, "fixture-revision")

        self.assertEqual(decision["decision_id"], "0001")
        self.assertEqual(decision["status"], "accepted")
        self.assertEqual(decision["date"], "2026-08-10")
        self.assertEqual(
            decision["numbered_decisions"],
            [str(number) for number in range(1, 13)],
        )

    def test_control_map_is_complete_without_release_overclaim(self) -> None:
        controls = control_map("fixture-revision")["controls"]
        disposition = reviewer_disposition("fixture-revision")

        self.assertEqual([item["id"] for item in controls], list(REQUIRED_CONTROLS))
        self.assertTrue(all(item["evidence"] for item in controls))
        self.assertTrue(all(item["release_control_satisfied"] is False for item in controls))
        self.assertFalse(disposition["independent_review_performed"])
        self.assertFalse(disposition["release_approval"])
        self.assertTrue(disposition["limitations"])

    def test_writer_refuses_to_replace_existing_evidence(self) -> None:
        with tempfile.TemporaryDirectory() as temp_dir:
            output = Path(temp_dir) / "story-0.2"
            output.mkdir()

            with self.assertRaisesRegex(Story02EvidenceError, "refusing to overwrite"):
                write_bundle(output, "HEAD")

        self.assertEqual(
            ARTIFACT_NAMES,
            (
                "control-map.json",
                "decision-record.json",
                "policy-hashes.json",
                "raw-checker-output.json",
                "reviewer-disposition.json",
                "summary.md",
                "workflow-identity.json",
            ),
        )

    def test_hash_tampering_is_detected_without_mutating_evidence(self) -> None:
        with tempfile.TemporaryDirectory() as temp_dir:
            copied = Path(temp_dir) / "story-0.2"
            copied.mkdir()
            for source in DEFAULT_OUTPUT.iterdir():
                if source.is_file():
                    (copied / source.name).write_bytes(source.read_bytes())
            target = copied / "workflow-identity.json"
            target.write_bytes(target.read_bytes() + b"\n")
            before = target.read_bytes()

            failures = check_bundle(copied)

            self.assertTrue(any("hash mismatch" in failure for failure in failures))
            self.assertEqual(target.read_bytes(), before)


if __name__ == "__main__":
    unittest.main()
