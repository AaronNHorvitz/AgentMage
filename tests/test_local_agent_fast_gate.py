"""Exercise change discovery, coverage selection and gate failure behavior."""

from contextlib import redirect_stdout, redirect_stderr
import io
import os
from pathlib import Path
import subprocess
import sys
import tempfile
import time
import unittest
from unittest.mock import patch

from scripts import local_agent_fast_gate as gate


class FastGateTests(unittest.TestCase):
    def setUp(self):
        self.temp = tempfile.TemporaryDirectory(prefix="agentmage-gate-")
        self.addCleanup(self.temp.cleanup)
        self.root = Path(self.temp.name)
        self.metadata = {"packages": [
            self.package("foundation", "kernel/base", []),
            self.package("runtime", "kernel/runtime", ["foundation"]),
            self.package("client", "shells/client", ["runtime"]),
            self.package("other", "capabilities/other", []),
        ]}

    def package(self, name, directory, dependencies):
        return {"name": name, "manifest_path": str(self.root / directory / "Cargo.toml"),
                "dependencies": [{"name": dep} for dep in dependencies]}

    def write(self, name, content="test\n"):
        path = self.root / name
        path.parent.mkdir(parents=True, exist_ok=True)
        path.write_text(content)
        return path

    def git(self, *args):
        return subprocess.check_output(["git", *args], cwd=self.root, stderr=subprocess.DEVNULL)

    def test_changes_include_committed_staged_unstaged_untracked_and_deleted(self):
        self.git("init", "-q")
        for name in ("committed.md", "staged.md", "working.md", "deleted.md"):
            self.write(name)
        self.git("add", ".")
        self.git("-c", "user.name=Synthetic", "-c", "user.email=synthetic@example.invalid",
                 "commit", "-qm", "base")
        base = self.git("rev-parse", "HEAD").decode().strip()
        self.write("committed.md", "changed\n")
        self.git("add", "committed.md")
        self.git("-c", "user.name=Synthetic", "-c", "user.email=synthetic@example.invalid",
                 "commit", "-qm", "increment")
        self.write("staged.md", "changed\n")
        self.git("add", "staged.md")
        self.write("working.md", "changed\n")
        (self.root / "deleted.md").unlink()
        self.write("space and\nnewline.md")
        self.write(".gitignore", "ignored\n")
        self.write("ignored")
        self.assertEqual(set(gate.changed_paths(self.root, base)), {
            ".gitignore", "committed.md", "staged.md", "working.md", "deleted.md", "space and\nnewline.md",
        })

    def test_base_cannot_be_a_git_option(self):
        self.git("init", "-q")
        with self.assertRaises(subprocess.CalledProcessError):
            gate.changed_paths(self.root, "--help")

    def test_reverse_dependencies_are_included_and_unrelated_crate_is_not(self):
        self.assertEqual(gate.affected_packages(self.metadata, ["kernel/base/src/lib.rs"], self.root),
                         ["client", "foundation", "runtime"])
        self.assertEqual(gate.affected_packages(self.metadata, ["shells/client/src/main.rs"], self.root),
                         ["client"])

    def test_workspace_configuration_requires_every_crate(self):
        for name in ("Cargo.toml", "Cargo.lock", "rust-toolchain.toml", "rustfmt.toml"):
            with self.subTest(name=name):
                self.assertEqual(gate.affected_packages(self.metadata, [name], self.root),
                                 ["client", "foundation", "other", "runtime"])

    def test_rust_plan_runs_all_targets_without_filters_or_ignores(self):
        commands = gate.plan(self.root, ["kernel/runtime/src/lib.rs"], self.metadata)
        test = next(c for c in commands if c[:2] == ["cargo", "test"])
        self.assertEqual(test, ["cargo", "test", "--locked", "--offline", "-p", "client", "-p", "runtime",
                                "--all-targets", "--no-fail-fast"])
        clippy = next(c for c in commands if c[:2] == ["cargo", "clippy"])
        self.assertEqual(clippy[-3:], ["--", "-D", "warnings"])

    def test_document_only_change_has_no_build_or_evidence_generator(self):
        self.write("docs/mission.md")
        commands = gate.plan(self.root, ["docs/mission.md"], self.metadata)
        self.assertEqual([c[0] for c in commands], ["git", "node"])
        self.assertEqual(commands[1][-1], "./docs/mission.md")

    def test_unknown_change_and_missing_tests_fail_closed(self):
        for name in ("web/app.js", "architecture/config.json", "scripts/new_tool.py"):
            with self.subTest(name=name), self.assertRaises(ValueError):
                gate.plan(self.root, [name], self.metadata)

    def test_script_and_test_changes_select_one_test_module(self):
        self.write("tests/test_example.py", "import unittest\n")
        commands = gate.plan(self.root, ["scripts/example.py", "tests/test_example.py"], self.metadata)
        self.assertEqual(commands[-1], [sys.executable, "-m", "unittest", "tests.test_example"])

    def test_python_lint_checks_syntax_indentation_and_whitespace(self):
        for source in ("x =\n", "x = 1 \n", "x = 1"):
            self.write("scripts/example.py", source)
            with self.subTest(source=source), self.assertRaises((SyntaxError, ValueError)):
                gate.python_lint(self.root, ["scripts/example.py"])
        self.write("scripts/example.py", "value = 1\n")
        gate.python_lint(self.root, ["scripts/example.py"])

    def test_failure_is_retained_even_when_later_check_passes(self):
        output = io.StringIO()
        with redirect_stdout(output):
            result = gate.execute(self.root, [[sys.executable, "-c", "raise SystemExit(7)"],
                                              [sys.executable, "-c", "pass"]], 10)
        self.assertEqual(result, 1)
        self.assertIn("exit=7", output.getvalue())
        self.assertIn("FAST GATE: FAIL", output.getvalue())

    def test_execution_limits_build_and_test_concurrency(self):
        script = "import os; from pathlib import Path; Path('limits').write_text(os.environ['CARGO_BUILD_JOBS']+','+os.environ['RUST_TEST_THREADS'])"
        with redirect_stdout(io.StringIO()), patch.dict(os.environ, {"CARGO_BUILD_JOBS": "100"}):
            self.assertEqual(gate.execute(self.root, [[sys.executable, "-c", script]], 10), 0)
        self.assertEqual((self.root / "limits").read_text(), "2,4")

    def test_missing_executable_cannot_pass(self):
        with redirect_stdout(io.StringIO()), redirect_stderr(io.StringIO()):
            self.assertEqual(gate.execute(self.root, [["/nonexistent/agentmage-check"]], 10), 1)

    def test_timeout_fails_and_terminates_owned_children(self):
        pidfile = self.root / "child-pid"
        script = (
            "import subprocess,sys,time; from pathlib import Path; "
            "child=subprocess.Popen([sys.executable,'-c',"
            "'import signal,time; signal.signal(signal.SIGTERM,signal.SIG_IGN); time.sleep(60)']); "
            "Path('child-pid').write_text(str(child.pid)); time.sleep(60)"
        )
        with redirect_stdout(io.StringIO()), redirect_stderr(io.StringIO()):
            self.assertEqual(gate.execute(self.root, [[sys.executable, "-c", script]], 1), 1)
        pid = int(pidfile.read_text())
        stat = Path(f"/proc/{pid}/stat")
        deadline = time.monotonic() + 1
        while stat.exists() and stat.read_text().rsplit(") ", 1)[1].split()[0] not in {"Z", "X"}:
            if time.monotonic() >= deadline:
                self.fail("timed-out check left its descendant running")
            time.sleep(0.01)
        if stat.exists():
            self.assertIn(stat.read_text().rsplit(") ", 1)[1].split()[0], {"Z", "X"})

    def test_empty_change_cannot_claim_green(self):
        with patch.object(gate, "changed_paths", return_value=[]), patch.object(sys, "argv", ["gate"]), \
                redirect_stderr(io.StringIO()):
            self.assertEqual(gate.main(), 1)


if __name__ == "__main__":
    unittest.main()
