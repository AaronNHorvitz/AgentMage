"""Check evaluator confinement, task pins and honest failure accounting."""
from __future__ import annotations

from contextlib import redirect_stderr
import ast
import hashlib
import io
import json
import os
from pathlib import Path
import subprocess
import tempfile
import time
import unittest
from unittest import mock

from scripts import coding_eval as evaluation


class CodingEvaluationTests(unittest.TestCase):
    def setUp(self):
        self.temporary = tempfile.TemporaryDirectory(prefix="agentmage-evaluator-test-")
        self.addCleanup(self.temporary.cleanup)
        self.root = Path(self.temporary.name)

    def grade_fixture(self, task, files, output):
        # Deliberately synthetic worker controls, never exposed through the CLI.
        # Production grade() refuses these before a worker can run. The bypass
        # here lets lifecycle tests exercise failure paths with tiny fixtures.
        with mock.patch.object(evaluation, "require_historical_candidate", return_value="synthetic-control"):
            return evaluation.grade(task, files, output)

    def candidate(self):
        original = {"scripts/a.py": b"original\n", "scripts/b.py": b"frozen\n"}
        root = self.root / "candidate"
        root.mkdir()
        for name, content in original.items():
            evaluation.write_file(root / name, content)
        return root, original

    def test_task_definitions_are_frozen_before_any_agent_measurement(self):
        fingerprint = hashlib.sha256(json.dumps(
            evaluation.TASKS[:3], sort_keys=True, separators=(",", ":"),
        ).encode()).hexdigest()
        self.assertEqual(fingerprint, "d5f5e365924ffea8c6b46c50c28d00e116c01f69de57ac0a9672ffa50e05e3c5")
        self.assertEqual(len(set(t["id"] for t in evaluation.TASKS)), 14)
        complete = evaluation.sha(json.dumps(evaluation.TASKS, sort_keys=True, separators=(",", ":")).encode())
        self.assertEqual(complete, "72442a4cf406abd83dc7684a5ef225474b55ee2cbc4cb6e5d34e6a840f0da618")
        self.assertEqual(sum(t["expected_tests"] for t in evaluation.TASKS), 70)
        # Each is a separate historical source change, not duplicated test weights.
        self.assertEqual(len({(t["base"], t["source"]) for t in evaluation.TASKS}), 14)

    def test_statements_name_new_module_interfaces_required_by_frozen_tests(self):
        for task in evaluation.TASKS:
            hidden = ast.parse(evaluation.pinned_blob(
                task["reference"], task["hidden_test"], task["hidden_sha256"]))
            base = ast.parse(evaluation.pinned_blob(
                task["base"], task["source"], task["base_sha256"]))
            source_module = Path(task["source"]).stem
            aliases = {alias.asname or alias.name for node in ast.walk(hidden)
                       if isinstance(node, ast.ImportFrom) and node.module == "scripts"
                       for alias in node.names if alias.name == source_module}
            base_attributes = {node.name for node in base.body
                               if isinstance(node, (ast.FunctionDef, ast.AsyncFunctionDef, ast.ClassDef))}
            # Supplied by Python's module loader, rather than a task implementation.
            base_attributes.update({"__file__", "__name__", "__package__", "__doc__",
                                    "__spec__", "__loader__", "__builtins__", "__cached__"})
            for node in base.body:
                if isinstance(node, (ast.Assign, ast.AnnAssign)):
                    targets = node.targets if isinstance(node, ast.Assign) else [node.target]
                    base_attributes.update(child.id for target in targets for child in ast.walk(target)
                                           if isinstance(child, ast.Name))
                elif isinstance(node, (ast.Import, ast.ImportFrom)):
                    base_attributes.update(alias.asname or alias.name.split(".")[0] for alias in node.names)
            inspected = hidden
            if "test_names" in task:
                selected = set(task["test_names"])
                inspected = ast.Module(body=[
                    node for node in hidden.body if not isinstance(node, ast.ClassDef)
                ] + [
                    method for cls in hidden.body if isinstance(cls, ast.ClassDef)
                    for method in cls.body if not isinstance(method, ast.FunctionDef)
                    or not method.name.startswith("test_") or cls.name + "." + method.name in selected
                ], type_ignores=[])
            required = {node.attr for node in ast.walk(inspected)
                        if isinstance(node, ast.Attribute) and isinstance(node.value, ast.Name)
                        and node.value.id in aliases}
            # Some historical modules import public functions directly instead.
            self.assertTrue(aliases or any(
                isinstance(node, ast.ImportFrom) and node.module == "scripts." + source_module
                for node in hidden.body), task["id"])
            for attribute in required - base_attributes:
                with self.subTest(task=task["id"], attribute=attribute):
                    self.assertIn(attribute, task["statement"])

    def test_v3_statement_repair_preserves_every_other_frozen_input(self):
        previous = ast.parse(evaluation.git(
            "show", "b1734b9957cebfbfbe8a2ebefa33322f3f5982eb:scripts/coding_eval.py"))
        frozen = ast.literal_eval(next(
            node.value for node in previous.body if isinstance(node, ast.Assign)
            and any(isinstance(target, ast.Name) and target.id == "TASKS" for target in node.targets)))
        self.assertEqual(len(frozen), len(evaluation.TASKS[:3]))
        changed = []
        for before, after in zip(frozen, evaluation.TASKS[:3]):
            self.assertEqual({k: v for k, v in before.items() if k != "statement"},
                             {k: v for k, v in after.items() if k != "statement"})
            if before["statement"] != after["statement"]:
                changed.append(after["id"])
        self.assertEqual(changed, ["dev-002", "dev-003"])

    def test_new_task_selections_are_exact_original_methods_and_never_rewrite_hidden_tests(self):
        for task in evaluation.TASKS[3:]:
            hidden = evaluation.pinned_blob(task["reference"], task["hidden_test"], task["hidden_sha256"])
            module = ast.parse(hidden)
            methods = {cls.name + "." + method.name for cls in module.body if isinstance(cls, ast.ClassDef)
                       for method in cls.body if isinstance(method, ast.FunctionDef)
                       and method.name.startswith("test_")}
            self.assertTrue(set(task["test_names"]).issubset(methods), task["id"])
            self.assertEqual(len(task["test_names"]), task["expected_tests"])
            self.assertTrue(task["test_names"])
            self.assertNotEqual(evaluation.task_driver(task), evaluation.DRIVER)
        for task in evaluation.TASKS[:3]:
            self.assertNotIn("test_names", task)
            self.assertEqual(evaluation.task_driver(task), evaluation.DRIVER)

    def test_v5_statement_correction_preserves_all_frozen_test_and_source_inputs(self):
        previous = ast.parse(evaluation.git(
            "show", "c86571bb85c85a17d70a51f84a5da5222938f58a:scripts/coding_eval.py"))
        definitions = {target.id: ast.literal_eval(node.value)
                       for node in previous.body if isinstance(node, ast.Assign)
                       for target in node.targets if isinstance(target, ast.Name)
                       and target.id in {"TASKS", "ADDITIONAL_TASKS"}}
        before = definitions["TASKS"] + definitions["ADDITIONAL_TASKS"]
        self.assertEqual(len(before), len(evaluation.TASKS))
        changed = []
        for frozen, current in zip(before, evaluation.TASKS):
            self.assertEqual({k: v for k, v in frozen.items() if k != "statement"},
                             {k: v for k, v in current.items() if k != "statement"})
            if frozen["statement"] != current["statement"]:
                changed.append(current["id"])
        self.assertEqual(changed, ["dev-004", "dev-011", "dev-014"])

    def test_test_selection_cannot_inject_code_repeat_methods_or_change_counts(self):
        for names, count in (([], 0), (["Case.test_ok"] * 2, 2), (["Case.test_ok"], 2),
                             (["Case.test_ok"], True), ("Case.test_ok", 1), ([None], 1),
                             (["Case"], 1), (["Case.setUp"], 1),
                             (["Case.test_ok);raise SystemExit(0)#"], 1)):
            with self.subTest(names=names), self.assertRaises(evaluation.InputIntegrityError):
                evaluation.task_driver({"test_names": names, "expected_tests": count})

    def test_selected_methods_run_in_actual_worker_and_keep_failure_skip_and_missing_errors(self):
        hidden = (b"import unittest\nclass Selected(unittest.TestCase):\n"
                  b" def test_pass(self): pass\n"
                  b" def test_fail(self): self.fail('unselected-control')\n"
                  b" @unittest.skip('synthetic selected skip')\n def test_skip(self): pass\n")
        task = {**evaluation.TASKS[0], "source": "scripts/a.py", "expected_tests": 1}
        with mock.patch.object(evaluation, "pinned_blob", return_value=hidden):
            for name, field in (("pass", None), ("fail", "failures"), ("missing", "errors"), ("skip", "skipped")):
                selected = {**task, "test_names": ["Selected.test_" + name]}
                report = self.grade_fixture(selected, {"scripts/a.py": b"# fixture\n"},
                                            self.root / ("selection-" + name))
                self.assertEqual(report["test_names"], selected["test_names"])
                self.assertEqual(report["tests"]["tests"], 1)
                self.assertEqual(report["passed"], field is None, report)
                self.assertIsNone(report["model_score"])
                if field is not None:
                    self.assertEqual(report["tests"][field], 1)
            full = self.grade_fixture({**task, "expected_tests": 3}, {"scripts/a.py": b"# fixture\n"},
                                      self.root / "selection-legacy")
            self.assertFalse(full["passed"])
            self.assertEqual(full["tests"]["tests"], 3)
            self.assertEqual(full["tests"]["failures"], 1)
            self.assertEqual(full["tests"]["skipped"], 1)

    def test_additional_task_actual_cli_prepares_and_grades_frozen_base_and_reference(self):
        task = evaluation.TASKS[5]  # Route-grant arguments: no fixture or model substitution.
        script = str(evaluation.ROOT / "scripts/coding_eval.py")
        candidate = self.root / "new-task-candidate"
        result = subprocess.run(["/usr/bin/python3", script, "prepare", "--task", task["id"],
                                 "--output", str(candidate)], capture_output=True, timeout=30)
        self.assertEqual(result.returncode, 0, result.stderr)
        for kind, expected_exit in (("base", 1), ("reference", 0)):
            if kind == "reference":
                (candidate / task["source"]).write_bytes(evaluation.pinned_blob(
                    task["reference"], task["source"], task["reference_sha256"]))
            output = self.root / ("new-task-" + kind)
            result = subprocess.run(["/usr/bin/python3", script, "score", "--task", task["id"],
                                     "--candidate", str(candidate), "--output", str(output)],
                                    capture_output=True, timeout=30)
            self.assertEqual(result.returncode, expected_exit, result.stderr)
            report = json.loads((output / "report.json").read_text())
            self.assertEqual(report["candidate_kind"], "frozen-" + kind)
            self.assertEqual(report["test_names"], task["test_names"])
            self.assertEqual(report["tests"]["tests"], task["expected_tests"])
            self.assertEqual(report["model_attempts"], 0)
            self.assertIsNone(report["model_score"])
        with (candidate / task["source"]).open("ab") as source:
            source.write(b"\n# generated candidate is still disabled\n")
        output = self.root / "new-task-refused"
        result = subprocess.run(["/usr/bin/python3", script, "score", "--task", task["id"],
                                 "--candidate", str(candidate), "--output", str(output)],
                                capture_output=True, timeout=30)
        self.assertEqual(result.returncode, 2)
        self.assertIn(b"evaluation.untrusted-candidate-disabled", result.stderr)
        self.assertFalse(output.exists())

    def test_refusal_diagnostics_include_fixed_code_and_type_without_private_messages(self):
        for error, detail in (
            (evaluation.EvaluationError("evaluation.input-hash-mismatch"),
             "code=evaluation.input-hash-mismatch type=EvaluationError"),
            (FileNotFoundError("private input path"), "code=evaluation.io-failed type=FileNotFoundError"),
            (subprocess.CalledProcessError(1, ["private command"]),
             "code=evaluation.process-failed type=CalledProcessError"),
        ):
            stream = io.StringIO()
            with mock.patch.object(evaluation, "main", side_effect=error), redirect_stderr(stream):
                self.assertEqual(evaluation.cli(), 2)
            self.assertIn(detail, stream.getvalue())
            self.assertNotIn("private input path", stream.getvalue())
            self.assertNotIn("private command", stream.getvalue())

    def test_infrastructure_exception_preserves_logs_and_cannot_pass_partial_counts(self):
        output = self.root / "infrastructure"

        def partial_case(*_):
            evaluation.write_file(output / "stderr.log", b"partial case log\n")
            evaluation.publish(output / "tests.json", {"tests": 12, "failures": 0})
            raise OSError("private cleanup path")

        with mock.patch.object(evaluation, "grade_case", side_effect=partial_case):
            report = self.grade_fixture(evaluation.TASKS[0], {evaluation.TASKS[0]["source"]: b"fixture"}, output)
        self.assertFalse(report["passed"])
        self.assertEqual(report["cause"], "infrastructure")
        self.assertIsNone(report["tests"])
        self.assertIsNone(report["exit_code"])
        self.assertEqual(report["infrastructure_error"], "code=evaluation.io-failed type=OSError")
        self.assertFalse((output / "tests.json").exists())
        self.assertEqual((output / "stderr.log").read_bytes(), b"partial case log\n")
        self.assertEqual(report["logs"]["stderr.log"], evaluation.sha(b"partial case log\n"))
        self.assertEqual(json.loads((output / "report.json").read_text()), report)

    def test_verifier_retains_infrastructure_failure_and_checks_remaining_cases(self):
        clean = {"tests": 12, "failures": 0, "errors": 0, "skipped": 0,
                 "expected_failures": 0, "unexpected_successes": 0}
        calls = []

        def run_case(task, files, output):
            calls.append(task["id"])
            if len(calls) == 1:
                raise OSError("private initialization failure")
            evaluation.write_file(output / "stdout.log", b"")
            evaluation.write_file(output / "stderr.log", b"fixture log\n")
            counts = {**clean, "tests": task["expected_tests"], "failures": len(calls) % 2}
            return len(calls) % 2, False, False, counts

        with mock.patch.object(evaluation, "grade_case", side_effect=run_case), \
                mock.patch.object(evaluation, "projection", side_effect=lambda t: {t["source"]: b"base"}), \
                mock.patch.object(evaluation, "pinned_blob", return_value=b"reference"), \
                mock.patch.object(evaluation, "require_historical_candidate", return_value="synthetic-control"):
            summary = evaluation.verify(self.root / "infrastructure-verify")
        self.assertEqual(len(calls), 30)
        self.assertFalse(summary["passed"])
        self.assertIsNone(summary["model_score"])
        self.assertEqual(summary["tasks"][0]["base"]["cause"], "infrastructure")
        self.assertEqual([r["valid"] for r in summary["tasks"]], [False] + [True] * 14)

    def test_exact_snapshot_allows_only_the_declared_source_edit(self):
        root, original = self.candidate()
        (root / "scripts/a.py").write_bytes(b"candidate\n")
        observed = evaluation.read_candidate(root, original, "scripts/a.py")
        self.assertEqual(observed["scripts/a.py"], b"candidate\n")
        self.assertEqual(observed["scripts/b.py"], original["scripts/b.py"])
        (root / "scripts/b.py").write_bytes(b"changed\n")
        with self.assertRaisesRegex(evaluation.EvaluationError, "out-of-scope-edit"):
            evaluation.read_candidate(root, original, "scripts/a.py")

    def test_deleted_added_hidden_test_and_empty_directory_are_refused(self):
        root, original = self.candidate()
        file = root / "scripts/a.py"
        file.unlink()
        with self.assertRaisesRegex(evaluation.EvaluationError, "file-missing"):
            evaluation.read_candidate(root, original, "scripts/a.py")
        file.write_bytes(original["scripts/a.py"])
        for name in ("test_hidden.py", "scripts/injected.py"):
            path = root / name
            path.write_text("outside projection\n")
            with self.assertRaisesRegex(evaluation.EvaluationError, "file-refused"):
                evaluation.read_candidate(root, original, "scripts/a.py")
            path.unlink()
        (root / "extra").mkdir()
        with self.assertRaisesRegex(evaluation.EvaluationError, "directory-refused"):
            evaluation.read_candidate(root, original, "scripts/a.py")

    def test_alias_hardlink_fifo_and_oversize_inputs_are_refused(self):
        root, original = self.candidate()
        path = root / "scripts/a.py"
        outside = self.root / "secret"
        outside.write_text("private\n")
        path.unlink()
        path.symlink_to(outside)
        with self.assertRaises((evaluation.EvaluationError, OSError)):
            evaluation.read_candidate(root, original, "scripts/a.py")
        path.unlink()
        os.link(outside, path)
        with self.assertRaisesRegex(evaluation.EvaluationError, "file-refused"):
            evaluation.read_candidate(root, original, "scripts/a.py")
        path.unlink()
        os.mkfifo(path)
        with self.assertRaisesRegex(evaluation.EvaluationError, "file-refused"):
            evaluation.read_candidate(root, original, "scripts/a.py")
        path.unlink()
        path.write_bytes(b"x" * (evaluation.MAX_FILE + 1))
        with self.assertRaisesRegex(evaluation.EvaluationError, "file-refused"):
            evaluation.read_candidate(root, original, "scripts/a.py")

    def test_linked_directory_is_not_walked(self):
        root, original = self.candidate()
        elsewhere = self.root / "elsewhere"
        (root / "scripts").rename(elsewhere)
        (root / "scripts").symlink_to(elsewhere, target_is_directory=True)
        with self.assertRaisesRegex(evaluation.EvaluationError, "directory-refused"):
            evaluation.read_candidate(root, original, "scripts/a.py")

    def test_existing_output_is_preserved(self):
        output = self.root / "output"
        output.mkdir()
        evaluation.write_file(output / "original", b"unchanged")
        with self.assertRaises(FileExistsError):
            evaluation.new_directory(output)
        with self.assertRaises(FileExistsError):
            evaluation.write_file(output / "original", b"overwritten")
        self.assertEqual((output / "original").read_bytes(), b"unchanged")

    def test_only_exact_registered_base_and_reference_snapshots_can_be_graded(self):
        task = evaluation.TASKS[0]
        files = evaluation.projection(task)
        self.assertEqual(evaluation.require_historical_candidate(task, files), "frozen-base")
        files[task["source"]] = evaluation.pinned_blob(
            task["reference"], task["source"], task["reference_sha256"])
        self.assertEqual(evaluation.require_historical_candidate(task, files), "frozen-reference")
        for snapshot in ({**files, task["source"]: files[task["source"]] + b"\n# generated\n"},
                         {**files, "scripts/foreign.py": b"outside scope"}):
            with self.subTest(keys=len(snapshot)), \
                    mock.patch.object(evaluation, "grade_case") as worker, \
                    self.assertRaisesRegex(evaluation.EvaluationError, "untrusted-candidate-disabled"):
                evaluation.grade(task, snapshot, self.root / "refused")
            worker.assert_not_called()
            self.assertFalse((self.root / "refused").exists())
        with self.assertRaisesRegex(evaluation.EvaluationError, "task-definition-refused"):
            evaluation.grade({**task, "expected_tests": 1}, files, self.root / "refused")

    def test_actual_cli_refuses_a_forged_success_before_candidate_execution(self):
        task = evaluation.TASKS[0]
        candidate = self.root / "forged-candidate"
        prepared = subprocess.run(["/usr/bin/python3", str(evaluation.ROOT / "scripts/coding_eval.py"),
                                   "prepare", "--task", task["id"], "--output", str(candidate)],
                                  capture_output=True, timeout=30)
        self.assertEqual(prepared.returncode, 0, prepared.stderr)
        source = candidate / task["source"]
        clean = {"tests": task["expected_tests"], "failures": 0, "errors": 0, "skipped": 0,
                 "expected_failures": 0, "unexpected_successes": 0}
        # This is the previously admitted result-forgery vector: candidate code
        # writes full passing counts, then exits without running the regressions.
        with source.open("ab") as stream:
            stream.write(("\nimport os,json\nwith open('/output/tests.json','w') as forged:\n"
                          f" json.dump({clean!r},forged)\nos._exit(0)\n").encode())
        output = self.root / "forged-result"
        scored = subprocess.run(["/usr/bin/python3", str(evaluation.ROOT / "scripts/coding_eval.py"),
                                 "score", "--task", task["id"], "--candidate", str(candidate),
                                 "--output", str(output)], capture_output=True, timeout=30)
        self.assertEqual(scored.returncode, 2)
        self.assertIn(b"evaluation.untrusted-candidate-disabled", scored.stderr)
        self.assertFalse(output.exists())
        self.assertNotIn(str(self.root).encode(), scored.stderr)
        self.assertEqual(scored.stdout, b"")

    def test_hidden_input_integrity_failure_never_launches_and_returns_exit_two(self):
        task = evaluation.TASKS[0]
        files = evaluation.projection(task)
        real_pinned_blob = evaluation.pinned_blob

        def corrupted_hidden(revision, name, expected):
            if name == task["hidden_test"]:
                raise evaluation.InputIntegrityError("evaluation.input-hash-mismatch")
            return real_pinned_blob(revision, name, expected)

        candidate = self.root / "base"
        for name, content in files.items():
            evaluation.write_file(candidate / name, content)
        output = self.root / "integrity-result"
        with mock.patch.object(evaluation, "pinned_blob", side_effect=corrupted_hidden), \
                mock.patch.object(evaluation.subprocess, "Popen") as worker, \
                mock.patch("sys.argv", ["coding_eval.py", "score", "--task", task["id"],
                                        "--candidate", str(candidate), "--output", str(output)]):
            # Projection Git runs must remain real; grade_case's Popen must not.
            with mock.patch.object(evaluation, "projection", return_value=files):
                self.assertEqual(evaluation.main(), 2)
            worker.assert_not_called()
        report = json.loads((output / "report.json").read_text())
        self.assertFalse(report["passed"])
        self.assertEqual(report["cause"], "input_integrity")
        self.assertIn("evaluation.input-hash-mismatch", report["input_error"])
        self.assertIsNone(report["infrastructure_error"])
        self.assertIsNone(report["tests"])
        self.assertIsNone(report["model_score"])
        self.assertEqual(report["candidate_kind"], "frozen-base")
        self.assertFalse((output / "tests.json").exists())

    def test_verifier_integrity_refusal_is_not_a_base_test_failure(self):
        with mock.patch.object(evaluation, "grade_case", side_effect=evaluation.InputIntegrityError(
                "evaluation.input-hash-mismatch")):
            summary = evaluation.verify(self.root / "verify-integrity")
        self.assertFalse(summary["passed"])
        self.assertTrue(summary["input_refused"])
        self.assertTrue(all(not row["valid"] for row in summary["tasks"]))
        self.assertTrue(all(case["cause"] == "input_integrity" for row in summary["tasks"]
                            for case in (row["base"], row["reference"])))
        with mock.patch.object(evaluation, "verify", return_value=summary), \
                mock.patch("sys.argv", ["coding_eval.py", "verify", "--output", "unused"]):
            self.assertEqual(evaluation.main(), 2)

    def test_workspace_retries_transient_cleanup_before_counts_are_published(self):
        task = {**evaluation.TASKS[0], "expected_tests": 1, "source": "scripts/a.py"}
        hidden = b"import unittest\nclass Control(unittest.TestCase):\n def test_control(self): pass\n"
        output = self.root / "retry"
        real_rmtree = evaluation.shutil.rmtree
        attempts = []

        def retry(path):
            attempts.append(path)
            self.assertFalse((output / "tests.json").exists())
            if len(attempts) == 1:
                raise OSError("synthetic transient failure")
            real_rmtree(path)

        with mock.patch.object(evaluation, "pinned_blob", return_value=hidden), \
                mock.patch.object(evaluation.shutil, "rmtree", side_effect=retry):
            report = self.grade_fixture(task, {"scripts/a.py": b"# fixture"}, output)
        self.assertTrue(report["passed"], report)
        self.assertEqual(json.loads((output / "tests.json").read_text())["tests"], 1)
        self.assertEqual(len(attempts), 2)
        self.assertFalse(attempts[-1].exists())
        self.assertFalse((output / "cleanup.json").exists())

    def test_missing_frozen_blob_is_an_input_refusal_without_private_exception_text(self):
        error = subprocess.CalledProcessError(1, ["private command with path"])
        with mock.patch.object(evaluation, "git", side_effect=error), \
                self.assertRaises(evaluation.InputIntegrityError) as raised:
            evaluation.pinned_blob("unavailable", "input", "0" * 64)
        self.assertEqual(str(raised.exception), "evaluation.input-unavailable")

    def test_workspace_is_removed_on_interruption_without_publishing_counts(self):
        output = self.root / "interrupted"
        output.mkdir()
        with self.assertRaises(KeyboardInterrupt):
            with evaluation.case_workspace(output) as workspace:
                evaluation.write_file(workspace / "partial", b"interrupted")
                raise KeyboardInterrupt
        self.assertFalse(workspace.exists())
        self.assertEqual(list(output.iterdir()), [])

    def test_persistent_cleanup_failure_cannot_mask_interruption_and_continue_verification(self):
        output = self.root / "interrupted-cleanup"
        output.mkdir()
        workspace = None
        try:
            with mock.patch.object(evaluation.shutil, "rmtree", side_effect=OSError("cleanup")) as cleanup, \
                    self.assertRaises(KeyboardInterrupt):
                with evaluation.case_workspace(output) as workspace:
                    evaluation.write_file(workspace / "partial", b"interrupted")
                    raise KeyboardInterrupt
            self.assertEqual(cleanup.call_count, 3)
            recovery = json.loads((output / "cleanup.json").read_text())
            self.assertEqual(Path(recovery["private_workspace"]), workspace)
            self.assertFalse(recovery["removed"])
            self.assertFalse((output / "tests.json").exists())
        finally:
            if workspace is not None:
                evaluation.shutil.rmtree(workspace, ignore_errors=True)

    def test_late_persistent_cleanup_failure_keeps_recovery_record_and_no_counts(self):
        task = {**evaluation.TASKS[0], "expected_tests": 1, "source": "scripts/a.py"}
        hidden = b"import unittest\nclass Control(unittest.TestCase):\n def test_control(self): pass\n"
        output = self.root / "cleanup-failed"
        real_rmtree = evaluation.shutil.rmtree
        attempts = []

        def fail_cleanup(path):
            attempts.append(path)
            self.assertFalse((output / "tests.json").exists())
            raise OSError("synthetic persistent cleanup failure")

        try:
            with mock.patch.object(evaluation, "pinned_blob", return_value=hidden), \
                    mock.patch.object(evaluation.shutil, "rmtree", side_effect=fail_cleanup):
                report = self.grade_fixture(task, {"scripts/a.py": b"# fixture"}, output)
            self.assertEqual(len(attempts), 3)
            self.assertFalse(report["passed"])
            self.assertEqual(report["cause"], "infrastructure")
            self.assertIn("evaluation.cleanup-failed", report["infrastructure_error"])
            self.assertIsNone(report["tests"])
            self.assertFalse((output / "tests.json").exists())
            recovery = json.loads((output / "cleanup.json").read_text())
            self.assertFalse(recovery["removed"])
            self.assertEqual(recovery["attempts"], 3)
            self.assertEqual(Path(recovery["private_workspace"]), attempts[-1])
            self.assertTrue(attempts[-1].exists())
            self.assertIn(b"OK", (output / "stderr.log").read_bytes())
        finally:
            for path in set(attempts):
                real_rmtree(path, ignore_errors=True)

    def test_task_id_rejects_ambiguous_or_unknown_selection(self):
        with mock.patch("sys.argv", ["coding_eval.py", "prepare", "--task", "held-out", "--output", "unused"]):
            with redirect_stderr(io.StringIO()), self.assertRaises(SystemExit) as raised:
                evaluation.main()
        self.assertEqual(raised.exception.code, 2)

    def test_public_task_listing_omits_hidden_test_and_reference(self):
        stream = io.StringIO()
        with mock.patch("sys.argv", ["coding_eval.py", "list"]), mock.patch("sys.stdout", stream):
            self.assertEqual(evaluation.main(), 0)
        value = json.loads(stream.getvalue())
        self.assertEqual(len(value["tasks"]), 15)
        for task in value["tasks"]:
            self.assertEqual(task["split"], "development")
            self.assertNotIn("reference", task)
            self.assertNotIn("hidden_test", task)
            self.assertTrue(task["statement"])

    def test_pinned_source_and_hidden_inputs_refuse_changed_bytes(self):
        for task in evaluation.TASKS:
            for revision, path, digest in (
                (task["reference"], task["hidden_test"], task["hidden_sha256"]),
                (task["base"], task["source"], task["base_sha256"]),
                (task["reference"], task["source"], task["reference_sha256"]),
            ):
                with self.subTest(task=task["id"], path=path):
                    blob = evaluation.pinned_blob(revision, path, digest)
                    self.assertEqual(hashlib.sha256(blob).hexdigest(), digest)
                    with self.assertRaisesRegex(evaluation.EvaluationError, "hash-mismatch"):
                        evaluation.pinned_blob(revision, path, "0" * 64)

    def test_projection_is_pinned_and_contains_neither_history_nor_hidden_tests(self):
        for task in evaluation.TASKS:
            files = evaluation.projection(task)
            self.assertTrue(files)
            self.assertTrue(all(p.startswith("scripts/") and p.endswith(".py") for p in files))
            self.assertNotIn(task["hidden_test"], files)
            self.assertNotIn(".git", files)
            with self.assertRaisesRegex(evaluation.EvaluationError, "projection-hash-mismatch"):
                evaluation.projection({**task, "projection_sha256": "0" * 64})

    def test_sandbox_has_no_network_home_repository_or_candidate_write_mount(self):
        command = evaluation.sandbox_command(Path("/owned/workspace"), Path("/owned/grader"), Path("/owned/output"))
        self.assertIn("--unshare-all", command)
        self.assertIn("--clearenv", command)
        self.assertIn("--disable-userns", command)
        self.assertEqual(command.count("--bind"), 1)
        self.assertIn(["--bind", "/owned/output", "/output"], [command[i:i + 3] for i in range(len(command))])
        self.assertIn(["--ro-bind", "/owned/workspace", "/workspace"],
                      [command[i:i + 3] for i in range(len(command))])
        self.assertNotIn(str(evaluation.ROOT), command)
        self.assertNotIn("--share-net", command)

    def test_grade_missing_counts_or_skipped_tests_cannot_pass(self):
        task = evaluation.TASKS[0]
        for index, counts in enumerate((None, {}, {"tests": 0},
                {"tests": 1, "failures": 0, "errors": 0, "skipped": 1,
                 "expected_failures": 0, "unexpected_successes": 0},
                {"tests": 1, "failures": 0, "errors": 0, "skipped": 0,
                 "expected_failures": 0, "unexpected_successes": 0})):
            output = self.root / f"grade-{index}"

            def launch(command, **kwargs):
                if counts is not None:
                    child_output = Path(command[command.index("--bind") + 1])
                    evaluation.publish(child_output / "tests.json", counts)
                return real_popen(["/usr/bin/true"], stdin=kwargs["stdin"], stdout=kwargs["stdout"],
                                  stderr=kwargs["stderr"], start_new_session=True)
            real_popen = subprocess.Popen
            with mock.patch.object(evaluation, "pinned_blob", return_value=b"unexecuted fixture"), \
                    mock.patch.object(evaluation.subprocess, "Popen", side_effect=launch), \
                    mock.patch.object(evaluation, "initialize_grading_repository"):
                report = self.grade_fixture({**task, "expected_tests": 1}, {task["source"]: b"synthetic"}, output)
            self.assertEqual(report["passed"], index == 4)
            self.assertEqual(report["model_attempts"], 0)
            self.assertIsNone(report["tokens"])
            self.assertEqual(report["gpu_minutes"], 0)

    def test_result_files_refuse_links_fifo_and_oversize_before_reading(self):
        outside = self.root / "private"
        outside.write_bytes(b"not an evaluator result")
        alias = self.root / "alias"
        alias.symlink_to(outside)
        with self.assertRaises(OSError):
            evaluation.read_regular(alias, 100)
        alias.unlink()
        os.mkfifo(alias)
        with self.assertRaisesRegex(evaluation.EvaluationError, "result-file-refused"):
            evaluation.read_regular(alias, 100)
        with self.assertRaisesRegex(evaluation.EvaluationError, "result-file-refused"):
            evaluation.read_regular(outside, 2)
        self.assertEqual(evaluation.read_regular(outside, 100), outside.read_bytes())

    def test_timeout_is_a_failure_and_cleans_the_owned_process_group(self):
        task = evaluation.TASKS[0]
        output = self.root / "timeout"
        late = self.root / "late"
        command = ["/usr/bin/python3", "-c",
                   "import pathlib,time; time.sleep(0.5); pathlib.Path(" + repr(str(late)) + ").touch()"]
        with mock.patch.object(evaluation, "pinned_blob", return_value=b"fixture"), \
                mock.patch.object(evaluation, "sandbox_command", return_value=command), \
                mock.patch.object(evaluation, "TIMEOUT", 0.03):
            report = self.grade_fixture(task, {task["source"]: b"fixture"}, output)
        self.assertFalse(report["passed"])
        self.assertEqual(report["cause"], "timeout")
        self.assertIsNone(report["exit_code"])
        self.assertFalse(late.exists())

    def test_grading_git_metadata_contains_one_candidate_commit_and_no_answers(self):
        workspace = self.root / "grading"
        workspace.mkdir()
        evaluation.write_file(workspace / "scripts/a.py", b"candidate source\n")
        evaluation.initialize_grading_repository(workspace)
        def git(*args):
            return subprocess.run(["/usr/bin/git", *args], cwd=workspace, capture_output=True, check=True).stdout
        self.assertEqual(git("rev-list", "--count", "HEAD"), b"1\n")
        self.assertEqual(git("show", "HEAD:scripts/a.py"), b"candidate source\n")
        self.assertEqual(git("status", "--porcelain"), b"")
        self.assertEqual(git("remote"), b"")
        self.assertEqual(git("tag"), b"")
        with self.assertRaises(subprocess.CalledProcessError):
            git("cat-file", "-e", evaluation.TASKS[0]["reference"])

    def test_grading_git_objects_remain_unchanged_after_auto_maintenance_window(self):
        workspace = self.root / "many-objects"
        workspace.mkdir()
        # More loose objects than the real projection that triggered detached packing.
        for index in range(800):
            evaluation.write_file(workspace / f"scripts/module_{index}.py", f"value = {index}\n".encode())
        evaluation.initialize_grading_repository(workspace)
        objects = workspace / ".git/objects"

        def snapshot():
            return {str(p.relative_to(objects)): evaluation.sha(p.read_bytes())
                    for p in objects.rglob("*") if p.is_file()}

        before = snapshot()
        self.assertGreaterEqual(len(before), 803)
        time.sleep(1.2)
        self.assertEqual(snapshot(), before)
        self.assertEqual(list((objects / "pack").iterdir()), [])

    def test_output_limit_retains_a_bounded_log_and_fails(self):
        task = evaluation.TASKS[0]
        output = self.root / "oversize-output"
        command = ["/usr/bin/python3", "-c", "import os; os.write(1, b'x' * (3 * 1024 * 1024))"]
        with mock.patch.object(evaluation, "pinned_blob", return_value=b"fixture"), \
                mock.patch.object(evaluation, "sandbox_command", return_value=command):
            report = self.grade_fixture(task, {task["source"]: b"fixture"}, output)
        self.assertFalse(report["passed"])
        self.assertTrue(report["output_truncated"])
        self.assertEqual(report["cause"], "output_limit")
        self.assertEqual((output / "stdout.log").stat().st_size, 2 * evaluation.MAX_FILE)

    def test_actual_worker_has_read_only_inputs_and_cannot_reach_host_listener(self):
        import socket
        listener = socket.socket()
        self.addCleanup(listener.close)
        listener.bind(("127.0.0.1", 0))
        listener.listen()
        port = listener.getsockname()[1]
        hidden = ("import pathlib,socket,unittest\n"
                  "class Boundary(unittest.TestCase):\n"
                  " def test_boundary(self):\n"
                  "  with self.assertRaises(OSError): pathlib.Path('/workspace/scripts/a.py').write_text('changed')\n"
                  "  with self.assertRaises(OSError): pathlib.Path('/grader/test.py').write_text('changed')\n"
                  f"  self.assertFalse(pathlib.Path({str(evaluation.ROOT)!r}).exists())\n"
                  f"  self.assertFalse(pathlib.Path({str(Path.home())!r}).exists())\n"
                  "  keys={p.split(b'=',1)[0] for p in pathlib.Path('/proc/self/environ').read_bytes().split(b'\\0') if p}\n"
                  "  self.assertEqual(keys,{b'PATH',b'HOME',b'LANG',b'PWD'})\n"
                  "  with socket.socket() as client:\n"
                  "   client.settimeout(0.5)\n"
                  f"   self.assertNotEqual(client.connect_ex(('127.0.0.1',{port})),0)\n").encode()
        task = {**evaluation.TASKS[0], "source": "scripts/a.py", "expected_tests": 1}
        with mock.patch.object(evaluation, "pinned_blob", return_value=hidden):
            report = self.grade_fixture(task, {"scripts/a.py": b"# synthetic boundary fixture\n"},
                                      self.root / "boundary")
        self.assertTrue(report["passed"], report)
        self.assertEqual(report["tests"]["tests"], 1)


if __name__ == "__main__":
    unittest.main()
