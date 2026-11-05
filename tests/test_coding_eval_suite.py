"""Exercise suite commitments and refusal boundaries without held-out feedback."""
from __future__ import annotations

from contextlib import contextmanager
import json
import os
from pathlib import Path
import tempfile
import unittest
from unittest import mock

from scripts import coding_eval as evaluation
from scripts import coding_eval_suite as suite


def counts(failures=0, errors=0):
    return {"tests": 1, "failures": failures, "errors": errors, "skipped": 0,
            "expected_failures": 0, "unexpected_successes": 0}


def synthetic_task():
    return {"id": "held-001", "split": "held-out", "base": "a" * 40, "reference": "b" * 40,
            "source": "scripts/example.py", "hidden_test": "tests/test_example.py",
            "title": "Synthetic unit-test task", "statement": "Synthetic statement, never a suite task.",
            "test_names": ["ExampleTests.test_synthetic"], "expected_tests": 1,
            "base_sha256": evaluation.sha(b"base"), "reference_sha256": evaluation.sha(b"reference"),
            "hidden_sha256": "c" * 64, "projection_sha256": "d" * 64}


@contextmanager
def descriptor(root, task=None, raw=None):
    task = synthetic_task() if task is None else task
    data = (json.dumps({"version": "coding-held-out-v1", "origin": "original-repository-history",
                        "tasks": [task]}, sort_keys=True).encode() if raw is None else raw)
    path = root / "private.json"
    evaluation.write_file(path, data)
    with mock.patch.object(suite, "HELD_OUT_SHA256", evaluation.sha(data)), \
            mock.patch.object(suite, "HELD_OUT_COUNT", 1), mock.patch.object(suite, "HELD_OUT_METHODS", 1):
        yield path


class CodingSuiteTests(unittest.TestCase):
    def test_development_contracts_remain_the_exact_thirty_accepted_tasks(self):
        self.assertEqual(len(suite.development_tasks()), 30)
        self.assertEqual(sum(t["expected_tests"] for t in suite.development_tasks()), 89)
        with mock.patch.object(evaluation, "collection", return_value=[]):
            with self.assertRaisesRegex(evaluation.InputIntegrityError, "development-changed"):
                suite.development_tasks()

    def test_valid_private_descriptor_returns_only_committed_definitions(self):
        with tempfile.TemporaryDirectory() as temporary, descriptor(Path(temporary)) as path:
            self.assertEqual(suite.held_out_tasks(path), [synthetic_task()])
            path.write_bytes(path.read_bytes() + b"\n")
            with self.assertRaisesRegex(evaluation.InputIntegrityError, "held-out-changed"):
                suite.held_out_tasks(path)

    def test_descriptor_aliases_shared_permissions_hardlinks_and_special_files_refuse(self):
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            with descriptor(root) as path:
                alias = root / "alias"
                alias.symlink_to(path)
                with self.assertRaises(OSError):
                    suite.held_out_tasks(alias)
                path.chmod(0o640)
                with self.assertRaisesRegex(evaluation.InputIntegrityError, "private-input-refused"):
                    suite.held_out_tasks(path)
                path.chmod(0o600)
                os.link(path, root / "hardlink")
                with self.assertRaisesRegex(evaluation.InputIntegrityError, "private-input-refused"):
                    suite.held_out_tasks(path)
            fifo = root / "fifo"
            os.mkfifo(fifo, 0o600)
            with self.assertRaisesRegex(evaluation.InputIntegrityError, "private-input-refused"):
                suite.private_bytes(fifo)

    def test_wrong_owner_and_oversize_descriptor_refuse_before_parsing(self):
        with tempfile.TemporaryDirectory() as temporary, descriptor(Path(temporary)) as path:
            with mock.patch.object(suite.os, "geteuid", return_value=os.geteuid() + 1):
                with self.assertRaisesRegex(evaluation.InputIntegrityError, "private-input-refused"):
                    suite.held_out_tasks(path)
            with mock.patch.object(evaluation, "MAX_FILE", 1):
                with self.assertRaisesRegex(evaluation.InputIntegrityError, "private-input-refused"):
                    suite.held_out_tasks(path)

    def test_duplicate_json_keys_are_not_silently_normalized(self):
        raw = b'{"version":"coding-held-out-v1","version":"coding-held-out-v1"}'
        with tempfile.TemporaryDirectory() as temporary, descriptor(Path(temporary), raw=raw) as path:
            with self.assertRaisesRegex(evaluation.InputIntegrityError, "duplicate-key"):
                suite.held_out_tasks(path)

    def test_malformed_contracts_never_get_an_execution_route(self):
        for change in ({"id": "dev-001"}, {"split": "development"}, {"base": "HEAD"},
                       {"reference": "b" * 39}, {"source": "scripts/../example.py"},
                       {"hidden_test": "scripts/test_example.py"}, {"title": " "},
                       {"expected_tests": True}, {"test_names": []}, {"test_names": ["arbitrary()"]},
                       {"reference_sha256": synthetic_task()["base_sha256"]}, {"extra": "override"}):
            with self.subTest(change=change), tempfile.TemporaryDirectory() as temporary:
                with descriptor(Path(temporary), task={**synthetic_task(), **change}) as path:
                    with self.assertRaises(evaluation.InputIntegrityError), \
                            mock.patch.object(evaluation, "grade_case") as worker:
                        suite.held_out_control(path, "held-001", "reference", Path(temporary) / "output")
                    worker.assert_not_called()
                    self.assertFalse((Path(temporary) / "output").exists())

    def test_control_entry_accepts_only_frozen_ids_and_kinds(self):
        with tempfile.TemporaryDirectory() as temporary, descriptor(Path(temporary)) as path:
            root = Path(temporary)
            for task_id, kind in (("dev-001", "base"), ("held-001", "generated"), ("held-002", "reference")):
                with self.assertRaisesRegex(evaluation.InputIntegrityError, "control-refused"), \
                        mock.patch.object(evaluation, "grade_case") as worker:
                    suite.held_out_control(path, task_id, kind, root / "out")
                worker.assert_not_called()
            self.assertFalse((root / "out").exists())

    def test_control_bytes_are_reconstructed_and_logs_bound_by_host(self):
        with tempfile.TemporaryDirectory() as temporary, descriptor(Path(temporary)) as path:
            root = Path(temporary)
            task = synthetic_task()

            def grade_case(actual_task, files, output):
                self.assertEqual(actual_task, task)
                self.assertEqual(files, {"scripts/example.py": b"reference"})
                evaluation.write_file(output / "stdout.log", b"observed output")
                evaluation.write_file(output / "stderr.log", b"")
                return 0, False, False, counts()

            with mock.patch.object(evaluation, "projection", return_value={"scripts/example.py": b"base"}), \
                    mock.patch.object(evaluation, "pinned_blob", return_value=b"reference") as blob, \
                    mock.patch.object(evaluation, "grade_case", side_effect=grade_case):
                result = suite.held_out_control(path, "held-001", "reference", root / "out")
            blob.assert_called_once_with(task["reference"], task["source"], task["reference_sha256"])
            self.assertTrue(result["passed"])
            self.assertEqual(result["logs"]["stdout.log"], evaluation.sha(b"observed output"))
            self.assertEqual(result["candidate_sha256"], task["reference_sha256"])
            self.assertIsNone(result["model_score"])
            self.assertEqual(result["model_attempts"], 0)

    def test_input_and_infrastructure_failures_never_validate_a_base(self):
        with tempfile.TemporaryDirectory() as temporary, descriptor(Path(temporary)) as path:
            root = Path(temporary)
            for index, error in enumerate((OSError("private text"), evaluation.InputIntegrityError("suite.pin"))):
                with mock.patch.object(evaluation, "projection", return_value={"scripts/example.py": b"base"}), \
                        mock.patch.object(evaluation, "grade_case", side_effect=error):
                    result = suite.held_out_control(path, "held-001", "base", root / str(index))
                self.assertFalse(result["passed"])
                self.assertIsNone(result["tests"])
                self.assertNotIn("private text", json.dumps(result))
                self.assertIn(result["cause"], ("infrastructure", "input_integrity"))

    def test_control_discrimination_requires_observed_failures_and_a_passing_reference(self):
        base = {"passed": False, "cause": "test_failure", "exit_code": 1, "tests": counts(failures=1)}
        reference = {"passed": True, "cause": "none"}
        self.assertTrue(suite.controls_valid(base, reference))
        for change in ({"passed": True}, {"cause": "infrastructure"}, {"cause": "input_integrity"},
                       {"exit_code": -9}, {"tests": None}, {"tests": counts()},
                       {"tests": {**counts(failures=1), "skipped": 1}},
                       {"tests": {**counts(failures=1), "unexpected_successes": 1}},
                       {"tests": {**counts(failures=1), "expected_failures": 1}}):
            with self.subTest(change=change):
                self.assertFalse(suite.controls_valid({**base, **change}, reference))
        self.assertFalse(suite.controls_valid(base, {"passed": False, "cause": "test_failure"}))

    def test_input_check_refuses_development_projection_leakage_without_running_hidden_code(self):
        with tempfile.TemporaryDirectory() as temporary, descriptor(Path(temporary)) as path:
            dev = {"base": "f" * 40, "reference": "e" * 40, "source": "scripts/other.py",
                   "reference_sha256": "e" * 64, "hidden_test": "tests/test_other.py",
                   "hidden_sha256": "a" * 64}
            with mock.patch.object(suite, "development_tasks", return_value=[dev]), \
                    mock.patch.object(evaluation, "projection", return_value={"scripts/example.py": b"reference"}), \
                    mock.patch.object(evaluation, "pinned_blob"), \
                    mock.patch.object(evaluation, "grade_case") as worker:
                with self.assertRaisesRegex(evaluation.InputIntegrityError, "split-leakage"):
                    suite.check_inputs(path)
                worker.assert_not_called()

    def test_verification_requires_row_close_before_reading_or_creating_anything(self):
        with mock.patch.object(suite, "check_inputs") as check, \
                mock.patch.object(evaluation, "new_directory") as mkdir:
            for purpose in (None, "development", "tune", "scoreboard"):
                with self.assertRaisesRegex(evaluation.InputIntegrityError, "purpose-refused"):
                    suite.verify(Path("absent"), Path("output"), purpose)
            check.assert_not_called()
            mkdir.assert_not_called()

    def test_input_check_observes_pins_without_executing_held_out_methods(self):
        with tempfile.TemporaryDirectory() as temporary, descriptor(Path(temporary)) as path:
            dev = {"base": "f" * 40, "reference": "e" * 40, "source": "scripts/other.py",
                   "reference_sha256": "e" * 64, "hidden_test": "tests/test_other.py",
                   "hidden_sha256": "a" * 64}
            with mock.patch.object(suite, "development_tasks", return_value=[dev]), \
                    mock.patch.object(evaluation, "projection", return_value={"scripts/example.py": b"base"}), \
                    mock.patch.object(evaluation, "git", return_value=("a" * 40 + "\n").encode()), \
                    mock.patch.object(evaluation, "pinned_blob") as pinned, \
                    mock.patch.object(evaluation, "grade_case") as worker:
                result = suite.check_inputs(path)
            self.assertTrue(result["passed"])
            self.assertEqual(result["held_out_test_methods_run"], 0)
            self.assertEqual(pinned.call_count, 4)
            worker.assert_not_called()

    def test_complete_orchestration_binds_raw_controls_but_printable_summary_has_no_task_feedback(self):
        with tempfile.TemporaryDirectory() as temporary, descriptor(Path(temporary)) as path:
            root = Path(temporary)

            def verify_existing(output):
                evaluation.new_directory(output)
                result = {"passed": True, "input_refused": False, "tasks": [{}]}
                evaluation.publish(output / "summary.json", result)
                return result

            def grade_case(task, files, output):
                evaluation.write_file(output / "stdout.log", b"private-test-diagnostic")
                evaluation.write_file(output / "stderr.log", b"")
                fixed = files["scripts/example.py"] == b"reference"
                return (0 if fixed else 1), False, False, counts(failures=0 if fixed else 1)

            with mock.patch.object(suite, "check_inputs", return_value={"model_attempts": 0, "model_score": None,
                                                                        "generated_scoring": False}), \
                    mock.patch.object(evaluation, "verify", side_effect=verify_existing), \
                    mock.patch.object(suite.oracle, "verify", side_effect=verify_existing), \
                    mock.patch.object(evaluation, "projection", return_value={"scripts/example.py": b"base"}), \
                    mock.patch.object(evaluation, "pinned_blob", return_value=b"reference"), \
                    mock.patch.object(evaluation, "grade_case", side_effect=grade_case):
                result = suite.verify(path, root / "run", "row-close")
                with self.assertRaises(FileExistsError):
                    suite.verify(path, root / "run", "row-close")
            self.assertTrue(result["passed"])
            self.assertEqual(result["held_out_test_methods_run"], 2)
            self.assertEqual(result["control_attempts"], 6)
            self.assertIsNone(result["model_score"])
            self.assertFalse(result["generated_scoring"])
            self.assertNotIn("held-001", json.dumps(result))
            self.assertNotIn("private-test-diagnostic", json.dumps(result))
            for name, digest in result["logs"].items():
                self.assertEqual(digest, evaluation.sha((root / "run" / name).read_bytes()))
            raw = json.loads((root / "run/held-out/controls.json").read_bytes())
            self.assertEqual(raw["tasks"][0]["task"], "held-001")
            self.assertTrue(raw["tasks"][0]["valid"])

    def test_baseexception_propagates_without_publishing_a_result(self):
        with tempfile.TemporaryDirectory() as temporary, descriptor(Path(temporary)) as path:
            root = Path(temporary)
            with mock.patch.object(evaluation, "projection", return_value={"scripts/example.py": b"base"}), \
                    mock.patch.object(evaluation, "grade_case", side_effect=KeyboardInterrupt):
                with self.assertRaises(KeyboardInterrupt):
                    suite.held_out_control(path, "held-001", "base", root / "out")
            self.assertFalse((root / "out/report.json").exists())


if __name__ == "__main__":
    unittest.main()
