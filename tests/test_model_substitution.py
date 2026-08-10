from __future__ import annotations

import copy
import tempfile
import unittest
from pathlib import Path

from scripts.model_substitution import (
    ModelSubstitutionError,
    PROFILES,
    build_report,
    changed_paths,
    read_json,
    run_scenario,
    scenarios,
    write_bundle,
)


class ModelSubstitutionTests(unittest.TestCase):
    def test_every_declared_one_field_substitution_is_refused_before_inference(self) -> None:
        report = build_report("HEAD")
        self.assertEqual(report["status"], "PASS")
        self.assertEqual(report["scenario_count"], 16)
        self.assertEqual(report["passed_count"], 16)
        self.assertEqual(report["inference_invocation_count"], 0)
        self.assertTrue(all(item["visible_refusal"] for item in report["scenarios"]))
        self.assertTrue(all(item["changed_field_count"] == 1 for item in report["scenarios"]))

    def test_every_profile_covers_each_required_category_once(self) -> None:
        expected = {
            "license",
            "lineage",
            "tokenizer",
            "context",
            "gguf",
            "model_oci_digest",
            "runtime_oci_digest",
            "runtime_build",
        }
        for profile in PROFILES:
            with self.subTest(profile=profile.profile_id):
                self.assertEqual({item.category for item in scenarios(profile)}, expected)

    def test_e4b_exact_but_substituted_lineage_revision_is_detected(self) -> None:
        profile = PROFILES[0]
        source = read_json(profile.source_path)
        artifact = read_json(profile.artifact_path)
        lineage = next(item for item in scenarios(profile) if item.category == "lineage")
        result = run_scenario(profile, lineage, source, artifact)
        self.assertTrue(result["substitution_detected"])
        self.assertIn("model identity mismatch: base_revision", result["validation_failures"])

    def test_changed_paths_reports_only_leaf_changes(self) -> None:
        before = {"a": {"b": 1}, "c": 2}
        after = copy.deepcopy(before)
        after["a"]["b"] = 3
        self.assertEqual(changed_paths(before, after), ["a.b"])

    def test_writer_refuses_to_overwrite_evidence(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            output = Path(temporary)
            with self.assertRaises(ModelSubstitutionError):
                write_bundle(output, "HEAD")


if __name__ == "__main__":
    unittest.main()
