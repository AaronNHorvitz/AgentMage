from __future__ import annotations

import copy
import io
import subprocess
import unittest
from contextlib import redirect_stderr
from pathlib import Path
from unittest.mock import patch

from scripts.product_ci import (
    ROOT,
    check_contract,
    read_json,
    run_lane,
    sanitize_output,
    validate_contract,
)


class ProductCiTests(unittest.TestCase):
    def setUp(self) -> None:
        self.policy = read_json(ROOT / "architecture/product-ci-policy.json")
        self.package = read_json(ROOT / "package.json")
        import tomllib

        self.rust = tomllib.loads((ROOT / "rust-toolchain.toml").read_text())
        self.workflow = (ROOT / ".github/workflows/product.yml").read_text()
        self.documentation = (
            ROOT / ".github/workflows/documentation.yml"
        ).read_text()
        self.macos = (ROOT / ".github/workflows/macos.yml").read_text()

    def validate(
        self,
        policy=None,
        workflow=None,
        package=None,
        rust=None,
        documentation=None,
        macos=None,
    ):
        return validate_contract(
            policy if policy is not None else self.policy,
            workflow if workflow is not None else self.workflow,
            package if package is not None else self.package,
            rust if rust is not None else self.rust,
            documentation if documentation is not None else self.documentation,
            macos if macos is not None else self.macos,
        )

    def test_checked_in_contract_is_valid(self) -> None:
        self.assertEqual(check_contract(), [])

    def test_rust_and_node_version_mismatches_are_rejected(self) -> None:
        rust = copy.deepcopy(self.rust)
        rust["toolchain"]["channel"] = "stable"
        package = copy.deepcopy(self.package)
        package["engines"]["node"] = ">=22 <23"
        self.assertTrue(self.validate(rust=rust))
        self.assertTrue(self.validate(package=package))

    def test_every_declared_command_failure_fails_its_lane(self) -> None:
        for lane, commands in self.policy["local_lanes"].items():
            for failed_index in range(len(commands)):
                results = [
                    subprocess.CompletedProcess(command, 0, "")
                    for command in commands[:failed_index]
                ]
                results.append(
                    subprocess.CompletedProcess(
                        commands[failed_index],
                        1,
                        f"failed under {ROOT} and {Path.home()}",
                    )
                )
                with patch("scripts.product_ci._run", side_effect=results):
                    with redirect_stderr(io.StringIO()) as stderr:
                        self.assertEqual(run_lane(lane), 1, (lane, failed_index))
                self.assertNotIn(str(ROOT), stderr.getvalue())
                self.assertNotIn(str(Path.home()), stderr.getvalue())

    def test_command_weakening_is_rejected(self) -> None:
        policy = copy.deepcopy(self.policy)
        policy["local_lanes"]["lint"][0].remove("--locked")
        self.assertTrue(self.validate(policy=policy))
        policy = copy.deepcopy(self.policy)
        policy["native_linux"]["inventory_command"].remove("--ignored")
        self.assertTrue(self.validate(policy=policy))

    def test_automatic_hosted_trigger_is_rejected(self) -> None:
        product = self.workflow.replace(
            "  workflow_dispatch:\n", "  workflow_dispatch:\n  push:\n", 1
        )
        macos = self.macos.replace(
            "  workflow_dispatch:\n", "  workflow_dispatch:\n  pull_request:\n", 1
        )
        self.assertTrue(self.validate(workflow=product))
        self.assertTrue(self.validate(macos=macos))

    def test_enabled_or_changed_sentinel_is_rejected(self) -> None:
        enabled = self.workflow.replace("    if: ${{ false }}\n", "", 1)
        renamed = self.workflow.replace(
            "  local_execution_only:\n", "  hosted_product:\n", 1
        )
        action = self.documentation.replace(
            "      - name: Explain local execution authority\n",
            "      - uses: actions/checkout@"
            "11d5960a326750d5838078e36cf38b85af677262\n",
            1,
        )
        self.assertTrue(self.validate(workflow=enabled))
        self.assertTrue(self.validate(workflow=renamed))
        self.assertTrue(self.validate(documentation=action))

    def test_macos_budget_and_apple_silicon_guards_are_required(self) -> None:
        no_budget = self.macos.replace(
            "    if: ${{ inputs.confirm_budget }}\n", "", 1
        )
        wrong_runner = self.macos.replace("macos-15", "ubuntu-24.04", 1)
        wrong_arch = self.macos.replace('"arm64"', '"x86_64"', 1)
        self.assertTrue(self.validate(macos=no_budget))
        self.assertTrue(self.validate(macos=wrong_runner))
        self.assertTrue(self.validate(macos=wrong_arch))

    def test_macos_secrets_and_unpinned_actions_are_rejected(self) -> None:
        secret = self.macos + "\n# ${{ secrets.APPLE_SIGNING_KEY }}\n"
        unpinned = self.macos.replace(
            "actions/checkout@11d5960a326750d5838078e36cf38b85af677262",
            "actions/checkout@v4",
            1,
        )
        self.assertTrue(self.validate(macos=secret))
        self.assertTrue(self.validate(macos=unpinned))

    def test_macos_result_retention_guards_are_required(self) -> None:
        no_record = self.macos.replace(
            "          if-no-files-found: error\n", "", 1
        )
        no_always = self.macos.replace(
            "        if: ${{ always() && steps.record.outcome == 'success' }}\n",
            "",
            1,
        )
        unpinned_upload = self.macos.replace(
            "actions/upload-artifact@043fb46d1a93c77aae656e7c1c64a875d1fc6a0a",
            "actions/upload-artifact@v7",
            1,
        )
        self.assertTrue(self.validate(macos=no_record))
        self.assertTrue(self.validate(macos=no_always))
        self.assertTrue(self.validate(macos=unpinned_upload))

    def test_failure_output_removes_private_host_paths(self) -> None:
        private = f"failure at {ROOT}/src and {Path.home()}/secret"
        sanitized = sanitize_output(private)
        self.assertNotIn(str(ROOT), sanitized)
        self.assertNotIn(str(Path.home()), sanitized)
        self.assertIn("<REPOSITORY>", sanitized)

    def test_platform_execution_dispositions_are_explicit(self) -> None:
        native = self.policy["native_linux"]
        self.assertEqual(native["execution_venue"], "local-disposable-kvm")
        self.assertEqual(
            native["disposition"], "complete-local-vm-execution"
        )
        self.assertEqual(len(native["expected_tests"]), 19)
        self.assertEqual(
            self.policy["native_windows"]["target"], "windows-11-x86_64"
        )
        self.assertEqual(
            self.policy["github_hosted"]["allowed_platform"], "macos-arm64"
        )


if __name__ == "__main__":
    unittest.main()
