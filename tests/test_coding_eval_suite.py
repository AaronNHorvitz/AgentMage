"""Exercise suite commitments and refusal boundaries without held-out feedback."""
from __future__ import annotations

from contextlib import contextmanager
import json
import os
from pathlib import Path
import subprocess
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
    def test_coverage_discloses_unsupported_tasks_and_binds_original_selections(self):
        with mock.patch.object(evaluation, "pinned_blob") as load, \
                mock.patch.object(evaluation, "grade_case") as run:
            manifest = suite.development_coverage()
        load.assert_not_called()
        run.assert_not_called()
        tasks = {task["id"]: task for task in suite.development_tasks()}
        rows = manifest["tasks"]
        self.assertEqual({row["task"] for row in rows}, set(tasks))
        self.assertEqual(len(rows), 30)
        self.assertEqual(sum(row["expected_tests"] for row in rows), 89)
        for row in rows:
            task = tasks[row["task"]]
            self.assertEqual(row["task_sha256"], suite.canonical_digest(task))
            self.assertEqual(row["hidden_sha256"], task["hidden_sha256"])
            self.assertEqual(row["method_selection"], task.get("test_names", "complete-original-module"))
            self.assertFalse(row["generated_scoring"])
            if task["id"] in suite.oracle.SUPPORTED:
                self.assertEqual(row["independent_adapter"], suite.oracle.coverage(task))
            else:
                self.assertIsNone(row["independent_adapter"])
                self.assertIn("independent-result-integrity", row["unestablished"])
        self.assertEqual([row["task"] for row in rows if row["known_races"]], ["dev-003"])
        self.assertEqual(manifest["attempt_policy"]["automatic_control_retries"], 0)
        self.assertFalse(manifest["attempt_policy"]["infrastructure_excluded_from_denominator"])
        self.assertFalse(manifest["attempt_policy"]["known_race_failure_is_pass"])

    def test_private_storage_refuses_repository_worktrees_and_configured_workspaces(self):
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            repository, worktree, workspace = (root / name for name in ("repository", "worktree", "workspace"))
            for path in (repository, worktree, workspace):
                path.mkdir()
            alias = root / "alias"
            alias.symlink_to(worktree, target_is_directory=True)
            discovery = b"worktree " + os.fsencode(worktree) + b"\0HEAD " + b"a" * 40 + b"\0\0"
            with mock.patch.object(evaluation, "ROOT", repository), \
                    mock.patch.object(evaluation, "git", return_value=discovery):
                for directory in (repository, worktree, workspace, alias):
                    with self.subTest(directory=directory.name):
                        with self.assertRaisesRegex(evaluation.InputIntegrityError, "private-location-exposed"), \
                                mock.patch.object(suite.os, "open") as opened:
                            suite.private_bytes(directory / "secret.json", (workspace,))
                        opened.assert_not_called()
                        with self.assertRaisesRegex(evaluation.InputIntegrityError, "private-location-exposed"), \
                                mock.patch.object(suite, "check_inputs") as check, \
                                mock.patch.object(evaluation, "new_directory") as mkdir:
                            suite.verify(root / "secret.json", directory / "new-output", "row-close", (workspace,))
                        check.assert_not_called()
                        mkdir.assert_not_called()
                        with self.assertRaisesRegex(evaluation.InputIntegrityError, "private-location-exposed"), \
                                mock.patch.object(suite, "held_out_tasks") as tasks:
                            suite.held_out_control(root / "secret.json", "held-001", "base",
                                                   directory / "new-control", (workspace,))
                        tasks.assert_not_called()
                suite.private_location(root / "private-output", (workspace,))
                # Resolve an ancestor alias even when the final output does not exist.
                self.assertFalse((alias / "new-output").exists())

    def test_private_storage_discovery_fails_closed(self):
        for raw in (b"", b"worktree relative\0", b"HEAD aaaa\0"):
            with self.subTest(raw=raw), mock.patch.object(evaluation, "git", return_value=raw):
                with self.assertRaisesRegex(evaluation.InputIntegrityError, "worktree-discovery-refused"):
                    suite.private_location(Path("unused"))

    def test_unresolvable_private_locations_and_workspace_roots_refuse_without_private_error_text(self):
        with tempfile.TemporaryDirectory() as temporary:
            path = Path(temporary) / "private-name"
            path.symlink_to(path)
            with self.assertRaisesRegex(evaluation.InputIntegrityError, "suite.private-location-refused") as refusal:
                suite.private_location(path)
            self.assertNotIn(str(path), str(refusal.exception))
            with self.assertRaisesRegex(evaluation.InputIntegrityError, "suite.worktree-discovery-refused"):
                suite.editing_roots((path,))

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

    def test_repeated_held_out_change_and_wrong_total_method_count_refuse(self):
        task = synthetic_task()
        data = json.dumps({"version": "coding-held-out-v1", "origin": "original-repository-history",
                           "tasks": [task, {**task, "id": "held-002"}]}).encode()
        with tempfile.TemporaryDirectory() as temporary, descriptor(Path(temporary), raw=data) as path, \
                mock.patch.object(suite, "HELD_OUT_COUNT", 2), mock.patch.object(suite, "HELD_OUT_METHODS", 2):
            with self.assertRaisesRegex(evaluation.InputIntegrityError, "held-out-identity"):
                suite.held_out_tasks(path)
        with tempfile.TemporaryDirectory() as temporary, descriptor(Path(temporary)) as path, \
                mock.patch.object(suite, "HELD_OUT_METHODS", 2):
            with self.assertRaisesRegex(evaluation.InputIntegrityError, "held-out-identity"):
                suite.held_out_tasks(path)

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

    def test_development_pairs_require_complete_discriminating_controls_even_if_legacy_summary_passes(self):
        base = {"passed": False, "cause": "test_failure", "exit_code": 1, "tests": counts(failures=1)}
        reference = {"passed": True, "cause": "none", "tests": counts()}
        row = {"task": "dev-synthetic", "base": base, "reference": reference}
        tasks = [{"id": "dev-synthetic", "expected_tests": 1}]
        with mock.patch.object(suite, "development_tasks", return_value=tasks):
            self.assertTrue(suite.development_controls_valid({"passed": True, "tasks": [row]}))
            self.assertFalse(suite.development_controls_valid({"passed": False, "tasks": [row]}))
            for rows in ([], [row, row], [{**row, "task": "unregistered"}],
                         [{**row, "base": {**base, "exit_code": 7}}],
                         [{**row, "base": {**base, "tests": {**counts(failures=1), "skipped": 1}}}],
                         [{**row, "base": {**base, "tests": {**counts(failures=1), "tests": 2}}}],
                         [{**row, "reference": {**reference, "tests": {**counts(), "tests": 2}}}]):
                with self.subTest(rows=rows):
                    self.assertFalse(suite.development_controls_valid({"passed": True, "tasks": rows}))

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
                    mock.patch.object(suite, "editing_roots", return_value={evaluation.ROOT}), \
                    mock.patch.object(evaluation, "git", return_value=("a" * 40 + "\n").encode()), \
                    mock.patch.object(suite, "is_ancestor", return_value=False), \
                    mock.patch.object(evaluation, "pinned_blob") as pinned, \
                    mock.patch.object(evaluation, "grade_case") as worker:
                result = suite.check_inputs(path)
            self.assertTrue(result["passed"])
            self.assertEqual(result["held_out_test_methods_run"], 0)
            self.assertEqual(pinned.call_count, 4)
            worker.assert_not_called()

    def test_parent_ancestry_and_exposed_hidden_inputs_refuse_without_execution(self):
        task = synthetic_task()
        dev = {**task, "id": "dev-synthetic", "base": "f" * 40, "reference": "e" * 40,
               "source": "scripts/other.py", "hidden_revision": "d" * 40}
        for reason in ("held-out-parent", "split-ancestry-leakage", "hidden-input-exposed"):
            with self.subTest(reason=reason), tempfile.TemporaryDirectory() as temporary, \
                    descriptor(Path(temporary)) as path:
                projection = {"scripts/example.py": b"base"}
                if reason == "hidden-input-exposed":
                    projection[task["hidden_test"]] = b"hidden-test-fixture-not-executed"
                with mock.patch.object(suite, "development_tasks", return_value=[dev]), \
                        mock.patch.object(suite, "editing_roots", return_value={evaluation.ROOT}), \
                        mock.patch.object(evaluation, "git", return_value=(
                            ("f" if reason == "held-out-parent" else "a") * 40 + "\n").encode()), \
                        mock.patch.object(suite, "is_ancestor", return_value=reason == "split-ancestry-leakage"), \
                        mock.patch.object(evaluation, "projection", return_value=projection), \
                        mock.patch.object(evaluation, "pinned_blob"), \
                        mock.patch.object(evaluation, "grade_case") as worker:
                    with self.assertRaisesRegex(evaluation.InputIntegrityError, reason):
                        suite.check_inputs(path)
                worker.assert_not_called()

    def test_ancestry_checks_each_development_base_and_hidden_revision(self):
        task = synthetic_task()
        dev = {**task, "id": "dev-synthetic", "base": "f" * 40, "reference": "e" * 40,
               "source": "scripts/other.py", "hidden_revision": "d" * 40}
        with tempfile.TemporaryDirectory() as temporary, descriptor(Path(temporary)) as path, \
                mock.patch.object(suite, "development_tasks", return_value=[dev]), \
                mock.patch.object(suite, "editing_roots", return_value={evaluation.ROOT}), \
                mock.patch.object(evaluation, "git", return_value=("a" * 40 + "\n").encode()), \
                mock.patch.object(evaluation, "projection", return_value={"scripts/example.py": b"base"}), \
                mock.patch.object(evaluation, "pinned_blob"), \
                mock.patch.object(suite, "is_ancestor", return_value=False) as ancestor:
            self.assertTrue(suite.check_inputs(path)["passed"])
            self.assertEqual(ancestor.call_args_list,
                             [mock.call(task["reference"], revision) for revision in ("d" * 40, "f" * 40)])

    def test_real_git_ancestry_distinguishes_descendants_without_inspecting_tests(self):
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            env = {"PATH": "/usr/bin", "HOME": str(root), "GIT_CONFIG_NOSYSTEM": "1",
                   "GIT_CONFIG_GLOBAL": "/dev/null", "GIT_AUTHOR_NAME": "Synthetic",
                   "GIT_AUTHOR_EMAIL": "synthetic@example.invalid", "GIT_COMMITTER_NAME": "Synthetic",
                   "GIT_COMMITTER_EMAIL": "synthetic@example.invalid"}

            def git(*args):
                return subprocess.check_output(["/usr/bin/git", *args], cwd=root, env=env,
                                               stderr=subprocess.DEVNULL, timeout=10).strip().decode()

            git("init", "--quiet", "--template=", "--initial-branch=synthetic")
            git("-c", "core.hooksPath=/dev/null", "commit", "--quiet", "--allow-empty", "-m", "First")
            first = git("rev-parse", "HEAD")
            git("-c", "core.hooksPath=/dev/null", "commit", "--quiet", "--allow-empty", "-m", "Second")
            second = git("rev-parse", "HEAD")
            with mock.patch.object(evaluation, "ROOT", root):
                self.assertTrue(suite.is_ancestor(first, second))
                self.assertTrue(suite.is_ancestor(second, second))
                self.assertFalse(suite.is_ancestor(second, first))
                with self.assertRaisesRegex(evaluation.InputIntegrityError, "ancestry-unavailable"):
                    suite.is_ancestor("f" * 40, second)

    def test_complete_orchestration_binds_raw_controls_but_printable_summary_has_no_task_feedback(self):
        with tempfile.TemporaryDirectory() as temporary, descriptor(Path(temporary)) as path:
            root = Path(temporary)
            flaky = False

            def verify_existing(output):
                evaluation.new_directory(output)
                base = {"passed": False, "cause": "test_failure", "exit_code": 1, "tests": counts(failures=1)}
                reference = {"passed": True, "cause": "none", "tests": counts()}
                result = {"passed": True, "input_refused": False,
                          "tasks": [{"task": "synthetic", "base": base, "reference": reference}]}
                evaluation.publish(output / "summary.json", result)
                return result

            def grade_case(task, files, output):
                evaluation.write_file(output / "stdout.log", b"private-test-diagnostic")
                evaluation.write_file(output / "stderr.log", b"")
                fixed = files["scripts/example.py"] == b"reference"
                if fixed and flaky:
                    return None, False, False, None
                return (0 if fixed else 1), False, False, counts(failures=0 if fixed else 1)

            with mock.patch.object(suite, "check_inputs", return_value={"model_attempts": 0, "model_score": None,
                                                                        "generated_scoring": False}), \
                    mock.patch.object(evaluation, "verify", side_effect=verify_existing), \
                    mock.patch.object(suite.oracle, "verify", side_effect=verify_existing), \
                    mock.patch.object(suite, "development_tasks", return_value=[{**synthetic_task(), "id": "synthetic"}]), \
                    mock.patch.object(evaluation, "projection", return_value={"scripts/example.py": b"base"}), \
                    mock.patch.object(evaluation, "pinned_blob", return_value=b"reference"), \
                    mock.patch.object(evaluation, "grade_case", side_effect=grade_case) as worker:
                result = suite.verify(path, root / "run", "row-close")
                with self.assertRaises(FileExistsError):
                    suite.verify(path, root / "run", "row-close")
                self.assertEqual(worker.call_count, 2)
                flaky = True
                failed = suite.verify(path, root / "flaky", "row-close")
                self.assertEqual(worker.call_count, 4, "failed reference must never retry itself")
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
            self.assertFalse(failed["passed"])
            self.assertFalse(failed["held_out_controls_valid"])
            self.assertEqual(failed["control_attempts"], 6)
            failed_controls = json.loads((root / "flaky/held-out/controls.json").read_bytes())
            self.assertEqual(failed_controls["tasks"][0]["reference"]["cause"], "infrastructure")
            self.assertIsNone(failed_controls["tasks"][0]["reference"]["tests"])
            self.assertEqual((root / "flaky/held-out/held-001-reference/stdout.log").read_bytes(),
                             b"private-test-diagnostic")

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
