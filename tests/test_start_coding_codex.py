"""Exercise launcher boundaries without starting Codex or touching real state."""

import fcntl
import json
import os
from pathlib import Path
import shutil
import subprocess
import tempfile
import unittest


SOURCE = Path(__file__).resolve().parents[1] / "scripts/start_coding_codex.sh"


@unittest.skipUnless(shutil.which("flock"), "Linux flock is required")
class CodingCodexLauncherTests(unittest.TestCase):
    def setUp(self):
        self.temp = tempfile.TemporaryDirectory()
        self.addCleanup(self.temp.cleanup)
        root = Path(self.temp.name)
        self.repo = root / "repo with spaces"
        self.script = self.repo / "scripts/start_coding_codex.sh"
        self.script.parent.mkdir(parents=True)
        shutil.copyfile(SOURCE, self.script)
        brief = self.repo / "docs/guides/codex-coding-implementation-brief.md"
        brief.parent.mkdir(parents=True)
        brief.write_text("Scoped implementation prompt\n", encoding="utf-8")
        self.home = root / "home"
        self.state = self.home / ".local/state/agentmage-codex-coding"
        self.state.mkdir(parents=True)
        self.stop = self.home / ".local/share/agentmage-run/STOP-CLAUDE"
        self.stop.parent.mkdir(parents=True)
        self.stop.write_text("Operator stopped the previous worker.\n", encoding="utf-8")
        self.bin = root / "bin"
        self.bin.mkdir()
        self.stub("git", '#!/bin/sh\nprintf "%s\\n" "${TEST_BRANCH:-demo/fedora-local-docs}"\n')
        self.stub("tmux", '#!/bin/sh\nexit "${TEST_OLD_SESSION_STATUS:-1}"\n')
        self.stub(
            "codex",
            "#!/usr/bin/env python3\n"
            "import json, os, sys\n"
            "print('CODEX_ARGS=' + json.dumps(sys.argv[1:]))\n"
            "sys.exit(int(os.environ.get('TEST_CODEX_STATUS', '0')))\n",
        )
        self.env = dict(os.environ, HOME=str(self.home), PATH=f"{self.bin}:{os.environ['PATH']}")

    def stub(self, name, contents):
        path = self.bin / name
        path.write_text(contents, encoding="utf-8")
        path.chmod(0o700)

    def run_launcher(self, **env):
        return subprocess.run(
            ["bash", str(self.script)],
            env=dict(self.env, **env),
            capture_output=True,
            text=True,
            timeout=10,
            check=False,
        )

    def assert_refused(self, result):
        self.assertNotEqual(result.returncode, 0)
        self.assertNotIn("CODEX_ARGS=", result.stdout)
        self.assertTrue(self.stop.exists())

    def test_fresh_launch_archives_old_marker_and_sets_explicit_flags(self):
        result = self.run_launcher()
        self.assertEqual(result.returncode, 0, result.stderr)
        args = json.loads(next(line.removeprefix("CODEX_ARGS=") for line in result.stdout.splitlines()
                               if line.startswith("CODEX_ARGS=")))
        self.assertEqual(args, [
            "--cd", str(self.repo), "--model", "gpt-5.6-sol",
            "--config", 'model_reasoning_effort="high"',
            "--ask-for-approval", "never", "--sandbox", "danger-full-access",
            "--search", "--no-alt-screen", "Scoped implementation prompt",
        ])
        self.assertFalse(self.stop.exists())
        archives = list(self.stop.parent.glob("STOP-CLAUDE.before-codex.*"))
        self.assertEqual(len(archives), 1)
        self.assertEqual(archives[0].read_text(), "Operator stopped the previous worker.\n")

    def test_active_codex_stop_marker_is_preserved(self):
        marker = self.state / "STOP-CODEX"
        marker.touch()
        self.assert_refused(self.run_launcher())
        self.assertTrue(marker.exists())

    def test_unexpected_branch_is_refused(self):
        self.assert_refused(self.run_launcher(TEST_BRANCH="main"))

    def test_old_claude_session_is_not_restarted_or_killed(self):
        self.assert_refused(self.run_launcher(TEST_OLD_SESSION_STATUS="0"))

    def test_second_writer_is_refused(self):
        with (self.state / "worker.lock").open("w") as lock:
            fcntl.flock(lock, fcntl.LOCK_EX | fcntl.LOCK_NB)
            self.assert_refused(self.run_launcher())

    def test_codex_failure_is_reported_without_automatic_relaunch(self):
        result = self.run_launcher(TEST_CODEX_STATUS="7")
        self.assertEqual(result.returncode, 7)
        self.assertEqual(result.stdout.count("CODEX_ARGS="), 1)


if __name__ == "__main__":
    unittest.main()
