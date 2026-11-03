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
    def test_all_twenty_accepted_definitions_keep_their_complete_contracts(self):
        tree = ast.parse(evaluation.git(
            "show", "10ad023285d96d36a601844e546cddcc03711f46:scripts/coding_eval_collection.py"))
        previous = ast.literal_eval(next(
            node.value for node in tree.body if isinstance(node, ast.Assign)
            and any(isinstance(target, ast.Name) and target.id == "TASKS" for target in node.targets)))
        self.assertEqual(collection.TASKS[:5], previous)
        # Together with the fifteen-definition test below this preserves every
        # accepted statement, source pin, hidden byte, selection and threshold.
        previous_collection = evaluation.TASKS + previous + rust.TASKS
        digest = evaluation.sha(json.dumps(previous_collection, sort_keys=True, separators=(",", ":")).encode())
        self.assertEqual(digest, "cd5fc51018e53364df2f26eaf91ea29a46805e19a6e7e069370372bd98cb8e46")

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

    def test_v9_statement_repair_preserves_all_other_frozen_inputs(self):
        tree = ast.parse(evaluation.git(
            "show", "57ea7ee64ec6ef218e411945e390734f494f9bf9:scripts/coding_eval_collection.py"))
        previous = [item for node in tree.body if isinstance(node, ast.Assign)
                    for target in node.targets if isinstance(target, ast.Name)
                    and target.id in {"TASKS", "ADDITIONAL_TASKS"}
                    for item in ast.literal_eval(node.value)]
        self.assertEqual(len(previous), len(collection.TASKS))
        changed = []
        for before, after in zip(previous, collection.TASKS):
            with self.subTest(task=before["id"]):
                self.assertEqual({k: v for k, v in before.items() if k != "statement"},
                                 {k: v for k, v in after.items() if k != "statement"})
                if before["statement"] != after["statement"]:
                    changed.append(before["id"])
        self.assertEqual(changed, ["dev-021", "dev-022", "dev-029"])
        # Only the three pre-measurement statements change the complete identity.
        old_tasks = evaluation.TASKS + previous + rust.TASKS
        self.assertEqual(evaluation.sha(json.dumps(
            old_tasks, sort_keys=True, separators=(",", ":")).encode()),
            "91f1eecfa91129f2f8392363aaff0e8f49243aa761947e69d5ec1452d7b4b04d")

    def test_new_selections_pin_complete_original_methods_and_all_inputs(self):
        self.assertEqual([t["id"] for t in collection.TASKS], [f"dev-{n:03}" for n in range(15, 30)])
        self.assertEqual(sum(t["expected_tests"] for t in collection.TASKS), 18)
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
        self.assertEqual(len(tasks), 30)
        self.assertEqual(len({task["id"] for task in tasks}), 30)
        self.assertEqual(sum(t["expected_tests"] for t in tasks), 89)
        self.assertEqual(len({(t["base"], t["source"]) for t in tasks}), 30)
        self.assertEqual(len({t["reference"] for t in collection.ADDITIONAL_TASKS}), 10)
        fingerprint = evaluation.sha(json.dumps(tasks, sort_keys=True, separators=(",", ":")).encode())
        self.assertEqual(fingerprint, "0c2a0624df93085540f61c4677260490a9810a0325efd6bcb75db0dce1461570")

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

    def test_every_new_task_refuses_generated_bytes_and_selection_changes(self):
        with tempfile.TemporaryDirectory() as temporary:
            for task in collection.ADDITIONAL_TASKS:
                with self.subTest(task=task["id"]):
                    files = evaluation.projection(task)
                    files[task["source"]] += b"\n# untrusted candidate\n"
                    output = Path(temporary) / task["id"]
                    with mock.patch.object(evaluation, "grade_case") as worker:
                        with self.assertRaisesRegex(evaluation.EvaluationError, "untrusted-candidate-disabled"):
                            evaluation.grade(task, files, output)
                        with self.assertRaisesRegex(evaluation.EvaluationError, "task-definition-refused"):
                            evaluation.grade({**task, "expected_tests": 0}, files, output)
                    worker.assert_not_called()
                    self.assertFalse(output.exists())

    def test_new_task_statements_name_required_new_module_attributes(self):
        for task in collection.ADDITIONAL_TASKS:
            base = ast.parse(evaluation.pinned_blob(task["base"], task["source"], task["base_sha256"]))
            hidden = ast.parse(evaluation.pinned_blob(
                task["reference"], task["hidden_test"], task["hidden_sha256"]))
            module = Path(task["source"]).stem
            aliases = {alias.asname or alias.name for node in hidden.body
                       if isinstance(node, ast.ImportFrom) and node.module == "scripts"
                       for alias in node.names if alias.name == module}
            existing = {node.name for node in base.body
                        if isinstance(node, (ast.FunctionDef, ast.ClassDef))}
            for node in base.body:
                if isinstance(node, (ast.Assign, ast.AnnAssign)):
                    targets = node.targets if isinstance(node, ast.Assign) else [node.target]
                    existing.update(target.id for target in targets if isinstance(target, ast.Name))
                elif isinstance(node, (ast.Import, ast.ImportFrom)):
                    existing.update(alias.asname or alias.name.split(".")[0] for alias in node.names)
            selected = [method for cls in hidden.body if isinstance(cls, ast.ClassDef)
                        for method in cls.body if isinstance(method, ast.FunctionDef)
                        and cls.name + "." + method.name in task["test_names"]]
            self.assertEqual(len(selected), task["expected_tests"])
            for method in selected:
                required = {node.attr for node in ast.walk(method) if isinstance(node, ast.Attribute)
                            and isinstance(node.value, ast.Name) and node.value.id in aliases}
                for attribute in required - existing - {"__file__"}:
                    with self.subTest(task=task["id"], attribute=attribute):
                        self.assertIn(attribute, task["statement"])

    def test_actual_cli_prepares_scores_and_refuses_a_new_development_task(self):
        task = collection.ADDITIONAL_TASKS[6]  # Finite collector deadlines.
        script = str(evaluation.ROOT / "scripts/coding_eval.py")
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            candidate = root / "candidate"
            result = subprocess.run(["/usr/bin/python3", script, "prepare", "--task", task["id"],
                                     "--output", str(candidate)], capture_output=True, timeout=30)
            self.assertEqual(result.returncode, 0, result.stderr)
            self.assertNotIn(task["hidden_test"], evaluation.projection(task))
            for kind, exit_code in (("base", 1), ("reference", 0), ("generated", 2)):
                source = candidate / task["source"]
                if kind == "reference":
                    source.write_bytes(evaluation.pinned_blob(
                        task["reference"], task["source"], task["reference_sha256"]))
                elif kind == "generated":
                    source.write_bytes(source.read_bytes() + b"\n# untrusted edit\n")
                output = root / kind
                result = subprocess.run(["/usr/bin/python3", script, "score", "--task", task["id"],
                                         "--candidate", str(candidate), "--output", str(output)],
                                        capture_output=True, timeout=30)
                self.assertEqual(result.returncode, exit_code, result.stderr)
                if kind == "generated":
                    self.assertFalse(output.exists())
                    self.assertIn(b"evaluation.untrusted-candidate-disabled", result.stderr)
                else:
                    report = json.loads((output / "report.json").read_text())
                    self.assertEqual(report["passed"], kind == "reference")
                    self.assertEqual(report["tests"]["tests"], 2)
                    self.assertEqual(report["tests"]["skipped"], 0)
                    self.assertEqual(report["model_attempts"], 0)
                    self.assertIsNone(report["model_score"])

    def test_direct_cli_lists_statements_without_hidden_inputs_or_references(self):
        result = subprocess.run(["/usr/bin/python3", str(evaluation.ROOT / "scripts/coding_eval.py"), "list"],
                                capture_output=True, timeout=30, check=True)
        public = json.loads(result.stdout)
        self.assertEqual(public["version"], "coding-development-v9")
        self.assertEqual(len(public["tasks"]), 30)
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
