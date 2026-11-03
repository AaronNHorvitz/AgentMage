"""Check append-only historical task contracts and the executable collection API."""
from __future__ import annotations

import ast
import json
from pathlib import Path
import subprocess
import tempfile
import unittest
from unittest import mock

from scripts import coding_eval as evaluation
from scripts import coding_eval_collection as collection
from scripts import coding_eval_rust as rust


class CodingCollectionTests(unittest.TestCase):
    def test_all_fifteen_accepted_task_definitions_remain_identical(self):
        # Compare data, not only counts, against the last independently accepted
        # evaluator commit. Neither hidden clauses nor statements can drift.
        for script, current, keys in (
            ("scripts/coding_eval.py", evaluation.TASKS, {"TASKS", "ADDITIONAL_TASKS"}),
            ("scripts/coding_eval_rust.py", rust.TASKS, {"TASKS"}),
        ):
            tree = ast.parse(evaluation.git("show", "8505c0bd298cea2ec2c5e706955b3456be3f6d13:" + script))
            previous = [item for node in tree.body if isinstance(node, ast.Assign)
                        for target in node.targets if isinstance(target, ast.Name) and target.id in keys
                        for item in ast.literal_eval(node.value)]
            self.assertEqual(current, previous)

    def test_new_selections_pin_complete_original_methods_and_all_inputs(self):
        self.assertEqual([t["id"] for t in collection.TASKS], [f"dev-{n:03}" for n in range(15, 20)])
        self.assertEqual(sum(t["expected_tests"] for t in collection.TASKS), 6)
        for task in collection.TASKS:
            with self.subTest(task=task["id"]):
                self.assertEqual(task["split"], "development")
                self.assertEqual(evaluation.git("rev-parse", task["reference"] + "^").decode().strip(),
                                 task["base"])
                for version, key in (("base", "base_sha256"), ("reference", "reference_sha256")):
                    evaluation.pinned_blob(task[version], task["source"], task[key])
                hidden = evaluation.pinned_blob(task["reference"], task["hidden_test"], task["hidden_sha256"])
                tree = ast.parse(hidden)
                methods = {node.name + "." + method.name for node in tree.body
                           if isinstance(node, ast.ClassDef) for method in node.body
                           if isinstance(method, ast.FunctionDef) and method.name.startswith("test_")}
                self.assertTrue(set(task["test_names"]) <= methods)
                self.assertEqual(len(set(task["test_names"])), task["expected_tests"])
                files = evaluation.projection(task)
                self.assertNotIn(task["hidden_test"], files)
                self.assertTrue(all(Path(name).parts[0] == "scripts" for name in files))
                self.assertEqual(evaluation.require_historical_candidate(task, files), "frozen-base")
                files[task["source"]] = evaluation.pinned_blob(
                    task["reference"], task["source"], task["reference_sha256"])
                self.assertEqual(evaluation.require_historical_candidate(task, files), "frozen-reference")

    def test_collection_identity_is_unique_and_frozen_before_measurement(self):
        tasks = evaluation.collection()
        self.assertEqual(len(tasks), 20)
        self.assertEqual(len({task["id"] for task in tasks}), 20)
        self.assertEqual(sum(t["expected_tests"] for t in tasks), 77)
        fingerprint = evaluation.sha(json.dumps(tasks, sort_keys=True, separators=(",", ":")).encode())
        self.assertEqual(fingerprint, "cd5fc51018e53364df2f26eaf91ea29a46805e19a6e7e069370372bd98cb8e46")

    def test_new_generated_candidate_and_task_mutation_never_reach_worker(self):
        task = collection.TASKS[0]
        files = evaluation.projection(task)
        files[task["source"]] += b"\n# generated edit\n"
        with tempfile.TemporaryDirectory() as temporary:
            output = Path(temporary) / "refused"
            with mock.patch.object(evaluation, "grade_case") as worker:
                with self.assertRaisesRegex(evaluation.EvaluationError, "untrusted-candidate-disabled"):
                    evaluation.grade(task, files, output)
                with self.assertRaisesRegex(evaluation.EvaluationError, "task-definition-refused"):
                    evaluation.grade({**task, "test_names": []}, files, output)
            worker.assert_not_called()
            self.assertFalse(output.exists())

    def test_direct_cli_lists_statements_without_hidden_inputs_or_references(self):
        result = subprocess.run(["/usr/bin/python3", str(evaluation.ROOT / "scripts/coding_eval.py"), "list"],
                                capture_output=True, timeout=30, check=True)
        public = json.loads(result.stdout)
        self.assertEqual(public["version"], "coding-development-v7")
        self.assertEqual(len(public["tasks"]), 20)
        for task in public["tasks"]:
            self.assertEqual(set(task), {"id", "split", "base", "source", "statement"})
        self.assertEqual(public["tasks"][14]["id"], "dev-015")

    def test_report_binds_task_module_without_claiming_a_model_attempt(self):
        task = collection.TASKS[-1]
        files = evaluation.projection(task)
        with tempfile.TemporaryDirectory() as temporary:
            with mock.patch.object(evaluation, "grade_case", side_effect=OSError):
                report = evaluation.grade(task, files, Path(temporary) / "report")
            self.assertEqual(report["collection_module_sha256"],
                             evaluation.sha(Path(collection.__file__).read_bytes()))
            self.assertEqual(report["cause"], "infrastructure")
            self.assertFalse(report["passed"])
            self.assertEqual(report["model_attempts"], 0)
            self.assertIsNone(report["model_score"])


if __name__ == "__main__":
    unittest.main()
