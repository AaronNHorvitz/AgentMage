from __future__ import annotations

import json
import shutil
import subprocess
import tempfile
import unittest
from pathlib import Path

from scripts.macos_release_runner_source_contract import (
    EXPECTED_FAILURES,
    POLICY_KEYS,
    POLICY_PATH,
    ROOT,
    RUNNER_PATH,
    SOURCE_PATHS,
    build_report,
    validate_policy,
    validate_sources,
)


class MacOSReleaseRunnerSourceContractTests(unittest.TestCase):
    def copy_inputs(self, destination: Path) -> None:
        for relative in SOURCE_PATHS:
            target = destination / relative
            target.parent.mkdir(parents=True, exist_ok=True)
            shutil.copy2(ROOT / relative, target)

    def mutate(self, relative: str, old: str, new: str) -> list[str]:
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            self.copy_inputs(root)
            path = root / relative
            text = path.read_text(encoding="utf-8")
            self.assertIn(old, text)
            path.write_text(text.replace(old, new, 1), encoding="utf-8")
            return validate_sources(root)

    def test_canonical_sources_and_shell_syntax_pass(self) -> None:
        self.assertEqual(validate_sources(), [])
        result = subprocess.run(
            ["bash", "-n", str(ROOT / RUNNER_PATH)], capture_output=True, check=False
        )
        self.assertEqual(result.returncode, 0, result.stderr.decode())

    def test_report_preserves_every_external_execution_blocker(self) -> None:
        report = build_report()
        self.assertEqual(report["status"], "prepared-source-only-blocked-macos")
        self.assertEqual(report["contract"]["refusal_count"], len(EXPECTED_FAILURES))
        self.assertEqual(report["contract"]["policy_field_count"], len(POLICY_KEYS))
        self.assertTrue(all(value is False for value in report["execution"].values()))
        self.assertTrue(all(value is False for value in report["claims"].values()))
        self.assertEqual(len(report["remaining_blockers"]), 8)
        json.dumps(report)

    def test_fixture_is_closed_synthetic_and_never_release_approved(self) -> None:
        fixture = json.loads((ROOT / POLICY_PATH).read_text(encoding="utf-8"))
        self.assertEqual(validate_policy(fixture), [])
        for mutate in (
            lambda item: item.update(status="release-approved"),
            lambda item: item.update(release_claim="authorized-candidate"),
            lambda item: item.update(credential_values_present=True),
            lambda item: item.update(password="secret"),
            lambda item: item["entitlements"]["metal_inference_service"].append(
                "com.apple.security.network.client"
            ),
            lambda item: item["bundle_identifiers"].pop("vscode_bridge"),
        ):
            changed = json.loads(json.dumps(fixture))
            mutate(changed)
            self.assertNotEqual(validate_policy(changed), [])

    def test_credential_and_automatic_release_surfaces_are_rejected(self) -> None:
        for term in ("--password", "--apple-id", "curl ", "sudo ", "gh release"):
            failures = self.mutate(
                RUNNER_PATH,
                '# Manual AgentMage macOS release ceremony. Never source this file.',
                f'# Manual AgentMage macOS release ceremony. Never source this file.\n{term}',
            )
            self.assertTrue(any("prohibited surface" in item for item in failures))

    def test_sign_notarize_staple_gatekeeper_order_is_closed(self) -> None:
        failures = self.mutate(
            RUNNER_PATH,
            'run_step "staple" /usr/bin/xcrun stapler staple "$SIGNED_PACKAGE"',
            'run_step "staple" /bin/true',
        )
        self.assertTrue(any("stapler staple" in item for item in failures))
        failures = self.mutate(
            RUNNER_PATH,
            'run_step "gatekeeper-install" /usr/sbin/spctl --assess --type install --verbose=4 "$SIGNED_PACKAGE"',
            'run_step "gatekeeper-install" /bin/true',
        )
        self.assertTrue(any("gatekeeper-install" in item for item in failures))

    def test_lifecycle_requires_uninstall_and_prior_package_rollback(self) -> None:
        failures = self.mutate(
            RUNNER_PATH,
            '/bin/rm -rf -- "$INSTALLED_APP"',
            '/bin/true',
        )
        self.assertTrue(any("removal count" in item for item in failures))
        failures = self.mutate(
            RUNNER_PATH,
            'run_step "rollback-install" /usr/sbin/installer -pkg "$PREVIOUS_PACKAGE" -target CurrentUserHomeDirectory',
            'run_step "rollback-install" /bin/true',
        )
        self.assertTrue(any("rollback" in item for item in failures))

    def test_fixture_live_invocation_fails_before_any_release_action(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            evidence = Path(temporary) / "evidence"
            result = subprocess.run(
                [
                    "bash",
                    str(ROOT / RUNNER_PATH),
                    str((ROOT / POLICY_PATH).resolve()),
                    str(evidence.resolve()),
                ],
                cwd=ROOT,
                capture_output=True,
                check=False,
            )
            self.assertNotEqual(result.returncode, 0)
            self.assertIn(b"macos.release-runner.policy-not-release-approved", result.stderr)
            self.assertFalse(evidence.exists())


if __name__ == "__main__":
    unittest.main()
