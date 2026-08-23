from __future__ import annotations

import copy
import unittest

from scripts.engineering_runtime_manifest import (
    ADDED_STORY_SUBTASK_COUNTS,
    OUTPUT,
    PLAN,
    _load,
    build_manifest,
    validate_manifest,
)


class EngineeringRuntimeManifestTests(unittest.TestCase):
    def test_committed_manifest_is_current_and_complete(self) -> None:
        manifest = _load(OUTPUT)

        self.assertEqual(manifest, build_manifest())
        self.assertEqual(validate_manifest(manifest), [])
        self.assertEqual(manifest["counts"]["product_requirements"], 24)
        self.assertEqual(manifest["counts"]["acceptance_tests"], 29)
        self.assertEqual(manifest["counts"]["new_schemas"], 19)
        self.assertEqual(manifest["counts"]["reused_schemas"], 6)
        self.assertEqual(manifest["counts"]["requirement_owner_stories"], 14)
        self.assertEqual(manifest["counts"]["planned_stories_added"], 20)
        self.assertEqual(manifest["counts"]["tasks_added"], 60)
        self.assertEqual(manifest["counts"]["sub_tasks_added"], 141)
        self.assertEqual(manifest["added_story_ids"], list(ADDED_STORY_SUBTASK_COUNTS))

    def test_requirement_mapping_deletion_fails_closed(self) -> None:
        changed = copy.deepcopy(build_manifest())
        changed["requirement_mappings"].pop()

        self.assertTrue(validate_manifest(changed))

    def test_requirement_owner_mutation_fails_closed(self) -> None:
        changed = copy.deepcopy(build_manifest())
        changed["requirement_mappings"][0]["story_id"] = "999.9"

        failures = validate_manifest(changed)
        self.assertTrue(any("stale" in failure or "story" in failure for failure in failures))

    def test_complete_planned_story_set_cannot_be_narrowed(self) -> None:
        changed = copy.deepcopy(build_manifest())
        changed["added_story_ids"].pop()

        failures = validate_manifest(changed)
        self.assertTrue(any("added-story" in failure or "stale" in failure for failure in failures))

    def test_every_planned_requirement_has_a_unique_mapping(self) -> None:
        mappings = build_manifest()["requirement_mappings"]
        identifiers = [item["requirement_id"] for item in mappings]

        self.assertEqual(identifiers, sorted(PLAN))
        self.assertEqual(len(identifiers), len(set(identifiers)))


if __name__ == "__main__":
    unittest.main()
