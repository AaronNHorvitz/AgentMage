"""Developer fixture launcher refusals; these never launch a network namespace."""
from __future__ import annotations

import os
from pathlib import Path
import subprocess
import unittest


ROOT = Path(__file__).resolve().parents[1]
SCRIPT = ROOT / "scripts/research_transport_fixtures.sh"


class ResearchTransportFixtureTests(unittest.TestCase):
    def test_missing_single_job_or_test_thread_setting_refuses_before_launch(self) -> None:
        for key in ("CARGO_BUILD_JOBS", "RUST_TEST_THREADS"):
            for value in (None, "0", "2"):
                env = {**os.environ, "CARGO_BUILD_JOBS": "1", "RUST_TEST_THREADS": "1"}
                env.pop(key, None)
                if value is not None:
                    env[key] = value
                result = subprocess.run(
                    ["bash", str(SCRIPT), "/bin/true", str(ROOT)],
                    env=env, capture_output=True, text=True, timeout=5, check=False,
                )
                self.assertNotEqual(result.returncode, 0)
                self.assertNotIn("TLS_FIXTURE_DIRECTORY", result.stdout)
                self.assertNotIn("RESEARCH_TRANSPORT_FIXTURES_COMPLETE", result.stdout)

    def test_namespace_entry_refuses_current_namespace_before_any_setup(self) -> None:
        result = subprocess.run(
            ["bash", str(SCRIPT), "--namespace", "/bin/true",
             os.readlink("/proc/self/ns/net"), os.readlink("/proc/self/ns/mnt"), "dns"],
            capture_output=True, text=True, timeout=5, check=False,
        )
        self.assertNotEqual(result.returncode, 0)
        self.assertEqual(result.stdout, "")
        self.assertEqual(result.stderr, "")

    def test_fixture_inputs_are_closed_synthetic_values_and_contain_no_key(self) -> None:
        fixture = ROOT / "fixtures/public-research/namespace"
        self.assertEqual({path.name for path in fixture.iterdir()}, {"resolv.conf", "nsswitch.conf", "leaf.ext"})
        self.assertEqual((fixture / "resolv.conf").read_text(), "nameserver 127.0.0.1\noptions timeout:1 attempts:1 ndots:0\n")
        self.assertEqual((fixture / "nsswitch.conf").read_text(), "hosts: dns\n")
        self.assertEqual((fixture / "leaf.ext").read_text().splitlines(), [
            "basicConstraints=critical,CA:FALSE", "keyUsage=critical,digitalSignature,keyEncipherment",
            "extendedKeyUsage=serverAuth", "subjectAltName=DNS:docs.example.com",
        ])


if __name__ == "__main__":
    unittest.main()
