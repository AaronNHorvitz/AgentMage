"""Exercise change discovery, coverage selection and gate failure behavior."""

from contextlib import redirect_stdout, redirect_stderr
import fcntl
import io
import os
from pathlib import Path
import signal
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
        self.assertEqual([c[0] for c in commands], ["git", sys.executable, "node"])
        self.assertEqual(commands[2][-1], "./docs/mission.md")

    def test_unknown_change_and_missing_tests_fail_closed(self):
        for name in ("web/app.js", "architecture/config.json", "scripts/new_tool.py",
                     "shells/client/assets/app.js", "shells/client/web/index.html",
                     "shells/client/web/style.css", "shells/client/unknown"):
            with self.subTest(name=name), self.assertRaises(ValueError):
                gate.plan(self.root, [name], self.metadata)

    def test_crate_data_inputs_keep_full_rust_coverage(self):
        for suffix in (".rs", ".toml", ".json", ".sql", ".txt"):
            with self.subTest(suffix=suffix):
                commands = gate.plan(self.root, ["shells/client/input" + suffix], self.metadata)
                self.assertIn("--all-targets", commands[-1])
        metadata = {"packages": [self.package("contracts", "kernel/contracts", [])]}
        commands = gate.plan(self.root, ["kernel/contracts/LICENSE"], metadata)
        self.assertIn("--all-targets", commands[-1])

    def test_untracked_whitespace_uses_git_rules_without_staging(self):
        self.git("init", "-q")
        for name in ("valid.txt", "space and\nnewline.txt"):
            self.write(name, "valid\n")
        self.write("binary.bin", b"\0 \n".decode())
        gate.untracked_whitespace(self.root)
        self.assertEqual(self.git("diff", "--cached", "--name-only"), b"")
        for name, content in (("valid.txt", "bad \n"), ("space and\nnewline.txt", "bad\t\n")):
            with self.subTest(name=name):
                self.write(name, content)
                with self.assertRaises(ValueError):
                    gate.untracked_whitespace(self.root)
                self.write(name, "valid\n")

    def test_text_review_has_its_own_format_check_without_editing_it(self):
        name = "docs/reviews/exact-review.txt"
        source = "Commit: " + "a" * 40 + "\nScope: synthetic-review\n\nOriginal report\n\nVerdict: FINDINGS\n"
        report = self.write(name, source)
        commands = gate.plan(self.root, [name], self.metadata)
        self.assertEqual(commands[-1], [sys.executable, "scripts/local_agent_fast_gate.py", "--lint-reviews", name])
        gate.review_text_lint(self.root, [name])
        self.assertEqual(report.read_text(), source)
        for invalid in (source.rstrip(), source.replace("a" * 40, "short"),
                        source.replace("Scope: synthetic-review", "Scope: "),
                        source.replace("Verdict: FINDINGS", "Verdict: unknown")):
            with self.subTest(invalid=invalid), self.assertRaises(ValueError):
                report.write_text(invalid)
                gate.review_text_lint(self.root, [name])
        for unsupported in ("docs/evidence/unknown.txt", "other/unknown.txt"):
            with self.subTest(unsupported=unsupported), self.assertRaises(ValueError):
                gate.plan(self.root, [unsupported], self.metadata)

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

    def assert_process_stopped(self, pid):
        stat = Path(f"/proc/{pid}/stat")
        deadline = time.monotonic() + 8
        while True:
            try:
                state = stat.read_text().rsplit(") ", 1)[1].split()[0]
            except FileNotFoundError:
                return
            if state in {"Z", "X"}:
                return
            if time.monotonic() >= deadline:
                self.fail(f"owned process {pid} still running")
            time.sleep(0.01)

    def check_tree(self, finish_normally=False, next_check=False):
        """A check and TERM-ignoring child share an inherited reservation lock."""
        child = (
            "import signal,time; from pathlib import Path; "
            "signal.signal(signal.SIGTERM,signal.SIG_IGN); "
            "Path('ready').write_text('ready'); time.sleep(60)"
        )
        check = (
            "import fcntl,os,subprocess,sys,time; from pathlib import Path\n"
            "lock=open('reservation','w')\n"
            "fcntl.flock(lock,fcntl.LOCK_EX)\n"
            f"child=subprocess.Popen([sys.executable,'-c',{child!r}],pass_fds=(lock.fileno(),))\n"
            "while not Path('ready').exists(): time.sleep(0.01)\n"
            "Path('pids').write_text(str(os.getpid())+','+str(child.pid))\n"
            + ("raise SystemExit(0)\n" if finish_normally else "time.sleep(60)\n")
        )
        commands = [[sys.executable, "-c", check]]
        if next_check:
            commands.append([sys.executable, "-c", "from pathlib import Path; Path('next-check').touch()"])
        script = (
            f"import sys; sys.path.insert(0,{str(gate.ROOT)!r}); "
            "from pathlib import Path; from scripts import local_agent_fast_gate as gate; "
            f"raise SystemExit(gate.execute(Path.cwd(),{commands!r},60))"
        )
        process = subprocess.Popen([sys.executable, "-c", script], cwd=self.root,
                                   start_new_session=True, stdout=subprocess.DEVNULL,
                                   stderr=subprocess.DEVNULL)

        owned_pidfds = []

        def cleanup():
            if process.poll() is None:
                os.killpg(process.pid, signal.SIGKILL)
            process.wait()
            for descriptor in owned_pidfds:
                try:
                    signal.pidfd_send_signal(descriptor, signal.SIGKILL)
                except ProcessLookupError:
                    pass
                finally:
                    os.close(descriptor)

        self.addCleanup(cleanup)
        deadline = time.monotonic() + 10
        while not (self.root / "pids").exists():
            if process.poll() is not None or time.monotonic() >= deadline:
                self.fail("check did not start")
            time.sleep(0.01)
        pids = list(map(int, (self.root / "pids").read_text().split(",")))
        for pid in pids:
            try:
                owned_pidfds.append(os.pidfd_open(pid))
            except ProcessLookupError:
                pass
        # The supervisor is distinct from the gate and check groups.
        supervisor = int(Path(f"/proc/{pids[0]}/stat").read_text().rsplit(") ", 1)[1].split()[1]) \
            if not finish_normally else None
        return process, pids, supervisor

    def assert_tree_stopped_and_reservation_released(self, pids):
        for pid in pids:
            self.assert_process_stopped(pid)
        with (self.root / "reservation").open("w") as lock:
            fcntl.flock(lock, fcntl.LOCK_EX | fcntl.LOCK_NB)

    def test_normal_exit_cleans_descendants_and_releases_reservation(self):
        process, pids, _ = self.check_tree(finish_normally=True)
        self.assertEqual(process.wait(timeout=10), 0)
        self.assert_tree_stopped_and_reservation_released(pids)

    def test_termination_cleans_descendants_and_starts_no_next_check(self):
        # Each case has fresh files and a fresh gate process.
        for signum in (signal.SIGTERM, signal.SIGHUP, signal.SIGINT):
            with self.subTest(signum=signum):
                process, pids, supervisor = self.check_tree(next_check=True)
                process.send_signal(signum)
                self.assertEqual(process.wait(timeout=10), 1)
                self.assert_tree_stopped_and_reservation_released([*pids, supervisor])
                self.assertFalse((self.root / "next-check").exists())
                (self.root / "pids").unlink()
                (self.root / "ready").unlink()

    def test_sigkill_of_gate_group_cleans_descendants_and_releases_reservation(self):
        process, pids, supervisor = self.check_tree()
        os.killpg(process.pid, signal.SIGKILL)
        self.assertEqual(process.wait(timeout=10), -signal.SIGKILL)
        self.assert_tree_stopped_and_reservation_released([*pids, supervisor])

    def test_supervisor_termination_cleans_check_group(self):
        process, pids, supervisor = self.check_tree()
        os.kill(supervisor, signal.SIGTERM)
        self.assertEqual(process.wait(timeout=10), 1)
        self.assert_tree_stopped_and_reservation_released([*pids, supervisor])

    def test_empty_change_cannot_claim_green(self):
        with patch.object(gate, "changed_paths", return_value=[]), patch.object(sys, "argv", ["gate"]), \
                redirect_stderr(io.StringIO()):
            self.assertEqual(gate.main(), 1)


if __name__ == "__main__":
    unittest.main()
