from __future__ import annotations

import copy
import json
import tempfile
import unittest
from pathlib import Path

from scripts.story_22_2_evidence_index import (
    EVIDENCE_PATHS,
    MAPPINGS,
    REPORT_PATH,
    build_index,
    check_index,
    validate_index,
)


class Story222EvidenceIndexTests(unittest.TestCase):
    def test_current_index_is_exact_and_hash_bound(self) -> None:
        self.assertEqual(check_index(), [])
        index = json.loads(REPORT_PATH.read_text(encoding="utf-8"))
        self.assertEqual(index, build_index(index["source_revision"]))
        self.assertEqual(len(index["index_sha256"]), 64)

    def test_every_subtask_has_one_truthful_status_and_task_anchor(self) -> None:
        index = build_index("0" * 40)
        self.assertEqual(
            [mapping["task_id"] for mapping in index["mappings"]],
            [mapping["task_id"] for mapping in MAPPINGS],
        )
        self.assertEqual(index["summary"]["complete_count"], 12)
        self.assertEqual(index["summary"]["partial_count"], 4)
        self.assertEqual(index["summary"]["open_count"], 1)
        self.assertFalse(index["summary"]["story_complete"])
        self.assertTrue(
            all(len(item["statement_sha256"]) == 64 for item in index["mappings"])
        )

    def test_omission_reorder_status_hash_and_completion_mutations_fail(self) -> None:
        index = build_index("0" * 40)
        mutations = []
        omitted = copy.deepcopy(index)
        omitted["mappings"].pop()
        mutations.append(omitted)
        reordered = copy.deepcopy(index)
        reordered["mappings"].reverse()
        mutations.append(reordered)
        status = copy.deepcopy(index)
        status["mappings"][0]["status"] = "open"
        mutations.append(status)
        artifact_hash = copy.deepcopy(index)
        artifact_hash["artifacts"][0]["sha256"] = "f" * 64
        mutations.append(artifact_hash)
        completed = copy.deepcopy(index)
        completed["summary"]["story_complete"] = True
        mutations.append(completed)
        digest = copy.deepcopy(index)
        digest["index_sha256"] = "f" * 64
        mutations.append(digest)
        for mutation in mutations:
            with self.subTest(mutation=mutation):
                self.assertTrue(validate_index(mutation))

    def test_all_mapped_paths_are_safe_unique_and_hashed_once(self) -> None:
        index = build_index("0" * 40)
        paths = [artifact["path"] for artifact in index["artifacts"]]
        self.assertEqual(paths, list(EVIDENCE_PATHS))
        self.assertEqual(len(paths), len(set(paths)))
        for relative in paths:
            path = Path(relative)
            self.assertFalse(path.is_absolute())
            self.assertNotIn("..", path.parts)

    def test_missing_evidence_and_task_status_drift_fail_closed(self) -> None:
        index = build_index("0" * 40)
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            for relative in EVIDENCE_PATHS:
                source = Path(__file__).resolve().parents[1] / relative
                destination = root / relative
                destination.parent.mkdir(parents=True, exist_ok=True)
                destination.write_bytes(source.read_bytes())
            (root / EVIDENCE_PATHS[0]).unlink()
            self.assertTrue(validate_index(index, root))


if __name__ == "__main__":
    unittest.main()
