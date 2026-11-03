"""Check real frozen Rust controls, input preservation and refused generated code."""
from __future__ import annotations

import ast
import json
from pathlib import Path
import subprocess
import tempfile
import unittest
from unittest import mock

from scripts import coding_eval as evaluation
from scripts import coding_eval_rust as rust


class RustCodingEvaluationTests(unittest.TestCase):
    def setUp(self):
        self.temporary = tempfile.TemporaryDirectory(prefix="agentmage-rust-evaluation-test-")
        self.addCleanup(self.temporary.cleanup)
        self.root = Path(self.temporary.name)
        self.task = rust.TASKS[0]

    def test_accepted_python_definitions_and_new_rust_task_are_frozen(self):
        previous = ast.parse(evaluation.git(
            "show", "054d87c75904294b4496b57d15e9271e285082ca:scripts/coding_eval.py"))
        definitions = {target.id: ast.literal_eval(node.value)
                       for node in previous.body if isinstance(node, ast.Assign)
                       for target in node.targets if isinstance(target, ast.Name)
                       and target.id in {"TASKS", "ADDITIONAL_TASKS"}}
        self.assertEqual(evaluation.TASKS, definitions["TASKS"] + definitions["ADDITIONAL_TASKS"])
        self.assertEqual(len(evaluation.collection()), 30)
        self.assertEqual(sum(t["expected_tests"] for t in evaluation.collection()), 89)
        self.assertEqual({t["split"] for t in evaluation.collection()}, {"development"})
        digest = evaluation.sha(json.dumps(evaluation.collection(), sort_keys=True, separators=(",", ":")).encode())
        self.assertEqual(digest, "91f1eecfa91129f2f8392363aaff0e8f49243aa761947e69d5ec1452d7b4b04d")

    def test_prepare_withholds_hidden_regression_corpus_and_git_history(self):
        files = evaluation.projection(self.task)
        self.assertEqual(len(files), 34)
        self.assertEqual(sum(map(len, files.values())), 501816)
        self.assertIn("kernel/contracts/src/path.rs", files)
        self.assertNotIn(self.task["hidden_test"], files)
        self.assertNotIn(self.task["fixture"], files)
        self.assertFalse(any(".git" in Path(p).parts or "tests" in Path(p).parts for p in files))
        self.assertEqual(files["Cargo.lock"], evaluation.git("show", self.task["base"] + ":Cargo.lock"))
        candidate = self.root / "candidate"
        result = subprocess.run([
            "/usr/bin/python3", str(evaluation.ROOT / "scripts/coding_eval.py"), "prepare",
            "--task", self.task["id"], "--output", str(candidate),
        ], capture_output=True, timeout=30)
        self.assertEqual(result.returncode, 0, result.stderr)
        self.assertEqual(evaluation.read_candidate(candidate, files, self.task["source"]), files)

    def test_grading_materialization_preserves_manifests_and_every_hidden_byte(self):
        workspace = self.root / "workspace"
        files = evaluation.projection(self.task)
        for name, data in files.items():
            evaluation.write_file(workspace / name, data)
        rust.materialize(workspace, self.task, evaluation)
        for name, data in files.items():
            self.assertEqual((workspace / name).read_bytes(), data, name)
        self.assertEqual((workspace / "kernel/contracts/tests/eval_hidden.rs").read_bytes(),
                         evaluation.pinned_blob(self.task["hidden_revision"], self.task["hidden_test"],
                                                self.task["hidden_sha256"]))
        self.assertEqual((workspace / self.task["fixture"]).read_bytes(), evaluation.pinned_blob(
            self.task["hidden_revision"], self.task["fixture"], self.task["fixture_sha256"]))
        self.assertFalse((workspace / ".git").exists())

    def test_rust_generated_and_altered_task_definitions_are_refused_before_launch(self):
        original = evaluation.projection(self.task)
        forged = dict(original)
        # A malicious Rust program can forge a textual test result or call exit(0).
        # Parent-side admission must refuse its bytes before mounting any grader.
        forged[self.task["source"]] += b'\nfn forge() { println!("test result: ok. 1 passed; 0 failed;"); std::process::exit(0); }\n'
        output = self.root / "refused"
        with mock.patch.object(evaluation, "grade_case") as launch:
            with self.assertRaisesRegex(evaluation.EvaluationError, "untrusted-candidate-disabled"):
                evaluation.grade(self.task, forged, output)
            with self.assertRaisesRegex(evaluation.EvaluationError, "task-definition-refused"):
                evaluation.grade({**self.task, "expected_tests": 0}, original, output)
        launch.assert_not_called()
        self.assertFalse(output.exists())

    def test_rust_projection_integrity_refuses_changes(self):
        original_git = evaluation.git

        def corrupted(*args):
            data = original_git(*args)
            if args == ("show", self.task["base"] + ":kernel/contracts/src/path.rs"):
                return data + b"\n// changed frozen input\n"
            return data

        with mock.patch.object(evaluation, "git", side_effect=corrupted):
            with self.assertRaisesRegex(evaluation.InputIntegrityError, "projection-hash-mismatch"):
                evaluation.projection(self.task)

    def test_corpus_integrity_refusal_has_no_counts_and_is_not_a_base_failure(self):
        files = evaluation.projection(self.task)
        original_git = evaluation.git

        def corrupted(*args):
            data = original_git(*args)
            if args == ("show", self.task["hidden_revision"] + ":" + self.task["fixture"]):
                return data + b"\n"
            return data

        with mock.patch.object(evaluation, "git", side_effect=corrupted), \
                mock.patch.object(evaluation.subprocess, "Popen", wraps=subprocess.Popen) as launch:
            # Git uses Popen too; distinguish it from the actual worker invocation.
            report = evaluation.grade(self.task, files, self.root / "corrupt-corpus")
        self.assertFalse(report["passed"])
        self.assertEqual(report["cause"], "input_integrity")
        self.assertIsNone(report["tests"])
        self.assertIsNone(report["exit_code"])
        self.assertFalse(any(call.args[0][0] == "/usr/bin/bwrap" for call in launch.call_args_list))

    def test_rust_mounts_only_runtime_registry_and_immutable_inputs(self):
        toolchain, registry = self.root / "toolchain", self.root / "registry"
        with mock.patch.object(rust, "runtime_paths", return_value=(toolchain, registry)):
            command = rust.sandbox_command(Path("/snapshot"), Path("/grader"), Path("/result"), evaluation)
        self.assertIn("--unshare-all", command)
        self.assertIn("--unshare-user", command)
        self.assertIn("--disable-userns", command)
        self.assertIn("--clearenv", command)
        self.assertIn("--cap-drop", command)
        self.assertIn("ALL", command)
        binds = [command[i + 1:i + 3] for i, v in enumerate(command) if v == "--ro-bind"]
        self.assertIn([str(registry), "/cargo/registry"], binds)
        self.assertIn([str(toolchain), "/toolchain"], binds)
        self.assertIn(["/snapshot", "/workspace"], binds)
        self.assertNotIn(str(Path.home()), command)
        self.assertIn('"--locked", "--offline", "--jobs", "2"', rust.DRIVER.decode())
        self.assertIn('"--test-threads=2"', rust.DRIVER.decode())

    def test_rust_out_of_scope_manifest_edits_and_added_tests_are_refused(self):
        files = evaluation.projection(self.task)
        candidate = self.root / "scope"
        candidate.mkdir()
        for name, data in files.items():
            evaluation.write_file(candidate / name, data)
        path = candidate / "Cargo.toml"
        path.write_bytes(files["Cargo.toml"] + b"\n# edited\n")
        with self.assertRaisesRegex(evaluation.EvaluationError, "out-of-scope-edit"):
            evaluation.read_candidate(candidate, files, self.task["source"])
        path.write_bytes(files["Cargo.toml"])
        evaluation.write_file(candidate / "kernel/contracts/tests/forged.rs", b"// forbidden\n")
        with self.assertRaises(evaluation.EvaluationError):
            evaluation.read_candidate(candidate, files, self.task["source"])

    def test_compiler_limits_narrow_cpu_affinity_without_raising_memory_or_file_caps(self):
        with mock.patch.object(rust.os, "sched_getaffinity", return_value={1, 3, 5, 7, 9, 11}), \
                mock.patch.object(rust.os, "sched_setaffinity") as affinity, \
                mock.patch.object(rust.resource, "setrlimit") as limits:
            rust.limits()
        affinity.assert_called_once_with(0, [1, 3, 5, 7])
        self.assertEqual(limits.call_args_list, [
            mock.call(rust.resource.RLIMIT_AS, (2 * 1024**3, 2 * 1024**3)),
            mock.call(rust.resource.RLIMIT_FSIZE, (128 * 1024**2, 128 * 1024**2)),
            mock.call(rust.resource.RLIMIT_CPU, (60, 60)),
            mock.call(rust.resource.RLIMIT_CORE, (0, 0)),
        ])

    def test_real_rust_cli_base_fails_reference_passes_and_generated_is_refused(self):
        original = evaluation.projection(self.task)
        candidate = self.root / "real-candidate"
        candidate.mkdir()
        for name, data in original.items():
            evaluation.write_file(candidate / name, data)
        source = candidate / self.task["source"]
        script = str(evaluation.ROOT / "scripts/coding_eval.py")
        for kind, expected_exit in (("base", 1), ("reference", 0)):
            if kind == "reference":
                source.write_bytes(evaluation.pinned_blob(self.task["reference"], self.task["source"],
                                                          self.task["reference_sha256"]))
            output = self.root / kind
            result = subprocess.run([
                "/usr/bin/python3", script, "score", "--task", self.task["id"],
                "--candidate", str(candidate), "--output", str(output),
            ], capture_output=True, timeout=120)
            self.assertEqual(result.returncode, expected_exit, result.stderr)
            report = json.loads((output / "report.json").read_text())
            self.assertIsNotNone(report["tests"], report)
            self.assertEqual(report["tests"]["tests"], 1)
            self.assertEqual(report["tests"]["failures"], 1 if kind == "base" else 0)
            self.assertEqual(report["cause"], "test_failure" if kind == "base" else "none")
            self.assertEqual(report["candidate_kind"], "frozen-" + kind)
            self.assertEqual(report["language"], "rust")
            self.assertEqual(report["fixture_sha256"], self.task["fixture_sha256"])
            self.assertEqual(report["rust_adapter_sha256"], evaluation.sha(Path(rust.__file__).read_bytes()))
            self.assertFalse(report["output_truncated"])
            self.assertEqual(report["model_attempts"], 0)
            self.assertIsNone(report["model_score"])
            self.assertIn(b"generated_path_corpus_fails_closed", (output / "stdout.log").read_bytes())
        source.write_bytes(source.read_bytes() + b"\n// not a frozen control\n")
        output = self.root / "generated"
        result = subprocess.run([
            "/usr/bin/python3", script, "score", "--task", self.task["id"],
            "--candidate", str(candidate), "--output", str(output),
        ], capture_output=True, timeout=30)
        self.assertEqual(result.returncode, 2)
        self.assertIn(b"evaluation.untrusted-candidate-disabled", result.stderr)
        self.assertFalse(output.exists())


if __name__ == "__main__":
    unittest.main()
