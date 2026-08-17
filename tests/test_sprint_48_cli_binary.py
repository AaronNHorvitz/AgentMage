from __future__ import annotations

import json
import subprocess
import unittest
from pathlib import Path


ROOT = Path(__file__).resolve().parents[1]
BINARY = ROOT / "target/debug/agent"
PUBLIC_BINARY = ROOT / "target/debug/agentmage"


class Sprint48CliBinaryTests(unittest.TestCase):
    @classmethod
    def setUpClass(cls) -> None:
        subprocess.run(
            [
                "cargo",
                "build",
                "-p",
                "agentmage-host",
                "--bins",
                "--locked",
            ],
            cwd=ROOT,
            check=True,
            capture_output=True,
            timeout=300,
        )

    def run_agentmage(self, *arguments: str) -> subprocess.CompletedProcess[str]:
        return subprocess.run(
            [str(PUBLIC_BINARY), *arguments],
            cwd=ROOT,
            check=False,
            capture_output=True,
            text=True,
            timeout=30,
        )

    def run_agent(self, *arguments: str) -> subprocess.CompletedProcess[str]:
        return subprocess.run(
            [str(BINARY), *arguments],
            cwd=ROOT,
            check=False,
            capture_output=True,
            text=True,
            timeout=30,
        )

    def test_help_version_and_completion_are_local_and_successful(self) -> None:
        help_result = self.run_agent("--help")
        self.assertEqual(help_result.returncode, 0)
        self.assertIn("AgentMage local CLI", help_result.stdout)
        self.assertEqual(help_result.stderr, "")

        version_result = self.run_agent("--version")
        self.assertEqual(version_result.returncode, 0)
        self.assertRegex(version_result.stdout, r"^agent [0-9]+\.[0-9]+\.[0-9]+\n$")

        completion_result = self.run_agent("completion", "bash")
        self.assertEqual(completion_result.returncode, 0)
        self.assertIn("complete -W", completion_result.stdout)

    def test_headless_command_fails_closed_without_product_transport(self) -> None:
        result = self.run_agent("--json", "--surface", "acp", "vault", "tasks")
        self.assertEqual(result.returncode, 5)
        self.assertEqual(result.stdout, "")
        error = json.loads(result.stderr)
        self.assertEqual(
            error,
            {
                "code": "client.transport.failed",
                "exit_code": 5,
                "kind": "error",
                "schema_version": 1,
            },
        )

    def test_public_coding_entry_is_selected_and_fails_closed_without_a_profile(self) -> None:
        help_result = self.run_agentmage("--help")
        self.assertEqual(help_result.returncode, 0)
        self.assertIn("Usage: agentmage", help_result.stdout)
        self.assertIn("\ncode\n", help_result.stdout)

        code_result = self.run_agentmage("code")
        self.assertEqual(code_result.returncode, 5)
        self.assertEqual(code_result.stdout, "")
        self.assertEqual(code_result.stderr, "client.transport.failed\n")

    def test_headless_human_output_and_invalid_input_fail_without_fallback(self) -> None:
        headless_human = self.run_agent("--surface", "json", "diagnostics")
        self.assertEqual(headless_human.returncode, 2)
        self.assertEqual(headless_human.stderr, "client.protocol.value_invalid\n")

        invalid = self.run_agent("--json", "unknown-command")
        self.assertEqual(invalid.returncode, 2)
        error = json.loads(invalid.stderr)
        self.assertEqual(error["code"], "client.protocol.value_invalid")
        self.assertEqual(error["exit_code"], 2)

    def test_failures_are_content_minimized(self) -> None:
        result = self.run_agent("--json", "chat", "synthetic-fixture-message")
        self.assertEqual(result.returncode, 5)
        encoded = result.stderr.lower()
        self.assertNotIn("synthetic-fixture-message", encoded)
        for prohibited in (
            "credential",
            "secret",
            "token",
            "repository_path",
            "remote_url",
            "prompt",
        ):
            self.assertNotIn(prohibited, encoded)


if __name__ == "__main__":
    unittest.main()
