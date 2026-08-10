from __future__ import annotations

import copy
import unittest

from scripts.story_1_1_artifacts import build_index, check_index, validate_index


class Story11ArtifactTests(unittest.TestCase):
    def setUp(self) -> None:
        self.index = build_index()

    def test_checked_in_index_is_current(self) -> None:
        self.assertEqual(check_index(), [])

    def test_generation_is_deterministic(self) -> None:
        self.assertEqual(self.index, build_index())

    def test_each_artifact_task_is_present_once(self) -> None:
        self.assertEqual(
            [item["task_id"] for item in self.index["artifacts"]],
            ["1.1.2.1", "1.1.2.2", "1.1.2.3", "1.1.2.4"],
        )

    def test_missing_artifact_file_is_rejected(self) -> None:
        mutated = copy.deepcopy(self.index)
        mutated["artifacts"][0]["files"].pop()
        self.assertTrue(validate_index(mutated))

    def test_checksum_mutation_is_rejected(self) -> None:
        mutated = copy.deepcopy(self.index)
        mutated["artifacts"][0]["files"][0]["sha256"] = "0" * 64
        failures = validate_index(mutated)
        self.assertTrue(any("checksum mismatch" in item for item in failures))

    def test_path_traversal_is_rejected(self) -> None:
        mutated = copy.deepcopy(self.index)
        mutated["artifacts"][0]["files"][0]["path"] = "../outside"
        failures = validate_index(mutated)
        self.assertTrue(any("file closure" in item or "unsafe" in item for item in failures))

    def test_macos_status_cannot_be_promoted(self) -> None:
        mutated = copy.deepcopy(self.index)
        mutated["platform_status"]["macos_build"] = "verified"
        self.assertTrue(validate_index(mutated))

    def test_command_omission_is_rejected(self) -> None:
        mutated = copy.deepcopy(self.index)
        mutated["commands"].pop()
        self.assertTrue(validate_index(mutated))


if __name__ == "__main__":
    unittest.main()
