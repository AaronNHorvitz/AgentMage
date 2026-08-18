from __future__ import annotations

import copy
import json
import tempfile
import unittest
from pathlib import Path

from scripts.story_23_4_evidence_index import (
    EVIDENCE_PATHS,
    MAPPINGS,
    REPORT_PATH,
    check_index,
    safe_relative_path,
    task_anchors,
    validate_index,
)


class Story234EvidenceIndexTests(unittest.TestCase):
    def test_mapping_statuses_are_truthful_and_story_stays_blocked(self) -> None:
        statuses = [mapping["status"] for mapping in MAPPINGS]
        self.assertEqual(len(MAPPINGS), 17)
        self.assertEqual(statuses.count("complete"), 14)
        self.assertEqual(statuses.count("partial"), 3)
        self.assertEqual(statuses.count("open"), 0)
        self.assertEqual(MAPPINGS[6]["task_id"], "23.4.1.7")
        self.assertEqual(MAPPINGS[13]["task_id"], "23.4.3.3")
        self.assertEqual(MAPPINGS[16]["task_id"], "23.4.3.6")

    def test_task_parser_requires_exact_order_and_checkbox_status(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            lines = []
            for mapping in MAPPINGS:
                check = "x" if mapping["status"] == "complete" else " "
                lines.append(
                    f"  - [{check}] **Sub-task {mapping['task_id']}:** Synthetic statement"
                )
            (root / "TASKS.md").write_text("\n".join(lines) + "\n", encoding="utf-8")
            anchors = task_anchors(root)
            self.assertEqual(list(anchors), [mapping["task_id"] for mapping in MAPPINGS])

            lines[0] = lines[0].replace("[x]", "[ ]")
            (root / "TASKS.md").write_text("\n".join(lines) + "\n", encoding="utf-8")
            with self.assertRaises(ValueError):
                task_anchors(root)

    def test_all_evidence_paths_are_safe_unique_and_existing(self) -> None:
        self.assertEqual(len(EVIDENCE_PATHS), len(set(EVIDENCE_PATHS)))
        self.assertTrue(all(safe_relative_path(path) for path in EVIDENCE_PATHS))
        root = Path(__file__).resolve().parents[1]
        self.assertTrue(all((root / path).is_file() for path in EVIDENCE_PATHS))

    @unittest.skipUnless(REPORT_PATH.is_file(), "retained index follows task reconciliation")
    def test_current_index_is_exact_and_hash_bound(self) -> None:
        self.assertEqual(check_index(), [])
        index = json.loads(REPORT_PATH.read_text(encoding="utf-8"))
        self.assertEqual(index["summary"]["complete_count"], 14)
        self.assertEqual(index["summary"]["partial_count"], 3)
        self.assertFalse(index["summary"]["story_complete"])
        self.assertFalse(index["summary"]["sprint_complete"])
        self.assertFalse(index["summary"]["release_approved"])

        completed = copy.deepcopy(index)
        completed["summary"]["story_complete"] = True
        self.assertTrue(validate_index(completed))
        digest = copy.deepcopy(index)
        digest["index_sha256"] = "f" * 64
        self.assertTrue(validate_index(digest))


if __name__ == "__main__":
    unittest.main()
